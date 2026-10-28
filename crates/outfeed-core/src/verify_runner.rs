//! Tier 4 verification runner selection: `cargo-nextest` with a `cargo test` fallback.

use std::path::Path;
use std::process::Command;

use crate::cmd::run_captured;
use crate::error::OutfeedError;
use crate::progress::ProgressReporter;

/// Test runner used by the Tier 4 verification gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestRunner {
    /// `cargo nextest run --workspace`, followed by `cargo test --doc --workspace`
    /// because nextest does not execute doctests.
    Nextest,
    /// `cargo test --workspace`, which already includes doctests.
    CargoTest,
}

impl TestRunner {
    /// Detects `cargo-nextest` on `PATH`.
    pub fn detect() -> Self {
        let available = Command::new("cargo")
            .args(["nextest", "--version"])
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        Self::choose(available)
    }

    /// Maps nextest availability onto a runner.
    ///
    /// Split out from [`Self::detect`] so the selection policy stays unit-testable without
    /// controlling the process `PATH`.
    pub fn choose(nextest_available: bool) -> Self {
        if nextest_available {
            Self::Nextest
        } else {
            Self::CargoTest
        }
    }

    /// Human-readable runner label used in progress output.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Nextest => "cargo nextest",
            Self::CargoTest => "cargo test",
        }
    }
}

/// Runs the workspace verification suite with the selected runner.
///
/// Returns the failing command's combined output wrapped in an [`OutfeedError::Git`] so the caller
/// can roll back manifest edits with a diagnostic intact.
pub fn run_workspace_tests(
    repo_dir: &Path,
    runner: TestRunner,
    repo_name: &str,
    reporter: &dyn ProgressReporter,
) -> Result<(), OutfeedError> {
    reporter.on_update(
        repo_name,
        &format!("running verification tests ({})...", runner.label()),
    );

    match runner {
        TestRunner::Nextest => {
            run_step(repo_dir, &["nextest", "run", "--workspace"], runner)?;
            run_step(repo_dir, &["test", "--doc", "--workspace"], runner)
        }
        TestRunner::CargoTest => run_step(repo_dir, &["test", "--workspace"], runner),
    }
}

/// Runs a single `cargo` subcommand, converting failure into a verification error.
fn run_step(repo_dir: &Path, args: &[&str], runner: TestRunner) -> Result<(), OutfeedError> {
    let mut cmd = Command::new("cargo");
    cmd.args(args).current_dir(repo_dir);
    let output = run_captured(&mut cmd)?;
    if output.success {
        return Ok(());
    }
    Err(OutfeedError::Git(format!(
        "Tier 4 verification failed ({}); rolled back all manifest edits:\n{}",
        runner.label(),
        output.combined_output()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choose_prefers_nextest_when_available() {
        assert_eq!(TestRunner::choose(true), TestRunner::Nextest);
        assert_eq!(TestRunner::choose(false), TestRunner::CargoTest);
    }

    #[test]
    fn test_runner_labels_are_stable() {
        assert_eq!(TestRunner::Nextest.label(), "cargo nextest");
        assert_eq!(TestRunner::CargoTest.label(), "cargo test");
    }

    #[test]
    fn test_detect_returns_a_supported_runner() {
        let runner = TestRunner::detect();
        assert!(matches!(
            runner,
            TestRunner::Nextest | TestRunner::CargoTest
        ));
    }
}
