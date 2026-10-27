//! Phase 2: Parallel commit evaluation, semantic version calculation, and plan presentation.

use std::path::Path;

use crate::engine::plan_release;
use crate::error::OutfeedError;
use crate::git::get_latest_tag;
use crate::install::SUITE_TOOLS;
use crate::manifest::get_repo_version;
use crate::model::ReleasePlan;

/// Action category for planned suite member repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuitePlanAction {
    /// Increments pre-release or patch version.
    ReleasePatch,
    /// Increments minor version due to breaking change.
    ReleaseMinor,
    /// Increments major version.
    ReleaseMajor,
    /// Tagged version is not yet published to registry.
    PublishPending,
    /// No commits to release.
    SkipClean,
}

impl SuitePlanAction {
    /// Returns human-readable label for action.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReleasePatch => "Release (Patch)",
            Self::ReleaseMinor => "Release (Minor)",
            Self::ReleaseMajor => "Release (Major)",
            Self::PublishPending => "Publish (Pending)",
            Self::SkipClean => "Skip (Clean)",
        }
    }
}

/// Plan entry for a single repository in the suite.
#[derive(Debug, Clone)]
pub struct SuiteRepoPlanEntry {
    /// Repository name.
    pub repo_name: String,
    /// Current released version or tag.
    pub current_version: String,
    /// Target release version if action is not skip.
    pub target_version: Option<String>,
    /// Release action category.
    pub action: SuitePlanAction,
    /// Number of unreleased commits since latest tag.
    pub unreleased_commits: usize,
    /// Referenced Storypole tasks.
    pub referenced_tasks: Vec<String>,
    /// Underlying ReleasePlan if release is needed.
    pub plan: Option<ReleasePlan>,
}

/// Full multi-repository suite release plan.
#[derive(Debug, Clone)]
pub struct SuiteReleasePlan {
    /// Member repository plan entries in topological order.
    pub entries: Vec<SuiteRepoPlanEntry>,
}

impl SuiteReleasePlan {
    /// Number of repositories requiring releases.
    pub fn planned_releases(&self) -> usize {
        self.entries
            .iter()
            .filter(|e| e.action != SuitePlanAction::SkipClean)
            .count()
    }

    /// Formats the plan into the Compact Summary Table.
    pub fn format_table(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("{}\n", "=".repeat(80)));
        out.push_str(" [outfeed] Suite Release Plan (10 Repositories)\n");
        out.push_str(&format!("{}\n", "=".repeat(80)));
        out.push_str(&format!(
            "{:<12} {:<15} {:<15} {:<17} {:<9} {}\n",
            "REPOSITORY", "CURRENT", "TARGET", "ACTION", "COMMITS", "TASKS"
        ));
        out.push_str(&format!("{}\n", "-".repeat(80)));

        for e in &self.entries {
            let target_str = e.target_version.as_deref().unwrap_or("-");
            let commit_str = if e.unreleased_commits > 0 {
                format!("+{}", e.unreleased_commits)
            } else {
                "0".to_string()
            };
            let tasks_str = if e.referenced_tasks.is_empty() {
                "-".to_string()
            } else {
                e.referenced_tasks.join(", ")
            };

            out.push_str(&format!(
                "{:<12} {:<15} {:<15} {:<17} {:<9} {}\n",
                e.repo_name,
                e.current_version,
                target_str,
                e.action.as_str(),
                commit_str,
                tasks_str
            ));
        }

        out.push_str(&format!("{}\n", "-".repeat(80)));
        let planned = self.planned_releases();
        let clean = self.entries.len() - planned;
        out.push_str(&format!(
            "Total Planned Releases: {} repositories ({} clean)\n",
            planned, clean
        ));
        out
    }
}

/// Evaluates all suite member repositories and compiles the SuiteReleasePlan.
/// Uses fast deterministic commit evaluation for the upfront summary table.
pub async fn plan_suite_releases(
    workspace_root: &Path,
    _no_llm: bool,
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
        match plan_release(&repo_dir, None, false, true).await {
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
                    crate::registry::are_repo_crates_published(&repo_dir, bare_version)
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
