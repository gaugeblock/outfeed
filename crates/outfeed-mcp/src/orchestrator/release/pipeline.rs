//! Phases 3-5 of a computed release plan: apply, install, push, and publish.
//!
//! Split out of `release::mod` so the orchestration entry points and the pipeline they drive stay
//! readable as separate units.

use std::path::Path;

use super::ReleaseSummary;
use outfeed_core::engine::apply_release_with_progress;
use outfeed_core::error::OutfeedError;
use outfeed_core::install::{SUITE_TOOLS, install_tool};
use outfeed_core::model::{InstallOpts, PublishOpts, PushOpts, ReleaseOpts, ReleasePlan};
use outfeed_core::progress::{ProgressReporter, RepoStatus};
use outfeed_core::publish::publish_repo;
use outfeed_core::push::push_repo;

/// Executes Phases 3-5 for a computed plan: apply, install, push, and publish.
pub(super) fn finish_planned_release(
    workspace_root: &Path,
    repo_dir: &Path,
    actual_name: &str,
    plan: &ReleasePlan,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
) -> Result<ReleaseSummary, OutfeedError> {
    let target_version = plan.target_version.clone();
    reporter.on_update(actual_name, &format!("planning v{target_version}..."));

    // 2. Apply (or simulate if dry_run)
    if opts.dry_run {
        tracing::info!(
            "[DRY RUN] Would release {} v{}",
            actual_name,
            target_version
        );
        reporter.on_complete(
            actual_name,
            &RepoStatus::Success(format!("[dry-run] proposed v{target_version}")),
        );
        return Ok(ReleaseSummary {
            repo_name: actual_name.to_string(),
            target_version,
            installed: !opts.skip_install,
            pushed: !opts.skip_push,
            published_crates: vec!["[simulated]".to_string()],
        });
    }

    if let Err(err) = apply_release_with_progress(repo_dir, plan, !opts.no_verify, reporter) {
        reporter.on_complete(actual_name, &RepoStatus::Failed(err.to_string()));
        return Err(err);
    }

    // The release ticket used to be created inside `apply_release`, fire-and-forget, from the core.
    // Recording it is orchestration, so it happens here and the outcome is reportable (`GB-286`).
    if !opts.dry_run {
        match crate::orchestrator::storypole::record_release(repo_dir, plan) {
            Some(ticket) => {
                reporter.on_update(actual_name, &format!("recorded release as {ticket}"))
            }
            None => reporter.on_update(actual_name, "no backlog found; release not recorded"),
        }
    }

    // 3. Install locally
    let mut installed = false;
    if !opts.skip_install
        && let Some(tool) = SUITE_TOOLS.iter().find(|t| t.name == actual_name)
    {
        reporter.on_update(actual_name, "compiling and installing binary...");
        if let Err(err) = install_tool(workspace_root, tool, &InstallOpts::default()) {
            reporter.on_complete(actual_name, &RepoStatus::Failed(err.to_string()));
            return Err(err);
        }
        installed = true;
    }

    // 4. Push to Git remote
    let mut pushed = false;
    if !opts.skip_push {
        reporter.on_update(actual_name, "pushing to Git remote...");
        if let Err(err) = push_repo(repo_dir, &PushOpts::default()) {
            reporter.on_complete(actual_name, &RepoStatus::Failed(err.to_string()));
            return Err(err);
        }
        pushed = true;
    }

    // 5. Publish to registry
    let mut published_crates = Vec::new();
    if !opts.skip_publish {
        reporter.on_update(actual_name, "publishing crates to registry...");
        published_crates = match publish_repo(repo_dir, &PublishOpts::default()) {
            Ok(crates) => crates,
            Err(err) => {
                reporter.on_complete(actual_name, &RepoStatus::Failed(err.to_string()));
                return Err(err);
            }
        };
    }

    reporter.on_complete(
        actual_name,
        &RepoStatus::Success(format!("released v{target_version}")),
    );

    Ok(ReleaseSummary {
        repo_name: actual_name.to_string(),
        target_version,
        installed,
        pushed,
        published_crates,
    })
}
