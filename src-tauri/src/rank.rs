//! Top-N selection per category, plus the final report shape.

use crate::dedup::UniqueSample;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Tiebreaker {
    /// Project count, then clip count (default).
    ProjectThenClip,
    /// Clip count, then project count.
    ClipThenProject,
}

impl Default for Tiebreaker {
    fn default() -> Self {
        Tiebreaker::ProjectThenClip
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryReport {
    pub path: String,
    pub display_name: String,
    pub components: Vec<String>,
    pub samples: Vec<UniqueSample>,
    /// Number of clip-level occurrences across all projects (for the summary
    /// "sampled from X occurrences across Y projects" line).
    pub total_occurrences: u64,
    /// Distinct projects this category drew samples from.
    pub project_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub categories: Vec<CategoryReport>,
    /// Per-project parse errors (filename + error message).
    pub parse_errors: Vec<ParseErrorEntry>,
    /// Output folder root.
    pub output_root: std::path::PathBuf,
    /// Total projects scanned.
    pub projects_scanned: u32,
    /// Total unique samples across all categories (post-dedup).
    pub unique_samples: u32,
    /// Total sample occurrences across all projects (clips).
    pub total_occurrences: u64,
    /// App version that produced this report.
    pub app_version: String,
    /// ISO-8601 timestamp.
    pub run_timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseErrorEntry {
    pub project_path: std::path::PathBuf,
    pub message: String,
}

/// Sort and slice per-category Top N.
pub fn rank(
    uniques: Vec<UniqueSample>,
    selected_paths: &[String],
    n_per_category: usize,
    tiebreaker: Tiebreaker,
    category_index: &BTreeMap<String, (String, Vec<String>)>, // path -> (display name, components)
) -> Vec<CategoryReport> {
    let mut by_cat: BTreeMap<String, Vec<UniqueSample>> = BTreeMap::new();
    for u in uniques {
        if let Some(cat) = &u.category {
            if selected_paths.iter().any(|p| p == cat) {
                by_cat.entry(cat.clone()).or_default().push(u);
            }
        }
    }

    let mut out = Vec::with_capacity(by_cat.len());
    for (cat_path, mut samples) in by_cat {
        sort_samples(&mut samples, tiebreaker);
        let total_occurrences: u64 = samples.iter().map(|s| s.clip_count as u64).sum();
        let projects: std::collections::BTreeSet<String> = samples
            .iter()
            .flat_map(|s| s.projects.iter().cloned())
            .collect();
        let project_count = projects.len() as u32;
        let picked: Vec<UniqueSample> = samples.into_iter().take(n_per_category).collect();
        let (display_name, components) = category_index
            .get(&cat_path)
            .cloned()
            .unwrap_or_else(|| (cat_path.clone(), vec![cat_path.clone()]));
        out.push(CategoryReport {
            path: cat_path,
            display_name,
            components,
            samples: picked,
            total_occurrences,
            project_count,
        });
    }
    out
}

fn sort_samples(samples: &mut [UniqueSample], tiebreaker: Tiebreaker) {
    samples.sort_by(|a, b| {
        let primary = match tiebreaker {
            Tiebreaker::ProjectThenClip => b.project_count.cmp(&a.project_count),
            Tiebreaker::ClipThenProject => b.clip_count.cmp(&a.clip_count),
        };
        if primary != std::cmp::Ordering::Equal {
            return primary;
        }
        let secondary = match tiebreaker {
            Tiebreaker::ProjectThenClip => b.clip_count.cmp(&a.clip_count),
            Tiebreaker::ClipThenProject => b.project_count.cmp(&a.project_count),
        };
        if secondary != std::cmp::Ordering::Equal {
            return secondary;
        }
        a.filename.to_lowercase().cmp(&b.filename.to_lowercase())
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn u(name: &str, projects: u32, clips: u32, cat: &str) -> UniqueSample {
        UniqueSample {
            canonical_path: PathBuf::from(name),
            filename: name.to_string(),
            file_size: None,
            content_hash: None,
            original_path: None,
            track_name: None,
            category: Some(cat.to_string()),
            project_count: projects,
            clip_count: clips,
            projects: (0..projects).map(|i| format!("p{}", i)).collect(),
            missing: false,
            factory: false,
        }
    }

    #[test]
    fn ranks_project_then_clip() {
        let mut idx = BTreeMap::new();
        idx.insert("Drums / Snares".to_string(), ("Snares".to_string(), vec!["Drums".to_string(), "Snares".to_string()]));
        let uniques = vec![
            u("a.wav", 1, 100, "Drums / Snares"),
            u("b.wav", 5, 5, "Drums / Snares"),
            u("c.wav", 3, 3, "Drums / Snares"),
        ];
        let r = rank(uniques, &["Drums / Snares".to_string()], 10, Tiebreaker::ProjectThenClip, &idx);
        assert_eq!(r[0].samples[0].filename, "b.wav");
        assert_eq!(r[0].samples[1].filename, "c.wav");
        assert_eq!(r[0].samples[2].filename, "a.wav");
    }

    #[test]
    fn ranks_clip_then_project() {
        let mut idx = BTreeMap::new();
        idx.insert("Drums / Snares".to_string(), ("Snares".to_string(), vec!["Drums".to_string(), "Snares".to_string()]));
        let uniques = vec![
            u("a.wav", 1, 100, "Drums / Snares"),
            u("b.wav", 5, 5, "Drums / Snares"),
        ];
        let r = rank(uniques, &["Drums / Snares".to_string()], 10, Tiebreaker::ClipThenProject, &idx);
        assert_eq!(r[0].samples[0].filename, "a.wav");
        assert_eq!(r[0].samples[1].filename, "b.wav");
    }
}
