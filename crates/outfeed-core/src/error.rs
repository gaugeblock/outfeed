//! Error types for outfeed operations.

use thiserror::Error;

/// Core error enum representing failure modes across outfeed.
#[derive(Debug, Error)]
pub enum OutfeedError {
    /// Git operation failed.
    #[error("Git operation failed: {0}")]
    Git(String),

    /// Nothing was found to check (`GB-327`).
    ///
    /// A check that reports success on an empty set is indistinguishable from a check that passed, and
    /// `outfeed lockfiles` run inside a member repository found zero repositories, printed
    /// "All 0 repositories track a fresh Cargo.lock" and exited 0. Discovery finding nothing is a
    /// failure to look, not a pass.
    #[error(
        "found no repositories to check under {root}: a directory counts when it holds Cargo.toml and a \
         .git entry. Run from a suite root, or pass --path"
    )]
    NoRepositories {
        /// The directory discovery searched.
        root: String,
    },

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

    /// A gate step could not be spawned at all.
    #[error("Gate error: {0}")]
    Gate(String),

    /// Registry publish failure.
    #[error("Publish error: {0}")]
    Publish(String),

    /// Namespace reservation failure.
    #[error("Reserve error: {0}")]
    Reserve(String),

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
