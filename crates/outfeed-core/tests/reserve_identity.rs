//! A placeholder carries the caller's identity, not the suite's (`GB-364`).
//!
//! crates.io fixes metadata per published version and a version can never be deleted, so a placeholder
//! published with the wrong homepage, license or repository is a permanent claim in somebody else's name. The
//! suite's own values used to be defaults here, and two of the three had no flag at all.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;

use outfeed_core::reserve::{Identity, ScaffoldOpts, scaffold_placeholders};
use tempfile::TempDir;

#[test]
fn a_caller_that_is_not_gaugeblock_gets_a_manifest_with_no_gaugeblock_in_it() {
    // The defect `GB-364` records: every placeholder used to be stamped with the suite's homepage, license and
    // repository, and two of the three had no flag. Published metadata is permanent, so this is not cosmetic -
    // the crate page would forever say somebody else's name.
    let temp = TempDir::new().expect("temp dir");
    let identity = Identity {
        homepage: "https://github.com/colinwolff/aerofoil".to_string(),
        license: "Apache-2.0 OR MIT".to_string(),
        repository_template: "https://github.com/colinwolff/aerofoil".to_string(),
    };
    let opts = ScaffoldOpts {
        staging_dir: temp.path().to_path_buf(),
        identity,
        repository_overrides: BTreeMap::new(),
    };

    scaffold_placeholders(&["aerofoil-mlat".to_string()], &opts).expect("scaffold");

    let dir = temp.path().join("aerofoil-mlat");
    let manifest = std::fs::read_to_string(dir.join("Cargo.toml")).expect("manifest");
    assert!(
        !manifest.to_lowercase().contains("gaugeblock"),
        "a stranger's placeholder must not name the suite anywhere: {manifest}"
    );
    // The SPDX expression is carried verbatim, OR included: a placeholder that claims one half of a
    // dual-licensed project is a licensing statement nobody made.
    assert!(
        manifest.contains("license = \"Apache-2.0 OR MIT\""),
        "the SPDX expression must survive verbatim: {manifest}"
    );
    assert!(manifest.contains("repository = \"https://github.com/colinwolff/aerofoil\""));
    assert!(manifest.contains("homepage = \"https://github.com/colinwolff/aerofoil\""));

    // The README the placeholder ships is part of what gets published too.
    let readme = std::fs::read_to_string(dir.join("README.md")).expect("readme");
    assert!(!readme.to_lowercase().contains("gaugeblock"), "{readme}");
}

#[test]
fn identity_is_derived_from_the_origin_remote_and_normalised() {
    // A mono repository publishing several crates omits {name}: every placeholder links to the one repository.
    let temp = TempDir::new().expect("temp dir");
    let repo = temp.path();
    let git = |args: &[&str]| {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git spawns");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "--quiet"]);
    git(&[
        "remote",
        "add",
        "origin",
        "git@github.com:colinwolff/aerofoil.git",
    ]);

    let identity = Identity::resolve(None, Some("MIT"), None, repo).expect("identity resolves");
    assert_eq!(
        identity.repository_template, "https://github.com/colinwolff/aerofoil",
        "an scp-style remote is a transport, not a URL a crate page can link to"
    );
    assert_eq!(identity.homepage, "https://github.com/colinwolff/aerofoil");
    assert_eq!(identity.license, "MIT", "a flag still wins over everything");
}

#[test]
fn a_caller_with_no_remote_and_no_flags_is_refused_rather_than_guessed() {
    // The old behaviour was to write the suite's identity here. A command that guesses publishes a permanent,
    // wrong-shaped claim on somebody else's behalf, so the only acceptable outcome is an error naming the flag.
    let temp = TempDir::new().expect("temp dir");
    let error = Identity::resolve(None, Some("MIT"), None, temp.path())
        .expect_err("no remote and no --repository must fail");
    let message = error.to_string();
    assert!(
        message.contains("--repository"),
        "the error must name the flag to pass: {message}"
    );

    let error = Identity::resolve(
        Some("https://example.invalid"),
        None,
        Some("x"),
        temp.path(),
    )
    .expect_err("no --license must fail");
    assert!(
        error.to_string().contains("--license"),
        "and the license has no derivation at all: {error}"
    );
}
