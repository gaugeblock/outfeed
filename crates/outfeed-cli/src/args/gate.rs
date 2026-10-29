//! Arguments for `outfeed gate`.

use clap::Args;

/// Run the Tier 3 verification matrix as one command.
#[derive(Debug, Args)]
pub struct GateArgs {
    /// Repository directory to gate (default: ".")
    #[arg(default_value = ".")]
    pub root: String,

    /// Step to skip, repeatable: fmt, clippy, tests, contracts, lockfiles, docs
    #[arg(long = "skip", value_name = "STEP")]
    pub skip: Vec<String>,

    /// Output the report as JSON
    #[arg(long)]
    pub json: bool,
}
