//! Command handler for `outfeed release` macro orchestrator.

use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};
use outfeed_core::{
    ReleaseOpts, check_suite_preflight, execute_suite_plan, plan_suite_releases, release_repo,
};

use super::suite_release::handle_suite_release;
use crate::args::ReleaseArgs;
use crate::progress::create_reporter;

pub async fn handle_release(root: &Path, args: ReleaseArgs) -> Result<()> {
    if args.suite || args.repo == "suite" {
        return handle_suite_release(root, &args).await;
    }

    let opts = ReleaseOpts {
        dry_run: args.dry_run,
        skip_install: args.skip_install,
        skip_push: args.skip_push,
        skip_publish: args.skip_publish,
        no_llm: args.no_llm,
        no_verify: args.no_verify,
        jobs: args.jobs,
    };

    let reporter = create_reporter(false, args.json);

    if args.all {
        // Phase 1: Parallel Pre-flight validation
        check_suite_preflight(root).context("Suite release pre-flight check failed")?;

        // Phase 2: Upfront Suite Release Plan
        let plan = plan_suite_releases(root, opts.no_llm)
            .await
            .context("Suite-wide release planning failed")?;

        if !args.json {
            println!("{}", plan.format_table());
        }

        if plan.planned_releases() == 0 {
            if args.json {
                println!("{}", serde_json::json!([]));
            } else {
                println!("✅ All suite repositories are clean. No releases needed.");
            }
            return Ok(());
        }

        // Confirmation gate (bypassed with --dry-run, --yes, or --json)
        if !args.dry_run && !args.yes && !args.json {
            print!("\nProceed with release? [y/N]: ");
            io::stdout().flush().context("Failed to flush stdout")?;
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .context("Failed to read user confirmation")?;
            let trimmed = input.trim().to_ascii_lowercase();
            if trimmed != "y" && trimmed != "yes" {
                println!("Release aborted by user.");
                return Ok(());
            }
            println!();
        }

        if !args.json {
            println!(
                "================================================================================"
            );
            println!(" [outfeed] Executing Suite Release Pipeline");
            if args.dry_run {
                println!(" Mode: DRY RUN (simulating plan -> apply -> install -> push -> publish)");
            }
            if args.no_llm {
                println!(" Strategy: DETERMINISTIC (bypassing external LLM synthesis)");
            }
            println!(
                "================================================================================"
            );
        }

        let summaries = execute_suite_plan(root, &plan, &opts, reporter.as_ref())
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

    if !args.json {
        println!(
            "================================================================================"
        );
        println!(
            " [outfeed] Orchestrated Release Pipeline for '{}'",
            args.repo
        );
        if args.dry_run {
            println!(" Mode: DRY RUN (simulating plan -> apply -> install -> push -> publish)");
        }
        if args.no_llm {
            println!(" Strategy: DETERMINISTIC (bypassing external LLM synthesis)");
        }
        println!(
            "================================================================================"
        );
    }

    let summary = release_repo(root, &args.repo, &opts, reporter.as_ref())
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
