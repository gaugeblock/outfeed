//! Command handler for `outfeed apply`.

use anyhow::{Context, Result};
use outfeed_core::{RepoStatus, apply_release_with_progress, plan_release};
use std::path::Path;

use super::plan::handle_plan;
use super::resolve_repo_dir;
use crate::args::{ApplyArgs, PlanArgs};
use crate::progress::create_reporter;

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
                no_llm: args.no_llm,
                json: args.json,
            },
        )
        .await;
    }

    let plan = plan_release(
        &repo_dir,
        args.override_version.as_deref(),
        args.skip_storypole,
        args.no_llm,
    )
    .await
    .context("Failed to generate release plan")?;

    let reporter = create_reporter(false, args.json);
    reporter.on_start(&plan.repo_name, "applying release plan...");

    if let Err(err) = apply_release_with_progress(
        &repo_dir,
        &plan,
        !args.no_verify,
        args.skip_storypole,
        reporter.as_ref(),
    ) {
        reporter.on_complete(&plan.repo_name, &RepoStatus::Failed(err.to_string()));
        return Err(err).context("Failed to execute release");
    }

    reporter.on_complete(
        &plan.repo_name,
        &RepoStatus::Success(format!("v{}", plan.target_version)),
    );

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
