//! Command line arguments for outfeed.

pub mod delivery;
pub mod hygiene;
pub mod release;

pub use delivery::{InstallArgs, McpArgs, PublishArgs, PushArgs};
pub use hygiene::{CleanArgs, PruneArgs};
pub use release::{ApplyArgs, PlanArgs, ReleaseArgs, StatusArgs};

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Outfeed — Neurosymbolic release gate, AST drift inspector & changelog synthesizer.
#[derive(Debug, Parser)]
#[command(name = "outfeed", version, about, long_about = None)]
pub struct Cli {
    /// Target repository directory (defaults to current dir)
    #[arg(short, long, global = true)]
    pub dir: Option<PathBuf>,

    /// Increase logging and child command verbosity (-v for child output, -vv for debug)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress progress spinners and non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

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

    /// Clean target compilation artifacts while preserving release binaries
    Clean(CleanArgs),

    /// Prune stale test binaries and old build artifacts
    Prune(PruneArgs),

    /// Start the JSON-RPC stdio MCP server for Antigravity & Cline
    Mcp(McpArgs),
}
