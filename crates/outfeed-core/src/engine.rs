//! High-level release orchestrator uniting git, SemVer, manifests, and Storypole.

use crate::changelog::plan_changelog_update;
use crate::error::OutfeedError;
use crate::git::{assert_clean_working_tree, create_release_commit_and_tag, get_commits_between, get_latest_tag};
use crate::manifest::{apply_manifest_diffs, get_repo_version, plan_manifest_updates, rollback_manifest_diffs};
use crate::model::{BumpType, ReleasePlan};
use crate::semver::{apply_bump, calculate_bump, parse_commit};
use crate::storypole::{certify_release_ticket, create_release_ticket, fetch_task_summaries, find_storypole_root};
use crate::synthesis::synthesize_release_notes;
use std::path::Path;
use std::process::Command;

/// Evaluates a repository and generates a proposed `ReleasePlan`.
pub async fn plan_release(
    repo_dir: &Path,
    override_version: Option<&str>,
    skip_storypole: bool,
) -> Result<ReleasePlan, OutfeedError> {
    assert_clean_working_tree(repo_dir)?;

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
    for c in &commits {
        for t in &c.referenced_tickets {
            if !referenced_tickets.contains(t) {
                referenced_tickets.push(t.clone());
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

    let storypole_root = if skip_storypole {
        None
    } else {
        find_storypole_root(repo_dir)
    };

    let task_summaries = if let Some(ref sp_root) = storypole_root {
        fetch_task_summaries(sp_root, &referenced_tickets).unwrap_or_default()
    } else {
        Vec::new()
    };

    let changelog_markdown = if commits.is_empty() {
        format!(
            "## [v{target_version}] - {}\n\n### Changed\n- Version promotion to {target_version}\n",
            chrono::Utc::now().format("%Y-%m-%d")
        )
    } else {
        synthesize_release_notes(&repo_name, &target_version, &commits, &task_summaries).await?
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

/// Executes a release plan with transactional safety and verification gating.
pub fn apply_release(
    repo_dir: &Path,
    plan: &ReleasePlan,
    verify_tier4: bool,
    skip_storypole: bool,
) -> Result<(), OutfeedError> {
    assert_clean_working_tree(repo_dir)?;

    // 1. Write manifests & changelog
    apply_manifest_diffs(repo_dir, &plan.manifest_diffs)?;

    // 2. Update Cargo.lock
    let lock_res = Command::new("cargo")
        .args(["check", "--workspace"])
        .current_dir(repo_dir)
        .output();

    if let Ok(ref out) = lock_res
        && !out.status.success()
    {
        let _ = rollback_manifest_diffs(repo_dir, &plan.manifest_diffs);
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(OutfeedError::Manifest(
            "Cargo.lock".to_string(),
            format!("cargo check failed during version synchronization: {stderr}"),
        ));
    }

    // 3. Optional Tier 4 verification
    if verify_tier4 {
        let test_status = Command::new("cargo")
            .args(["test", "--workspace"])
            .current_dir(repo_dir)
            .status();

        match test_status {
            Ok(status) if status.success() => {}
            _ => {
                let _ = rollback_manifest_diffs(repo_dir, &plan.manifest_diffs);
                return Err(OutfeedError::Git(
                    "Tier 4 verification failed; rolled back all manifest edits.".to_string(),
                ));
            }
        }
    }

    // 4. Git commit & tag
    create_release_commit_and_tag(repo_dir, &plan.target_version)?;

    // 5. Closed-loop Storypole integration
    if !skip_storypole
        && let Some(sp_root) = find_storypole_root(repo_dir)
        && let Ok(ticket_key) = create_release_ticket(
            &sp_root,
            &plan.repo_name,
            &plan.target_version,
            &plan.bump_rationale,
            &plan.referenced_tickets,
        )
    {
        let _ = certify_release_ticket(&sp_root, &ticket_key, &plan.target_version);
    }

    Ok(())
}
