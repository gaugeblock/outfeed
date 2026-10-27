//! `outfeed-core` — Core domain, Git inspection, SemVer engine, and manifest synchronizer.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod changelog;
pub mod clean;
pub mod clone;
pub mod cmd;
pub mod engine;
pub mod error;
pub mod git;
pub mod install;
pub mod manifest;
pub mod mcp_sync;
pub mod model;
pub mod progress;
pub mod publish;
pub mod push;
pub mod release;
pub mod semver;
pub mod storypole;
pub mod suite_lock;
pub mod suite_release;
pub mod sync;
pub mod sync_model;
pub mod synthesis;

pub use changelog::{format_release_section, plan_changelog_update};
pub use clean::{
    calculate_disk_usage, clean_workspace, clean_workspace_with_progress, dir_size, format_bytes,
    prune_workspace, prune_workspace_with_progress,
};
pub use clone::{CloneOpts, CloneStatus, CloneSummary, GitProtocol, clone_suite, clone_tool};
pub use cmd::{CommandOutput, run_captured};
pub use engine::{apply_release, apply_release_with_progress, plan_release};
pub use error::OutfeedError;
pub use git::{
    assert_clean_working_tree, assert_main_branch, create_release_commit_and_tag,
    get_commits_between, get_head_commit_sha, get_latest_tag,
};
pub use install::{
    InstallOutcome, SUITE_TOOLS, SuiteTool, get_installed_version, install_all,
    install_all_with_progress, install_tool, install_tool_with_progress,
};
pub use manifest::{
    apply_manifest_diffs, get_repo_version, plan_manifest_updates, rollback_manifest_diffs,
};
pub use mcp_sync::sync_antigravity_mcp;
pub use model::{
    BumpType, CleanOpts, CleanReport, CommitInfo, ConventionalKind, DiskUsage, InstallOpts,
    ManifestDiff, PruneOpts, PublishOpts, PushOpts, ReleaseOpts, ReleasePlan,
};
pub use progress::{NoopProgress, ProgressReporter, RepoStatus};
pub use publish::{MemberCrate, publish_repo, publish_repo_with_progress, resolve_publish_order};
pub use push::{PushOutcome, push_all, push_all_with_progress, push_repo, push_repo_with_progress};
pub use release::{ReleaseSummary, release_all, release_repo};
pub use semver::{apply_bump, calculate_bump, parse_commit};
pub use storypole::{
    certify_release_ticket, create_release_ticket, fetch_task_summaries, find_storypole_root,
};
pub use suite_lock::{
    SuiteLock, SuiteMetadata, SuiteReleaseOpts, SuiteReleaseSummary, SuiteToolEntry,
    generate_suite_lock, write_suite_lock,
};
pub use suite_release::{determine_suite_version, release_suite, update_root_changelog};
pub use sync::{audit_repo_sync, sync_suite, sync_tool};
pub use sync_model::{
    BranchDivergence, RepoSyncStatus, SyncOpts, calculate_divergence, is_working_tree_dirty,
};
pub use synthesis::synthesize_release_notes;
