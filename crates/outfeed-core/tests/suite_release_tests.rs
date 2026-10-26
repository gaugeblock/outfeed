#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Integration tests for suite release orchestration and `suite.lock` generation.

use outfeed_core::suite_lock::{
    SuiteLock, SuiteMetadata, SuiteReleaseOpts, SuiteToolEntry, write_suite_lock,
};
use outfeed_core::suite_release::{determine_suite_version, release_suite, update_root_changelog};
use tempfile::tempdir;

#[test]
fn test_suite_lock_roundtrip_serialization() {
    let lock = SuiteLock {
        suite: SuiteMetadata {
            version: "0.1.0-beta.3".to_string(),
            created_at: "2026-10-26T12:00:00Z".to_string(),
            meta_harness: "gaugeblock".to_string(),
        },
        tools: vec![
            SuiteToolEntry {
                name: "miter".to_string(),
                version: "0.1.0-beta.1".to_string(),
                commit: "abc1234567890".to_string(),
                tag: Some("v0.1.0-beta.1".to_string()),
            },
            SuiteToolEntry {
                name: "storypole".to_string(),
                version: "0.1.0-beta.2".to_string(),
                commit: "def0987654321".to_string(),
                tag: Some("v0.1.0-beta.2".to_string()),
            },
        ],
    };

    let serialized = toml::to_string_pretty(&lock).expect("failed to serialize suite lock");
    assert!(serialized.contains("0.1.0-beta.3"));
    assert!(serialized.contains("miter"));
    assert!(serialized.contains("storypole"));

    let deserialized: SuiteLock =
        toml::from_str(&serialized).expect("failed to deserialize suite lock");
    assert_eq!(lock, deserialized);
}

#[test]
fn test_determine_suite_version_explicit_and_prefix() {
    let temp = tempdir().expect("failed to create tempdir");
    let explicit = determine_suite_version(temp.path(), Some("0.1.0-beta.4"))
        .expect("determine version failed");
    assert_eq!(explicit, "0.1.0-beta.4");

    let with_v = determine_suite_version(temp.path(), Some("v0.2.0-beta.1"))
        .expect("determine version failed");
    assert_eq!(with_v, "0.2.0-beta.1");
}

#[test]
fn test_write_suite_lock_file() {
    let temp = tempdir().expect("failed to create tempdir");
    let lock = SuiteLock {
        suite: SuiteMetadata {
            version: "0.1.0-beta.3".to_string(),
            created_at: "2026-10-26T12:00:00Z".to_string(),
            meta_harness: "gaugeblock".to_string(),
        },
        tools: vec![],
    };

    let path = write_suite_lock(temp.path(), &lock).expect("failed to write suite.lock");
    assert!(path.exists());
    let read_back = std::fs::read_to_string(&path).expect("failed to read written file");
    assert!(read_back.contains("version = \"0.1.0-beta.3\""));
}

#[test]
fn test_update_root_changelog_insertion() {
    let temp = tempdir().expect("failed to create tempdir");
    let changelog_path = temp.path().join("CHANGELOG.md");
    let initial_content =
        "# Changelog\n\n## [0.1.0-beta.2] - 2026-10-26\n\n### Changed\n- Previous changes\n";
    std::fs::write(&changelog_path, initial_content).expect("failed to write changelog");

    let tools = vec![SuiteToolEntry {
        name: "benchwork".to_string(),
        version: "0.1.0-beta.4".to_string(),
        commit: "111222333".to_string(),
        tag: Some("v0.1.0-beta.4".to_string()),
    }];

    let updated = update_root_changelog(temp.path(), "0.1.0-beta.3", &tools)
        .expect("update changelog failed");
    assert!(updated);

    let content =
        std::fs::read_to_string(&changelog_path).expect("failed to read updated changelog");
    assert!(content.contains("## [0.1.0-beta.3]"));
    assert!(content.contains("`benchwork`: v0.1.0-beta.4"));
    assert!(
        content.find("## [0.1.0-beta.3]").unwrap() < content.find("## [0.1.0-beta.2]").unwrap()
    );

    // Idempotent: second run does not re-insert
    let second_run = update_root_changelog(temp.path(), "0.1.0-beta.3", &tools)
        .expect("update changelog failed");
    assert!(!second_run);
}

#[test]
fn test_suite_release_dry_run_simulation() {
    let temp = tempdir().expect("failed to create tempdir");
    let opts = SuiteReleaseOpts {
        target_version: Some("0.1.0-beta.99".to_string()),
        dry_run: true,
        skip_push: true,
    };

    let summary = release_suite(temp.path(), &opts).expect("suite release dry run failed");
    assert_eq!(summary.target_version, "0.1.0-beta.99");
    assert_eq!(summary.tag, "suite-v0.1.0-beta.99");
    assert!(!summary.committed);
    assert!(!summary.pushed);
    assert!(!temp.path().join("suite.lock").exists());
}
