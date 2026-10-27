#![allow(clippy::unwrap_used, clippy::expect_used)]

use outfeed_core::OutfeedError;
use outfeed_core::git::assert_main_branch;
use outfeed_core::model::PushOpts;
use outfeed_core::publish::resolve_publish_order;
use outfeed_core::push::push_repo;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_resolve_publish_order_topological() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    // Create root Cargo.toml
    let root_cargo = r#"
[workspace]
resolver = "3"
members = ["crates/*", "."]

[package]
name = "myfacade"
version = "0.1.0"
edition = "2024"

[dependencies]
mycore = { path = "crates/mycore", version = "0.1.0" }
mycli = { path = "crates/mycli", version = "0.1.0" }
"#;
    fs::write(root.join("Cargo.toml"), root_cargo).expect("write root");

    // Create crates/mycore
    let core_dir = root.join("crates/mycore");
    fs::create_dir_all(&core_dir).expect("mkdir core");
    let core_cargo = r#"
[package]
name = "mycore"
version = "0.1.0"
edition = "2024"

[dependencies]
"#;
    fs::write(core_dir.join("Cargo.toml"), core_cargo).expect("write core");

    // Create crates/mycli
    let cli_dir = root.join("crates/mycli");
    fs::create_dir_all(&cli_dir).expect("mkdir cli");
    let cli_cargo = r#"
[package]
name = "mycli"
version = "0.1.0"
edition = "2024"

[dependencies]
mycore = { path = "../mycore", version = "0.1.0" }
"#;
    fs::write(cli_dir.join("Cargo.toml"), cli_cargo).expect("write cli");

    let order = resolve_publish_order(root).expect("order");
    assert_eq!(order.len(), 3);

    let names: Vec<String> = order.iter().map(|m| m.name.clone()).collect();
    // mycore must be first (leaf)
    assert_eq!(names[0], "mycore");
    // mycli depends on mycore, so it must precede myfacade (which depends on mycli)
    assert_eq!(names[1], "mycli");
    assert_eq!(names[2], "myfacade");
}

#[test]
fn test_push_repo_dry_run() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    // Initialize git repo
    let _ = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(root)
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(root)
        .output();
    let _ = std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(root)
        .output();

    fs::write(root.join("README.md"), "# Test").expect("write");
    let _ = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "initial commit"])
        .current_dir(root)
        .output();

    let opts = PushOpts {
        remote: "origin".to_string(),
        dry_run: true,
    };

    let outcome = push_repo(root, &opts).expect("push");
    assert!(outcome.dry_run);
    assert_eq!(outcome.branch, "main");
    assert_eq!(outcome.remote, "origin");
}

#[test]
fn test_assert_main_branch_enforcement() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    let _ = std::process::Command::new("git")
        .args(["init", "-b", "feat/my-cool-feature"])
        .current_dir(root)
        .output();

    let res = assert_main_branch(root);
    assert!(res.is_err());
    match res {
        Err(OutfeedError::InvalidBranch(msg)) => {
            assert!(msg.contains("feat/my-cool-feature"));
        }
        other => panic!("Expected InvalidBranch, got {:?}", other),
    }

    // Switch to main
    let _ = std::process::Command::new("git")
        .args(["checkout", "-b", "main"])
        .current_dir(root)
        .output();

    let ok_res = assert_main_branch(root);
    assert!(ok_res.is_ok());
}

#[tokio::test]
async fn test_release_all_on_empty_or_clean_workspace() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    let opts = outfeed_core::model::ReleaseOpts {
        dry_run: true,
        skip_install: true,
        skip_push: true,
        skip_publish: true,
        no_llm: true,
        no_verify: false,
    };

    // When no member repos exist in root, release_all skips safely and returns empty vec
    let summaries = outfeed_core::release_all(root, &opts, &outfeed_core::progress::NoopProgress)
        .await
        .expect("release_all");
    assert!(summaries.is_empty());
}

#[tokio::test]
async fn test_release_all_with_progress_reporting() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    struct RecordingReporter {
        started: std::sync::Mutex<Vec<String>>,
    }
    impl outfeed_core::progress::ProgressReporter for RecordingReporter {
        fn on_start(&self, repo: &str, _msg: &str) {
            self.started.lock().unwrap().push(repo.to_string());
        }
        fn on_update(&self, _repo: &str, _msg: &str) {}
        fn on_complete(&self, _repo: &str, _status: &outfeed_core::progress::RepoStatus) {}
    }

    let reporter = RecordingReporter {
        started: std::sync::Mutex::new(Vec::new()),
    };

    let opts = outfeed_core::model::ReleaseOpts {
        dry_run: true,
        skip_install: true,
        skip_push: true,
        skip_publish: true,
        no_llm: true,
        no_verify: false,
    };

    let summaries = outfeed_core::release_all(root, &opts, &reporter)
        .await
        .expect("release_all");
    assert!(summaries.is_empty());
}
