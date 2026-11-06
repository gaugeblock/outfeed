//! Namespace reservation CLI arguments (`OF-5`).

use clap::{Args, Subcommand};

/// Arguments for `outfeed reserve`.
#[derive(Debug, Args)]
pub struct ReserveArgs {
    /// Reservation subcommand to run.
    #[command(subcommand)]
    pub command: ReserveCommand,
}

/// The three reservation steps.
#[derive(Debug, Subcommand)]
pub enum ReserveCommand {
    /// List free and taken names from the registry (read only)
    Check(ReserveCheckArgs),
    /// Write standalone placeholder crates for the given names
    Scaffold(ReserveScaffoldArgs),
    /// Publish scaffolded placeholders to claim their names
    Publish(ReservePublishArgs),
}

/// Arguments for `outfeed reserve check`.
#[derive(Debug, Args)]
pub struct ReserveCheckArgs {
    /// Crate names to query
    #[arg(required = true)]
    pub names: Vec<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed reserve scaffold`.
#[derive(Debug, Args)]
pub struct ReserveScaffoldArgs {
    /// Crate names to scaffold placeholders for
    #[arg(required = true)]
    pub names: Vec<String>,

    /// Directory the placeholder crates are written into
    #[arg(long, default_value = "reserve-staging")]
    pub staging: String,

    /// Repository URL template; `{name}` is substituted with the crate name
    #[arg(long, default_value = "https://github.com/gaugeblock/{name}")]
    pub repository: String,

    /// Per-name repository override, repeatable: NAME=URL
    #[arg(long = "repo-override", value_name = "NAME=URL")]
    pub repo_override: Vec<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Arguments for `outfeed reserve publish`.
#[derive(Debug, Args)]
pub struct ReservePublishArgs {
    /// Crate names to publish placeholders for
    #[arg(required = true)]
    pub names: Vec<String>,

    /// Directory holding the scaffolded placeholders
    #[arg(long, default_value = "reserve-staging")]
    pub staging: String,

    /// Registry to publish to (defaults to crates.io)
    #[arg(long)]
    pub registry: Option<String>,

    /// Confirm that the placeholder versions should be published permanently
    #[arg(long)]
    pub confirm: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}
