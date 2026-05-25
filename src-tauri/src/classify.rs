//! Sample classification — three modes per the spec:
//!
//! * Mode A: Use Groups — start from the deepest group name in the sample's
//!   group_path and walk up matching against taxonomy keywords.
//! * Mode B: Auto-detect — keyword match the filename, then track name.
//! * Mode C: Manual — same as B but with user-supplied keyword lists.
//!
//! Track-name fallback applies in all modes.

use crate::parse::SampleOccurrence;
use crate::taxonomy::FlatCategory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    UseGroups,
    AutoDetect,
    Manual,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManualKeywords {
    /// category path -> list of user-supplied keywords
    pub per_category: HashMap<String, Vec<String>>,
}

/// Returns the *path* of the matched category (or None for uncategorized).
/// `selected_paths` filters the universe of allowed categories.
pub fn classify(
    sample: &SampleOccurrence,
    selected_paths: &[String],
    categories: &[FlatCategory],
    mode: MatchMode,
    manual: &ManualKeywords,
) -> Option<String> {
    let cats: Vec<&FlatCategory> = categories
        .iter()
        .filter(|c| selected_paths.iter().any(|p| p == &c.path))
        .collect();
    if cats.is_empty() {
        return None;
    }

    match mode {
        MatchMode::UseGroups => classify_use_groups(sample, &cats)
            .or_else(|| classify_filename(sample, &cats, mode, manual)),
        MatchMode::AutoDetect => classify_filename(sample, &cats, mode, manual),
        MatchMode::Manual => classify_filename(sample, &cats, mode, manual),
    }
}

fn classify_use_groups(sample: &SampleOccurrence, cats: &[&FlatCategory]) -> Option<String> {
    // Walk from deepest to shallowest. Last entry is innermost (the track), so
    // try in reverse including the track name itself.
    let mut chain: Vec<String> = sample.group_path.iter().rev().cloned().collect();
    if let Some(tn) = &sample.track_name {
        chain.insert(0, tn.clone());
    }
    for name in &chain {
        if let Some(cat) = best_match_against(&normalize(name), cats, Vec::new()) {
            return Some(cat.path.clone());
        }
    }
    None
}

fn classify_filename(
    sample: &SampleOccurrence,
    cats: &[&FlatCategory],
    mode: MatchMode,
    manual: &ManualKeywords,
) -> Option<String> {
    let stem = std::path::Path::new(&sample.filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&sample.filename);
    let normalized = normalize(stem);
    if let Some(cat) = best_match_against(&normalized, cats, manual_extra(cats, mode, manual)) {
        return Some(cat.path.clone());
    }
    if let Some(tn) = &sample.track_name {
        let tnn = normalize(tn);
        if let Some(cat) = best_match_against(&tnn, cats, manual_extra(cats, mode, manual)) {
            return Some(cat.path.clone());
        }
    }
    None
}

fn manual_extra<'a>(
    cats: &'a [&FlatCategory],
    mode: MatchMode,
    manual: &'a ManualKeywords,
) -> Vec<(&'a FlatCategory, Vec<String>)> {
    if mode != MatchMode::Manual {
        return Vec::new();
    }
    cats.iter()
        .map(|c| {
            let kws = manual
                .per_category
                .get(&c.path)
                .map(|v| v.iter().map(|k| k.to_lowercase()).collect::<Vec<_>>())
                .unwrap_or_default();
            (*c, kws)
        })
        .collect()
}

/// Among all keywords across `cats`, return the category whose longest matched
/// keyword is the longest overall. Manual overrides replace each category's
/// keywords when `replace_keywords` has an entry for that category.
fn best_match_against<'a>(
    normalized: &str,
    cats: &[&'a FlatCategory],
    replace_keywords: Vec<(&'a FlatCategory, Vec<String>)>,
) -> Option<&'a FlatCategory> {
    // Tokenize by common separators + keep contiguous substring matches.
    // We do a word-boundary aware longest-match.
    let tokens: Vec<&str> = normalized
        .split(|c: char| !(c.is_ascii_alphanumeric()))
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.is_empty() {
        return None;
    }

    // Rank by (matched keyword length DESC, category depth DESC). The depth
    // tiebreak ensures a more specific child category (e.g. "Drums / Hats")
    // wins over its parent ("Drums"), since parents inherit all descendant
    // keywords for selection purposes.
    let mut best: Option<(usize, usize, &FlatCategory)> = None; // (len, depth, cat)

    for cat in cats {
        let manual_for = replace_keywords
            .iter()
            .find(|(c, _)| c.path == cat.path)
            .map(|(_, kws)| kws.clone());
        let keywords: &[String] = match manual_for.as_deref() {
            Some(kws) if !kws.is_empty() => kws,
            _ => &cat.keywords,
        };
        for kw in keywords {
            if kw.is_empty() {
                continue;
            }
            if tokens.iter().any(|t| *t == kw) {
                let len = kw.len();
                let depth = cat.depth;
                let better = match best {
                    None => true,
                    Some((bl, bd, _)) => len > bl || (len == bl && depth > bd),
                };
                if better {
                    best = Some((len, depth, cat));
                }
            }
        }
    }
    best.map(|(_, _, c)| c)
}

pub fn normalize(s: &str) -> String {
    s.to_lowercase()
        // Replace common punctuation with spaces; we tokenize later.
        .replace([',', '.', '!', '?', '\'', '"', '(', ')', '[', ']'], " ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taxonomy::Taxonomy;
    use std::path::PathBuf;

    fn make_sample(filename: &str, track: Option<&str>, group: Vec<&str>) -> SampleOccurrence {
        SampleOccurrence {
            path: PathBuf::from(format!("/x/{}", filename)),
            filename: filename.to_string(),
            declared_size: None,
            declared_crc: None,
            original_path: None,
            track_name: track.map(|s| s.to_string()),
            group_path: group.into_iter().map(|s| s.to_string()).collect(),
            project_name: "p".to_string(),
            project_root: PathBuf::from("/p"),
            context: crate::parse::SampleContext::AudioClip,
        }
    }

    fn all_selected(t: &Taxonomy) -> Vec<String> {
        t.flatten().into_iter().map(|c| c.path).collect()
    }

    #[test]
    fn filename_keyword_match_bd() {
        let t = Taxonomy::default_bundled().unwrap();
        let cats = t.flatten();
        let s = make_sample("001 BD-Dark.wav", None, vec![]);
        let result = classify(
            &s,
            &all_selected(&t),
            &cats,
            MatchMode::AutoDetect,
            &ManualKeywords::default(),
        );
        assert_eq!(result.as_deref(), Some("Drums / Kicks & Bassdrums"));
    }

    #[test]
    fn track_name_fallback_for_unrecognizable_filename() {
        let t = Taxonomy::default_bundled().unwrap();
        let cats = t.flatten();
        let s = make_sample("bdj_clk.wav", Some("CLAP"), vec![]);
        let result = classify(
            &s,
            &all_selected(&t),
            &cats,
            MatchMode::AutoDetect,
            &ManualKeywords::default(),
        );
        assert_eq!(
            result.as_deref(),
            Some("Drums / Snares, Claps, & Rims / Claps")
        );
    }

    #[test]
    fn use_groups_innermost_wins() {
        let t = Taxonomy::default_bundled().unwrap();
        let cats = t.flatten();
        let s = make_sample(
            "anything_random.wav",
            Some("Lead"),
            vec!["DRUMS", "CLAP"],
        );
        let result = classify(
            &s,
            &all_selected(&t),
            &cats,
            MatchMode::UseGroups,
            &ManualKeywords::default(),
        );
        assert_eq!(
            result.as_deref(),
            Some("Drums / Snares, Claps, & Rims / Claps")
        );
    }

    #[test]
    fn ch_matches_hats() {
        let t = Taxonomy::default_bundled().unwrap();
        let cats = t.flatten();
        let s = make_sample("606 CH.wav", None, vec![]);
        let result = classify(
            &s,
            &all_selected(&t),
            &cats,
            MatchMode::AutoDetect,
            &ManualKeywords::default(),
        );
        assert_eq!(result.as_deref(), Some("Drums / Hats"));
    }

    #[test]
    fn longest_match_wins_sidestick() {
        let t = Taxonomy::default_bundled().unwrap();
        let cats = t.flatten();
        let s = make_sample("LiquidGold_sidestick_SP_01.wav", None, vec![]);
        let result = classify(
            &s,
            &all_selected(&t),
            &cats,
            MatchMode::AutoDetect,
            &ManualKeywords::default(),
        );
        assert_eq!(
            result.as_deref(),
            Some("Drums / Snares, Claps, & Rims / Rims")
        );
    }
}
