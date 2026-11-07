//! Plan vocabulary: what a suite release intends to do to each repository.
//!
//! Split out of `suite_plan::mod` (GB-139) so the action taxonomy and the rendered table stand
//! apart from the evaluation that produces them.

use outfeed_core::model::ReleasePlan;

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
    /// Version is published, but the branch still carries commits the remote does not have.
    PushPending,
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
            Self::PushPending => "Push (Ahead)",
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
