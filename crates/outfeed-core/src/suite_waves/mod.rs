//! Topological release waves and the bounded worker pool that executes them.
//!
//! Member repositories are released in leaf-first dependency waves so repositories that share no
//! unreleased changes can progress together:
//!
//! | Wave | Repositories |
//! |:--|:--|
//! | 1 | `miter` |
//! | 2 | `storypole`, `sawhorse`, `dado`, `rabbet` |
//! | 3 | `chalkline`, `batten` |
//! | 4 | `joiner` |
//! | 5 | `benchwork`, `outfeed` |
//!
//! Within a wave, at most `ReleaseOpts::jobs` entries run at once. Concurrency uses scoped OS
//! threads (each driving its own current-thread Tokio runtime) because the release work is
//! dominated by blocking subprocess work — `cargo`, `git` — that would otherwise serialize on a
//! single async task. Two shared resources are deliberately serialized:
//!
//! * the **adaptive disk guard**, because it mutates `target/debug/deps` and must never prune
//!   beneath an in-flight build, and
//! * the **target directory**, whose cargo lock the toolchain already arbitrates.

use std::path::Path;
use std::sync::Mutex;

use crate::error::OutfeedError;
use crate::model::{PublishOpts, PushOpts, ReleaseOpts};
use crate::progress::{ProgressReporter, RepoStatus};
use crate::release::{ReleaseSummary, release_repo};
use crate::suite_plan::{SuitePlanAction, SuiteReleasePlan, SuiteRepoPlanEntry};

/// Leaf-first dependency waves covering all 10 suite repositories.
pub const SUITE_RELEASE_WAVES: &[&[&str]] = &[
    // The contracts crate publishes before any member that depends on it.
    &["common"],
    &["miter"],
    &["storypole", "sawhorse", "dado", "rabbet"],
    &["chalkline", "batten"],
    &["joiner"],
    &["benchwork", "outfeed"],
];

/// Name-tagged outcome of releasing a single repository.
pub(crate) type EntryOutcome = (String, Result<Option<ReleaseSummary>, OutfeedError>);

/// Token that serializes adaptive disk-guard passes across concurrent workers.
pub(crate) type HeadroomLock = Mutex<()>;

/// Partitions a suite plan into topological waves of releasable entries.
///
/// Entries are never dropped: repositories that do not appear in [`SUITE_RELEASE_WAVES`] are
/// released last, in plan order, so an unknown or newly added member still gets released.
pub fn partition_release_waves<'a>(plan: &'a SuiteReleasePlan) -> Vec<Vec<&'a SuiteRepoPlanEntry>> {
    let releasable = |entry: &SuiteRepoPlanEntry| entry.action != SuitePlanAction::SkipClean;

    let mut waves: Vec<Vec<&'a SuiteRepoPlanEntry>> = Vec::new();
    let mut scheduled: Vec<&'a str> = Vec::new();

    for wave in SUITE_RELEASE_WAVES {
        let mut members: Vec<&'a SuiteRepoPlanEntry> = Vec::new();
        for entry in plan.entries.iter().filter(|entry| releasable(entry)) {
            if wave.contains(&entry.repo_name.as_str()) {
                members.push(entry);
                scheduled.push(entry.repo_name.as_str());
            }
        }
        if !members.is_empty() {
            waves.push(members);
        }
    }

    let unscheduled: Vec<&'a SuiteRepoPlanEntry> = plan
        .entries
        .iter()
        .filter(|entry| releasable(entry) && !scheduled.contains(&entry.repo_name.as_str()))
        .collect();
    if !unscheduled.is_empty() {
        waves.push(unscheduled);
    }

    waves
}

/// Measures free space and applies adaptive remediation, one worker at a time.
fn guard_disk_headroom(
    workspace_root: &Path,
    opts: &ReleaseOpts,
    repo_name: &str,
    reporter: &dyn ProgressReporter,
    headroom: &HeadroomLock,
) -> Result<(), OutfeedError> {
    if opts.skip_disk_check || opts.dry_run {
        return Ok(());
    }
    let _permit = headroom.lock().map_err(|_| {
        OutfeedError::Clean("disk headroom lock poisoned by a failed guard pass".to_string())
    })?;
    let status = crate::headroom::enforce_disk_headroom(workspace_root)?;
    reporter.on_update(repo_name, &status.summary());
    Ok(())
}

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

mod pool;
pub(crate) use pool::run_wave;

#[cfg(test)]
mod tests;
