//! Error types for outfeed operations.

use thiserror::Error;

/// Core error enum representing failure modes across outfeed.
#[derive(Debug, Error)]
pub enum OutfeedError {
    /// Git operation failed.
    #[error("Git operation failed: {0}")]
    Git(String),

    /// Working tree is dirty.
    #[error("Working tree is dirty: {0}")]
    DirtyWorkingTree(String),

    /// Invalid branch for release.
    #[error("Invalid branch '{0}': releases must occur on 'main'")]
    InvalidBranch(String),

    /// SemVer parsing or calculation failure.
    #[error("SemVer error: {0}")]
    SemVer(String),

    /// Manifest parsing or modification failure.
    #[error("Manifest error in {0}: {1}")]
    Manifest(String, String),

    /// Changelog parsing or formatting failure.
    #[error("Changelog error: {0}")]
    Changelog(String),

    /// Storypole integration failure.
    #[error("Storypole integration error: {0}")]
    Storypole(String),

    /// LLM synthesis failure.
    #[error("Synthesis error: {0}")]
    Synthesis(String),

    /// Binary installation failure.
    #[error("Install error: {0}")]
    Install(String),

    /// Git push failure.
    #[error("Push error: {0}")]
    Push(String),

    /// Registry publish failure.
    #[error("Publish error: {0}")]
    Publish(String),

    /// Cache hygiene or clean failure.
    #[error("Clean error: {0}")]
    Clean(String),

    /// Insufficient available disk space for release.
    #[error(
        "Insufficient disk space: available {available}, required {required}. Run 'outfeed clean --all' to reclaim compilation cache."
    )]
    InsufficientDiskSpace {
        /// Human-readable available bytes string.
        available: String,
        /// Human-readable required bytes string.
        required: String,
    },

    /// General I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
