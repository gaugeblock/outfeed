//! Command handler for `outfeed clone`.

use std::path::Path;

use anyhow::{Context, Result};
use outfeed_core::{CloneOpts, GitProtocol, clone_suite};

use crate::args::clone::CloneArgs;
use crate::progress::create_reporter;

pub fn handle_clone(root: &Path, args: CloneArgs, quiet: bool) -> Result<()> {
    let protocol = match args.protocol.to_lowercase().as_str() {
        "https" => GitProtocol::Https,
        _ => GitProtocol::Ssh,
    };

    let opts = CloneOpts {
        all: args.all,
        locked: args.locked,
        protocol,
        dry_run: args.dry_run,
        install: args.install,
        sync_mcp: args.sync_mcp,
    };

    let target_refs: Vec<&str> = args.tools.iter().map(|s| s.as_str()).collect();

    if !args.json && !quiet {
        println!(
            "================================================================================"
        );
        println!(" [outfeed] Suite Repository Cloner & Bootstrapper");
        if args.dry_run {
            println!(" Mode: DRY RUN (simulating clone operations)");
        }
        println!(
            "================================================================================"
        );
    }

    let reporter = create_reporter(quiet, args.json);
    let summaries = clone_suite(root, &target_refs, &opts, reporter.as_ref())
        .context("Failed to clone suite repositories")?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&summaries)?);
    } else {
        println!(
            "✨ Suite clone complete across {} repositories:",
            summaries.len()
        );
        for s in &summaries {
            let status_desc = match &s.status {
                outfeed_core::CloneStatus::Cloned => "✅ cloned".to_string(),
                outfeed_core::CloneStatus::SkippedExisting => {
                    "⏩ existing checkout preserved".to_string()
                }
                outfeed_core::CloneStatus::PinnedCommit(c) => {
                    let short = if c.len() > 8 { &c[..8] } else { c };
                    format!("📌 pinned to commit {short}")
                }
            };
            println!("  * {}: {status_desc}", s.repo);
        }
    }

    Ok(())
}
