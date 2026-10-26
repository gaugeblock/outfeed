//! Command handler for `outfeed push`.

use std::path::Path;
use anyhow::{Context, Result};
use outfeed_core::{push_all, push_repo, PushOpts};

use crate::args::PushArgs;
use super::resolve_repo_dir;

pub fn handle_push(root: &Path, args: PushArgs) -> Result<()> {
    let opts = PushOpts {
        remote: args.remote.clone(),
        dry_run: args.dry_run,
    };

    if args.all {
        let results = push_all(root, &opts).context("Failed to push suite repositories")?;
        if args.json {
            println!("{}", serde_json::to_string_pretty(&results.iter().map(|(r, o)| {
                serde_json::json!({
                    "repo": r,
                    "remote": o.remote,
                    "branch": o.branch,
                    "tag": o.tag,
                    "dry_run": o.dry_run,
                })
            }).collect::<Vec<_>>())?);
        } else {
            for (repo, outcome) in results {
                let tag_str = outcome.tag.as_deref().unwrap_or("none");
                let sim_str = if outcome.dry_run { " [DRY RUN]" } else { "" };
                println!("🚀 {repo}{sim_str}: pushed branch '{}' and tag '{tag_str}' to {}", outcome.branch, outcome.remote);
            }
        }
        return Ok(());
    }

    let repo_dir = resolve_repo_dir(root, &args.repo);
    let outcome = push_repo(&repo_dir, &opts)
        .with_context(|| format!("Failed to push {}", repo_dir.display()))?;

    if args.json {
        println!("{}", serde_json::json!({
            "repo": args.repo,
            "remote": outcome.remote,
            "branch": outcome.branch,
            "tag": outcome.tag,
            "dry_run": outcome.dry_run,
        }));
    } else {
        let tag_str = outcome.tag.as_deref().unwrap_or("none");
        let sim_str = if outcome.dry_run { " [DRY RUN]" } else { "" };
        println!("🚀 {}{sim_str}: pushed branch '{}' and tag '{tag_str}' to {}", args.repo, outcome.branch, outcome.remote);
    }

    Ok(())
}

