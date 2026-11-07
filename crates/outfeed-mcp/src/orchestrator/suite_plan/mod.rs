//! Phase 2: Parallel commit evaluation, semantic version calculation, and plan presentation.

use std::path::Path;

mod actions;

pub use actions::{SuitePlanAction, SuiteReleasePlan, SuiteRepoPlanEntry};

use outfeed_core::error::OutfeedError;
use outfeed_core::git::get_latest_tag;
use outfeed_core::install::SUITE_TOOLS;
use outfeed_core::manifest::get_repo_version;

/// Evaluates all suite member repositories and compiles the SuiteReleasePlan.
/// Uses fast deterministic commit evaluation for the upfront summary table.
pub async fn plan_suite_releases(
    workspace_root: &Path,
    no_llm: bool,
) -> Result<SuiteReleasePlan, OutfeedError> {
    let mut entries = Vec::new();

    for tool in SUITE_TOOLS {
        let repo_dir = workspace_root.join(tool.name);
        if !repo_dir.exists() {
            continue;
        }

        let current_version = get_latest_tag(&repo_dir).ok().flatten().unwrap_or_else(|| {
            get_repo_version(&repo_dir)
                .map(|v| format!("v{v}"))
                .unwrap_or_else(|_| "v0.1.0-beta.1".to_string())
        });

        // Fast deterministic SemVer evaluation for upfront plan presentation
        match plan_for_suite(&repo_dir, no_llm).await {
            Ok(p) if !p.commits.is_empty() => {
                let mut tasks = Vec::new();
                for c in &p.commits {
                    for t in &c.referenced_tickets {
                        if !tasks.contains(t) {
                            tasks.push(t.clone());
                        }
                    }
                }
                let target_v = format!("v{}", p.target_version);
                let action = if p.commits.iter().any(|c| c.is_breaking) {
                    SuitePlanAction::ReleaseMinor
                } else {
                    SuitePlanAction::ReleasePatch
                };

                entries.push(SuiteRepoPlanEntry {
                    repo_name: tool.name.to_string(),
                    current_version,
                    target_version: Some(target_v),
                    action,
                    unreleased_commits: p.commits.len(),
                    referenced_tasks: tasks,
                    plan: Some(p),
                });
            }
            _ => {
                let bare_version = current_version.trim_start_matches('v');
                let is_published =
                    outfeed_core::registry::are_repo_crates_published(&repo_dir, bare_version)
                        .unwrap_or(true);

                if !is_published {
                    entries.push(SuiteRepoPlanEntry {
                        repo_name: tool.name.to_string(),
                        current_version: current_version.clone(),
                        target_version: Some(current_version),
                        action: SuitePlanAction::PublishPending,
                        unreleased_commits: 0,
                        referenced_tasks: Vec::new(),
                        plan: None,
                    });
                } else if outfeed_core::push::is_ahead_of_remote(&repo_dir).unwrap_or(false) {
                    // Published, but the release commit or tag never reached the remote.
                    entries.push(SuiteRepoPlanEntry {
                        repo_name: tool.name.to_string(),
                        current_version,
                        target_version: None,
                        action: SuitePlanAction::PushPending,
                        unreleased_commits: 0,
                        referenced_tasks: Vec::new(),
                        plan: None,
                    });
                } else {
                    entries.push(SuiteRepoPlanEntry {
                        repo_name: tool.name.to_string(),
                        current_version,
                        target_version: None,
                        action: SuitePlanAction::SkipClean,
                        unreleased_commits: 0,
                        referenced_tasks: Vec::new(),
                        plan: None,
                    });
                }
            }
        }
    }

    Ok(SuiteReleasePlan { entries })
}

/// Plans one repository for the suite view, gathering the inputs this layer may touch.
async fn plan_for_suite(
    repo_dir: &std::path::Path,
    no_llm: bool,
) -> Result<outfeed_core::model::ReleasePlan, outfeed_core::OutfeedError> {
    let basis = outfeed_core::engine::plan_basis(repo_dir, None)?;
    let (_summaries, notes) =
        crate::orchestrator::storypole::gather(repo_dir, &basis, no_llm).await;
    outfeed_core::engine::finish_plan(repo_dir, basis, notes.as_deref())
}
