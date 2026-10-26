//! Release planning and status CLI arguments.

use clap::Args;

/// Arguments for `outfeed plan`.
#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Target repository path or name
    #[arg(default_value = ".")]
    pub repo: String,

    /// Optional explicit version string to use instead of computed bump
    #[arg(long)]
    pub override_version: Option<String>,

    /// Skip Storypole ledger inspection
    #[arg(long)]
    pub skip_storypole: bool,

    /// Output full plan as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed apply`.
#[derive(Debug, Args)]
pub struct ApplyArgs {
    /// Target repository path or name
    #[arg(default_value = ".")]
    pub repo: String,

    /// Optional explicit version string
    #[arg(long)]
    pub override_version: Option<String>,

    /// Skip Tier 4 verification gate before committing
    #[arg(long)]
    pub no_verify: bool,

    /// Skip Storypole chore ticket creation and verification
    #[arg(long)]
    pub skip_storypole: bool,

    /// Dry run: plan and display diffs without writing to disk or making git commits
    #[arg(long)]
    pub dry_run: bool,

    /// Output result as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed status`.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Target repository path or name
    #[arg(default_value = ".")]
    pub repo: String,

    /// Check all repositories across the workspace
    #[arg(long)]
    pub all: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed release`.
#[derive(Debug, Args)]
pub struct ReleaseArgs {
    /// Target repository path or name
    #[arg(default_value = ".")]
    pub repo: String,

    /// Release all suite repositories in topological order
    #[arg(long)]
    pub all: bool,

    /// Dry run: simulate release without modifying disk, pushing, or publishing
    #[arg(long)]
    pub dry_run: bool,

    /// Skip installing binary locally
    #[arg(long)]
    pub skip_install: bool,

    /// Skip pushing to Git remote
    #[arg(long)]
    pub skip_push: bool,

    /// Skip publishing crates to registry
    #[arg(long)]
    pub skip_publish: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
