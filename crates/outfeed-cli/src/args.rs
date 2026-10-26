//! Command line arguments for outfeed.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Outfeed — Neurosymbolic release gate, AST drift inspector & changelog synthesizer.
#[derive(Debug, Parser)]
#[command(name = "outfeed", version, about, long_about = None)]
pub struct Cli {
    /// Target repository directory (defaults to current dir)
    #[arg(short, long, global = true)]
    pub dir: Option<PathBuf>,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands in outfeed.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Plan a release: evaluate unreleased commits, calculate SemVer bump, and preview changelog
    Plan(PlanArgs),

    /// Apply a release: bump manifests, update changelog, run Tier 4 gate, commit, tag, and update Storypole
    Apply(ApplyArgs),

    /// Check release status and unreleased commits across one or all suite repositories
    Status(StatusArgs),

    /// Compile and install CLI binaries into global PATH (~/.cargo/bin)
    Install(InstallArgs),

    /// Push branch and release tags to Git remote
    Push(PushArgs),

    /// Publish member crates to crates.io in topological order with --dry-run gate
    Publish(PublishArgs),

    /// Orchestrated release pipeline: plan -> apply -> install -> push -> publish
    Release(ReleaseArgs),

    /// Start the JSON-RPC stdio MCP server for Antigravity & Cline
    Mcp(McpArgs),
}

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

    /// Scan all 9 member repositories in the meta-harness
    #[arg(long)]
    pub all: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed mcp`.
#[derive(Debug, Args)]
pub struct McpArgs {}

/// Arguments for `outfeed install`.
#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Target repository or tool name (e.g. "sawhorse")
    #[arg(default_value = "all")]
    pub tool: String,

    /// Install all tools across the suite
    #[arg(long)]
    pub all: bool,

    /// Build with debug profile instead of release
    #[arg(long)]
    pub debug: bool,

    /// Force rebuild and install even if versions match
    #[arg(short, long)]
    pub force: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed push`.
#[derive(Debug, Args)]
pub struct PushArgs {
    /// Target repository path or name
    #[arg(default_value = ".")]
    pub repo: String,

    /// Push all suite repositories
    #[arg(long)]
    pub all: bool,

    /// Git remote name (default: "origin")
    #[arg(short, long, default_value = "origin")]
    pub remote: String,

    /// Dry run: simulate push without contacting remote
    #[arg(long)]
    pub dry_run: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed publish`.
#[derive(Debug, Args)]
pub struct PublishArgs {
    /// Target repository path or name
    #[arg(default_value = ".")]
    pub repo: String,

    /// Target registry (default: crates.io)
    #[arg(long)]
    pub registry: Option<String>,

    /// Dry run: simulate packaging without uploading
    #[arg(long)]
    pub dry_run: bool,

    /// Delay in seconds between intra-repo dependent crates (default: 15)
    #[arg(long, default_value = "15")]
    pub delay_secs: u64,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed release`.
#[derive(Debug, Args)]
pub struct ReleaseArgs {
    /// Target repository path or name
    pub repo: String,

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

