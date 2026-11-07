//! Suite-wide release execution and self-healing orchestration.

use std::path::Path;
use std::sync::Mutex;

use crate::release::ReleaseSummary;
use crate::suite_plan::SuiteReleasePlan;
use crate::suite_waves::{EntryOutcome, partition_release_waves, release_entry, run_wave};
use outfeed_core::error::OutfeedError;
use outfeed_core::model::ReleaseOpts;
use outfeed_core::progress::ProgressReporter;

/// Executes the release lifecycle across all suite repositories using the 3-phase pipeline.
pub async fn release_all(
    workspace_root: &Path,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<ReleaseSummary>, OutfeedError> {
    // Phase 1: Pre-flight validation
    outfeed_core::suite_check::check_suite_preflight(workspace_root)?;

    // Phase 2: Upfront Suite Release Plan
    let plan = crate::suite_plan::plan_suite_releases(workspace_root, opts.no_llm).await?;

    // Phase 3: Execute Suite Plan
    execute_suite_plan(workspace_root, &plan, opts, reporter).await
}

/// Executes Phase 3 on a pre-compiled `SuiteReleasePlan` in topological dependency waves.
///
/// Repositories inside a wave share no unreleased changes, so up to `opts.jobs` of them are
/// released concurrently. A single `jobs` value keeps the strictly serial behaviour and runs on
/// the caller's runtime; disk-guard passes stay serialized through a shared mutex in either mode.
pub async fn execute_suite_plan(
    workspace_root: &Path,
    plan: &SuiteReleasePlan,
    opts: &ReleaseOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<ReleaseSummary>, OutfeedError> {
    let headroom = Mutex::new(());
    let mut summaries = Vec::new();

    for wave in partition_release_waves(plan) {
        let outcomes = if opts.jobs <= 1 || wave.len() == 1 {
            // Sequential fast path: stay on the caller's runtime, no nested runtimes.
            let mut outcomes = Vec::with_capacity(wave.len());
            for entry in &wave {
                let name = entry.repo_name.clone();
                let outcome = release_entry(workspace_root, entry, opts, reporter, &headroom).await;
                outcomes.push((name, outcome));
            }
            outcomes
        } else {
            run_wave(workspace_root, opts, &wave, reporter, &headroom)
        };

        collect_outcomes(outcomes, &mut summaries)?;
    }

    Ok(summaries)
}

/// Accumulates per-repository summaries, surfacing the first repository failure.
fn collect_outcomes(
    outcomes: Vec<EntryOutcome>,
    summaries: &mut Vec<ReleaseSummary>,
) -> Result<(), OutfeedError> {
    for (_repo_name, outcome) in outcomes {
        if let Some(summary) = outcome? {
            summaries.push(summary);
        }
    }
    Ok(())
}
