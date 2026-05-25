//! Cooperative cancellation primitive — set by Stop/X buttons in the UI, checked
//! between projects, between sample records, and between copies.

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Default)]
pub struct CancellationToken {
    flag: AtomicBool,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            flag: AtomicBool::new(false),
        }
    }

    /// Mark the run as cancelled. Idempotent.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Reset before starting a new scan. Single-call sites (commands.rs).
    pub fn reset(&self) {
        self.flag.store(false, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Relaxed)
    }
}
