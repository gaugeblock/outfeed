//! Which allowlist decides a contracts check (`OF-39`).
//!
//! The allowlist declares non-mirrors - fields deliberately absent on one side, like a secret that never
//! reaches TypeScript. It lives at the suite root while the check runs in a member, so a member checking
//! without it reported the suite's own decision as drift: `role_override_config.api_key` failed a
//! `benchwork` gate whose sibling root run was clean.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::Path;

use super::contracts_allowlist;

/// Both sides canonicalised: the resolver canonicalises the member before asking for membership, and on macOS a
/// tempdir is reached through the `/var` symlink.
fn canon(path: &Path) -> std::path::PathBuf {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => match std::fs::canonicalize(parent) {
            Ok(dir) => dir.join(name),
            Err(_) => path.to_path_buf(),
        },
        _ => path.to_path_buf(),
    }
}

fn member(root: &Path, name: &str) {
    fs::create_dir_all(root.join(name).join(".git")).expect("member checkout");
}

fn suite_with_root_allowlist() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join(".gaugeblock").join("miter")).expect("miter dir");
    fs::write(
        root.path().join(".gaugeblock").join("config.toml"),
        "[suite]\nmembers = [\"alpha\"]\n",
    )
    .expect("suite config");
    fs::write(
        root.path().join(".gaugeblock").join("miter").join("contracts-allowlist.toml"),
        "[[non_mirror]]\nentity = \"role_override_config\"\nfield = \"api_key\"\nside = \"typescript\"\n",
    )
    .expect("allowlist");
    member(root.path(), "alpha");
    root
}

#[test]
fn a_member_uses_the_suite_allowlist_because_that_is_where_the_decision_lives() {
    let root = suite_with_root_allowlist();
    let found =
        contracts_allowlist(&root.path().join("alpha")).expect("the suite allowlist is found");
    assert_eq!(
        canon(&found),
        canon(
            &root
                .path()
                .join(".gaugeblock/miter/contracts-allowlist.toml")
        )
    );
}

#[test]
fn a_repository_with_no_suite_uses_its_own_allowlist_or_none() {
    let lone = tempfile::tempdir().expect("tempdir");
    member(lone.path(), "solo");
    assert!(
        contracts_allowlist(&lone.path().join("solo")).is_none(),
        "no allowlist anywhere means no declared non-mirrors, which is a legitimate state"
    );

    fs::create_dir_all(lone.path().join("solo/.gaugeblock/miter")).expect("miter dir");
    fs::write(
        lone.path()
            .join("solo/.gaugeblock/miter/contracts-allowlist.toml"),
        "# empty\n",
    )
    .expect("allowlist");
    let found = contracts_allowlist(&lone.path().join("solo")).expect("own allowlist");
    assert_eq!(
        canon(&found),
        canon(
            &lone
                .path()
                .join("solo/.gaugeblock/miter/contracts-allowlist.toml")
        )
    );
}
