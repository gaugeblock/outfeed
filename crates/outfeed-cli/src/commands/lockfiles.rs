//! Command handler for `outfeed lockfiles`.

use std::path::Path;

use anyhow::{Context, Result};
use outfeed_core::format_lockfile_report;

use crate::args::LockfileArgs;

pub fn handle_lockfiles(args: &LockfileArgs, workspace_root: &Path) -> Result<()> {
    let root = crate::commands::resolve_repo_dir(workspace_root, &args.root);

    // Resolution lives in the core crate, which knows both the discovery rule and suite membership
    // (`GB-327`); the handler's job is to say which directory was actually checked.
    let (checked_root, report) = outfeed_core::check_lockfiles_resolved(&root)
        .with_context(|| format!("Lockfile check failed for {}", root.display()))?;
    if checked_root != root {
        // Not only in a log: the report below describes a different directory than the one asked for.
        eprintln!(
            "outfeed: {} holds no repositories; checked the suite at {}",
            root.display(),
            checked_root.display()
        );
    }

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
