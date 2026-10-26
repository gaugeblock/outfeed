//! Command handler for `outfeed apply`.

use std::path::Path;
use anyhow::{Context, Result};
use outfeed_core::{apply_release, plan_release};

use crate::args::{ApplyArgs, PlanArgs};
use super::plan::handle_plan;
use super::resolve_repo_dir;

pub async fn handle_apply(root: &Path, args: ApplyArgs) -> Result<()> {
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
