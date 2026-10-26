//! Workspace target cache hygiene CLI arguments.

use clap::Args;

/// Arguments for `outfeed clean`.
#[derive(Debug, Args)]
pub struct CleanArgs {
    /// Target repository path or workspace root (default: ".")
    #[arg(default_value = ".")]
    pub repo: String,

    /// Full clean of entire target/ directory across all workspaces
    #[arg(long)]
    pub all: bool,

    /// Clean documentation artifacts (target/doc)
    #[arg(long)]
    pub doc: bool,

    /// Clean release artifacts (target/release)
    #[arg(long)]
    pub release: bool,

    /// Audit and report sizes without deleting files
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed prune`.
#[derive(Debug, Args)]
pub struct PruneArgs {
    /// Target repository path or workspace root (default: ".")
    #[arg(default_value = ".")]
    pub repo: String,

    /// Delete test and build artifacts older than N days (default: 3)
    #[arg(long, default_value = "3")]
    pub keep_days: u32,

    /// Audit and report prune candidates without deleting
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
