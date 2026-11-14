//! Step execution and reporting for the Tier 3 gate.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{GateCommand, GateReport, GateStepResult};
use crate::error::OutfeedError;

/// Executes one step and reduces its output to a single reporting line.
pub(super) fn execute(dir: &Path, command: &GateCommand) -> Result<GateStepResult, OutfeedError> {
    match command {
        GateCommand::Lockfiles => {
            // Two callers, two meanings. The gate runs at whatever directory it is handed, and "no
            // member repositories here" is a statement about scope, so it skips. The CLI errors on the
            // same condition, because there the caller asked for a check and would otherwise be told
            // everything passed while nothing was examined (`GB-327`).
            let report = match crate::lockfiles::check_lockfiles(dir) {
                Ok(report) => report,
                Err(OutfeedError::NoRepositories { .. }) => {
                    return Ok(GateStepResult {
                        name: String::new(),
                        passed: true,
                        skipped: true,
                        detail: format!("no member repositories under {}", dir.display()),
                    });
                }
                Err(other) => return Err(other),
            };
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
        GateCommand::Contracts => {
            // `miter contracts` reads an index; a member repository has none until somebody builds one by
            // hand, so this step used to fail precisely where a gate must work (`OF-39`). Building it here,
            // into the temp directory, keeps the repository clean and makes the result independent of any
            // index left lying around by an earlier run.
            let db = std::env::temp_dir().join(format!(
                "outfeed-gate-contracts-{}-{}.redb",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0)
            ));
            let indexed = execute(
                dir,
                &GateCommand::Process {
                    program: "miter".to_string(),
                    args: vec![
                        "index".to_string(),
                        // `.` because the child already runs with `dir` as its working directory: passing
                        // `dir` as well doubled a relative root into `repo/repo` and miter reported an
                        // ingestion i/o error, which read like a missing index rather than a bad argument.
                        ".".to_string(),
                        "--db".to_string(),
                        db.display().to_string(),
                    ],
                },
            )?;
            if !indexed.passed {
                let _ = std::fs::remove_file(&db);
                return Ok(GateStepResult {
                    name: String::new(),
                    passed: false,
                    skipped: false,
                    detail: format!(
                        "miter index failed, so contract drift cannot be judged: {}",
                        indexed.detail
                    ),
                });
            }
            let checked = execute(
                dir,
                &GateCommand::Process {
                    program: "miter".to_string(),
                    args: {
                        let mut args = vec![
                            "contracts".to_string(),
                            "--check".to_string(),
                            "--db".to_string(),
                            db.display().to_string(),
                        ];
                        // The allowlist declares non-mirrors - fields that are deliberately absent on one
                        // side, like a secret that never reaches TypeScript. It lives at the suite root, so
                        // a member checking without it reported the suite's own decision as drift
                        // (`role_override_config.api_key`). Same shape as `Layers`: the allowlist is a gate
                        // input, and a gate input that lives elsewhere has to be passed.
                        if let Some(allowlist) = contracts_allowlist(dir) {
                            args.push("--allowlist".to_string());
                            args.push(allowlist.display().to_string());
                            args.push("--path".to_string());
                            args.push(
                                allowlist
                                    .parent()
                                    .and_then(|miter_dir| miter_dir.parent())
                                    .and_then(|gaugeblock_dir| gaugeblock_dir.parent())
                                    .map(|root| root.display().to_string())
                                    .unwrap_or_else(|| dir.display().to_string()),
                            );
                        }
                        args
                    },
                },
            );
            let _ = std::fs::remove_file(&db);
            checked
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

/// The contracts allowlist that governs `dir`: the suite's when this is a member, else its own.
///
/// The allowlist declares non-mirrors, so the file that holds the decision decides the result - and it lives at
/// the suite root while the check runs in a member (`OF-39`). Reads only; a gate never writes an allowlist.
fn contracts_allowlist(dir: &Path) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let mut candidates = Vec::new();
    if let Some(membership) = gaugeblock_workspace::Membership::containing(&canonical) {
        candidates.push(
            membership
                .suite_root
                .join(".gaugeblock/miter/contracts-allowlist.toml"),
        );
    }
    candidates.push(canonical.join(".gaugeblock/miter/contracts-allowlist.toml"));
    candidates.into_iter().find(|path| path.is_file())
}

#[cfg(test)]
mod allowlist_tests;
