//! Dedup tests — verify copies at different paths collapse into one UniqueSample.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::tempdir;

use soundvault_lib::cancel::CancellationToken;
use soundvault_lib::dedup::{
    dedup_and_count, filter_artifact_folders, ClassifiedOccurrence, FilterFlags,
};
use soundvault_lib::parse::{SampleContext, SampleOccurrence};

fn make_occurrence(path: PathBuf, project: &str, category: &str) -> ClassifiedOccurrence {
    ClassifiedOccurrence {
        occurrence: SampleOccurrence {
            path: path.clone(),
            filename: path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string(),
            declared_size: None,
            declared_crc: None,
            original_path: None,
            track_name: None,
            group_path: vec![],
            project_name: project.to_string(),
            project_root: PathBuf::from(format!("/{}", project)),
            context: SampleContext::AudioClip,
        },
        category: category.to_string(),
    }
}

#[test]
fn identical_files_at_different_paths_collapse() {
    let tmp = tempdir().unwrap();
    let p1 = tmp.path().join("a/foo.wav");
    let p2 = tmp.path().join("b/foo.wav");
    fs::create_dir_all(p1.parent().unwrap()).unwrap();
    fs::create_dir_all(p2.parent().unwrap()).unwrap();
    fs::write(&p1, b"hello world").unwrap();
    fs::write(&p2, b"hello world").unwrap();
    let occurrences = vec![
        make_occurrence(p1, "ProjectA", "Drums / Kicks & Bassdrums"),
        make_occurrence(p2, "ProjectB", "Drums / Kicks & Bassdrums"),
    ];
    let cancel = Arc::new(CancellationToken::new());
    let flags = FilterFlags::default();
    let filtered = filter_artifact_folders(occurrences, &flags);
    let uniques = dedup_and_count(filtered, &flags, &cancel, |_, _| {});
    assert_eq!(uniques.len(), 1, "expected dedup to collapse identical content");
    assert_eq!(uniques[0].project_count, 2);
    assert_eq!(uniques[0].clip_count, 2);
}

#[test]
fn multiple_uses_in_one_project_count_as_one_project() {
    let tmp = tempdir().unwrap();
    let p = tmp.path().join("foo.wav");
    fs::write(&p, b"abc").unwrap();
    let occurrences = vec![
        make_occurrence(p.clone(), "ProjectX", "Drums / Kicks & Bassdrums"),
        make_occurrence(p.clone(), "ProjectX", "Drums / Kicks & Bassdrums"),
        make_occurrence(p, "ProjectX", "Drums / Kicks & Bassdrums"),
    ];
    let cancel = Arc::new(CancellationToken::new());
    let flags = FilterFlags::default();
    let uniques = dedup_and_count(filter_artifact_folders(occurrences, &flags), &flags, &cancel, |_, _| {});
    assert_eq!(uniques.len(), 1);
    assert_eq!(uniques[0].project_count, 1);
    assert_eq!(uniques[0].clip_count, 3);
}

#[test]
fn missing_files_dropped_by_default() {
    let occurrences = vec![make_occurrence(PathBuf::from("/nope/missing.wav"), "ProjectZ", "Drums / Kicks & Bassdrums")];
    let cancel = Arc::new(CancellationToken::new());
    let flags = FilterFlags::default();
    let uniques = dedup_and_count(filter_artifact_folders(occurrences, &flags), &flags, &cancel, |_, _| {});
    assert!(uniques.is_empty());
}

#[test]
fn missing_files_kept_when_opted_in() {
    let occurrences = vec![make_occurrence(PathBuf::from("/nope/missing.wav"), "ProjectZ", "Drums / Kicks & Bassdrums")];
    let cancel = Arc::new(CancellationToken::new());
    let mut flags = FilterFlags::default();
    flags.include_missing = true;
    let uniques = dedup_and_count(filter_artifact_folders(occurrences, &flags), &flags, &cancel, |_, _| {});
    assert_eq!(uniques.len(), 1);
    assert!(uniques[0].missing);
}
