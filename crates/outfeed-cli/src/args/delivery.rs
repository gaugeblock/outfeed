//! Delivery, packaging, and MCP CLI arguments.

use clap::Args;

/// Arguments for `outfeed install`.
#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Target tool name to install, or omit with --all
    #[arg(default_value = "")]
    pub tool: String,

    /// Install all 10 suite CLI tools
    #[arg(long)]
    pub all: bool,

    /// Build and install with debug profile instead of release
    #[arg(long)]
    pub debug: bool,

    /// Force reinstall even if version matches currently installed binary
    #[arg(long)]
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

/// Arguments for `outfeed mcp`.
#[derive(Debug, Args)]
pub struct McpArgs {}
