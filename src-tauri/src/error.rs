//! Unified error type. Implements `Serialize` so the frontend gets a stable
//! string on `invoke().catch`.

use serde::{Serialize, Serializer};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("scan was cancelled by user")]
    Cancelled,

    #[error("output folder cannot be inside (or equal to) a scanned project folder: {0}")]
    OutputInsideInput(PathBuf),

    #[error("no Ableton projects found under: {0}")]
    NoProjectsFound(PathBuf),

    #[error("at least one sample type must be selected")]
    NoCategoriesSelected,

    #[error("path does not exist: {0}")]
    PathNotFound(PathBuf),

    #[error("path is not a directory: {0}")]
    NotADirectory(PathBuf),

    #[error("taxonomy file is malformed: {0}")]
    BadTaxonomy(String),

    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("XML parse error in {path}: {message}")]
    Xml { path: PathBuf, message: String },

    #[error("gzip decompression failed in {path}: {message}")]
    Gzip { path: PathBuf, message: String },

    #[error("configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Other(String),
}

impl ScanError {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

impl Serialize for ScanError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type ScanResult<T> = Result<T, ScanError>;
