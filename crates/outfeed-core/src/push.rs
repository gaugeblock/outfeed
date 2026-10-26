//! Git push operations for branches and release tags.

use std::path::Path;
use std::process::Command;

use crate::cmd::run_captured;
use crate::error::OutfeedError;
use crate::git::{assert_clean_working_tree, get_current_branch, get_latest_tag};
use crate::install::SUITE_TOOLS;
use crate::model::PushOpts;
use crate::progress::{NoopProgress, ProgressReporter, RepoStatus};

/// Push result description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushOutcome {
    /// Remote pushed to.
    pub remote: String,
    /// Branch pushed.
    pub branch: String,
    /// Tag pushed, if any.
    pub tag: Option<String>,
    /// Whether this was a simulated dry-run.
    pub dry_run: bool,
    /// Captured output from git push invocations.
    pub output: String,
}

/// Pushes the active branch and release tag for a repository using a default reporter.
pub fn push_repo(
    repo_dir: &Path,
    opts: &PushOpts,
) -> Result<PushOutcome, OutfeedError> {
    push_repo_with_progress(repo_dir, opts, &NoopProgress)
}

/// Pushes the active branch and release tag for a repository reporting progress events.
pub fn push_repo_with_progress(
    repo_dir: &Path,
    opts: &PushOpts,
    reporter: &dyn ProgressReporter,
) -> Result<PushOutcome, OutfeedError> {
    let repo_name = repo_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo");

    assert_clean_working_tree(repo_dir)?;
    let branch = get_current_branch(repo_dir)?;
    let tag = get_latest_tag(repo_dir)?;

    if opts.dry_run {
        let tag_desc = tag.as_deref().unwrap_or("none");
        reporter.on_complete(
            repo_name,
            &RepoStatus::Success(format!("{branch} / {tag_desc} [DRY RUN]")),
        );
        return Ok(PushOutcome {
            remote: opts.remote.clone(),
            branch,
            tag,
            dry_run: true,
            output: String::new(),
        });
    }

    reporter.on_start(repo_name, &format!("pushing branch '{branch}'"));

    // Push branch
    let mut branch_cmd = Command::new("git");
    branch_cmd
        .arg("-C")
        .arg(repo_dir)
        .arg("push")
        .arg(&opts.remote)
        .arg(&branch);

    let branch_out = run_captured(&mut branch_cmd)?;
    if !branch_out.success {
        let err_text = branch_out.combined_output();
        reporter.on_complete(repo_name, &RepoStatus::Failed(err_text.clone()));
        return Err(OutfeedError::Push(format!(
            "git push {} {} failed: {}",
            opts.remote, branch, err_text
        )));
    }

    let mut combined_out = branch_out.combined_output();

    // Push latest tag if present
    if let Some(ref t) = tag {
        reporter.on_update(repo_name, &format!("pushing tag '{t}'"));
        let mut tag_cmd = Command::new("git");
        tag_cmd
            .arg("-C")
            .arg(repo_dir)
            .arg("push")
            .arg(&opts.remote)
            .arg(t);

        let tag_out = run_captured(&mut tag_cmd)?;
        if !tag_out.success {
            let err_text = tag_out.combined_output();
            reporter.on_complete(repo_name, &RepoStatus::Failed(err_text.clone()));
            return Err(OutfeedError::Push(format!(
                "git push {} {} failed: {}",
                opts.remote, t, err_text
            )));
        }

        let tag_text = tag_out.combined_output();
        if !tag_text.is_empty() {
            if !combined_out.is_empty() {
                combined_out.push('\n');
            }
            combined_out.push_str(&tag_text);
        }
    }

    let tag_desc = tag.as_deref().unwrap_or("none");
    reporter.on_complete(
        repo_name,
        &RepoStatus::Success(format!("{branch} / {tag_desc}")),
    );

    Ok(PushOutcome {
        remote: opts.remote.clone(),
        branch,
        tag,
        dry_run: false,
        output: combined_out,
    })
}

/// Executes git push across all repositories in the suite with a default reporter.
pub fn push_all(
    workspace_root: &Path,
    opts: &PushOpts,
) -> Result<Vec<(String, PushOutcome)>, OutfeedError> {
    push_all_with_progress(workspace_root, opts, &NoopProgress)
}

/// Executes git push across all repositories in the suite with progress reporting.
pub fn push_all_with_progress(
    workspace_root: &Path,
    opts: &PushOpts,
    reporter: &dyn ProgressReporter,
) -> Result<Vec<(String, PushOutcome)>, OutfeedError> {
    let mut results = Vec::new();
    for tool in SUITE_TOOLS {
        let repo_dir = workspace_root.join(tool.name);
        if repo_dir.exists() && repo_dir.join(".git").exists() {
            let outcome = push_repo_with_progress(&repo_dir, opts, reporter)?;
            results.push((tool.name.to_string(), outcome));
        }
    }
    Ok(results)
}
