//! Taxonomy loading + lookup utilities.
//!
//! The default taxonomy ships at `resources/taxonomy.json` and is loaded at
//! startup. Users may override by selecting a custom file (future v2 — for v1
//! we always use the default).

use crate::error::{ScanError, ScanResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A taxonomy node is either a leaf (list of keywords) or a parent with named
/// children. Both variants implicitly contribute keywords matched against a
/// sample's filename/group/track name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaxonomyNode {
    Leaf(Vec<String>),
    Parent(BTreeMap<String, TaxonomyNode>),
}

/// Top-level taxonomy is keyed by category name (e.g. "Drums").
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Taxonomy {
    pub roots: BTreeMap<String, TaxonomyNode>,
}

/// Flattened category — what classifiers and rankers actually consume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatCategory {
    /// Fully-qualified path from root to this category, joined by " / "
    pub path: String,
    /// Path components (for building the output folder hierarchy).
    pub components: Vec<String>,
    /// Display name (the leaf name).
    pub name: String,
    /// Keywords matched against sample/track/group names. Normalized lowercase.
    pub keywords: Vec<String>,
    /// Depth: 1 = top-level, 2 = child, etc.
    pub depth: usize,
}

impl Taxonomy {
    pub fn from_str(s: &str) -> ScanResult<Self> {
        let value: BTreeMap<String, TaxonomyNode> = serde_json::from_str(s)
            .map_err(|e| ScanError::BadTaxonomy(e.to_string()))?;
        Ok(Taxonomy { roots: value })
    }

    /// Load the default bundled taxonomy. In a packaged Tauri build the JSON is
    /// embedded at compile time so we don't need filesystem access here.
    pub fn default_bundled() -> ScanResult<Self> {
        const DEFAULT: &str = include_str!("../resources/taxonomy.json");
        Self::from_str(DEFAULT)
    }

    /// Walk the taxonomy and return every category (leaf + parent) flattened.
    pub fn flatten(&self) -> Vec<FlatCategory> {
        let mut out = Vec::new();
        for (name, node) in &self.roots {
            walk(node, vec![name.clone()], &mut out, 1);
        }
        out
    }

    /// Find the flat category by its full path.
    pub fn find(&self, path: &str) -> Option<FlatCategory> {
        self.flatten().into_iter().find(|c| c.path == path)
    }
}

fn walk(node: &TaxonomyNode, path: Vec<String>, out: &mut Vec<FlatCategory>, depth: usize) {
    let name = path.last().cloned().unwrap_or_default();
    match node {
        TaxonomyNode::Leaf(keywords) => {
            out.push(FlatCategory {
                path: path.join(" / "),
                components: path.clone(),
                name,
                keywords: keywords.iter().map(|k| k.to_lowercase()).collect(),
                depth,
            });
        }
        TaxonomyNode::Parent(children) => {
            // A parent doesn't have its own keywords but contributes itself as a
            // category that *selects all descendants* by default. Its keywords
            // are the union of its descendants' keywords.
            let mut all_keywords: Vec<String> = Vec::new();
            for child in children.values() {
                collect_keywords(child, &mut all_keywords);
            }
            all_keywords.sort();
            all_keywords.dedup();
            out.push(FlatCategory {
                path: path.join(" / "),
                components: path.clone(),
                name,
                keywords: all_keywords,
                depth,
            });
            for (child_name, child_node) in children {
                let mut child_path = path.clone();
                child_path.push(child_name.clone());
                walk(child_node, child_path, out, depth + 1);
            }
        }
    }
}

fn collect_keywords(node: &TaxonomyNode, out: &mut Vec<String>) {
    match node {
        TaxonomyNode::Leaf(kw) => {
            for k in kw {
                out.push(k.to_lowercase());
            }
        }
        TaxonomyNode::Parent(children) => {
            for child in children.values() {
                collect_keywords(child, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bundled_parses() {
        let t = Taxonomy::default_bundled().unwrap();
        let flat = t.flatten();
        assert!(flat.iter().any(|c| c.name == "Snares"));
        assert!(flat.iter().any(|c| c.name == "Kicks & Bassdrums"));
    }

    #[test]
    fn parent_aggregates_child_keywords() {
        let t = Taxonomy::default_bundled().unwrap();
        let drums = t.find("Drums").expect("Drums root");
        assert!(drums.keywords.iter().any(|k| k == "snare"));
        assert!(drums.keywords.iter().any(|k| k == "kick"));
    }
}
