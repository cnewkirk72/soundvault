//! Progress events streamed to the frontend via the "scan-progress" channel.
//!
//! The frontend matches on `kind` (snake_case discriminant) and unpacks the
//! remaining fields per variant.

use crate::rank::AnalysisReport;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};

pub const SCAN_PROGRESS_EVENT: &str = "scan-progress";

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScanEvent {
    DiscoveryStarted {
        root: PathBuf,
    },
    ProjectFound {
        path: PathBuf,
        total: u32,
    },
    ProjectParsed {
        path: PathBuf,
        index: u32,
        total: u32,
        samples_found: u32,
    },
    ParseError {
        path: PathBuf,
        error: String,
    },
    DedupStarted {
        total_samples: u64,
    },
    DedupProgress {
        processed: u64,
        total: u64,
    },
    CopyStarted {
        total: u32,
    },
    CopyProgress {
        copied: u32,
        total: u32,
        current_filename: String,
    },
    Complete {
        report: AnalysisReport,
    },
    Cancelled,
}

pub fn emit(app: &AppHandle, event: ScanEvent) {
    let _ = app.emit(SCAN_PROGRESS_EVENT, event);
}
