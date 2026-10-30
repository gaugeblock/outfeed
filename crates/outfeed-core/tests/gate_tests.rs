//! Tests for the Tier 3 gate (GB-168).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::Path;

use outfeed_core::gate::{GateCommand, GateStep};
use outfeed_core::gate::{format_gate_report, gate_steps, run_gate, run_gate_steps};
use outfeed_core::progress::NoopProgress;

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn the_matrix_runs_in_release_checklist_order() {
    let names: Vec<String> = gate_steps().into_iter().map(|step| step.name).collect();
    assert_eq!(
        names,
        vec![
            "fmt",
            "clippy",
            "tests",
            "contracts",
            "layers",
            "lockfiles",
            "docs"
        ],
        "a gate that reorders its steps is a different gate"
    );
}

#[test]
fn skipping_every_step_touches_nothing_and_reports_clean() {
    let dir = temp_dir();
    let steps = gate_steps();
    let expected = steps.len();
    let skip: Vec<String> = steps.into_iter().map(|step| step.name).collect();

    let report = run_gate(dir.path(), &skip, &NoopProgress).expect("gate");

    assert!(report.is_clean(), "skips do not fail the gate");
    assert_eq!(
        report.skipped(),
        expected,
        "every step in the matrix is skipped, however many there are"
    );
    assert_eq!(report.passed(), 0, "a skip is not a pass");
    assert!(report.first_failure().is_none());
}

#[test]
fn a_missing_tool_fails_its_step_instead_of_panicking() {
    let dir = temp_dir();
    let steps = vec![GateStep {
        name: "ghost".to_string(),
        description: "a binary that does not exist".to_string(),
        command: GateCommand::Process {
            program: "gaugeblock-definitely-not-installed".to_string(),
            args: vec!["--check".to_string()],
        },
    }];

    let error = run_gate_steps(dir.path(), steps, &[], &NoopProgress)
        .expect_err("a missing binary is a gate error, not a silent pass");
    assert!(
        error
            .to_string()
            .contains("gaugeblock-definitely-not-installed"),
        "the error names the tool: {error}"
    );
}

#[test]
fn a_failing_command_is_reported_with_its_own_line() {
    let dir = temp_dir();
    let steps = vec![GateStep {
        name: "explodes".to_string(),
        description: "exits non-zero".to_string(),
        command: GateCommand::Process {
            program: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "echo 'boom: bad thing' >&2; exit 3".to_string(),
            ],
        },
    }];

    let report = run_gate_steps(dir.path(), steps, &[], &NoopProgress).expect("gate");

    assert!(!report.is_clean());
    let failure = report.first_failure().expect("a failure is recorded");
    assert_eq!(failure.name, "explodes");
    assert!(
        failure.detail.contains("boom"),
        "the child's line is surfaced: {}",
        failure.detail
    );
}

#[test]
fn the_report_marks_each_outcome_distinctly() {
    let dir = temp_dir();
    let steps = vec![
        GateStep {
            name: "good".to_string(),
            description: "passes".to_string(),
            command: GateCommand::Process {
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "echo fine".to_string()],
            },
        },
        GateStep {
            name: "bad".to_string(),
            description: "fails".to_string(),
            command: GateCommand::Process {
                program: "sh".to_string(),
                args: vec!["-c".to_string(), "echo nope >&2; exit 1".to_string()],
            },
        },
    ];
    let report = run_gate_steps(dir.path(), steps, &[], &NoopProgress).expect("gate");
    let rendered = format_gate_report(&report);

    assert!(rendered.contains("[ ok ] good"), "got:\n{rendered}");
    assert!(rendered.contains("[fail] bad"), "got:\n{rendered}");
    assert!(rendered.contains("1 passed, 1 failed"), "got:\n{rendered}");
}

#[test]
fn the_lockfile_step_is_native_not_a_subprocess() {
    // The gate must not shell back into `outfeed`, or a gate run becomes recursive.
    let lockfile_step = gate_steps()
        .into_iter()
        .find(|step| step.name == "lockfiles")
        .expect("lockfiles step");
    assert_eq!(lockfile_step.command, GateCommand::Lockfiles);

    // And on a directory with no member repositories it reports a skip rather than a false pass.
    let dir = temp_dir();
    let report = run_gate(Path::new(dir.path()), &[], &NoopProgress).expect("gate");
    let lockfiles = report
        .results
        .iter()
        .find(|result| result.name == "lockfiles")
        .expect("lockfiles result");
    assert!(lockfiles.skipped, "no members means nothing to check");
}

#[test]
fn the_layer_step_skips_where_the_map_cannot_span_repositories() {
    // GB-219: the layer map is a suite property. In a repository that has no allowlist there is no
    // suite, so a green result would be meaningless - the step must report a skip, not a pass.
    let dir = temp_dir();
    let layers = gate_steps()
        .into_iter()
        .find(|step| step.name == "layers")
        .expect("layers step");
    assert_eq!(layers.command, GateCommand::Layers);

    let report = run_gate_steps(dir.path(), vec![layers], &[], &NoopProgress).expect("gate");
    let result = &report.results[0];
    assert!(result.skipped, "expected a skip, got: {}", result.detail);
    assert!(
        result.detail.contains("not the suite root"),
        "the skip must say why: {}",
        result.detail
    );
}
