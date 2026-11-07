//! High-level release orchestrator uniting git, SemVer, manifests, and Storypole.

use crate::changelog::{format_release_section, plan_changelog_update};
mod apply;

pub use apply::{apply_release, apply_release_with_progress};

use crate::error::OutfeedError;
use crate::git::{
    assert_clean_working_tree, assert_main_branch, get_commits_between, get_latest_tag,
};
use crate::manifest::{get_repo_version, plan_manifest_updates};
use crate::model::{BumpType, ReleasePlan};
use crate::semver::{apply_bump, calculate_bump, parse_commit};
use std::path::Path;

/// Evaluates a repository and generates a proposed `ReleasePlan`.
/// Everything a release plan is derived from, computed without writing anything.
///
/// Exposed so an integrator can synthesize changelog prose - which needs the target version - before
/// asking for the plan itself (`GB-285`). The alternative was for the core to call an inference backend,
/// which is the edge this split exists to remove.
#[derive(Debug, Clone)]
pub struct PlanBasis {
    /// Repository directory name.
    pub repo_name: String,
    /// Version currently in the manifest.
    pub current_version: String,
    /// Version this release would carry.
    pub target_version: String,
    /// Semantic classification of the bump.
    pub bump_type: BumpType,
    /// Why that classification was chosen.
    pub bump_rationale: String,
    /// Commits since the last tag, parsed.
    pub commits: Vec<crate::model::CommitInfo>,
    /// Ticket keys those commits reference.
    pub referenced_tickets: Vec<String>,
}

/// Computes the [`PlanBasis`] for a repository, validating the tree first.
pub fn plan_basis(
    repo_dir: &Path,
    override_version: Option<&str>,
) -> Result<PlanBasis, OutfeedError> {
    assert_clean_working_tree(repo_dir)?;
    assert_main_branch(repo_dir)?;

    let repo_name = repo_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("crate")
        .to_string();

    let current_version = get_repo_version(repo_dir)?;
    let latest_tag = get_latest_tag(repo_dir)?;
    let raw_commits = get_commits_between(repo_dir, latest_tag.as_deref(), "HEAD")?;

    if raw_commits.is_empty() && override_version.is_none() {
        let tag_info = latest_tag.unwrap_or_else(|| "repository root".to_string());
        return Err(OutfeedError::Git(format!(
            "No unreleased commits found on {repo_name} since {tag_info}"
        )));
    }

    let commits: Vec<_> = raw_commits
        .iter()
        .map(|(sha, summary, body)| parse_commit(sha, summary, body.as_deref()))
        .collect();

    let mut referenced_tickets = Vec::new();
    for commit in &commits {
        for ticket in &commit.referenced_tickets {
            if !referenced_tickets.contains(ticket) {
                referenced_tickets.push(ticket.clone());
            }
        }
    }

    let (bump_type, bump_rationale) = if let Some(override_ver) = override_version {
        (
            BumpType::PreReleasePatch,
            format!("Explicit version override to {override_ver}"),
        )
    } else {
        calculate_bump(&current_version, &commits)?
    };

    let target_version = match override_version {
        Some(v) => v.to_string(),
        None => apply_bump(&current_version, bump_type)?,
    };

    Ok(PlanBasis {
        repo_name,
        current_version,
        target_version,
        bump_type,
        bump_rationale,
        commits,
        referenced_tickets,
    })
}

/// Evaluates a repository and generates a proposed `ReleasePlan`.
///
/// `synthesized_notes` is changelog prose produced by an integrator, which is the only layer allowed to
/// call an inference backend (`GB-285`). `None` falls back to deterministic Keep-a-Changelog formatting.
pub async fn plan_release(
    repo_dir: &Path,
    override_version: Option<&str>,
    synthesized_notes: Option<&str>,
) -> Result<ReleasePlan, OutfeedError> {
    let basis = plan_basis(repo_dir, override_version)?;
    finish_plan(repo_dir, basis, synthesized_notes)
}

/// Builds the release plan from a computed basis and optional pre-synthesized notes.
pub fn finish_plan(
    repo_dir: &Path,
    basis: PlanBasis,
    synthesized_notes: Option<&str>,
) -> Result<ReleasePlan, OutfeedError> {
    let PlanBasis {
        repo_name,
        current_version,
        target_version,
        bump_type,
        bump_rationale,
        commits,
        referenced_tickets,
    } = basis;

    let changelog_markdown = if commits.is_empty() {
        format!(
            "## [v{target_version}] - {}\n\n### Changed\n- Version promotion to {target_version}\n",
            chrono::Utc::now().format("%Y-%m-%d")
        )
    } else {
        synthesized_notes
            .filter(|notes| !notes.trim().is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format_release_section(&target_version, &commits))
    };

    let mut manifest_diffs = plan_manifest_updates(repo_dir, &target_version)?;
    let changelog_diff = plan_changelog_update(repo_dir, &changelog_markdown)?;
    manifest_diffs.push(changelog_diff);

    Ok(ReleasePlan {
        repo_name,
        current_version,
        target_version,
        bump_type,
        bump_rationale,
        commits,
        referenced_tickets,
        changelog_markdown,
        manifest_diffs,
    })
}
