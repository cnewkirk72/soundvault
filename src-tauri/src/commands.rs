//! Tauri command surface and the orchestrator that runs a full scan.

use crate::cancel::CancellationToken;
use crate::classify::{classify, ManualKeywords, MatchMode};
use crate::copy::{copy_samples, write_manifest, CopiedSample};
use crate::dedup::{dedup_and_count, filter_artifact_folders, ClassifiedOccurrence, FilterFlags};
use crate::discover::{discover_from_many, discover_projects, DiscoveredProject};
use crate::error::{ScanError, ScanResult};
use crate::parse::parse_project;
use crate::progress::{emit, ScanEvent};
use crate::rank::{rank, AnalysisReport, ParseErrorEntry, Tiebreaker};
use crate::readonly::{path_is_within, ReadOnlyProject};
use crate::taxonomy::{FlatCategory, Taxonomy};
use crate::AppState;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanSource {
    /// One folder to recursively walk for projects.
    Root(PathBuf),
    /// A multi-select of specific project folders.
    Projects(Vec<PathBuf>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub source: ScanSource,
    pub output_folder: PathBuf,
    pub selected_categories: Vec<String>,
    pub top_n: usize,
    pub match_mode: MatchMode,
    pub manual_keywords: ManualKeywords,
    /// Advanced settings.
    pub include_freeze: bool,
    pub include_processed: bool,
    pub include_recorded: bool,
    pub include_missing: bool,
    pub tiebreaker: Tiebreaker,
}

#[derive(Debug, Serialize)]
pub struct TaxonomyResponse {
    pub categories: Vec<FlatCategory>,
}

#[tauri::command]
pub fn load_taxonomy() -> ScanResult<TaxonomyResponse> {
    let t = Taxonomy::default_bundled()?;
    Ok(TaxonomyResponse {
        categories: t.flatten(),
    })
}

#[tauri::command]
pub fn app_version() -> String {
    APP_VERSION.to_string()
}

/// Pre-flight check — used by the UI to disable the Start button with a clear
/// reason rather than blowing up mid-scan.
#[tauri::command]
pub fn validate_config(config: ScanConfig) -> ScanResult<()> {
    if config.selected_categories.is_empty() {
        return Err(ScanError::NoCategoriesSelected);
    }
    if !config.output_folder.exists() {
        // Allow non-existent output (we create it), but parent must exist.
        if let Some(parent) = config.output_folder.parent() {
            if !parent.exists() {
                return Err(ScanError::PathNotFound(parent.to_path_buf()));
            }
        }
    }
    let inputs = match &config.source {
        ScanSource::Root(p) => vec![p.clone()],
        ScanSource::Projects(ps) => ps.clone(),
    };
    for input in &inputs {
        if !input.exists() {
            return Err(ScanError::PathNotFound(input.clone()));
        }
        if path_is_within(&config.output_folder, input) {
            return Err(ScanError::OutputInsideInput(input.clone()));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) {
    state.cancel.cancel();
}

#[tauri::command]
pub fn reveal_path(
    path: PathBuf,
    app: AppHandle,
) -> ScanResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .reveal_item_in_dir(&path)
        .or_else(|_| app.opener().open_path(path.to_string_lossy().to_string(), None::<&str>))
        .map_err(|e| ScanError::Other(format!("could not reveal path: {e}")))?;
    Ok(())
}

/// Main entry point — runs the entire pipeline on a background thread so the
/// frontend can subscribe to "scan-progress" while we work.
#[tauri::command]
pub async fn start_scan(
    config: ScanConfig,
    app: AppHandle,
    state: State<'_, AppState>,
) -> ScanResult<AnalysisReport> {
    validate_config(config.clone())?;
    state.cancel.reset();
    let cancel = state.cancel.clone();

    // Heavy lifting on a blocking thread so we don't tie up the Tauri runtime.
    let app_clone = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || run_pipeline(config, app_clone, cancel))
        .await
        .map_err(|e| ScanError::Other(format!("scan task panic: {e}")))?;
    result
}

fn run_pipeline(
    config: ScanConfig,
    app: AppHandle,
    cancel: Arc<CancellationToken>,
) -> ScanResult<AnalysisReport> {
    // --- 1. Discovery ---
    let input_root_display = match &config.source {
        ScanSource::Root(p) => p.clone(),
        ScanSource::Projects(ps) => ps.first().cloned().unwrap_or_default(),
    };
    emit(
        &app,
        ScanEvent::DiscoveryStarted {
            root: input_root_display.clone(),
        },
    );

    let projects: Vec<DiscoveredProject> = match &config.source {
        ScanSource::Root(p) => discover_projects(p)?,
        ScanSource::Projects(ps) => discover_from_many(ps)?,
    };

    if projects.is_empty() {
        return Err(ScanError::NoProjectsFound(input_root_display));
    }

    let total_projects = projects.len() as u32;
    for p in &projects {
        emit(
            &app,
            ScanEvent::ProjectFound {
                path: p.project_dir.clone(),
                total: total_projects,
            },
        );
    }

    if cancel.is_cancelled() {
        emit(&app, ScanEvent::Cancelled);
        return Err(ScanError::Cancelled);
    }

    // --- 2. Parse in parallel ---
    let parsed: Vec<Result<crate::parse::ProjectAnalysis, (PathBuf, String)>> = projects
        .par_iter()
        .map(|p| -> Result<crate::parse::ProjectAnalysis, (PathBuf, String)> {
            if cancel.is_cancelled() {
                return Err((p.als_path.clone(), "cancelled".to_string()));
            }
            let ro = ReadOnlyProject::new(&p.project_dir, &p.als_path);
            parse_project(&ro).map_err(|e| (p.als_path.clone(), e.to_string()))
        })
        .collect();

    let mut parse_errors: Vec<ParseErrorEntry> = Vec::new();
    let mut analyses: Vec<crate::parse::ProjectAnalysis> = Vec::new();
    let mut idx: u32 = 0;
    for (project, result) in projects.iter().zip(parsed.into_iter()) {
        idx += 1;
        if cancel.is_cancelled() {
            emit(&app, ScanEvent::Cancelled);
            return Err(ScanError::Cancelled);
        }
        match result {
            Ok(a) => {
                let n = a.samples.len() as u32;
                emit(
                    &app,
                    ScanEvent::ProjectParsed {
                        path: project.project_dir.clone(),
                        index: idx,
                        total: total_projects,
                        samples_found: n,
                    },
                );
                analyses.push(a);
            }
            Err((path, msg)) => {
                emit(
                    &app,
                    ScanEvent::ParseError {
                        path: path.clone(),
                        error: msg.clone(),
                    },
                );
                parse_errors.push(ParseErrorEntry {
                    project_path: project.project_dir.clone(),
                    message: msg,
                });
            }
        }
    }

    // --- 3. Classify ---
    let taxonomy = Taxonomy::default_bundled()?;
    let categories: Vec<FlatCategory> = taxonomy.flatten();
    let mut classified: Vec<ClassifiedOccurrence> = Vec::new();
    for a in &analyses {
        for occ in &a.samples {
            if let Some(cat) = classify(
                occ,
                &config.selected_categories,
                &categories,
                config.match_mode,
                &config.manual_keywords,
            ) {
                classified.push(ClassifiedOccurrence {
                    occurrence: occ.clone(),
                    category: cat,
                });
            }
        }
    }

    // --- 4. Filter artifact folders ---
    let flags = FilterFlags {
        include_freeze: config.include_freeze,
        include_processed: config.include_processed,
        include_recorded: config.include_recorded,
        include_missing: config.include_missing,
    };
    let filtered = filter_artifact_folders(classified, &flags);

    // --- 5. Dedup ---
    emit(
        &app,
        ScanEvent::DedupStarted {
            total_samples: filtered.len() as u64,
        },
    );
    let app_for_dedup = app.clone();
    let total_for_dedup = filtered.len() as u64;
    let cancel_dedup = cancel.clone();
    let progress_state: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
    let progress_state_clone = progress_state.clone();
    let uniques = dedup_and_count(
        filtered,
        &flags,
        &cancel_dedup,
        move |processed, _total| {
            let mut last = progress_state_clone.lock().unwrap();
            // Throttle: emit every 1% or every 64 samples.
            let step = (total_for_dedup / 100).max(1);
            if processed - *last >= step || processed == total_for_dedup {
                *last = processed;
                emit(
                    &app_for_dedup,
                    ScanEvent::DedupProgress {
                        processed,
                        total: total_for_dedup,
                    },
                );
            }
        },
    );

    if cancel.is_cancelled() {
        emit(&app, ScanEvent::Cancelled);
        return Err(ScanError::Cancelled);
    }

    // --- 6. Rank ---
    let category_index: BTreeMap<String, (String, Vec<String>)> = categories
        .iter()
        .map(|c| (c.path.clone(), (c.name.clone(), c.components.clone())))
        .collect();

    let report_categories = rank(
        uniques,
        &config.selected_categories,
        config.top_n,
        config.tiebreaker,
        &category_index,
    );

    let total_unique_samples: u32 = report_categories
        .iter()
        .map(|c| c.samples.len() as u32)
        .sum();
    let total_occurrences: u64 = report_categories.iter().map(|c| c.total_occurrences).sum();

    // --- 7. Copy ---
    emit(
        &app,
        ScanEvent::CopyStarted {
            total: total_unique_samples,
        },
    );
    let app_for_copy = app.clone();
    let cancel_copy = cancel.clone();
    let copied = copy_samples(
        &report_categories,
        &config.output_folder,
        &cancel_copy,
        move |copied, total, current| {
            emit(
                &app_for_copy,
                ScanEvent::CopyProgress {
                    copied,
                    total,
                    current_filename: current.to_string(),
                },
            );
        },
    )?;

    // --- 8. Manifest + Complete ---
    let now = current_iso8601();
    let report = AnalysisReport {
        categories: report_categories,
        parse_errors,
        output_root: config.output_folder.clone(),
        projects_scanned: analyses.len() as u32,
        unique_samples: total_unique_samples,
        total_occurrences,
        app_version: APP_VERSION.to_string(),
        run_timestamp: now,
    };

    let _ = write_manifest(
        &config.output_folder,
        &report,
        &copied,
        &input_root_display.to_string_lossy(),
        &serde_json::to_value(&MinimalConfigSummary::from(&config)).unwrap_or(serde_json::Value::Null),
    );

    emit(&app, ScanEvent::Complete { report: report.clone() });
    Ok(report)
}

/// What we serialize into the manifest for the user. Mirrors the user-facing
/// choices, not internal flags.
#[derive(Serialize)]
struct MinimalConfigSummary {
    source: serde_json::Value,
    output: String,
    selected_categories: Vec<String>,
    top_n: usize,
    match_mode: MatchMode,
    manual_keywords: ManualKeywords,
    include_freeze: bool,
    include_processed: bool,
    include_recorded: bool,
    include_missing: bool,
    tiebreaker: Tiebreaker,
}

impl From<&ScanConfig> for MinimalConfigSummary {
    fn from(c: &ScanConfig) -> Self {
        let source = match &c.source {
            ScanSource::Root(p) => serde_json::json!({"kind": "root", "path": p.to_string_lossy()}),
            ScanSource::Projects(ps) => serde_json::json!({
                "kind": "projects",
                "paths": ps.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()
            }),
        };
        MinimalConfigSummary {
            source,
            output: c.output_folder.to_string_lossy().to_string(),
            selected_categories: c.selected_categories.clone(),
            top_n: c.top_n,
            match_mode: c.match_mode,
            manual_keywords: c.manual_keywords.clone(),
            include_freeze: c.include_freeze,
            include_processed: c.include_processed,
            include_recorded: c.include_recorded,
            include_missing: c.include_missing,
            tiebreaker: c.tiebreaker,
        }
    }
}

// Use std::time::SystemTime to avoid bringing in heavy chrono.
fn current_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Use `time` crate (already a dep) for proper ISO-8601.
    let dt = time::OffsetDateTime::from_unix_timestamp(secs as i64).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    dt.format(&time::format_description::well_known::Iso8601::DEFAULT)
        .unwrap_or_else(|_| format!("{}", secs))
}

// Allow CopiedSample to be referenced in serde_json (avoid unused import warn).
#[allow(dead_code)]
fn _suppress_unused_copied(_: &[CopiedSample]) {}
