//! Command-line arguments for `outfeed clone`.

use clap::Args;

/// Arguments for `outfeed clone`.
#[derive(Debug, Args)]
pub struct CloneArgs {
    /// Target tool name(s) to clone (e.g. miter, storypole)
    #[arg(num_args = 0..)]
    pub tools: Vec<String>,

    /// Clone all 10 suite repositories
    #[arg(long)]
    pub all: bool,

    /// Align checkouts to exact commit SHAs pinned in suite.lock
    #[arg(long)]
    pub locked: bool,

    /// Git clone protocol: ssh or https (default: ssh)
    #[arg(long, default_value = "ssh")]
    pub protocol: String,

    /// Automatically compile and install binaries to ~/.cargo/bin after clone
    #[arg(long)]
    pub install: bool,

    /// Synchronize Antigravity and Cline MCP configs after installation
    #[arg(long)]
    pub sync_mcp: bool,

    /// Dry run: simulate clone without modifying disk
    #[arg(long)]
    pub dry_run: bool,

    /// Output results as JSON
    #[arg(long)]
    pub json: bool,
}
