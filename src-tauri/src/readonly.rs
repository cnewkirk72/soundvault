//! Read-only project-file wrapper.
//!
//! Anything inside a scanned project directory must be opened via this type.
//! `ReadOnlyProject` exposes only read methods at the API surface — there is
//! no way to obtain a writable handle on a project path.

use crate::error::{ScanError, ScanResult};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ReadOnlyProject {
    root: PathBuf,
    als: PathBuf,
}

impl ReadOnlyProject {
    /// `root` is the project folder, `als` is the .als file at the root of that
    /// folder (sibling to `Samples/`, `Backup/`, etc.).
    pub fn new(root: impl Into<PathBuf>, als: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            als: als.into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn als_path(&self) -> &Path {
        &self.als
    }

    /// Open the .als with explicit `read(true).write(false)`. The OS handle is
    /// read-only.
    pub fn open_als(&self) -> ScanResult<File> {
        OpenOptions::new()
            .read(true)
            .write(false)
            .open(&self.als)
            .map_err(|e| ScanError::io(self.als.clone(), e))
    }

    /// Open any file *inside the project root* read-only.
    pub fn open_within(&self, path: impl AsRef<Path>) -> ScanResult<File> {
        let path = path.as_ref();
        if !path_is_within(path, &self.root) {
            return Err(ScanError::Other(format!(
                "attempted to open a path outside the project root: {} (root: {})",
                path.display(),
                self.root.display()
            )));
        }
        OpenOptions::new()
            .read(true)
            .write(false)
            .open(path)
            .map_err(|e| ScanError::io(path.to_path_buf(), e))
    }
}

/// Returns true if `path` equals `root` or is contained within `root`. Uses
/// canonicalized comparison when possible, falling back to lexical compare.
pub fn path_is_within(path: &Path, root: &Path) -> bool {
    let p = dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let r = dunce::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    p == r || p.starts_with(&r)
}
