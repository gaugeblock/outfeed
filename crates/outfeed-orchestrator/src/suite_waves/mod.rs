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

use crate::release::ReleaseSummary;
use crate::suite_plan::{SuitePlanAction, SuiteReleasePlan, SuiteRepoPlanEntry};
use outfeed_core::error::OutfeedError;
use outfeed_core::model::ReleaseOpts;
use outfeed_core::progress::ProgressReporter;

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
    let status = outfeed_core::headroom::enforce_disk_headroom(workspace_root)?;
    reporter.on_update(repo_name, &status.summary());
    Ok(())
}

mod entry;
mod pool;
pub(crate) use entry::release_entry;
pub(crate) use pool::run_wave;

#[cfg(test)]
mod tests;
