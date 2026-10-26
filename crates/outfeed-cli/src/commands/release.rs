//! Command handler for `outfeed release` macro orchestrator.

use anyhow::{Context, Result};
use outfeed_core::{ReleaseOpts, SuiteReleaseOpts, release_all, release_repo, release_suite};
use std::path::Path;

use crate::args::ReleaseArgs;

pub async fn handle_release(root: &Path, args: ReleaseArgs) -> Result<()> {
    if args.suite || args.repo == "suite" {
        return handle_suite_release(root, &args).await;
    }

    let opts = ReleaseOpts {
        dry_run: args.dry_run,
        skip_install: args.skip_install,
        skip_push: args.skip_push,
        skip_publish: args.skip_publish,
    };

    if args.all {
        println!(
            "================================================================================"
        );
        println!(" [outfeed] Orchestrated Release Pipeline for ALL Suite Repositories");
        if args.dry_run {
            println!(" Mode: DRY RUN (simulating plan -> apply -> install -> push -> publish)");
        }
        println!(
            "================================================================================"
        );

        let summaries = release_all(root, &opts)
            .await
            .context("Suite-wide release pipeline failed")?;

        if args.json {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &summaries
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "repo": s.repo_name,
                                "target_version": s.target_version,
                                "installed": s.installed,
                                "pushed": s.pushed,
                                "published": s.published_crates,
                            })
                        })
                        .collect::<Vec<_>>()
                )?
            );
        } else if summaries.is_empty() {
            println!("✅ All suite repositories are clean. No releases needed.");
        } else {
            println!(
                "✨ Suite release pipeline complete. Released {} repositories:",
                summaries.len()
            );
            for s in &summaries {
                println!(
                    "  * {} v{}: installed={}, pushed={}, published={}",
                    s.repo_name,
                    s.target_version,
                    s.installed,
                    s.pushed,
                    s.published_crates.len()
                );
            }
        }
        return Ok(());
    }

    println!("================================================================================");
    println!(
        " [outfeed] Orchestrated Release Pipeline for '{}'",
        args.repo
    );
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
                println!(
                    "{}",
                    serde_json::json!({
                        "repo": s.repo_name,
                        "target_version": s.target_version,
                        "installed": s.installed,
                        "pushed": s.pushed,
                        "published": s.published_crates,
                    })
                );
            } else {
                println!(
                    "✨ Release pipeline complete for {} v{}",
                    s.repo_name, s.target_version
                );
                println!(
                    "  - Manifests & Changelog: committed & tagged (v{})",
                    s.target_version
                );
                println!(
                    "  - Local binary install:   {}",
                    if s.installed {
                        "✅ installed"
                    } else {
                        "⏩ skipped"
                    }
                );
                println!(
                    "  - Git remote push:        {}",
                    if s.pushed {
                        "✅ pushed"
                    } else {
                        "⏩ skipped"
                    }
                );
                println!(
                    "  - Crates.io publication:  {}",
                    if !s.published_crates.is_empty() {
                        format!("✅ published ({} crates)", s.published_crates.len())
                    } else {
                        "⏩ skipped".to_string()
                    }
                );
            }
        }
        None => {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({ "repo": args.repo, "status": "clean" })
                );
            } else {
                println!(
                    "✅ No unreleased commits found for {}. Repository is clean.",
                    args.repo
                );
            }
        }
    }

    Ok(())
}

async fn handle_suite_release(root: &Path, args: &ReleaseArgs) -> Result<()> {
    if !args.json {
        println!(
            "================================================================================"
        );
        println!(" [outfeed] Orchestrated Meta-Harness Suite Release");
        if args.dry_run {
            println!(" Mode: DRY RUN (simulating suite.lock generation, changelog update & tag)");
        }
        println!(
            "================================================================================"
        );
    }

    let suite_opts = SuiteReleaseOpts {
        target_version: args.suite_version.clone(),
        dry_run: args.dry_run,
        skip_push: args.skip_push,
    };

    let summary = release_suite(root, &suite_opts).context("Meta-harness suite release failed")?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("✨ Meta-harness suite release complete: {}", summary.tag);
        println!("  - Target version: v{}", summary.target_version);
        println!(
            "  - Pinned tools:   {} member repositories",
            summary.tool_count
        );
        println!("  - Manifest:       {}", summary.lock_path);
        println!(
            "  - Commit & Tag:   {}",
            if summary.committed {
                format!("✅ committed & tagged ({})", summary.tag)
            } else {
                "⏩ skipped (dry run)".to_string()
            }
        );
        println!(
            "  - Remote push:    {}",
            if summary.pushed {
                "✅ pushed"
            } else {
                "⏩ skipped"
            }
        );
    }

    Ok(())
}
