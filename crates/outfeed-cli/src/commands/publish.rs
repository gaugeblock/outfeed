//! Command handler for `outfeed publish`.

use std::path::Path;
use anyhow::{Context, Result};
use outfeed_core::{publish_repo_with_progress, resolve_publish_order, PublishOpts};

use crate::args::PublishArgs;
use crate::progress::create_reporter;
use super::resolve_repo_dir;

pub fn handle_publish(
    root: &Path,
    args: PublishArgs,
    _verbose: u8,
    quiet: bool,
) -> Result<()> {
    let repo_dir = resolve_repo_dir(root, &args.repo);
    let opts = PublishOpts {
        registry: args.registry.clone(),
        dry_run: args.dry_run,
        delay_secs: args.delay_secs,
    };
    let reporter = create_reporter(quiet, args.json);

    let order = resolve_publish_order(&repo_dir)
        .with_context(|| format!("Failed to resolve dependency order for {}", repo_dir.display()))?;

    if args.dry_run && !quiet && !args.json {
        println!("[DRY RUN] Simulating cargo publish for {} in topological order:", args.repo);
        for (i, m) in order.iter().enumerate() {
            println!("  {}. {} (deps: {:?})", i + 1, m.name, m.internal_deps);
        }
    }

    let published = publish_repo_with_progress(&repo_dir, &opts, reporter.as_ref())
        .with_context(|| format!("Failed to publish crates for {}", repo_dir.display()))?;

    if args.json {
        println!("{}", serde_json::json!({
            "repo": args.repo,
            "dry_run": args.dry_run,
            "published": published,
        }));
    } else if !args.dry_run && !quiet {
        println!("📦 Successfully published {} crate(s) to registry:", published.len());
        for name in published {
            println!("  - {name}");
        }
    }

    Ok(())
}
