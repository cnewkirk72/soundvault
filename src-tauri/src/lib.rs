//! Soundvault — Tauri entry point.
//!
//! Wires plugins, registers commands, and shares the cancellation token across
//! all background scans.

pub mod cancel;
pub mod classify;
pub mod commands;
pub mod copy;
pub mod dedup;
pub mod discover;
pub mod error;
pub mod parse;
pub mod progress;
pub mod rank;
pub mod readonly;
pub mod taxonomy;

use cancel::CancellationToken;
use std::sync::Arc;

/// Shared state injected into every Tauri command.
pub struct AppState {
    pub cancel: Arc<CancellationToken>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            cancel: Arc::new(CancellationToken::new()),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::load_taxonomy,
            commands::validate_config,
            commands::start_scan,
            commands::cancel_scan,
            commands::reveal_path,
            commands::app_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Soundvault");
}
