//! Models for disk usage, cache hygiene, clean, and prune.

use serde::{Deserialize, Serialize};

/// Breakdown of disk usage across target directories.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskUsage {
    /// Bytes occupied by debug artifacts (target/debug).
    pub debug_bytes: u64,
    /// Bytes occupied by release artifacts (target/release).
    pub release_bytes: u64,
    /// Bytes occupied by documentation artifacts (target/doc).
    pub doc_bytes: u64,
    /// Other target bytes.
    pub other_bytes: u64,
    /// Total target bytes.
    pub total_bytes: u64,
}

/// Options controlling workspace cache cleaning.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanOpts {
    /// Full target clean across all member workspaces.
    pub clean_all: bool,
    /// Clean only documentation artifacts (target/doc).
    pub clean_doc: bool,
    /// Clean release artifacts (target/release).
    pub clean_release: bool,
    /// Audit and report sizes without deleting files.
    pub dry_run: bool,
}

/// Options controlling age-based cache pruning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneOpts {
    /// Delete test and build artifacts older than N days.
    pub keep_days: u32,
    /// Audit and report prune candidates without deleting.
    pub dry_run: bool,
}

impl Default for PruneOpts {
    fn default() -> Self {
        Self {
            keep_days: 3,
            dry_run: false,
        }
    }
}

/// Summary report returned after a clean or prune operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanReport {
    /// Disk usage before operation.
    pub initial_usage: DiskUsage,
    /// Estimated or actual bytes reclaimed.
    pub reclaimed_bytes: u64,
    /// Target paths modified or removed.
    pub paths_removed: Vec<String>,
    /// Whether this was a dry run.
    pub dry_run: bool,
}
