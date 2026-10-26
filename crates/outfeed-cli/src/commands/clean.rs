//! Handlers for `outfeed clean` and `outfeed prune` subcommands.

use anyhow::Result;
use outfeed_core::clean::{clean_workspace, format_bytes, prune_workspace};
use outfeed_core::model::{CleanOpts, PruneOpts};
use std::path::Path;

use crate::args::{CleanArgs, PruneArgs};

/// Handles the `outfeed clean` command.
pub fn handle_clean(args: &CleanArgs, workspace_root: &Path) -> Result<()> {
    let opts = CleanOpts {
        clean_all: args.all,
        clean_doc: args.doc,
        clean_release: args.release,
        dry_run: args.dry_run,
    };

    let report = clean_workspace(workspace_root, &opts)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("================================================================================");
    println!(" [outfeed] Target Cache Hygiene Report");
    println!("================================================================================");
    println!(" Target Directory: {}", workspace_root.join("target").display());
    println!("   - Debug Artifacts:   {}", format_bytes(report.initial_usage.debug_bytes));
    println!("   - Release Binaries:  {}", format_bytes(report.initial_usage.release_bytes));
    println!("   - Doc Artifacts:     {}", format_bytes(report.initial_usage.doc_bytes));
    println!("   - Total Usage:       {}", format_bytes(report.initial_usage.total_bytes));
    println!("--------------------------------------------------------------------------------");

    if report.dry_run {
        println!(" [Dry Run] Estimated reclaimable space: {}", format_bytes(report.reclaimed_bytes));
        println!(" Paths that would be removed:");
        for p in &report.paths_removed {
            println!("   - {p}");
        }
    } else {
        println!(" [Cleaned] Successfully reclaimed: {}", format_bytes(report.reclaimed_bytes));
        for p in &report.paths_removed {
            println!("   - Removed: {p}");
        }
    }
    println!("================================================================================");

    Ok(())
}

/// Handles the `outfeed prune` command.
pub fn handle_prune(args: &PruneArgs, workspace_root: &Path) -> Result<()> {
    let opts = PruneOpts {
        keep_days: args.keep_days,
        dry_run: args.dry_run,
    };

    let report = prune_workspace(workspace_root, &opts)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("================================================================================");
    println!(" [outfeed] Cache Prune Report (Keep Days: {})", args.keep_days);
    println!("================================================================================");
    if report.dry_run {
        println!(" [Dry Run] Prune candidates found: {}", report.paths_removed.len());
        println!(" Estimated reclaimable space: {}", format_bytes(report.reclaimed_bytes));
        for p in &report.paths_removed {
            println!("   - {p}");
        }
    } else {
        println!(" [Pruned] Pruned {} artifact(s)", report.paths_removed.len());
        println!(" Successfully reclaimed: {}", format_bytes(report.reclaimed_bytes));
    }
    println!("================================================================================");

    Ok(())
}
