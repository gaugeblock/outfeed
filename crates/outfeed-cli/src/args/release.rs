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

    /// Bypass LLM release note synthesis and use deterministic formatting
    #[arg(long, visible_alias = "deterministic")]
    pub no_llm: bool,

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

    /// Bypass LLM release note synthesis and use deterministic formatting
    #[arg(long, visible_alias = "deterministic")]
    pub no_llm: bool,

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

    /// Orchestrate meta-harness suite release and generate suite.lock
    #[arg(long)]
    pub suite: bool,

    /// Optional explicit suite milestone version (e.g. 0.1.0-beta.3)
    #[arg(long)]
    pub suite_version: Option<String>,

    /// Dry run: simulate release without modifying disk, pushing, or publishing
    #[arg(long)]
    pub dry_run: bool,

    /// Automatically confirm suite release plan without interactive prompt
    #[arg(short = 'y', long = "yes")]
    pub yes: bool,

    /// Maximum concurrent release jobs (default: 1)
    #[arg(short = 'j', long = "jobs", default_value = "1")]
    pub jobs: usize,

    /// Skip installing binary locally
    #[arg(long)]
    pub skip_install: bool,

    /// Skip pushing to Git remote
    #[arg(long)]
    pub skip_push: bool,

    /// Skip publishing crates to registry
    #[arg(long)]
    pub skip_publish: bool,

    /// Skip Tier 4 verification test suite during release
    #[arg(long)]
    pub no_verify: bool,

    /// Bypass LLM release note synthesis and use deterministic formatting
    #[arg(long, visible_alias = "deterministic")]
    pub no_llm: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
