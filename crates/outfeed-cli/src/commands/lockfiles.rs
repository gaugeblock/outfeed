//! Command handler for `outfeed lockfiles`.

use std::path::Path;

use anyhow::{Context, Result};
use outfeed_core::{check_lockfiles, format_lockfile_report};

use crate::args::LockfileArgs;

pub fn handle_lockfiles(args: &LockfileArgs, workspace_root: &Path) -> Result<()> {
    let root = crate::commands::resolve_repo_dir(workspace_root, &args.root);

    let report = check_lockfiles(&root)
        .with_context(|| format!("Lockfile check failed for {}", root.display()))?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print!("{}", format_lockfile_report(&report));
    }

    if !report.is_ok() {
        anyhow::bail!(
            "{} repository(ies) do not track a fresh Cargo.lock",
            report.failures().len()
        );
    }
    Ok(())
}
