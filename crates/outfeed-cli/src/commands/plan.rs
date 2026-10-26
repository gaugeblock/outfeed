//! Command handler for `outfeed plan`.

use std::path::Path;
use anyhow::{Context, Result};
use outfeed_core::plan_release;

use crate::args::PlanArgs;
use super::resolve_repo_dir;

pub async fn handle_plan(root: &Path, args: PlanArgs) -> Result<()> {
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

