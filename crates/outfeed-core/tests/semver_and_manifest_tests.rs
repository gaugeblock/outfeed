use outfeed_core::changelog::format_release_section;
use outfeed_core::manifest::{apply_manifest_diffs, get_repo_version, plan_manifest_updates};
use outfeed_core::model::{BumpType, ConventionalKind};
use outfeed_core::semver::{apply_bump, calculate_bump, parse_commit};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_parse_conventional_commits() {
    let c1 = parse_commit("sha1", "feat(core): add shiny new feature (#GB-41)", None);
    assert_eq!(c1.kind, ConventionalKind::Feat);
    assert_eq!(c1.scope, Some("core".to_string()));
    assert!(!c1.is_breaking);
    assert_eq!(c1.referenced_tickets, vec!["GB-41".to_string()]);

    let c2 = parse_commit(
        "sha2",
        "fix!: change public trait signature",
        Some("BREAKING CHANGE: breaks all API"),
    );
    assert_eq!(c2.kind, ConventionalKind::Fix);
    assert!(c2.is_breaking);

    let c3 = parse_commit("sha3", "refactor(cli): clean up main", None);
    assert_eq!(c3.kind, ConventionalKind::Refactor);
    assert!(!c3.is_breaking);
}

#[test]
fn test_calculate_and_apply_bump() {
    let commits_fix = vec![parse_commit("1", "fix: small bug", None)];
    let (bump, _) = calculate_bump("0.1.0-beta.2", &commits_fix).expect("calculate_bump");
    assert_eq!(bump, BumpType::PreReleasePatch);
    let next = apply_bump("0.1.0-beta.2", bump).expect("apply_bump");
    assert_eq!(next, "0.1.0-beta.3");

    let commits_feat = vec![parse_commit("2", "feat: new capability", None)];
    let (bump2, _) = calculate_bump("0.1.0-beta.2", &commits_feat).expect("calculate_bump");
    assert_eq!(bump2, BumpType::PreReleasePatch);
    let next2 = apply_bump("0.1.0-beta.2", bump2).expect("apply_bump");
    assert_eq!(next2, "0.1.0-beta.3");

    let commits_breaking = vec![parse_commit("3", "feat!: breaking change", None)];
    let (bump3, _) = calculate_bump("0.1.0-beta.2", &commits_breaking).expect("calculate_bump");
    assert_eq!(bump3, BumpType::PreReleaseMinor);
    let next3 = apply_bump("0.1.0-beta.2", bump3).expect("apply_bump");
    assert_eq!(next3, "0.2.0-beta.1");

    let (bump_alpha, _) = calculate_bump("0.1.0-alpha.3", &commits_feat).expect("calculate_bump");
    assert_eq!(bump_alpha, BumpType::PreReleaseAlpha);
    let next_alpha = apply_bump("0.1.0-alpha.3", bump_alpha).expect("apply_bump");
    assert_eq!(next_alpha, "0.1.0-alpha.4");
}

#[test]
fn test_manifest_planning_and_application() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let root_cargo = r#"
[workspace]
members = [".", "crates/*"]

[workspace.package]
version = "0.1.0-beta.2"
edition = "2024"

[dependencies]
sub-crate = { path = "crates/sub-crate", version = "0.1.0-beta.2" }
"#;

    fs::write(root.join("Cargo.toml"), root_cargo).expect("write root Cargo.toml");

    let sub_dir = root.join("crates").join("sub-crate");
    fs::create_dir_all(&sub_dir).expect("mkdir sub");
    let sub_cargo = r#"
[package]
name = "sub-crate"
version.workspace = true
edition.workspace = true

[dependencies]
sibling = { path = "../sibling", version = "0.1.0-beta.2" }
"#;
    fs::write(sub_dir.join("Cargo.toml"), sub_cargo).expect("write sub Cargo.toml");

    let cur_ver = get_repo_version(root).expect("get_repo_version");
    assert_eq!(cur_ver, "0.1.0-beta.2");

    let diffs = plan_manifest_updates(root, "0.1.0-beta.3").expect("plan_manifest_updates");
    assert_eq!(diffs.len(), 2);

    apply_manifest_diffs(root, &diffs).expect("apply_manifest_diffs");

    let updated_ver = get_repo_version(root).expect("updated get_repo_version");
    assert_eq!(updated_ver, "0.1.0-beta.3");

    let updated_root = fs::read_to_string(root.join("Cargo.toml")).expect("read root");
    assert!(updated_root.contains("version = \"0.1.0-beta.3\""));

    let updated_sub = fs::read_to_string(sub_dir.join("Cargo.toml")).expect("read sub");
    assert!(updated_sub.contains("version = \"0.1.0-beta.3\""));
}

#[test]
fn test_format_release_section() {
    let commits = vec![
        parse_commit("1", "feat(core): add feature A", Some("Fixes #GB-10")),
        parse_commit("2", "fix(cli): fix bug B", None),
    ];

    let section = format_release_section("0.1.0-beta.3", &commits);
    assert!(section.contains("## [v0.1.0-beta.3]"));
    assert!(section.contains("### Added"));
    assert!(section.contains("`core`: add feature A (GB-10)"));
    assert!(section.contains("### Fixed"));
    assert!(section.contains("`cli`: fix bug B"));
}
