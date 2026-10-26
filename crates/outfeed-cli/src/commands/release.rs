//! Command handler for `outfeed release` macro orchestrator.

use std::path::Path;
use anyhow::{Context, Result};
use outfeed_core::{release_repo, ReleaseOpts};

use crate::args::ReleaseArgs;

pub async fn handle_release(root: &Path, args: ReleaseArgs) -> Result<()> {
    let opts = ReleaseOpts {
        dry_run: args.dry_run,
        skip_install: args.skip_install,
        skip_push: args.skip_push,
        skip_publish: args.skip_publish,
    };

    println!("================================================================================");
    println!(" [outfeed] Orchestrated Release Pipeline for '{}'", args.repo);
    if args.dry_run {
        println!(" Mode: DRY RUN (simulating plan -> apply -> install -> push -> publish)");
    }
    println!("================================================================================");

    let summary = release_repo(root, &args.repo, &opts)
        .await
        .with_context(|| format!("Release pipeline failed for {}", args.repo))?;

    match summary {
        Some(s) => {
            if args.json {
                println!("{}", serde_json::json!({
                    "repo": s.repo_name,
                    "target_version": s.target_version,
                    "installed": s.installed,
                    "pushed": s.pushed,
                    "published": s.published_crates,
                }));
            } else {
                println!("✨ Release pipeline complete for {} v{}", s.repo_name, s.target_version);
                println!("  - Manifests & Changelog: committed & tagged (v{})", s.target_version);
                println!("  - Local binary install:   {}", if s.installed { "✅ installed" } else { "⏩ skipped" });
                println!("  - Git remote push:        {}", if s.pushed { "✅ pushed" } else { "⏩ skipped" });
                println!("  - Crates.io publication:  {}", if !s.published_crates.is_empty() { format!("✅ published ({} crates)", s.published_crates.len()) } else { "⏩ skipped".to_string() });
            }
        }
        None => {
            if args.json {
                println!("{}", serde_json::json!({ "repo": args.repo, "status": "clean" }));
            } else {
                println!("✅ No unreleased commits found for {}. Repository is clean.", args.repo);
            }
        }
    }

    Ok(())
}

