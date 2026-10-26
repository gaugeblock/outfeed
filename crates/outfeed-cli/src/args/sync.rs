//! Command-line arguments for `outfeed sync`.

use clap::Args;

/// Arguments for `outfeed sync`.
#[derive(Debug, Args)]
pub struct SyncArgs {
    /// Target tool name(s) to sync (e.g. miter, storypole)
    #[arg(num_args = 0..)]
    pub tools: Vec<String>,

    /// Sync or audit all 10 suite repositories
    #[arg(long)]
    pub all: bool,

    /// Align local checkouts to exact commits recorded in suite.lock
    #[arg(long)]
    pub locked: bool,

    /// Only audit repository status and divergence without modifying checkouts
    #[arg(long)]
    pub status: bool,

    /// Dry run: simulate sync operations without modifying checkouts
    #[arg(long)]
    pub dry_run: bool,

    /// Output results as JSON
    #[arg(long)]
    pub json: bool,
}
