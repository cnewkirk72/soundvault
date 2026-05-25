//! End-to-end parse tests against synthetic .als fixtures covering the three
//! SampleRef contexts: AudioClip, TakeLane, and Sampler.

use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

use soundvault_lib::parse::{parse_project, SampleContext};
use soundvault_lib::readonly::ReadOnlyProject;

fn write_gz_als(path: &Path, xml: &str) {
    let f = fs::File::create(path).unwrap();
    let mut enc = GzEncoder::new(f, Compression::default());
    enc.write_all(xml.as_bytes()).unwrap();
    enc.finish().unwrap();
}

const NESTED_GROUPS_XML: &str = r#"<?xml version="1.0"?>
<Ableton>
  <LiveSet>
    <Tracks>
      <GroupTrack Id="100"><Name><EffectiveName Value="DRUMS"/></Name><TrackGroupId Value="-1"/></GroupTrack>
      <GroupTrack Id="110"><Name><EffectiveName Value="Snares, Claps, &amp; Rims"/></Name><TrackGroupId Value="100"/></GroupTrack>
      <GroupTrack Id="111"><Name><EffectiveName Value="CLAP"/></Name><TrackGroupId Value="110"/></GroupTrack>
      <AudioTrack Id="200">
        <Name><EffectiveName Value="my clap track"/></Name>
        <TrackGroupId Value="111"/>
        <AudioClip>
          <SampleRef>
            <FileRef>
              <Path Value="/x/clap.wav"/>
              <OriginalFileSize Value="100"/>
            </FileRef>
          </SampleRef>
        </AudioClip>
      </AudioTrack>
    </Tracks>
  </LiveSet>
</Ableton>"#;

const TAKE_LANE_XML: &str = r#"<?xml version="1.0"?>
<Ableton>
  <LiveSet><Tracks>
    <AudioTrack Id="1">
      <Name><EffectiveName Value="Vox"/></Name>
      <TrackGroupId Value="-1"/>
      <TakeLanes>
        <TakeLane>
          <AudioClip>
            <SampleRef>
              <FileRef>
                <Path Value="/x/take1.wav"/>
              </FileRef>
            </SampleRef>
          </AudioClip>
        </TakeLane>
      </TakeLanes>
    </AudioTrack>
  </Tracks></LiveSet>
</Ableton>"#;

const SAMPLER_XML: &str = r#"<?xml version="1.0"?>
<Ableton>
  <LiveSet><Tracks>
    <MidiTrack Id="1">
      <Name><EffectiveName Value="Sampler"/></Name>
      <TrackGroupId Value="-1"/>
      <DeviceChain>
        <OriginalSimpler>
          <Player>
            <MultiSampleMap>
              <SampleParts>
                <MultiSamplePart>
                  <SampleRef>
                    <FileRef>
                      <Path Value="/x/sampler.wav"/>
                    </FileRef>
                  </SampleRef>
                </MultiSamplePart>
              </SampleParts>
            </MultiSampleMap>
          </Player>
        </OriginalSimpler>
      </DeviceChain>
    </MidiTrack>
  </Tracks></LiveSet>
</Ableton>"#;

fn write_project(tmp: &Path, name: &str, xml: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let dir = tmp.join(name);
    fs::create_dir_all(&dir).unwrap();
    let als = dir.join(format!("{name}.als"));
    write_gz_als(&als, xml);
    (dir, als)
}

#[test]
fn nested_groups_resolve_three_deep() {
    let tmp = tempdir().unwrap();
    let (dir, als) = write_project(tmp.path(), "Nested", NESTED_GROUPS_XML);
    let ro = ReadOnlyProject::new(dir, als);
    let r = parse_project(&ro).unwrap();
    assert_eq!(r.samples.len(), 1);
    let s = &r.samples[0];
    assert_eq!(
        s.group_path,
        vec!["DRUMS".to_string(), "Snares, Claps, & Rims".to_string(), "CLAP".to_string(), "my clap track".to_string()]
    );
    assert_eq!(s.context, SampleContext::AudioClip);
}

#[test]
fn detects_take_lane_context() {
    let tmp = tempdir().unwrap();
    let (dir, als) = write_project(tmp.path(), "TakeLane", TAKE_LANE_XML);
    let ro = ReadOnlyProject::new(dir, als);
    let r = parse_project(&ro).unwrap();
    assert_eq!(r.samples.len(), 1);
    assert_eq!(r.samples[0].context, SampleContext::TakeLane);
}

#[test]
fn detects_sampler_context() {
    let tmp = tempdir().unwrap();
    let (dir, als) = write_project(tmp.path(), "Sampler", SAMPLER_XML);
    let ro = ReadOnlyProject::new(dir, als);
    let r = parse_project(&ro).unwrap();
    assert_eq!(r.samples.len(), 1);
    assert_eq!(r.samples[0].context, SampleContext::Sampler);
}
