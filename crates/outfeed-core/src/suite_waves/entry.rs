//! Executing one plan entry: the pending-publish and pending-push shortcuts, then a full release.
//!
//! Split out of `suite_waves::mod` (GB-139) so the wave schedule stays separate from the work it
//! schedules.

use std::path::Path;

use super::{HeadroomLock, guard_disk_headroom};
use crate::error::OutfeedError;
use crate::model::{PublishOpts, PushOpts, ReleaseOpts};
use crate::progress::{ProgressReporter, RepoStatus};
use crate::release::{ReleaseSummary, release_repo};
use crate::suite_plan::{SuitePlanAction, SuiteRepoPlanEntry};

/// Releases a single plan entry, including the pending-publish shortcut and guard passes.
pub(crate) async fn release_entry(
    workspace_root: &Path,
    entry: &SuiteRepoPlanEntry,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
    headroom: &HeadroomLock,
) -> Result<Option<ReleaseSummary>, OutfeedError> {
    if entry.action == SuitePlanAction::PublishPending {
        let repo_dir = workspace_root.join(&entry.repo_name);
        let target_version = entry
            .target_version
            .clone()
            .unwrap_or_else(|| entry.current_version.clone());
        reporter.on_start(
            &entry.repo_name,
            &format!("publishing pending {target_version} to registry"),
        );
        let publish_opts = PublishOpts {
            dry_run: opts.dry_run,
            ..Default::default()
        };
        let published_crates =
            crate::publish::publish_repo_with_progress(&repo_dir, &publish_opts, reporter)?;
        reporter.on_complete(
            &entry.repo_name,
            &RepoStatus::Success(format!("published {target_version}")),
        );
        return Ok(Some(ReleaseSummary {
            repo_name: entry.repo_name.clone(),
            target_version,
            installed: false,
            pushed: false,
            published_crates,
        }));
    }

    if entry.action == SuitePlanAction::PushPending {
        if opts.skip_push {
            reporter.on_complete(
                &entry.repo_name,
                &RepoStatus::Skipped("ahead of remote, push skipped".to_string()),
            );
            return Ok(None);
        }
        let repo_dir = workspace_root.join(&entry.repo_name);
        reporter.on_start(&entry.repo_name, "pushing commits the remote is missing");
        let push_opts = PushOpts {
            dry_run: opts.dry_run,
            ..Default::default()
        };
        let outcome = crate::push::push_repo_with_progress(&repo_dir, &push_opts, reporter)?;
        return Ok(Some(ReleaseSummary {
            repo_name: entry.repo_name.clone(),
            target_version: entry.current_version.clone(),
            installed: false,
            pushed: !outcome.dry_run,
            published_crates: Vec::new(),
        }));
    }

    // Adaptive headroom guard: measure free space before this repository is compiled.
    guard_disk_headroom(workspace_root, opts, &entry.repo_name, reporter, headroom)?;

    let Some(summary) = release_from_plan(workspace_root, entry, opts, reporter).await? else {
        return Ok(None);
    };

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

    // Adaptive headroom guard: re-measure after the build and test wave.
    guard_disk_headroom(workspace_root, opts, &entry.repo_name, reporter, headroom)?;

    Ok(Some(summary))
}

/// Releases an entry, reusing the Phase 2 plan when one was computed.
async fn release_from_plan(
    workspace_root: &Path,
    entry: &SuiteRepoPlanEntry,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Option<ReleaseSummary>, OutfeedError> {
    match entry.plan.as_ref() {
        Some(plan) => {
            crate::release::release_repo_with_plan(
                workspace_root,
                &entry.repo_name,
                plan,
                opts,
                reporter,
            )
            .await
        }
        None => release_repo(workspace_root, &entry.repo_name, opts, reporter).await,
    }
}
