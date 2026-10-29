//! Release orchestration over a workspace, with and without progress reporting.

use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_release_all_on_empty_or_clean_workspace() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

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
    fs::write(root.join("README.md"), "# Suite Root").expect("write root readme");
    let _ = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "chore: init suite root"])
        .current_dir(root)
        .output();

    let opts = outfeed_core::model::ReleaseOpts {
        dry_run: true,
        skip_install: true,
        skip_push: true,
        skip_publish: true,
        no_llm: true,
        no_verify: false,
        jobs: 1,
        ..Default::default()
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
    fs::write(root.join("README.md"), "# Suite Root").expect("write root readme");
    let _ = std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .output();
    let _ = std::process::Command::new("git")
        .args(["commit", "-m", "chore: init suite root"])
        .current_dir(root)
        .output();

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
        jobs: 1,
        ..Default::default()
    };

    let summaries = outfeed_core::release_all(root, &opts, &reporter)
        .await
        .expect("release_all");
    assert!(summaries.is_empty());
}

#[test]
fn test_release_opts_defaults_are_cache_and_packaging_first() {
    let opts = outfeed_core::model::ReleaseOpts::default();

    assert!(
        opts.skip_install,
        "binary installation is opt-in so releases stay packaging operations"
    );
    assert!(
        !opts.clean_between_repos,
        "the compilation cache is retained between repositories by default"
    );
    assert_eq!(
        opts.jobs, 1,
        "library callers stay serial unless they opt in"
    );
}
