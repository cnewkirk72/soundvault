//! Read-only filesystem safety test.
//!
//! Builds a synthetic Ableton project on disk (.als with valid gzipped XML +
//! a Samples/ folder with real sample files), runs the full discover → parse →
//! classify → dedup → copy pipeline, and asserts that **no file inside the
//! project root was modified** by snapshotting:
//!   - File contents (Blake3 hash)
//!   - File modification time (mtime)
//!
//! Any mismatch fails the test. This is the spec's "Safety" guarantee in §10.

use blake3::Hasher;
use filetime::FileTime;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::tempdir;

use soundvault_lib::cancel::CancellationToken;
use soundvault_lib::classify::{classify, ManualKeywords, MatchMode};
use soundvault_lib::copy::copy_samples;
use soundvault_lib::dedup::{dedup_and_count, filter_artifact_folders, ClassifiedOccurrence, FilterFlags};
use soundvault_lib::discover::discover_projects;
use soundvault_lib::parse::parse_project;
use soundvault_lib::rank::{rank, Tiebreaker};
use soundvault_lib::readonly::ReadOnlyProject;
use soundvault_lib::taxonomy::Taxonomy;

const PROJECT_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Ableton MajorVersion="5" Creator="Ableton Live 12.0.0">
  <LiveSet>
    <Tracks>
      <GroupTrack Id="10">
        <Name><EffectiveName Value="DRUMS"/></Name>
        <TrackGroupId Value="-1"/>
      </GroupTrack>
      <AudioTrack Id="11">
        <Name><EffectiveName Value="KICK"/></Name>
        <TrackGroupId Value="10"/>
        <DeviceChain>
          <MainSequencer>
            <ClipSlotList>
              <ClipSlot>
                <AudioClip>
                  <SampleRef>
                    <FileRef>
                      <Path Value="__KICK_PATH__"/>
                      <OriginalFileSize Value="64"/>
                    </FileRef>
                  </SampleRef>
                </AudioClip>
              </ClipSlot>
            </ClipSlotList>
          </MainSequencer>
        </DeviceChain>
      </AudioTrack>
      <AudioTrack Id="12">
        <Name><EffectiveName Value="SNARE"/></Name>
        <TrackGroupId Value="10"/>
        <DeviceChain>
          <MainSequencer>
            <ClipSlotList>
              <ClipSlot>
                <AudioClip>
                  <SampleRef>
                    <FileRef>
                      <Path Value="__SNARE_PATH__"/>
                      <OriginalFileSize Value="64"/>
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

fn write_gz(path: &Path, xml: &str) {
    let f = fs::File::create(path).unwrap();
    let mut enc = GzEncoder::new(f, Compression::default());
    enc.write_all(xml.as_bytes()).unwrap();
    enc.finish().unwrap();
}

fn make_project(root: &Path, name: &str) -> (PathBuf, PathBuf) {
    let project_dir = root.join(name);
    fs::create_dir_all(project_dir.join("Samples/Imported")).unwrap();
    fs::create_dir_all(project_dir.join("Backup")).unwrap();
    fs::create_dir_all(project_dir.join("Ableton Project Info")).unwrap();

    let kick = project_dir.join("Samples/Imported/My_kick_01.wav");
    let snare = project_dir.join("Samples/Imported/My_snare_01.wav");
    fs::write(&kick, vec![0u8; 64]).unwrap();
    fs::write(&snare, vec![1u8; 64]).unwrap();

    let xml = PROJECT_XML
        .replace("__KICK_PATH__", &kick.to_string_lossy())
        .replace("__SNARE_PATH__", &snare.to_string_lossy());
    let als = project_dir.join(format!("{}.als", name));
    write_gz(&als, &xml);

    // A bogus older snapshot in Backup — we must NOT read this.
    write_gz(&project_dir.join("Backup/old.als"), &xml);

    (project_dir, als)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileSnapshot {
    hash: String,
    mtime_secs: i64,
    mtime_nanos: u32,
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, FileSnapshot> {
    let mut out = BTreeMap::new();
    for entry in walkdir::WalkDir::new(root) {
        let entry = entry.unwrap();
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path().to_path_buf();
        let mut hasher = Hasher::new();
        let mut f = fs::File::open(&path).unwrap();
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = f.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        let meta = fs::metadata(&path).unwrap();
        let mt = FileTime::from_last_modification_time(&meta);
        out.insert(
            path,
            FileSnapshot {
                hash: hasher.finalize().to_hex().to_string(),
                mtime_secs: mt.seconds(),
                mtime_nanos: mt.nanoseconds(),
            },
        );
    }
    out
}

#[test]
fn scanning_a_project_makes_zero_writes() {
    let workspace = tempdir().unwrap();
    let projects_root = workspace.path().join("projects");
    let output_root = workspace.path().join("output");
    fs::create_dir_all(&projects_root).unwrap();

    let (proj_a, _) = make_project(&projects_root, "Track A");
    let (proj_b, _) = make_project(&projects_root, "Track B");

    // Snapshot every file inside both project roots BEFORE the scan.
    let before_a = snapshot(&proj_a);
    let before_b = snapshot(&proj_b);

    // Run the pipeline manually.
    let cancel = Arc::new(CancellationToken::new());
    let taxonomy = Taxonomy::default_bundled().unwrap();
    let categories = taxonomy.flatten();
    let selected: Vec<String> = categories.iter().map(|c| c.path.clone()).collect();

    let projects = discover_projects(&projects_root).unwrap();
    assert_eq!(projects.len(), 2);

    let mut classified: Vec<ClassifiedOccurrence> = Vec::new();
    for p in &projects {
        let ro = ReadOnlyProject::new(&p.project_dir, &p.als_path);
        let analysis = parse_project(&ro).unwrap();
        for occ in analysis.samples {
            if let Some(cat) = classify(
                &occ,
                &selected,
                &categories,
                MatchMode::AutoDetect,
                &ManualKeywords::default(),
            ) {
                classified.push(ClassifiedOccurrence {
                    occurrence: occ,
                    category: cat,
                });
            }
        }
    }

    let flags = FilterFlags::default();
    let filtered = filter_artifact_folders(classified, &flags);
    let uniques = dedup_and_count(filtered, &flags, &cancel, |_, _| {});

    let cat_index: BTreeMap<String, (String, Vec<String>)> = categories
        .iter()
        .map(|c| (c.path.clone(), (c.name.clone(), c.components.clone())))
        .collect();
    let report = rank(uniques, &selected, 50, Tiebreaker::ProjectThenClip, &cat_index);
    let _ = copy_samples(&report, &output_root, &cancel, |_, _, _| {}).unwrap();

    // Snapshot AFTER the scan.
    let after_a = snapshot(&proj_a);
    let after_b = snapshot(&proj_b);

    // The two snapshots must be identical, key-for-key.
    assert_eq!(
        before_a, after_a,
        "Project A was modified during the scan — read-only invariant broken!"
    );
    assert_eq!(
        before_b, after_b,
        "Project B was modified during the scan — read-only invariant broken!"
    );

    // And the output folder must contain the copied samples (sanity check that
    // copying actually happened — we're not measuring read-only by accident).
    let copies: Vec<PathBuf> = walkdir::WalkDir::new(&output_root)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();
    let wavs: Vec<_> = copies
        .iter()
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("wav"))
                .unwrap_or(false)
        })
        .collect();
    assert!(!wavs.is_empty(), "no samples were copied to output");
    let manifest = copies
        .iter()
        .find(|p| p.file_name().and_then(|s| s.to_str()) == Some("manifest.json"));
    // Manifest isn't written by copy_samples — it's written from commands.rs.
    // Don't assert here.
    let _ = manifest;
}

#[test]
fn output_inside_input_is_rejected_at_validate_time() {
    use soundvault_lib::commands::{ScanConfig, ScanSource};
    use soundvault_lib::error::ScanError;

    let tmp = tempdir().unwrap();
    let projects_root = tmp.path().join("projects");
    fs::create_dir_all(&projects_root).unwrap();
    let (_proj, _) = make_project(&projects_root, "Track A");

    let bad_output = projects_root.join("Track A").join("oops");
    let cfg = ScanConfig {
        source: ScanSource::Root(projects_root.clone()),
        output_folder: bad_output,
        selected_categories: vec!["Drums".to_string()],
        top_n: 25,
        match_mode: MatchMode::AutoDetect,
        manual_keywords: ManualKeywords::default(),
        include_freeze: false,
        include_processed: false,
        include_recorded: false,
        include_missing: false,
        tiebreaker: Tiebreaker::ProjectThenClip,
    };
    let err = soundvault_lib::commands::validate_config(cfg).unwrap_err();
    matches!(err, ScanError::OutputInsideInput(_));
}
