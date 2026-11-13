//! Unit tests for reservation checking and placeholder scaffolding.
//!
//! Registry lookups are deliberately not exercised here: they need the network, and a test that
//! silently passes offline would be worthless. `check_names` is covered through the report helpers,
//! and the parsing it depends on is tested in `index_poll`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use super::*;
use std::collections::BTreeMap;
use tempfile::TempDir;

fn status(name: &str, state: NameState) -> NameStatus {
    NameStatus {
        name: name.to_string(),
        state,
        published_version: None,
    }
}

#[test]
fn report_partitions_names_by_state() {
    let report = ReserveCheckReport {
        entries: vec![
            status("free-a", NameState::Free),
            status("taken-a", NameState::Taken),
            status("unknown-a", NameState::Unknown),
            status("free-b", NameState::Free),
        ],
    };

    assert_eq!(report.free(), vec!["free-a", "free-b"]);
    assert_eq!(report.taken(), vec!["taken-a"]);
    assert_eq!(report.unknown(), vec!["unknown-a"]);
    assert!(!report.all_free());
}

#[test]
fn unknown_is_not_treated_as_free() {
    // A registry that did not answer must never be reported as a name that can be claimed.
    let report = ReserveCheckReport {
        entries: vec![status("unreachable", NameState::Unknown)],
    };
    assert!(report.free().is_empty());
    assert!(!report.all_free());
}

#[test]
fn empty_check_is_vacuously_free() {
    assert!(ReserveCheckReport::default().all_free());
}

/// The suite's own identity, stated explicitly because it is no longer a default (`GB-364`).
fn suite_identity() -> Identity {
    Identity {
        homepage: "https://gaugeblock.dev".to_string(),
        license: "Apache-2.0".to_string(),
        repository_template: "https://github.com/gaugeblock/{name}".to_string(),
    }
}

/// Options for the suite, used by the tests that are not about identity.
fn suite_opts(staging: &Path) -> ScaffoldOpts {
    ScaffoldOpts {
        staging_dir: staging.to_path_buf(),
        identity: suite_identity(),
        repository_overrides: BTreeMap::new(),
    }
}

#[test]
fn scaffold_writes_a_standalone_placeholder() {
    let temp = TempDir::new().expect("temp dir");
    let opts = suite_opts(temp.path());

    let outcome = scaffold_placeholders(&["gaugeblock-inventory".to_string()], &opts)
        .expect("scaffold succeeds");

    assert_eq!(outcome.written.len(), 1);
    let dir = temp.path().join("gaugeblock-inventory");
    let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).expect("manifest written");

    assert!(manifest.contains("name = \"gaugeblock-inventory\""));
    assert!(manifest.contains(PLACEHOLDER_VERSION));
    assert!(
        manifest.contains("repository = \"https://github.com/gaugeblock/gaugeblock-inventory\"")
    );
    // The empty workspace table is what keeps cargo from enrolling the placeholder as a member.
    assert!(manifest.contains("[workspace]"));
    assert!(dir.join("src").join("lib.rs").is_file());
    assert!(dir.join("README.md").is_file());
}

#[test]
fn scaffold_skips_an_existing_placeholder() {
    let temp = TempDir::new().expect("temp dir");
    let opts = suite_opts(temp.path());
    let names = vec!["gaugeblock-workspace".to_string()];

    let first = scaffold_placeholders(&names, &opts).expect("first scaffold");
    let second = scaffold_placeholders(&names, &opts).expect("second scaffold");

    assert_eq!(first.written.len(), 1);
    assert_eq!(second.written.len(), 0);
    assert_eq!(second.skipped.len(), 1);
}

#[test]
fn scaffold_prefers_a_repository_override() {
    let temp = TempDir::new().expect("temp dir");
    let mut overrides = BTreeMap::new();
    overrides.insert(
        "gaugeblock-inventory".to_string(),
        "https://github.com/gaugeblock/common".to_string(),
    );
    let opts = ScaffoldOpts {
        repository_overrides: overrides,
        ..suite_opts(temp.path())
    };

    scaffold_placeholders(&["gaugeblock-inventory".to_string()], &opts).expect("scaffold");

    let manifest =
        std::fs::read_to_string(temp.path().join("gaugeblock-inventory").join("Cargo.toml"))
            .expect("manifest");
    assert!(manifest.contains("repository = \"https://github.com/gaugeblock/common\""));
}

#[test]
fn parent_exclude_hint_names_the_missing_entry() {
    let temp = TempDir::new().expect("temp dir");
    let manifest = temp.path().join("Cargo.toml");
    std::fs::write(&manifest, "[workspace]\nmembers = [\".\"]\n").expect("write manifest");

    let hint = parent_exclude_hint(std::path::Path::new("reserve-staging"), &manifest);
    assert!(hint.is_some_and(|text| text.contains("reserve-staging")));
}

#[test]
fn parent_exclude_hint_is_silent_when_already_excluded() {
    let temp = TempDir::new().expect("temp dir");
    let manifest = temp.path().join("Cargo.toml");
    std::fs::write(&manifest, "[workspace]\nexclude = [\"reserve-staging\"]\n")
        .expect("write manifest");

    assert!(parent_exclude_hint(std::path::Path::new("reserve-staging"), &manifest).is_none());
}

#[test]
fn publish_summary_separates_the_three_outcomes() {
    let summary = PublishSummary {
        outcomes: vec![
            ("a".to_string(), PublishOutcome::Published),
            ("b".to_string(), PublishOutcome::AlreadyTaken("x".into())),
            ("c".to_string(), PublishOutcome::Failed("y".into())),
        ],
    };

    assert_eq!(summary.published(), vec!["a"]);
    assert_eq!(summary.already_taken(), vec!["b"]);
    assert_eq!(summary.failed(), vec!["c"]);
}
