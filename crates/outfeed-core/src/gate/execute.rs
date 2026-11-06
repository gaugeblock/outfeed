//! Step execution and reporting for the Tier 3 gate.

use std::path::Path;
use std::process::Command;

use super::{GateCommand, GateReport, GateStepResult};
use crate::error::OutfeedError;

/// Executes one step and reduces its output to a single reporting line.
pub(super) fn execute(dir: &Path, command: &GateCommand) -> Result<GateStepResult, OutfeedError> {
    match command {
        GateCommand::Lockfiles => {
            let report = crate::lockfiles::check_lockfiles(dir)?;
            if report.repositories.is_empty() {
                return Ok(GateStepResult {
                    name: String::new(),
                    passed: true,
                    skipped: true,
                    detail: format!("no member repositories under {}", dir.display()),
                });
            }
            let passed = report.is_ok();
            let detail = if passed {
                format!(
                    "{} repositories tracked and fresh",
                    report.repositories.len()
                )
            } else {
                format!("{} repository lockfile problem(s)", report.failures().len())
            };
            Ok(GateStepResult {
                name: String::new(),
                passed,
                skipped: false,
                detail,
            })
        }
        GateCommand::Layers => {
            // The allowlist marks the suite root. Elsewhere the map would only ever contain one
            // repository's crates, where every edge is internal or downward, so a green result would
            // mean nothing - report it as skipped rather than as a pass.
            let allowlist = dir.join(".gaugeblock/miter/layers-allowlist.toml");
            if !allowlist.exists() {
                return Ok(GateStepResult {
                    name: String::new(),
                    passed: true,
                    skipped: true,
                    detail: format!("not the suite root (no {})", allowlist.display()),
                });
            }
            execute(
                dir,
                &GateCommand::Process {
                    program: "miter".to_string(),
                    args: ["deps", "--layers", "--check"]
                        .iter()
                        .map(|arg| (*arg).to_string())
                        .collect(),
                },
            )
        }
        GateCommand::Process { program, args } => {
            // GB-243: hand the run's trace to the child so its spans join ours. One trace per gate
            // process, so every step of a run renders inside a single waterfall instead of as unrelated
            // roots. No parent span id: a gate step is a root of its own sub-tree.
            let mut command = Command::new(program);
            command.args(args).current_dir(dir).env(
                crate::trace::TRACE_ENV,
                crate::trace::trace_id().to_string(),
            );
            let output = command.output().map_err(|error| {
                OutfeedError::Gate(format!("{program} could not be spawned: {error}"))
            })?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let passed = output.status.success();
            Ok(GateStepResult {
                name: String::new(),
                passed,
                skipped: false,
                detail: first_meaningful_line(&stdout, &stderr, passed),
            })
        }
    }
}

/// Picks the line worth showing: an error line when failing, a summary when passing.
fn first_meaningful_line(stdout: &str, stderr: &str, passed: bool) -> String {
    let source = if passed { stdout } else { stderr };

    let wanted = |trimmed: &str| {
        if passed {
            trimmed.starts_with("Summary")
        } else {
            trimmed.to_ascii_lowercase().starts_with("error")
        }
    };
    if let Some(line) = source.lines().map(str::trim).find(|line| wanted(line)) {
        return line.to_string();
    }

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("warning: patch")
            || trimmed.starts_with("help:")
            || trimmed.starts_with('=')
            || trimmed.starts_with('|')
        {
            continue;
        }
        return trimmed.to_string();
    }

    if passed {
        "ok".to_string()
    } else {
        "failed with no output".to_string()
    }
}

/// Renders the matrix for a terminal.
#[must_use]
pub fn format_gate_report(report: &GateReport) -> String {
    let mut out = String::new();
    for result in &report.results {
        let marker = if result.skipped {
            "[skip]"
        } else if result.passed {
            "[ ok ]"
        } else {
            "[fail]"
        };
        out.push_str(&format!("{marker} {:<10} {}\n", result.name, result.detail));
    }
    out.push_str(&format!(
        "{} passed, {} failed, {} skipped\n",
        report.passed(),
        report.failed(),
        report.skipped()
    ));
    out
}
