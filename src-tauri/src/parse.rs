//! .als parsing: gunzip + streaming XML, extracting SampleRef occurrences with
//! their track + group context. Read-only against the source file.

use crate::error::{ScanError, ScanResult};
use crate::readonly::ReadOnlyProject;
use flate2::read::GzDecoder;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleOccurrence {
    pub path: PathBuf,
    pub filename: String,
    pub declared_size: Option<u64>,
    pub declared_crc: Option<u64>,
    pub original_path: Option<PathBuf>,
    pub track_name: Option<String>,
    pub group_path: Vec<String>,
    pub project_name: String,
    pub project_root: PathBuf,
    pub context: SampleContext,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SampleContext {
    AudioClip,
    TakeLane,
    Sampler,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ProjectAnalysis {
    pub project_name: String,
    pub project_root: PathBuf,
    pub als_path: PathBuf,
    pub samples: Vec<SampleOccurrence>,
}

pub fn parse_project(project: &ReadOnlyProject) -> ScanResult<ProjectAnalysis> {
    let project_name = project
        .als_path()
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let file = project.open_als()?;
    let reader = BufReader::new(GzDecoder::new(file));
    let mut xml = Reader::from_reader(reader);
    xml.config_mut().trim_text(true);

    let mut buf = Vec::with_capacity(8 * 1024);
    let mut state = ParseState::new(project_name.clone(), project.root().to_path_buf());

    loop {
        match xml.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => state.on_start(&e)?,
            Ok(Event::Empty(e)) => state.on_empty(&e)?,
            Ok(Event::End(e)) => state.on_end(&e)?,
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => {
                return Err(ScanError::Xml {
                    path: project.als_path().to_path_buf(),
                    message: err.to_string(),
                });
            }
        }
        buf.clear();
    }

    state.finalize(project.als_path());

    Ok(ProjectAnalysis {
        project_name,
        project_root: project.root().to_path_buf(),
        als_path: project.als_path().to_path_buf(),
        samples: state.samples,
    })
}

#[derive(Debug, Clone)]
struct TrackFrame {
    id: String,
    #[allow(dead_code)]
    kind: TrackKind,
    name: Option<String>,
    parent_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackKind {
    Audio,
    Midi,
    Group,
    Return,
}

struct ParseState {
    project_name: String,
    project_root: PathBuf,
    tracks: HashMap<String, TrackFrame>,
    track_stack: Vec<String>,
    context_stack: Vec<SampleContext>,
    awaiting_track_name_for: Option<String>,
    in_name_block: bool,
    in_sample_ref: bool,
    sample_depth: usize,
    in_source_context: usize,
    current_file_ref_path: Option<PathBuf>,
    current_file_ref_relative: Option<String>,
    current_file_ref_size: Option<u64>,
    current_file_ref_crc: Option<u64>,
    current_source_path: Option<PathBuf>,
    samples: Vec<SampleOccurrence>,
}

impl ParseState {
    fn new(project_name: String, project_root: PathBuf) -> Self {
        Self {
            project_name,
            project_root,
            tracks: HashMap::new(),
            track_stack: Vec::new(),
            context_stack: Vec::new(),
            awaiting_track_name_for: None,
            in_name_block: false,
            in_sample_ref: false,
            sample_depth: 0,
            in_source_context: 0,
            current_file_ref_path: None,
            current_file_ref_relative: None,
            current_file_ref_size: None,
            current_file_ref_crc: None,
            current_source_path: None,
            samples: Vec::new(),
        }
    }

    fn finalize(&mut self, _als_path: &Path) {
        let resolved: Vec<(Vec<String>, Option<String>)> = self
            .samples
            .iter()
            .map(|s| {
                if let Some(track_id) = Self::track_for_sample_id(s) {
                    let path = self.compute_group_path(&track_id);
                    let track_name = self.tracks.get(&track_id).and_then(|t| t.name.clone());
                    (path, track_name)
                } else {
                    (s.group_path.clone(), s.track_name.clone())
                }
            })
            .collect();
        for (s, (path, name)) in self.samples.iter_mut().zip(resolved.into_iter()) {
            s.group_path = path;
            if s.track_name.is_none() {
                s.track_name = name;
            }
        }
    }

    fn track_for_sample_id(s: &SampleOccurrence) -> Option<String> {
        s.group_path
            .first()
            .and_then(|s| s.strip_prefix("__id:"))
            .map(|s| s.to_string())
    }

    fn compute_group_path(&self, leaf_id: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut cur = Some(leaf_id.to_string());
        let mut guard = 0;
        while let Some(id) = cur {
            guard += 1;
            if guard > 64 {
                break;
            }
            match self.tracks.get(&id) {
                Some(frame) => {
                    if let Some(name) = frame.name.clone() {
                        chain.push(name);
                    } else {
                        chain.push(format!("Track {}", frame.id));
                    }
                    cur = frame.parent_id.clone();
                }
                None => break,
            }
        }
        chain.reverse();
        chain
    }

    fn current_track_id(&self) -> Option<&String> {
        self.track_stack.last()
    }

    fn on_start(&mut self, e: &quick_xml::events::BytesStart) -> ScanResult<()> {
        let local = local_name(e);
        match local.as_str() {
            "AudioTrack" | "MidiTrack" | "GroupTrack" | "ReturnTrack" => {
                let kind = match local.as_str() {
                    "AudioTrack" => TrackKind::Audio,
                    "MidiTrack" => TrackKind::Midi,
                    "GroupTrack" => TrackKind::Group,
                    _ => TrackKind::Return,
                };
                let id = attr(e, "Id").unwrap_or_default();
                if !id.is_empty() {
                    let parent_id = self.track_stack.last().cloned();
                    self.tracks.insert(
                        id.clone(),
                        TrackFrame {
                            id: id.clone(),
                            kind,
                            name: None,
                            parent_id,
                        },
                    );
                    self.track_stack.push(id.clone());
                    self.awaiting_track_name_for = Some(id);
                }
            }
            "Name" => {
                if self.awaiting_track_name_for.is_some() {
                    self.in_name_block = true;
                }
            }
            "AudioClip" => self.context_stack.push(SampleContext::AudioClip),
            "TakeLane" => self.context_stack.push(SampleContext::TakeLane),
            "OriginalSimpler" => self.context_stack.push(SampleContext::Sampler),
            "SampleRef" => {
                self.in_sample_ref = true;
                self.sample_depth = 1;
                self.current_file_ref_path = None;
                self.current_file_ref_relative = None;
                self.current_file_ref_size = None;
                self.current_file_ref_crc = None;
                self.current_source_path = None;
                self.in_source_context = 0;
            }
            "SourceContext" if self.in_sample_ref => {
                self.in_source_context += 1;
            }
            _ => {
                if self.in_sample_ref {
                    self.sample_depth += 1;
                }
            }
        }
        Ok(())
    }

    fn on_empty(&mut self, e: &quick_xml::events::BytesStart) -> ScanResult<()> {
        let local = local_name(e);
        match local.as_str() {
            "EffectiveName" => {
                if let (Some(id), Some(name)) =
                    (self.awaiting_track_name_for.clone(), attr(e, "Value"))
                {
                    if self.in_name_block {
                        if let Some(t) = self.tracks.get_mut(&id) {
                            t.name = Some(name);
                        }
                    }
                }
            }
            "TrackGroupId" => {
                if let (Some(track_id), Some(val)) =
                    (self.awaiting_track_name_for.clone(), attr(e, "Value"))
                {
                    if let Some(t) = self.tracks.get_mut(&track_id) {
                        if val.trim() != "-1" && !val.is_empty() {
                            t.parent_id = Some(val);
                        } else {
                            t.parent_id = None;
                        }
                    }
                }
            }
            "Path" if self.in_sample_ref => {
                if let Some(v) = attr(e, "Value") {
                    if v.is_empty() {
                    } else if self.in_source_context > 0 {
                        if self.current_source_path.is_none() {
                            self.current_source_path = Some(PathBuf::from(v));
                        }
                    } else if self.current_file_ref_path.is_none() {
                        self.current_file_ref_path = Some(PathBuf::from(v));
                    }
                }
            }
            "RelativePath" if self.in_sample_ref && self.in_source_context == 0 => {
                if let Some(v) = attr(e, "Value") {
                    if !v.is_empty() {
                        self.current_file_ref_relative = Some(v);
                    }
                }
            }
            "OriginalFileSize" if self.in_sample_ref && self.in_source_context == 0 => {
                if let Some(v) = attr(e, "Value") {
                    if let Ok(n) = v.parse::<u64>() {
                        self.current_file_ref_size = Some(n);
                    }
                }
            }
            "OriginalCrc" if self.in_sample_ref && self.in_source_context == 0 => {
                if let Some(v) = attr(e, "Value") {
                    if let Ok(n) = v.parse::<u64>() {
                        self.current_file_ref_crc = Some(n);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn on_end(&mut self, e: &quick_xml::events::BytesEnd) -> ScanResult<()> {
        let local = local_name_end(e);
        match local.as_str() {
            "AudioTrack" | "MidiTrack" | "GroupTrack" | "ReturnTrack" => {
                self.track_stack.pop();
                self.awaiting_track_name_for = self.track_stack.last().cloned();
                self.in_name_block = false;
            }
            "Name" => self.in_name_block = false,
            "AudioClip" => {
                if matches!(self.context_stack.last(), Some(SampleContext::AudioClip)) {
                    self.context_stack.pop();
                }
            }
            "TakeLane" => {
                if matches!(self.context_stack.last(), Some(SampleContext::TakeLane)) {
                    self.context_stack.pop();
                }
            }
            "OriginalSimpler" => {
                if matches!(self.context_stack.last(), Some(SampleContext::Sampler)) {
                    self.context_stack.pop();
                }
            }
            "SourceContext" if self.in_sample_ref => {
                if self.in_source_context > 0 {
                    self.in_source_context -= 1;
                }
            }
            "SampleRef" if self.in_sample_ref => self.flush_sample_ref(),
            _ => {
                if self.in_sample_ref && self.sample_depth > 0 {
                    self.sample_depth -= 1;
                }
            }
        }
        Ok(())
    }

    fn flush_sample_ref(&mut self) {
        let resolved_path = self.resolved_sample_path();
        if let Some(path) = resolved_path {
            let filename = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            // Prefer the more specific containers (TakeLane / Sampler) over
            // the inner AudioClip when both appear in the stack.
            let context = self
                .context_stack
                .iter()
                .rev()
                .find(|c| matches!(c, SampleContext::TakeLane | SampleContext::Sampler))
                .copied()
                .unwrap_or_else(|| {
                    *self.context_stack.last().unwrap_or(&SampleContext::Unknown)
                });
            let group_path_marker = self
                .current_track_id()
                .map(|t| vec![format!("__id:{}", t)])
                .unwrap_or_default();
            self.samples.push(SampleOccurrence {
                path,
                filename,
                declared_size: self.current_file_ref_size,
                declared_crc: self.current_file_ref_crc,
                original_path: self.current_source_path.clone(),
                track_name: None,
                group_path: group_path_marker,
                project_name: self.project_name.clone(),
                project_root: self.project_root.clone(),
                context,
            });
        }
        self.in_sample_ref = false;
        self.sample_depth = 0;
        self.in_source_context = 0;
    }

    fn resolved_sample_path(&self) -> Option<PathBuf> {
        if let Some(p) = &self.current_file_ref_path {
            return Some(p.clone());
        }
        if let Some(rel) = &self.current_file_ref_relative {
            return Some(self.project_root.join(rel));
        }
        None
    }
}

fn local_name(e: &quick_xml::events::BytesStart) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}
fn local_name_end(e: &quick_xml::events::BytesEnd) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}
fn attr(e: &quick_xml::events::BytesStart, key: &str) -> Option<String> {
    for a in e.attributes().with_checks(false).flatten() {
        let k = String::from_utf8_lossy(a.key.as_ref()).into_owned();
        if k == key {
            let raw = String::from_utf8_lossy(&a.value);
            return Some(
                quick_xml::escape::unescape(&raw)
                    .map(|s| s.into_owned())
                    .unwrap_or_else(|_| raw.into_owned()),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    use tempfile::tempdir;

    fn write_gz_als(path: &Path, xml: &str) {
        let f = std::fs::File::create(path).unwrap();
        let mut gz = GzEncoder::new(f, Compression::default());
        gz.write_all(xml.as_bytes()).unwrap();
        gz.finish().unwrap();
    }

    const MINIMAL_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Ableton MajorVersion="5" Creator="Ableton Live 12.0.0">
  <LiveSet>
    <Tracks>
      <GroupTrack Id="100">
        <Name><EffectiveName Value="DRUMS"/></Name>
        <TrackGroupId Value="-1"/>
      </GroupTrack>
      <AudioTrack Id="101">
        <Name><EffectiveName Value="CLAP"/></Name>
        <TrackGroupId Value="100"/>
        <DeviceChain>
          <MainSequencer>
            <ClipSlotList>
              <ClipSlot>
                <AudioClip>
                  <SampleRef>
                    <FileRef>
                      <Path Value="/tmp/clap.wav"/>
                      <OriginalFileSize Value="12345"/>
                      <OriginalCrc Value="42"/>
                    </FileRef>
                  </SampleRef>
                </AudioClip>
              </ClipSlot>
            </ClipSlotList>
          </MainSequencer>
        </DeviceChain>
      </AudioTrack>
    </Tracks>
  </LiveSet>
</Ableton>"#;

    #[test]
    fn parses_basic_sample_ref_with_group_path() {
        let tmp = tempdir().unwrap();
        let proj = tmp.path().join("MyProj");
        std::fs::create_dir_all(&proj).unwrap();
        let als = proj.join("MyProj.als");
        write_gz_als(&als, MINIMAL_XML);
        let ro = ReadOnlyProject::new(&proj, &als);
        let result = parse_project(&ro).unwrap();
        assert_eq!(result.samples.len(), 1);
        let s = &result.samples[0];
        assert_eq!(s.filename, "clap.wav");
        assert_eq!(s.declared_size, Some(12345));
        assert_eq!(s.group_path, vec!["DRUMS".to_string(), "CLAP".to_string()]);
        assert_eq!(s.context, SampleContext::AudioClip);
    }
}
