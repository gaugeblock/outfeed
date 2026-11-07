//! Bounded worker pool that executes one release wave at a time.

use super::{EntryOutcome, HeadroomLock, release_entry};
use crate::release::ReleaseSummary;
use crate::suite_plan::SuiteRepoPlanEntry;
use outfeed_core::error::OutfeedError;
use outfeed_core::model::ReleaseOpts;
use outfeed_core::progress::ProgressReporter;
use std::path::Path;

/// Runs one wave, running at most `ReleaseOpts::jobs` entries concurrently.
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
pub(super) fn run_wave_with<F>(
    entries: &[&SuiteRepoPlanEntry],
    jobs: usize,
    worker: F,
) -> Vec<EntryOutcome>
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
