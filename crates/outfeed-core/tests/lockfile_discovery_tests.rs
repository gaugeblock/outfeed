//! Discovery that finds nothing is a failure, not a pass (`GB-327`).
//!
//! multi-repo: a suite with two member checkouts, which is the only way to see that running inside one
//! member checks the suite instead of reporting zero repositories.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use outfeed_core::{OutfeedError, check_lockfiles, check_lockfiles_resolved};
use tempfile::tempdir;

/// A directory that discovery counts as a repository: a manifest and a `.git` entry.
fn repo(dir: &Path) {
    fs::create_dir_all(dir.join(".git")).expect("git entry");
    fs::write(dir.join("Cargo.toml"), "[workspace]\n").expect("manifest");
}

/// A suite that declares `members`, with a storypole-free `.gaugeblock` declaration.
fn suite(root: &Path, members: &[&str]) {
    fs::create_dir_all(root.join(".gaugeblock")).expect("suite dir");
    let list: Vec<String> = members.iter().map(|m| format!("\"{m}\"")).collect();
    fs::write(
        root.join(".gaugeblock").join("config.toml"),
        format!("[suite]\nmembers = [{}]\n", list.join(", ")),
    )
    .expect("declaration");
}

#[test]
fn a_directory_with_no_repositories_is_an_error_not_a_pass() {
    let tmp = tempdir().expect("tempdir");
    // A member-shaped layout: children are directories, none of them repositories.
    fs::create_dir_all(tmp.path().join("crates").join("engine")).expect("member layout");

    let error = check_lockfiles(tmp.path()).expect_err("nothing to check is a failure");
    match error {
        OutfeedError::NoRepositories { root } => {
            assert!(root.contains("NoRepos") || !root.is_empty(), "{root}");
        }
        other => panic!("expected NoRepositories, got {other:?}"),
    }
}

#[test]
fn running_inside_a_member_checks_the_suite_and_says_which() {
    let tmp = tempdir().expect("tempdir");
    suite(tmp.path(), &["alpha", "beta"]);
    repo(&tmp.path().join("alpha"));
    repo(&tmp.path().join("beta"));

    // Inside `alpha`: its own children hold no repositories, so this is the case that used to report
    // "All 0 repositories" and exit 0.
    let member = tmp.path().join("alpha");
    let (checked, report) = check_lockfiles_resolved(&member).expect("the suite is checked");

    assert_eq!(
        checked,
        std::fs::canonicalize(tmp.path()).expect("canonical suite root"),
        "the suite root is what was checked"
    );
    assert_eq!(
        report.repositories.len(),
        2,
        "both members were checked: {:?}",
        report
            .repositories
            .iter()
            .map(|r| &r.repo)
            .collect::<Vec<_>>()
    );
}

#[test]
fn a_member_of_no_suite_still_refuses_to_pass_on_nothing() {
    let tmp = tempdir().expect("tempdir");
    // A repository that belongs to no suite, whose children are not repositories either.
    repo(tmp.path());
    fs::create_dir_all(tmp.path().join("docs")).expect("docs");

    assert!(
        check_lockfiles_resolved(tmp.path()).is_err(),
        "with no suite to widen to, the honest answer is that nothing was checked"
    );
}
