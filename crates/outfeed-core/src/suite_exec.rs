//! Suite-wide release execution and self-healing orchestration.

use std::path::Path;

use crate::error::OutfeedError;
use crate::model::ReleaseOpts;
use crate::progress::ProgressReporter;
use crate::release::{ReleaseSummary, release_repo};
use crate::suite_plan::SuiteReleasePlan;

/// Executes the release lifecycle across all suite repositories using the 3-phase pipeline.
pub async fn release_all(
    workspace_root: &Path,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<ReleaseSummary>, OutfeedError> {
    // Phase 1: Pre-flight validation
    crate::suite_check::check_suite_preflight(workspace_root)?;

    // Phase 2: Upfront Suite Release Plan
    let plan = crate::suite_plan::plan_suite_releases(workspace_root, opts.no_llm).await?;

    // Phase 3: Execute Suite Plan
    execute_suite_plan(workspace_root, &plan, opts, reporter).await
}

/// Executes Phase 3 on a pre-compiled SuiteReleasePlan in topological order.
pub async fn execute_suite_plan(
    workspace_root: &Path,
    plan: &SuiteReleasePlan,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<ReleaseSummary>, OutfeedError> {
    let mut summaries = Vec::new();
    for entry in &plan.entries {
        if entry.action == crate::suite_plan::SuitePlanAction::SkipClean {
            continue;
        }

        if entry.action == crate::suite_plan::SuitePlanAction::PublishPending {
            let repo_dir = workspace_root.join(&entry.repo_name);
            let target_version = entry
                .target_version
                .clone()
                .unwrap_or_else(|| entry.current_version.clone());
            reporter.on_start(
                &entry.repo_name,
                &format!("publishing pending {target_version} to registry"),
            );
            let publish_opts = crate::model::PublishOpts {
                dry_run: opts.dry_run,
                ..Default::default()
            };
            let published_crates =
                crate::publish::publish_repo_with_progress(&repo_dir, &publish_opts, reporter)?;
            reporter.on_complete(
                &entry.repo_name,
                &crate::progress::RepoStatus::Success(format!("published {target_version}")),
            );
            summaries.push(ReleaseSummary {
                repo_name: entry.repo_name.clone(),
                target_version,
                installed: false,
                pushed: false,
                published_crates,
            });
            continue;
        }

        if let Some(summary) =
            release_repo(workspace_root, &entry.repo_name, opts, reporter).await?
        {
            summaries.push(summary);

            if opts.clean_between_repos
                && !opts.dry_run
                && let Ok(reclaimed) = crate::disk_space::clean_repo_target_cache(workspace_root)
                && reclaimed > 0
            {
                let formatted = crate::clean::format_bytes(reclaimed);
                reporter.on_update(
                    &entry.repo_name,
                    &format!("reclaimed {formatted} compilation cache"),
                );
            }
        }
    }
    Ok(summaries)
}
