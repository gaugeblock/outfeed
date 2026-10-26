//! Command handler for `outfeed sync`.

use std::path::Path;

use anyhow::{Context, Result};
use outfeed_core::{BranchDivergence, SyncOpts, sync_suite};

use crate::args::sync::SyncArgs;
use crate::progress::create_reporter;

pub fn handle_sync(root: &Path, args: SyncArgs, quiet: bool) -> Result<()> {
    let opts = SyncOpts {
        all: args.all,
        locked: args.locked,
        status_only: args.status,
        dry_run: args.dry_run,
    };

    let target_refs: Vec<&str> = args.tools.iter().map(|s| s.as_str()).collect();

    if !args.json && !quiet {
        println!(
            "================================================================================"
        );
        if args.status {
            println!(" [outfeed] Multi-Repository Workspace Status Audit");
        } else {
            println!(" [outfeed] Multi-Repository Workspace Synchronization");
            if args.dry_run {
                println!(" Mode: DRY RUN (simulating pull / pin operations)");
            }
        }
        println!(
            "================================================================================"
        );
    }

    let reporter = create_reporter(quiet, args.json);
    let statuses = sync_suite(root, &target_refs, &opts, reporter.as_ref())
        .context("Failed to sync/audit suite repositories")?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&statuses)?);
    } else {
        println!(
            "{:<12} {:<15} {:<10} {:<15} {:<25}",
            "REPOSITORY", "BRANCH", "DIRTY", "DIVERGENCE", "ACTION / STATUS"
        );
        println!("{}", "-".repeat(80));

        for s in &statuses {
            let div_str = match &s.divergence {
                BranchDivergence::UpToDate => "in sync".to_string(),
                BranchDivergence::Ahead(n) => format!("ahead +{n}"),
                BranchDivergence::Behind(n) => format!("behind -{n}"),
                BranchDivergence::Diverged { ahead, behind } => format!("+{ahead}/-{behind}"),
                BranchDivergence::Detached => "detached".to_string(),
                BranchDivergence::Untracked => "untracked".to_string(),
            };

            let dirty_str = if s.dirty { "⚠️  dirty" } else { "clean" };
            let action_str = s.action_taken.as_deref().unwrap_or("-");

            println!(
                "{:<12} {:<15} {:<10} {:<15} {:<25}",
                s.repo, s.branch, dirty_str, div_str, action_str
            );
        }
    }

    Ok(())
}
