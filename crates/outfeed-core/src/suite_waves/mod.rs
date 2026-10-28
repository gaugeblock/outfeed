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
use crate::model::{PublishOpts, ReleaseOpts};
use crate::progress::{ProgressReporter, RepoStatus};
use crate::release::{ReleaseSummary, release_repo};
use crate::suite_plan::{SuitePlanAction, SuiteReleasePlan, SuiteRepoPlanEntry};

/// Leaf-first dependency waves covering all 10 suite repositories.
pub const SUITE_RELEASE_WAVES: &[&[&str]] = &[
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

    // Adaptive headroom guard: measure free space before this repository is compiled.
    guard_disk_headroom(workspace_root, opts, &entry.repo_name, reporter, headroom)?;

    let Some(summary) = release_repo(workspace_root, &entry.repo_name, opts, reporter).await?
    else {
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

/// Executes one wave, running at most `ReleaseOpts::jobs` entries concurrently.
///
/// Results are returned in wave order regardless of completion order, so callers can report
/// deterministically. Each worker owns a current-thread Tokio runtime; nothing is spawned onto
/// the caller's runtime, which keeps the release loop free of `'static` bounds.
pub(crate) fn run_wave(
    workspace_root: &Path,
    opts: &ReleaseOpts,
    entries: &[&SuiteRepoPlanEntry],
    reporter: &dyn ProgressReporter,
    headroom: &HeadroomLock,
) -> Vec<EntryOutcome> {
    run_wave_with(
        entries,
        opts.jobs,
        |entry| match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(release_entry(
                workspace_root,
                entry,
                opts,
                reporter,
                headroom,
            )),
            Err(err) => Err(OutfeedError::Install(format!(
                "failed to start release worker runtime for {}: {err}",
                entry.repo_name
            ))),
        },
    )
}

/// Runs `worker` for every entry, at most `jobs` at a time, preserving entry order in the result.
fn run_wave_with<F>(entries: &[&SuiteRepoPlanEntry], jobs: usize, worker: F) -> Vec<EntryOutcome>
where
    F: Fn(&SuiteRepoPlanEntry) -> Result<Option<ReleaseSummary>, OutfeedError> + Sync,
{
    let jobs = jobs.max(1);
    let mut outcomes = Vec::with_capacity(entries.len());

    for chunk in entries.chunks(jobs) {
        let results: Vec<EntryOutcome> = std::thread::scope(|scope| {
            let handles: Vec<(&str, _)> = chunk
                .iter()
                .map(|entry| {
                    let name = entry.repo_name.as_str();
                    let worker = &worker;
                    let handle = scope.spawn(move || (name.to_string(), worker(entry)));
                    (name, handle)
                })
                .collect();

            handles
                .into_iter()
                .map(|(name, handle)| match handle.join() {
                    Ok(outcome) => outcome,
                    Err(_) => (
                        name.to_string(),
                        Err(OutfeedError::Install(format!(
                            "release worker for {name} panicked"
                        ))),
                    ),
                })
                .collect()
        });
        outcomes.extend(results);
    }

    outcomes
}

#[cfg(test)]
mod tests;
