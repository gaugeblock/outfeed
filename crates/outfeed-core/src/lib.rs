//! `outfeed-core` — Core domain, Git inspection, SemVer engine, and manifest synchronizer.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod changelog;
pub mod engine;
pub mod error;
pub mod git;
pub mod manifest;
pub mod model;
pub mod semver;
pub mod storypole;
pub mod synthesis;

pub use changelog::{format_release_section, plan_changelog_update};
pub use engine::{apply_release, plan_release};
pub use error::OutfeedError;
pub use git::{assert_clean_working_tree, create_release_commit_and_tag, get_commits_between, get_latest_tag};
pub use manifest::{apply_manifest_diffs, get_repo_version, plan_manifest_updates, rollback_manifest_diffs};
pub use model::{BumpType, CommitInfo, ConventionalKind, ManifestDiff, ReleasePlan};
pub use semver::{apply_bump, calculate_bump, parse_commit};
pub use storypole::{certify_release_ticket, create_release_ticket, fetch_task_summaries, find_storypole_root};
pub use synthesis::synthesize_release_notes;

