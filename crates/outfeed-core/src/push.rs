//! Git push operations for branches and release tags.

use std::path::Path;

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
pub fn push_repo(repo_dir: &Path, opts: &PushOpts) -> Result<PushOutcome, OutfeedError> {
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

    let repo = gaugeblock_git::Repository::discover(repo_dir)
        .map_err(|e| OutfeedError::Git(format!("{e}")))?;

    // One atomic push when there is a tag: either both refs land or neither does. Pushing them
    // separately meant a rejected tag left the branch published and the release half-made, which is
    // the failure mode --atomic exists to remove.
    let pushed = match tag.as_deref() {
        Some(tag_name) => repo.push_branch_and_tag(&opts.remote, &branch, tag_name),
        None => repo.push_branch(&opts.remote, &branch),
    };
    if let Err(error) = pushed {
        let err_text = error.to_string();
        reporter.on_complete(repo_name, &RepoStatus::Failed(err_text.clone()));
        return Err(OutfeedError::Push(format!(
            "git push {} {branch} failed: {err_text}",
            opts.remote
        )));
    }
    let combined_out = String::new();

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

/// Reports whether the current branch carries work the configured remote does not have.
///
/// A repository with no remote cannot be pushed, so it reports `false`. A branch that was never
/// pushed has no upstream to compare against and reports `true` - that is the case a release pass
/// must not mistake for "clean", because the release commit and its tag exist only on this machine.
pub fn is_ahead_of_remote(repo_dir: &Path) -> Result<bool, OutfeedError> {
    let repo = gaugeblock_git::Repository::discover(repo_dir)
        .map_err(|e| OutfeedError::Git(format!("{e}")))?;
    if repo
        .remotes()
        .map_err(|e| OutfeedError::Git(format!("{e}")))?
        .is_empty()
    {
        return Ok(false);
    }
    // No upstream means the branch exists here and nowhere else, which is exactly the case a release
    // pass must not mistake for clean.
    match repo.ahead_behind("@{upstream}", "HEAD") {
        Ok(counts) => Ok(counts.ahead > 0),
        Err(_) => Ok(true),
    }
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
