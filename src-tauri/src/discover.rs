//! Project discovery — walks a root folder and returns the .als file at the
//! root of each Ableton project directory. Backup/ folders are skipped.

use crate::error::{ScanError, ScanResult};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

/// A discovered project: the project folder + the .als at its root.
#[derive(Debug, Clone)]
pub struct DiscoveredProject {
    pub project_dir: PathBuf,
    pub als_path: PathBuf,
}

/// Identify project folders under `root`. A folder is a project iff it
/// contains at least one `.als` directly inside it (not in subfolders) AND it
/// is not itself called `Backup`.
///
/// If the user picked specific project folders, pass each one as a `root` and
/// they'll be returned individually (the same logic handles "single project"
/// since we only descend until we find one).
pub fn discover_projects(root: &Path) -> ScanResult<Vec<DiscoveredProject>> {
    if !root.exists() {
        return Err(ScanError::PathNotFound(root.to_path_buf()));
    }
    if !root.is_dir() {
        return Err(ScanError::NotADirectory(root.to_path_buf()));
    }

    let mut projects = Vec::new();

    // We want to find every directory that contains an .als at depth 0 (no
    // recursion below the project). Skip Backup/ and anything beneath it.
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_backup_dir(e));

    for entry in walker.flatten() {
        if !entry.file_type().is_dir() {
            continue;
        }
        if let Some(als) = first_root_als(entry.path()) {
            projects.push(DiscoveredProject {
                project_dir: entry.path().to_path_buf(),
                als_path: als,
            });
        }
    }

    // Deduplicate: if root itself is a project AND we descended into it, we'd
    // end up with parents and children. Keep parent (one project per folder),
    // and remove any project whose root is inside another project's root.
    projects.sort_by(|a, b| a.project_dir.cmp(&b.project_dir));
    projects.dedup_by(|a, b| a.project_dir == b.project_dir);

    // Filter: drop entries whose project_dir is strictly inside another
    // entry's project_dir (e.g. nested duplicate project folders).
    let kept: Vec<DiscoveredProject> = projects
        .iter()
        .filter(|p| {
            !projects.iter().any(|q| {
                q.project_dir != p.project_dir && p.project_dir.starts_with(&q.project_dir)
            })
        })
        .cloned()
        .collect();

    Ok(kept)
}

/// Discover from multiple roots (e.g. user multi-selected project folders).
/// Returns a deduped list.
pub fn discover_from_many(roots: &[PathBuf]) -> ScanResult<Vec<DiscoveredProject>> {
    let mut all = Vec::new();
    for r in roots {
        all.extend(discover_projects(r)?);
    }
    all.sort_by(|a, b| a.project_dir.cmp(&b.project_dir));
    all.dedup_by(|a, b| a.project_dir == b.project_dir);
    Ok(all)
}

fn is_backup_dir(entry: &DirEntry) -> bool {
    entry.file_type().is_dir()
        && entry
            .file_name()
            .to_str()
            .map(|n| n.eq_ignore_ascii_case("Backup"))
            .unwrap_or(false)
}

/// Return the first .als directly in `dir` (not recursively). When multiple
/// .als exist at the root, prefer one whose stem matches the folder name; else
/// alphabetic order.
fn first_root_als(dir: &Path) -> Option<PathBuf> {
    let read_dir = std::fs::read_dir(dir).ok()?;
    let mut candidates: Vec<PathBuf> = read_dir
        .flatten()
        .filter(|e| {
            let p = e.path();
            p.is_file()
                && p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("als"))
                    .unwrap_or(false)
        })
        .map(|e| e.path())
        .collect();
    if candidates.is_empty() {
        return None;
    }
    let folder_name = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    candidates.sort_by(|a, b| {
        let a_match = a
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase() == folder_name)
            .unwrap_or(false);
        let b_match = b
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase() == folder_name)
            .unwrap_or(false);
        match (a_match, b_match) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.cmp(b),
        }
    });
    candidates.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn make_project(dir: &Path, name: &str) {
        let p = dir.join(name);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join(format!("{name}.als")), b"\x1f\x8b\x08\x00").unwrap();
        fs::create_dir_all(p.join("Samples")).unwrap();
        // A Backup with another .als that we must NOT pick up.
        fs::create_dir_all(p.join("Backup")).unwrap();
        fs::write(p.join("Backup/old.als"), b"\x1f\x8b\x08\x00").unwrap();
    }

    #[test]
    fn finds_project_skipping_backup() {
        let tmp = tempdir().unwrap();
        make_project(tmp.path(), "Track A");
        make_project(tmp.path(), "Track B");
        let found = discover_projects(tmp.path()).unwrap();
        assert_eq!(found.len(), 2);
        for p in &found {
            assert!(!p.als_path.to_string_lossy().contains("Backup"));
        }
    }
}
