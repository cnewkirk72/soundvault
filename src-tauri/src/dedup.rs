//! Deduplication & clustering.
//!
//! Three-tier check, cheap-to-expensive:
//!   1. Same canonical absolute path → same sample.
//!   2. Same filename + same file size → candidate.
//!   3. Same Blake3 content hash → confirmed.
//!
//! After clustering we tally project_count / clip_count per unique sample.

use crate::cancel::CancellationToken;
use crate::parse::SampleOccurrence;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Result of the dedup stage — one entry per unique sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniqueSample {
    pub canonical_path: PathBuf,
    pub filename: String,
    pub file_size: Option<u64>,
    pub content_hash: Option<String>,
    pub original_path: Option<PathBuf>,
    pub track_name: Option<String>,
    pub category: Option<String>,
    pub project_count: u32,
    pub clip_count: u32,
    pub projects: Vec<String>,
    pub missing: bool,
    pub factory: bool,
}

#[derive(Debug, Clone)]
pub struct ClassifiedOccurrence {
    pub occurrence: SampleOccurrence,
    pub category: String,
}

#[derive(Debug, Default, Clone)]
pub struct FilterFlags {
    pub include_freeze: bool,
    pub include_processed: bool,
    pub include_recorded: bool,
    pub include_missing: bool,
}

pub fn filter_artifact_folders(
    occurrences: Vec<ClassifiedOccurrence>,
    flags: &FilterFlags,
) -> Vec<ClassifiedOccurrence> {
    occurrences
        .into_iter()
        .filter(|c| !is_excluded_artifact(&c.occurrence.path, flags))
        .collect()
}

/// Path-component aware match. Freeze must be checked *before* Processed so a
/// freeze path isn't double-classified as Processed.
pub fn is_excluded_artifact(path: &Path, flags: &FilterFlags) -> bool {
    let comps: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let has_subpath = |a: &str, b: &str| -> bool {
        comps
            .windows(2)
            .any(|w| w[0].eq_ignore_ascii_case(a) && w[1].eq_ignore_ascii_case(b))
    };
    let has_component = |a: &str| -> bool { comps.iter().any(|c| c.eq_ignore_ascii_case(a)) };
    let is_in_samples = comps.iter().any(|c| c.eq_ignore_ascii_case("Samples"));

    if is_in_samples && has_subpath("Processed", "Freeze") && !flags.include_freeze {
        return true;
    }
    if is_in_samples
        && comps
            .windows(2)
            .any(|w| w[0].eq_ignore_ascii_case("Samples") && w[1].eq_ignore_ascii_case("Processed"))
        && !flags.include_processed
        && !has_subpath("Processed", "Freeze")
    {
        return true;
    }
    if is_in_samples && has_component("Recorded")
        && comps
            .windows(2)
            .any(|w| w[0].eq_ignore_ascii_case("Samples") && w[1].eq_ignore_ascii_case("Recorded"))
        && !flags.include_recorded
    {
        return true;
    }
    false
}

pub fn dedup_and_count(
    occurrences: Vec<ClassifiedOccurrence>,
    flags: &FilterFlags,
    cancel: &Arc<CancellationToken>,
    mut on_progress: impl FnMut(u64, u64),
) -> Vec<UniqueSample> {
    let total = occurrences.len() as u64;
    let mut processed: u64 = 0;

    let mut size_cache: HashMap<PathBuf, Option<u64>> = HashMap::new();
    let mut canonical_cache: HashMap<PathBuf, PathBuf> = HashMap::new();

    #[derive(Default)]
    struct Bucket {
        groups: Vec<UniqueSample>,
    }

    let mut buckets: HashMap<(String, Option<u64>), Bucket> = HashMap::new();

    for c in occurrences {
        if cancel.is_cancelled() {
            return Vec::new();
        }
        processed += 1;
        if processed % 32 == 0 {
            on_progress(processed, total);
        }

        let path = &c.occurrence.path;
        let canonical = canonical_cache
            .entry(path.clone())
            .or_insert_with(|| dunce::canonicalize(path).unwrap_or_else(|_| path.clone()))
            .clone();

        let size = size_cache
            .entry(canonical.clone())
            .or_insert_with(|| std::fs::metadata(&canonical).ok().map(|m| m.len()))
            .clone();

        let exists = canonical.exists();
        let missing = !exists;

        if missing && !flags.include_missing {
            continue;
        }

        let key = (c.occurrence.filename.to_lowercase(), size);
        let bucket = buckets.entry(key).or_default();

        if let Some(g) = bucket.groups.iter_mut().find(|g| g.canonical_path == canonical) {
            merge_into(g, &c);
            continue;
        }

        // If the bucket already has at least one group whose path differs from
        // this canonical path, we need to confirm via content hash. Lazily
        // compute hashes for any existing group that doesn't have one yet,
        // then compare against this new sample's hash.
        let mut matched = false;
        if !bucket.groups.is_empty() && exists {
            let new_hash = blake3_of(&canonical).ok();
            if let Some(new_h) = new_hash.clone() {
                for g in bucket.groups.iter_mut() {
                    if g.content_hash.is_none() && !g.missing {
                        g.content_hash = blake3_of(&g.canonical_path).ok();
                    }
                    if g.content_hash.as_deref() == Some(new_h.as_str()) {
                        merge_into(g, &c);
                        matched = true;
                        break;
                    }
                }
            }
            if matched {
                continue;
            }
            bucket.groups.push(UniqueSample {
                canonical_path: canonical.clone(),
                filename: c.occurrence.filename.clone(),
                file_size: size,
                content_hash: new_hash,
                original_path: c.occurrence.original_path.clone(),
                track_name: c.occurrence.track_name.clone(),
                category: Some(c.category.clone()),
                project_count: 0,
                clip_count: 0,
                projects: Vec::new(),
                missing,
                factory: is_factory_path(&canonical),
            });
        } else {
            bucket.groups.push(UniqueSample {
                canonical_path: canonical.clone(),
                filename: c.occurrence.filename.clone(),
                file_size: size,
                content_hash: None,
                original_path: c.occurrence.original_path.clone(),
                track_name: c.occurrence.track_name.clone(),
                category: Some(c.category.clone()),
                project_count: 0,
                clip_count: 0,
                projects: Vec::new(),
                missing,
                factory: is_factory_path(&canonical),
            });
        }
        let g = bucket.groups.last_mut().unwrap();
        merge_into(g, &c);
    }

    on_progress(total, total);

    let mut out: Vec<UniqueSample> = buckets
        .into_values()
        .flat_map(|b| b.groups.into_iter())
        .collect();

    for u in &mut out {
        let unique: BTreeSet<String> = u.projects.iter().cloned().collect();
        u.projects = unique.into_iter().collect();
        u.project_count = u.projects.len() as u32;
    }

    out
}

fn merge_into(u: &mut UniqueSample, c: &ClassifiedOccurrence) {
    u.clip_count += 1;
    if !u.projects.contains(&c.occurrence.project_name) {
        u.projects.push(c.occurrence.project_name.clone());
    }
    if u.track_name.is_none() {
        u.track_name = c.occurrence.track_name.clone();
    }
    if u.original_path.is_none() {
        u.original_path = c.occurrence.original_path.clone();
    }
}

fn blake3_of(path: &Path) -> std::io::Result<String> {
    let mut hasher = Hasher::new();
    let f = File::open(path)?;
    let mut reader = BufReader::with_capacity(64 * 1024, f);
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

fn is_factory_path(p: &Path) -> bool {
    let s = p.to_string_lossy().to_lowercase();
    s.contains("/ableton/factory") || s.contains("\\ableton\\factory")
        || s.contains("/live packs/") || s.contains("\\live packs\\")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn excludes_freeze_by_default() {
        let p = PathBuf::from("/Users/x/Project/Samples/Processed/Freeze/foo.wav");
        let f = FilterFlags::default();
        assert!(is_excluded_artifact(&p, &f));
    }

    #[test]
    fn freeze_opt_in_keeps_it() {
        let p = PathBuf::from("/Users/x/Project/Samples/Processed/Freeze/foo.wav");
        let mut f = FilterFlags::default();
        f.include_freeze = true;
        assert!(!is_excluded_artifact(&p, &f));
    }

    #[test]
    fn excludes_processed_by_default() {
        let p = PathBuf::from("/Users/x/Project/Samples/Processed/Crop/foo.wav");
        let f = FilterFlags::default();
        assert!(is_excluded_artifact(&p, &f));
    }

    #[test]
    fn excludes_recorded_by_default() {
        let p = PathBuf::from("/Users/x/Project/Samples/Recorded/foo.wav");
        let f = FilterFlags::default();
        assert!(is_excluded_artifact(&p, &f));
    }

    #[test]
    fn keeps_imported() {
        let p = PathBuf::from("/Users/x/Project/Samples/Imported/foo.wav");
        let f = FilterFlags::default();
        assert!(!is_excluded_artifact(&p, &f));
    }
}
