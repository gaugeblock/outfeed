//! Do the members stand alone? (`OF-41`)
//!
//! Law 2 says each repository compiles and is useful without the suite. While developing multi-repo, the
//! root `.cargo/config.toml` rewrites `[patch.crates-io]` to sibling paths, so a member builds against
//! whatever is on disk rather than against the versions it declares - which hides both a wrong version
//! constraint and an API that has not been published yet. The check is a resolution from *outside* the
//! suite: cargo reads `.cargo/config.toml` from the working directory upward, so running elsewhere with
//! `--manifest-path` is what makes the patches inapplicable.
//!
//! Scope, deliberately: this resolves the dependency graph and does not compile it. That answers "published
//! crates only", which is what the ticket asks. Whether the code still *compiles* against those versions is
//! a different question that needs a build, and it is filed separately rather than pretended here.

use std::path::{Path, PathBuf};
use std::process::Command;

/// What happened when one member resolved outside the harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StandaloneOutcome {
    /// Resolved against the published ecosystem with no patches applied.
    Standalone,
    /// Resolution reached a crate this repository names but crates.io does not publish yet.
    NeedsUnpublished {
        /// The first package the registry could not supply.
        package: String,
    },
    /// Resolution failed for any other reason, which is a real defect.
    Broken {
        /// First useful line of cargo's output.
        detail: String,
    },
}

/// One member's verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandaloneMember {
    /// Directory name, which is also the member name.
    pub member: String,
    /// What the resolution did.
    pub outcome: StandaloneOutcome,
}

/// The whole fleet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandaloneReport {
    /// One entry per member, in directory order.
    pub members: Vec<StandaloneMember>,
}

impl StandaloneReport {
    /// True when every member resolved against published crates.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.members
            .iter()
            .all(|member| member.outcome == StandaloneOutcome::Standalone)
    }

    /// Members that are not standalone, with the reason.
    #[must_use]
    pub fn failures(&self) -> Vec<&StandaloneMember> {
        self.members
            .iter()
            .filter(|member| member.outcome != StandaloneOutcome::Standalone)
            .collect()
    }
}

/// Resolves every member in `suite_root` from outside the suite (`OF-41`).
///
/// `scratch` is where cargo runs; it must not be inside the suite, because that is precisely what makes the
/// `[patch.crates-io]` table inapplicable. Callers pass a temp directory.
#[must_use]
pub fn check_standalone(suite_root: &Path, scratch: &Path, members: &[String]) -> StandaloneReport {
    let mut report = Vec::new();
    for member in members {
        let manifest = suite_root.join(member).join("Cargo.toml");
        if !manifest.is_file() {
            report.push(StandaloneMember {
                member: member.clone(),
                outcome: StandaloneOutcome::Broken {
                    detail: "no Cargo.toml".to_string(),
                },
            });
            continue;
        }
        report.push(StandaloneMember {
            member: member.clone(),
            outcome: resolve(scratch, &manifest),
        });
    }
    StandaloneReport { members: report }
}

/// Runs one resolution and classifies the result.
fn resolve(scratch: &Path, manifest: &Path) -> StandaloneOutcome {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(manifest)
        .current_dir(scratch)
        .output();
    let Ok(output) = output else {
        return StandaloneOutcome::Broken {
            detail: "cargo could not be spawned".to_string(),
        };
    };
    if output.status.success() {
        return StandaloneOutcome::Standalone;
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    match unpublished_package(&stderr) {
        Some(package) => StandaloneOutcome::NeedsUnpublished { package },
        None => StandaloneOutcome::Broken {
            detail: first_meaningful_line(&stderr),
        },
    }
}

/// The package cargo says it cannot supply, if that is what happened.
///
/// Both spellings are recognised: a version that does not exist, and a name that is not published at all.
/// Anything else - a parse error, a missing path - is a real defect and must not be excused as "not
/// published yet", because that excuse is how a gate stops being one.
fn unpublished_package(stderr: &str) -> Option<String> {
    for line in stderr.lines() {
        let line = line.trim();
        if let Some(rest) = line.split("no matching package named").nth(1) {
            return Some(first_quoted(rest));
        }
        if line.contains("failed to select a version")
            && let Some(rest) = line.split("requirement `").nth(1)
        {
            // The requirement reads `name = "^0.2.0"`, so the name ends at the first space.
            return Some(
                rest.split([' ', '`'])
                    .next()
                    .unwrap_or("<unidentified>")
                    .to_string(),
            );
        }
    }
    None
}

/// The contents of the first quoted word in `text`.
fn first_quoted(text: &str) -> String {
    text.trim_start()
        .trim_start_matches(['`', '"'])
        .split(['`', '"'])
        .next()
        .unwrap_or("<unidentified>")
        .to_string()
}

/// The first line that is not cargo's boilerplate, so a report reads as a sentence.
fn first_meaningful_line(stderr: &str) -> String {
    stderr
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty() && !line.starts_with("Updating") && !line.starts_with("Blocking")
        })
        .unwrap_or("cargo failed with no output")
        .to_string()
}

/// Where a caller should run the resolutions: a temp directory outside the suite.
#[must_use]
pub fn scratch_dir() -> PathBuf {
    std::env::temp_dir().join(format!("outfeed-standalone-{}", std::process::id()))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn an_unpublished_name_is_recognised() {
        let stderr = "error: failed to get `storypole-core` as a dependency of package `rabbet`\n\
                      Caused by: no matching package named `storypole-core` found";
        assert_eq!(
            unpublished_package(stderr).as_deref(),
            Some("storypole-core")
        );
    }

    #[test]
    fn an_unpublished_version_is_recognised() {
        let stderr =
            "error: failed to select a version for the requirement `miter-engine = \"^0.2.0\"`";
        assert_eq!(unpublished_package(stderr).as_deref(), Some("miter-engine"));
    }

    #[test]
    fn a_real_defect_is_not_excused_as_unpublished() {
        // The excuse is how a gate stops being one: a manifest that does not parse must fail the check,
        // not be reported as "waiting on a publish".
        let stderr = "error: failed to parse manifest at `/suite/miter/Cargo.toml`";
        assert_eq!(unpublished_package(stderr), None);
    }

    #[test]
    fn the_report_summarises_failures() {
        let report = StandaloneReport {
            members: vec![
                StandaloneMember {
                    member: "a".into(),
                    outcome: StandaloneOutcome::Standalone,
                },
                StandaloneMember {
                    member: "b".into(),
                    outcome: StandaloneOutcome::NeedsUnpublished {
                        package: "x".into(),
                    },
                },
            ],
        };
        assert!(!report.is_ok());
        assert_eq!(report.failures().len(), 1);
        assert_eq!(report.failures()[0].member, "b");
    }
}
