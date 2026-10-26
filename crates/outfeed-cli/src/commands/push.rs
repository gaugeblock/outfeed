//! Command handler for `outfeed push`.

use anyhow::{Context, Result};
use outfeed_core::{PushOpts, push_all_with_progress, push_repo_with_progress};
use std::path::Path;

use super::resolve_repo_dir;
use crate::args::PushArgs;
use crate::progress::create_reporter;

pub fn handle_push(root: &Path, args: PushArgs, verbose: u8, quiet: bool) -> Result<()> {
    let opts = PushOpts {
        remote: args.remote.clone(),
        dry_run: args.dry_run,
    };
    let reporter = create_reporter(quiet, args.json);

    if args.all {
        let results = push_all_with_progress(root, &opts, reporter.as_ref())
            .context("Failed to push suite repositories")?;
        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &results
                        .iter()
                        .map(|(r, o)| {
                            serde_json::json!({
                                "repo": r,
                                "remote": o.remote,
                                "branch": o.branch,
                                "tag": o.tag,
                                "dry_run": o.dry_run,
                            })
                        })
                        .collect::<Vec<_>>()
                )?
            );
        } else if verbose > 0 {
            for (repo, outcome) in &results {
                if !outcome.output.is_empty() {
                    let formatted = outcome
                        .output
                        .lines()
                        .map(|l| format!("  [{repo}] {l}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    println!("{formatted}");
                }
            }
        }
        return Ok(());
    }

    let repo_dir = resolve_repo_dir(root, &args.repo);
    let outcome = push_repo_with_progress(&repo_dir, &opts, reporter.as_ref())
        .with_context(|| format!("Failed to push {}", repo_dir.display()))?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "repo": args.repo,
                "remote": outcome.remote,
                "branch": outcome.branch,
                "tag": outcome.tag,
                "dry_run": outcome.dry_run,
            })
        );
    } else if verbose > 0 && !outcome.output.is_empty() {
        let formatted = outcome
            .output
            .lines()
            .map(|l| format!("  [{}] {l}", args.repo))
            .collect::<Vec<_>>()
            .join("\n");
        println!("{formatted}");
    }

    Ok(())
}
