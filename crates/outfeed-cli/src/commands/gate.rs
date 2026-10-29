//! Command handler for `outfeed gate`.

use std::path::Path;

use anyhow::{Result, anyhow};
use outfeed_core::progress::NoopProgress;
use outfeed_core::{format_gate_report, run_gate};

use crate::args::GateArgs;

pub fn handle_gate(args: &GateArgs, workspace_root: &Path) -> Result<()> {
    // The positional defaults to ".", which is always a directory, so `resolve_repo_dir` would
    // return the *current* directory and silently discard a global `--dir`. Honour the global
    // directory whenever no positional was given.
    let root = if args.root == "." {
        workspace_root.to_path_buf()
    } else {
        crate::commands::resolve_repo_dir(workspace_root, &args.root)
    };
    let report = run_gate(&root, &args.skip, &NoopProgress)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", format_gate_report(&report));
    }

    match report.first_failure() {
        Some(failure) => Err(anyhow!(
            "gate failed at step '{}': {}",
            failure.name,
            failure.detail
        )),
        None => Ok(()),
    }
}
