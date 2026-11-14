//! The Tier 3 gate: one command that runs the whole verification matrix for a repository.
//!
//! The matrix was typed by hand for every commit, which is how a lockfile once went unstaged and a
//! `#[cfg(test)]` attribute once ended up attached to the wrong module. Each step is one process
//! (or, for lockfiles, a native call so the gate cannot recurse into itself), run in order, with a
//! non-zero exit when any step fails.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::OutfeedError;
use crate::progress::{ProgressReporter, RepoStatus};

mod execute;

use execute::execute;
pub use execute::format_gate_report;

/// What a gate step actually executes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateCommand {
    /// Spawn `program` with `args` inside the repository.
    Process {
        /// Executable name resolved on `PATH`.
        program: String,
        /// Arguments, including any `--` separator the child needs.
        args: Vec<String>,
    },
    /// Run the native lockfile audit instead of a subprocess.
    Lockfiles,
    /// Run the suite layer gate, which is only meaningful where the allowlist lives: the layer map
    /// spans 42 crates across 11 repositories, so a single repository cannot evaluate it (`GB-200`).
    Layers,
    /// Index the repository into a temporary database, then check polyglot contract drift against it
    /// (`OF-39`).
    ///
    /// `miter contracts` reads an index, and a member repository has none until somebody runs `miter index`
    /// there by hand - so the step failed from a clean checkout, which is exactly where a gate has to work.
    /// It indexes into the temp directory rather than the repository's own database so that running a gate
    /// leaves no artefact behind, and so that a stale checked-in index can never decide a result.
    Contracts,
}

/// One entry in the verification matrix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateStep {
    /// Stable short name, used by `--skip` and in reports.
    pub name: String,
    /// One-line description of what the step proves.
    pub description: String,
    /// The command to run.
    pub command: GateCommand,
}

/// Outcome of one step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateStepResult {
    /// Step name.
    pub name: String,
    /// Whether the step passed; a skip is not a pass.
    pub passed: bool,
    /// Whether the step was skipped by request.
    pub skipped: bool,
    /// The child's own summary line.
    pub detail: String,
}

/// Aggregate result of a gate run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReport {
    /// One result per step, in execution order.
    pub results: Vec<GateStepResult>,
}

impl GateReport {
    /// `true` when no step failed; skipped steps do not fail the gate.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.failed() == 0
    }

    /// Number of steps that passed.
    #[must_use]
    pub fn passed(&self) -> usize {
        self.results
            .iter()
            .filter(|result| result.passed && !result.skipped)
            .count()
    }

    /// Number of steps that failed.
    #[must_use]
    pub fn failed(&self) -> usize {
        self.results.iter().filter(|result| !result.passed).count()
    }

    /// Number of steps skipped on request.
    #[must_use]
    pub fn skipped(&self) -> usize {
        self.results.iter().filter(|result| result.skipped).count()
    }

    /// Name of the first failing step, for a one-line error.
    #[must_use]
    pub fn first_failure(&self) -> Option<&GateStepResult> {
        self.results.iter().find(|result| !result.passed)
    }
}

/// The canonical Tier 3 matrix, in the order the release checklist prescribes.
#[must_use]
pub fn gate_steps() -> Vec<GateStep> {
    let step = |name: &str, description: &str, program: &str, args: &[&str]| GateStep {
        name: name.to_string(),
        description: description.to_string(),
        command: GateCommand::Process {
            program: program.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
        },
    };

    vec![
        step(
            "fmt",
            "formatting is canonical",
            "cargo",
            &["fmt", "--all", "--", "--check"],
        ),
        step(
            "clippy",
            "no lint survives as a warning",
            "cargo",
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        step(
            "tests",
            "the whole workspace suite passes",
            "cargo",
            &["nextest", "run", "--workspace"],
        ),
        // `--check` is what makes this a gate: without the flag `miter contracts` describes drift and
        // succeeds, which is why this step could never fail (GB-223). The index it reads is built by the
        // step itself, into a temp database (OF-39).
        GateStep {
            name: "contracts".to_string(),
            description: "polyglot wire contracts agree".to_string(),
            command: GateCommand::Contracts,
        },
        GateStep {
            name: "layers".to_string(),
            description: "no cross-product crate edge is unexplained".to_string(),
            command: GateCommand::Layers,
        },
        GateStep {
            name: "lockfiles".to_string(),
            description: "every lockfile is tracked and fresh".to_string(),
            command: GateCommand::Lockfiles,
        },
        step(
            "docs",
            "documentation matches the AST",
            "chalkline",
            &["docs", "check", "-d", "."],
        ),
    ]
}

/// Runs the matrix in `dir`, skipping any step named in `skip`.
///
/// # Errors
///
/// [`OutfeedError::Gate`] when a step cannot be spawned at all (a missing binary is a gate failure,
/// not an environment excuse) and [`OutfeedError::Io`] when the lockfile audit cannot read `dir`.
/// A step that runs and fails is reported inside the [`GateReport`], so every outcome is visible.
pub fn run_gate(
    dir: &Path,
    skip: &[String],
    reporter: &dyn ProgressReporter,
) -> Result<GateReport, OutfeedError> {
    run_gate_steps(dir, gate_steps(), skip, reporter)
}

/// Runs an explicit step list, which is how the matrix is exercised in tests.
///
/// # Errors
///
/// As [`run_gate`].
pub fn run_gate_steps(
    dir: &Path,
    steps: Vec<GateStep>,
    skip: &[String],
    reporter: &dyn ProgressReporter,
) -> Result<GateReport, OutfeedError> {
    let mut results = Vec::new();

    for step in steps {
        if skip.iter().any(|name| name == &step.name) {
            reporter.on_complete(
                &step.name,
                &RepoStatus::Skipped("skipped by request".into()),
            );
            results.push(GateStepResult {
                name: step.name,
                passed: true,
                skipped: true,
                detail: "skipped".to_string(),
            });
            continue;
        }

        reporter.on_start(&step.name, &step.description);
        let mut result = execute(dir, &step.command)?;
        result.name = step.name.clone();

        let status = if result.passed {
            RepoStatus::Success(result.detail.clone())
        } else {
            RepoStatus::Failed(result.detail.clone())
        };
        reporter.on_complete(&step.name, &status);
        results.push(result);
    }

    Ok(GateReport { results })
}
