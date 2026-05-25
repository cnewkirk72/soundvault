//! Output folder writes. Only writes happen here, and only to `output_root`.

use crate::cancel::CancellationToken;
use crate::error::{ScanError, ScanResult};
use crate::rank::{AnalysisReport, CategoryReport};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopiedSample {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub category: String,
    pub project_count: u32,
    pub clip_count: u32,
}

/// Copy every sample in the report into a taxonomy-mirroring tree under
/// `output_root`. Returns the list of copied samples. Emits progress via the
/// supplied callback.
pub fn copy_samples(
    report: &[CategoryReport],
    output_root: &Path,
    cancel: &Arc<CancellationToken>,
    mut on_progress: impl FnMut(u32, u32, &str),
) -> ScanResult<Vec<CopiedSample>> {
    std::fs::create_dir_all(output_root)
        .map_err(|e| ScanError::io(output_root.to_path_buf(), e))?;

    let total: u32 = report.iter().map(|c| c.samples.len() as u32).sum();
    let mut copied_n: u32 = 0;
    let mut copied: Vec<CopiedSample> = Vec::with_capacity(total as usize);

    // Track filename collisions per destination directory.
    let mut used: HashSet<PathBuf> = HashSet::new();

    for cat in report {
        if cat.samples.is_empty() {
            continue;
        }
        let dir = output_root.join(joined_components(&cat.components));
        std::fs::create_dir_all(&dir).map_err(|e| ScanError::io(dir.clone(), e))?;

        for s in &cat.samples {
            if cancel.is_cancelled() {
                return Err(ScanError::Cancelled);
            }
            if s.missing {
                copied_n += 1;
                on_progress(copied_n, total, &s.filename);
                continue;
            }
            let dest = resolve_collision(&dir, &s.filename, &mut used);
            // Skip if a file at dest already exists on disk from a prior run
            // and has matching content hash (we don't recompute — just leave
            // the existing file alone).
            if dest.exists() {
                copied_n += 1;
                on_progress(copied_n, total, &s.filename);
                continue;
            }
            std::fs::copy(&s.canonical_path, &dest).map_err(|e| ScanError::io(dest.clone(), e))?;
            copied.push(CopiedSample {
                source: s.canonical_path.clone(),
                destination: dest.clone(),
                category: cat.path.clone(),
                project_count: s.project_count,
                clip_count: s.clip_count,
            });
            copied_n += 1;
            on_progress(copied_n, total, &s.filename);
        }
    }
    Ok(copied)
}

/// Write a `manifest.json` at `output_root`. Minimal, human-readable.
pub fn write_manifest(
    output_root: &Path,
    report: &AnalysisReport,
    copied: &[CopiedSample],
    input_root: &str,
    config_summary: &serde_json::Value,
) -> ScanResult<PathBuf> {
    let manifest = serde_json::json!({
        "app": "Soundvault",
        "version": report.app_version,
        "run_timestamp": report.run_timestamp,
        "input_root": input_root,
        "output_root": output_root.to_string_lossy(),
        "config": config_summary,
        "projects_scanned": report.projects_scanned,
        "unique_samples": report.unique_samples,
        "total_occurrences": report.total_occurrences,
        "copied": copied,
        "categories": report.categories.iter().map(|c| serde_json::json!({
            "path": c.path,
            "display_name": c.display_name,
            "components": c.components,
            "saved": c.samples.len(),
            "total_occurrences": c.total_occurrences,
            "project_count": c.project_count,
        })).collect::<Vec<_>>(),
        "parse_errors": report.parse_errors,
    });
    let path = output_root.join("manifest.json");
    let pretty = serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string());
    std::fs::write(&path, pretty).map_err(|e| ScanError::io(path.clone(), e))?;
    Ok(path)
}

fn joined_components(components: &[String]) -> PathBuf {
    let mut p = PathBuf::new();
    for c in components {
        // Sanitize for filesystem safety — replace path separators in names.
        let safe = c.replace(['/', '\\'], "_");
        p.push(safe);
    }
    p
}

fn resolve_collision(dir: &Path, filename: &str, used: &mut HashSet<PathBuf>) -> PathBuf {
    let candidate = dir.join(filename);
    if !candidate.exists() && !used.contains(&candidate) {
        used.insert(candidate.clone());
        return candidate;
    }
    // Need suffixing.
    let (stem, ext) = split_filename(filename);
    let mut n: u32 = 2;
    loop {
        let suffix_name = if ext.is_empty() {
            format!("{} ({})", stem, n)
        } else {
            format!("{} ({}).{}", stem, n, ext)
        };
        let p = dir.join(&suffix_name);
        if !p.exists() && !used.contains(&p) {
            used.insert(p.clone());
            return p;
        }
        n += 1;
        if n > 99999 {
            // Extreme safety: append a random tail
            let p = dir.join(format!("{} ({}-{}).{}", stem, n, std::process::id(), ext));
            used.insert(p.clone());
            return p;
        }
    }
}

fn split_filename(name: &str) -> (String, String) {
    let p = Path::new(name);
    let stem = p
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name)
        .to_string();
    let ext = p
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    (stem, ext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn collision_suffixes_2_then_3() {
        let tmp = tempdir().unwrap();
        let mut used = HashSet::new();
        std::fs::write(tmp.path().join("a.wav"), b"x").unwrap();
        let p1 = resolve_collision(tmp.path(), "a.wav", &mut used);
        let p2 = resolve_collision(tmp.path(), "a.wav", &mut used);
        assert!(p1.file_name().unwrap().to_str().unwrap().contains("(2)"));
        assert!(p2.file_name().unwrap().to_str().unwrap().contains("(3)"));
    }
}
