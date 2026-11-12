//! Does every member still build? (`OF-43`)
//!
//! `SP-41` added three optional fields to a shared contract type. The owning repository's gate ran green,
//! because it built its own workspace. Two consumers construct that type literally and do not spread
//! defaults - `joiner-core` and a `benchwork-core` test - so both stopped compiling, and nothing reported it
//! for ten tickets. It surfaced when an unrelated verification happened to pull those crates into its build.
//!
//! Nothing in the suite builds across repositories, which is the whole gap: `miter contracts` compares
//! schemas, `outfeed standalone` resolves without compiling, and each repository's chain sees only itself.
//! This compiles every member against the workspace as checked out - harness config in effect, the opposite
//! of `standalone` - and reports the crate plus its first diagnostic.
//!
//! Cost is bounded by the shared target directory: a warm suite re-checks in seconds per member, because
//! nothing it needs has changed. A cold one pays a full build, which is why this belongs at the
//! atomic-commit gate for changes that touch a shared crate rather than in every inner loop.

use std::path::Path;
use std::process::Command;

/// One member's build verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerBuild {
    /// Directory name, which is also the member name.
    pub member: String,
    /// The first diagnostic when the build failed, `None` when it succeeded.
    pub diagnostic: Option<String>,
}

/// Every member's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerReport {
    /// One entry per member, in the order given.
    pub builds: Vec<ConsumerBuild>,
}

impl ConsumerReport {
    /// True when every member built.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.builds.iter().all(|build| build.diagnostic.is_none())
    }

    /// Members that did not build, with the diagnostic that says why.
    #[must_use]
    pub fn failures(&self) -> Vec<&ConsumerBuild> {
        self.builds
            .iter()
            .filter(|build| build.diagnostic.is_some())
            .collect()
    }
}

/// The members a suite declares, for a gate that has to walk all of them.
///
/// Empty when there is no suite config, which callers must treat as an error rather than a pass: a gate that
/// checks nothing and reports success is the failure mode `lockfiles` already hit once.
#[must_use]
pub fn suite_members(root: &Path) -> Vec<String> {
    gaugeblock_workspace::SuiteConfig::load(root)
        .map(|config| config.members().to_vec())
        .unwrap_or_default()
}

/// Compiles every member in `suite_root`, from inside the suite (`OF-43`).
///
/// Deliberately the opposite of [`crate::standalone`]: that one resolves from outside to prove the published
/// ecosystem works, this one builds from inside to prove the *checkout* does. A member that fails here is a
/// consumer left behind by a change somewhere else, which is exactly what nobody was checking.
#[must_use]
pub fn check_consumers(suite_root: &Path, members: &[String]) -> ConsumerReport {
    check_consumers_with(suite_root, members, |_| {})
}

/// Same, reporting each verdict as it lands.
///
/// A full suite takes minutes on a cold target directory, and the first live run sat silent that whole time
/// because the report is printed at the end - indistinguishable from a hang. Callers stream the callback
/// somewhere a human can see, which is why this is a separate entry point rather than an extra field.
#[must_use]
pub fn check_consumers_with<F>(
    suite_root: &Path,
    members: &[String],
    mut on_member: F,
) -> ConsumerReport
where
    F: FnMut(&ConsumerBuild),
{
    let mut builds = Vec::new();
    for member in members {
        let manifest = suite_root.join(member).join("Cargo.toml");
        let diagnostic = if manifest.is_file() {
            diagnostic_of(suite_root, &manifest)
        } else {
            Some("no Cargo.toml".to_string())
        };
        let build = ConsumerBuild {
            member: member.clone(),
            diagnostic,
        };
        on_member(&build);
        builds.push(build);
    }
    ConsumerReport { builds }
}

/// Runs one `cargo check` and returns its first diagnostic, or `None` on success.
fn diagnostic_of(suite_root: &Path, manifest: &Path) -> Option<String> {
    let output = Command::new("cargo")
        .args([
            "check",
            "--quiet",
            "--message-format",
            "short",
            "--manifest-path",
        ])
        .arg(manifest)
        .current_dir(suite_root)
        .output()
        .ok()?;
    if output.status.success() {
        return None;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Some(first_error(&stderr))
}

/// The first line that names a problem rather than progress.
///
/// `--message-format short` renders a diagnostic as `path:line:col: error[E0063]: ...`, which is the one line
/// a reader needs: it names the crate's file and the compiler's reason together.
fn first_error(stderr: &str) -> String {
    stderr
        .lines()
        .map(str::trim)
        .find(|line| line.contains(": error"))
        .or_else(|| {
            stderr
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty() && !line.starts_with("Blocking"))
        })
        .unwrap_or("cargo check failed with no output")
        .to_string()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn a_short_diagnostic_is_preferred_over_progress_noise() {
        let stderr = "Blocking waiting for file lock on package cache\n\
                      Updating crates.io index\n\
                      crates/consumer/src/lib.rs:12:9: error[E0063]: missing field `diff` in initializer\n\
                      error: could not compile `consumer` (lib)";
        assert_eq!(
            first_error(stderr),
            "crates/consumer/src/lib.rs:12:9: error[E0063]: missing field `diff` in initializer"
        );
    }

    #[test]
    fn a_report_names_the_member_that_failed() {
        let report = ConsumerReport {
            builds: vec![
                ConsumerBuild {
                    member: "healthy".into(),
                    diagnostic: None,
                },
                ConsumerBuild {
                    member: "left-behind".into(),
                    diagnostic: Some("error[E0063]: missing field".into()),
                },
            ],
        };
        assert!(!report.is_ok());
        let failures = report.failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].member, "left-behind");
    }
}
