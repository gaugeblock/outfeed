//! Main CLI entrypoint for outfeed.

mod args;

use anyhow::{Context, Result};
use args::{ApplyArgs, Cli, Commands, PlanArgs, StatusArgs};
use clap::Parser;
use outfeed_core::engine::{apply_release, plan_release};
use outfeed_core::git::{get_commits_between, get_latest_tag};
use outfeed_core::manifest::get_repo_version;
use std::path::{Path, PathBuf};

fn resolve_repo_dir(base: &Path, input: &str) -> PathBuf {
    let p = PathBuf::from(input);
    if p.is_dir() {
        p
    } else {
        base.join(input)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let root_dir = cli.dir.unwrap_or_else(|| PathBuf::from("."));

    match cli.command {
        Commands::Plan(args) => handle_plan(&root_dir, args).await?,
        Commands::Apply(args) => handle_apply(&root_dir, args).await?,
        Commands::Status(args) => handle_status(&root_dir, args)?,
        Commands::Mcp(_) => {
            outfeed_mcp::run_stdio_server(&root_dir)
                .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))?;
        }
    }

    Ok(())
}

async fn handle_plan(root: &Path, args: PlanArgs) -> Result<()> {
    let repo_dir = resolve_repo_dir(root, &args.repo);
    let plan = plan_release(
        &repo_dir,
        args.override_version.as_deref(),
        args.skip_storypole,
    )
    .await
    .context("Failed to plan release")?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("================================================================================");
        println!(" [outfeed] Release Plan for '{}'", plan.repo_name);
        println!("================================================================================");
        println!(" Current Version:  {}", plan.current_version);
        println!(" Target Version:   {} ({:?})", plan.target_version, plan.bump_type);
        println!(" Rationale:        {}", plan.bump_rationale);
        println!(" Unreleased Commits: {}", plan.commits.len());
        if !plan.referenced_tickets.is_empty() {
            println!(" Shipped Tickets:  {}", plan.referenced_tickets.join(", "));
        }
        println!(" Modified Files:   {}", plan.manifest_diffs.len());
        println!("--------------------------------------------------------------------------------");
        println!(" Proposed CHANGELOG.md Section:\n");
        println!("{}", plan.changelog_markdown);
        println!("================================================================================");
    }
    Ok(())
}

async fn handle_apply(root: &Path, args: ApplyArgs) -> Result<()> {
    let repo_dir = resolve_repo_dir(root, &args.repo);

    if args.dry_run {
        println!("[DRY RUN] Simulating release without modifying disk or Git...");
        return handle_plan(
            root,
            PlanArgs {
                repo: args.repo,
                override_version: args.override_version,
                skip_storypole: args.skip_storypole,
                json: args.json,
            },
        )
        .await;
    }

    let plan = plan_release(
        &repo_dir,
        args.override_version.as_deref(),
        args.skip_storypole,
    )
    .await
    .context("Failed to generate release plan")?;

    apply_release(&repo_dir, &plan, !args.no_verify, args.skip_storypole)
        .context("Failed to execute release")?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "success": true,
                "repo": plan.repo_name,
                "version": plan.target_version,
                "tag": format!("v{}", plan.target_version)
            })
        );
    } else {
        println!(
            "✅ Successfully released {} v{} (tag: v{})",
            plan.repo_name, plan.target_version, plan.target_version
        );
    }

    Ok(())
}

fn handle_status(root: &Path, args: StatusArgs) -> Result<()> {
    let repos = if args.all {
        vec![
            "storypole", "sawhorse", "chalkline", "dado", "batten", "joiner", "benchwork",
            "rabbet", "miter", "outfeed",
        ]
    } else {
        vec![args.repo.as_str()]
    };

    println!("{:<14} | {:<12} | {:<14} | {:<18}", "REPOSITORY", "VERSION", "LATEST TAG", "UNRELEASED COMMITS");
    println!("--------------------------------------------------------------------------------");

    for r in repos {
        let repo_dir = resolve_repo_dir(root, r);
        if !repo_dir.is_dir() {
            continue;
        }

        let ver = get_repo_version(&repo_dir).unwrap_or_else(|_| "unknown".to_string());
        let tag = get_latest_tag(&repo_dir).unwrap_or(None).unwrap_or_else(|| "none".to_string());
        let commits = get_commits_between(&repo_dir, if tag == "none" { None } else { Some(&tag) }, "HEAD")
            .unwrap_or_default();

        let count = commits.len();
        let status_str = if count > 0 {
            format!("⚠️  {count} commit(s)")
        } else {
            "✅ clean".to_string()
        };

        println!("{:<14} | {:<12} | {:<14} | {:<18}", r, ver, tag, status_str);
    }

    Ok(())
}

