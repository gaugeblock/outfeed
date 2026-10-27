//! Command handler for `outfeed publish`.

use anyhow::{Context, Result};
use outfeed_core::{
    PublishOpts, publish_all_with_progress, publish_repo_with_progress, resolve_publish_order,
};
use std::path::Path;

use super::resolve_repo_dir;
use crate::args::PublishArgs;
use crate::progress::create_reporter;

pub fn handle_publish(root: &Path, args: PublishArgs, _verbose: u8, quiet: bool) -> Result<()> {
    let opts = PublishOpts {
        registry: args.registry.clone(),
        dry_run: args.dry_run,
        delay_secs: args.delay_secs,
    };
    let reporter = create_reporter(quiet, args.json);

    if args.all {
        if args.dry_run && !quiet && !args.json {
            println!(
                "[DRY RUN] Simulating cargo publish across all suite repositories in topological order"
            );
        }

        let results = publish_all_with_progress(root, &opts, reporter.as_ref())
            .context("Failed to publish all suite repositories")?;

        if args.json {
            println!(
                "{}",
                serde_json::json!({
                    "all": true,
                    "dry_run": args.dry_run,
                    "results": results.iter().map(|(r, c)| {
                        serde_json::json!({ "repo": r, "published": c })
                    }).collect::<Vec<_>>(),
                })
            );
        } else if !args.dry_run && !quiet {
            println!("📦 Successfully completed suite publication:");
            for (repo, crates) in &results {
                println!("  {} ({} crate(s)):", repo, crates.len());
                for name in crates {
                    println!("    - {name}");
                }
            }
        }

        return Ok(());
    }

    let repo_dir = resolve_repo_dir(root, &args.repo);
    let opts = PublishOpts {
        registry: args.registry.clone(),
        dry_run: args.dry_run,
        delay_secs: args.delay_secs,
    };
    let reporter = create_reporter(quiet, args.json);

    let order = resolve_publish_order(&repo_dir).with_context(|| {
        format!(
            "Failed to resolve dependency order for {}",
            repo_dir.display()
        )
    })?;

    if args.dry_run && !quiet && !args.json {
        println!(
            "[DRY RUN] Simulating cargo publish for {} in topological order:",
            args.repo
        );
        for (i, m) in order.iter().enumerate() {
            println!("  {}. {} (deps: {:?})", i + 1, m.name, m.internal_deps);
        }
    }

    let published = publish_repo_with_progress(&repo_dir, &opts, reporter.as_ref())
        .with_context(|| format!("Failed to publish crates for {}", repo_dir.display()))?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "repo": args.repo,
                "dry_run": args.dry_run,
                "published": published,
            })
        );
    } else if !args.dry_run && !quiet {
        println!(
            "📦 Successfully published {} crate(s) to registry:",
            published.len()
        );
        for name in published {
            println!("  - {name}");
        }
    }

    Ok(())
}
