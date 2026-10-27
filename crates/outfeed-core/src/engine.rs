//! High-level release orchestrator uniting git, SemVer, manifests, and Storypole.

use crate::changelog::plan_changelog_update;
use crate::cmd::run_captured;
use crate::error::OutfeedError;
use crate::git::{
    assert_clean_working_tree, assert_main_branch, create_release_commit_and_tag,
    get_commits_between, get_latest_tag,
};
use crate::manifest::{
    apply_manifest_diffs, get_repo_version, plan_manifest_updates, rollback_manifest_diffs,
};
use crate::model::{BumpType, ReleasePlan};
use crate::progress::{NoopProgress, ProgressReporter};
use crate::semver::{apply_bump, calculate_bump, parse_commit};
use crate::storypole::{
    certify_release_ticket, create_release_ticket, fetch_task_summaries, find_storypole_root,
};
use crate::synthesis::synthesize_release_notes;
use std::path::Path;
use std::process::Command;

/// Evaluates a repository and generates a proposed `ReleasePlan`.
pub async fn plan_release(
    repo_dir: &Path,
    override_version: Option<&str>,
    skip_storypole: bool,
    no_llm: bool,
) -> Result<ReleasePlan, OutfeedError> {
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
        synthesize_release_notes(
            &repo_name,
            &target_version,
            &commits,
            &task_summaries,
            no_llm,
        )
        .await?
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
    apply_release_with_progress(repo_dir, plan, verify_tier4, skip_storypole, &NoopProgress)
}

/// Executes a release plan with granular progress events and verification gating.
pub fn apply_release_with_progress(
    repo_dir: &Path,
    plan: &ReleasePlan,
    verify_tier4: bool,
    skip_storypole: bool,
    reporter: &dyn ProgressReporter,
) -> Result<(), OutfeedError> {
    assert_clean_working_tree(repo_dir)?;
    assert_main_branch(repo_dir)?;

    let repo_name = &plan.repo_name;

    // 1. Write manifests & changelog
    reporter.on_update(repo_name, "updating manifests & changelog...");
    apply_manifest_diffs(repo_dir, &plan.manifest_diffs)?;

    // 2. Update Cargo.lock
    reporter.on_update(repo_name, "synchronizing Cargo.lock...");
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

    // 3. Optional Tier 4 verification (captured to prevent terminal corruption)
    if verify_tier4 {
        reporter.on_update(repo_name, "running verification tests (cargo test)...");
        let mut test_cmd = Command::new("cargo");
        test_cmd.args(["test", "--workspace"]).current_dir(repo_dir);

        let test_output = run_captured(&mut test_cmd)?;
        if !test_output.success {
            let _ = rollback_manifest_diffs(repo_dir, &plan.manifest_diffs);
            let combined = test_output.combined_output();
            return Err(OutfeedError::Git(format!(
                "Tier 4 verification failed; rolled back all manifest edits:\n{combined}"
            )));
        }
    }

    // 4. Closed-loop Storypole integration
    if !skip_storypole && let Some(sp_root) = find_storypole_root(repo_dir) {
        reporter.on_update(repo_name, "certifying release in Storypole...");
        if let Ok(ticket_key) = create_release_ticket(
            &sp_root,
            &plan.repo_name,
            &plan.target_version,
            &plan.bump_rationale,
            &plan.referenced_tickets,
        ) {
            let _ = certify_release_ticket(&sp_root, &ticket_key, &plan.target_version);
        }
    }

    // 5. Git commit & tag
    reporter.on_update(
        repo_name,
        &format!("committing and tagging v{}...", plan.target_version),
    );
    create_release_commit_and_tag(repo_dir, &plan.target_version)?;

    Ok(())
}
