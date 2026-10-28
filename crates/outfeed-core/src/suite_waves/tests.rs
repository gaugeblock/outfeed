//! Unit tests for wave partitioning and the bounded worker pool.

#![allow(clippy::expect_used)]

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

fn entry(repo_name: &str) -> SuiteRepoPlanEntry {
    SuiteRepoPlanEntry {
        repo_name: repo_name.to_string(),
        current_version: "v0.1.0-beta.1".to_string(),
        target_version: Some("v0.1.0-beta.2".to_string()),
        action: SuitePlanAction::ReleasePatch,
        unreleased_commits: 1,
        referenced_tasks: Vec::new(),
        plan: None,
    }
}

fn observe(entries: &[SuiteRepoPlanEntry], jobs: usize) -> (usize, Vec<String>) {
    let refs: Vec<&SuiteRepoPlanEntry> = entries.iter().collect();
    let in_flight = AtomicUsize::new(0);
    let max_in_flight = AtomicUsize::new(0);

    let outcomes = run_wave_with(&refs, jobs, |_entry| {
        let active = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        max_in_flight.fetch_max(active, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(60));
        in_flight.fetch_sub(1, Ordering::SeqCst);
        Ok(None)
    });

    (
        max_in_flight.load(Ordering::SeqCst),
        outcomes.into_iter().map(|(name, _)| name).collect(),
    )
}

#[test]
fn test_run_wave_with_caps_concurrency_to_jobs() {
    let entries = vec![
        entry("storypole"),
        entry("sawhorse"),
        entry("dado"),
        entry("rabbet"),
    ];

    let (max_in_flight, order) = observe(&entries, 2);

    assert_eq!(max_in_flight, 2, "at most `jobs` workers may overlap");
    assert_eq!(
        order,
        vec!["storypole", "sawhorse", "dado", "rabbet"],
        "outcomes must follow entry order regardless of completion order"
    );
}

#[test]
fn test_run_wave_with_single_job_is_serial() {
    let entries = vec![entry("chalkline"), entry("batten"), entry("joiner")];

    let (max_in_flight, order) = observe(&entries, 1);

    assert_eq!(max_in_flight, 1, "jobs = 1 must never overlap workers");
    assert_eq!(order, vec!["chalkline", "batten", "joiner"]);
}

#[test]
fn test_run_wave_with_treats_zero_jobs_as_serial() {
    let entries = vec![entry("benchwork"), entry("outfeed")];

    let (max_in_flight, order) = observe(&entries, 0);

    assert_eq!(max_in_flight, 1, "jobs = 0 falls back to a single worker");
    assert_eq!(order, vec!["benchwork", "outfeed"]);
}

#[test]
fn test_run_wave_with_isolates_panicking_worker() {
    let entries = [entry("miter"), entry("storypole")];
    let refs: Vec<&SuiteRepoPlanEntry> = entries.iter().collect();

    let outcomes = run_wave_with(&refs, 2, |entry| {
        if entry.repo_name == "miter" {
            panic!("simulated worker failure");
        }
        Ok(None)
    });

    assert_eq!(outcomes[0].0, "miter");
    assert!(
        outcomes[0].1.is_err(),
        "the panicking worker reports an error"
    );
    assert!(
        format!("{:?}", outcomes[0].1.as_ref().err()).contains("panicked"),
        "the panic is surfaced as a typed error, not a process abort"
    );
    assert!(outcomes[1].1.is_ok(), "sibling workers still complete");
}
