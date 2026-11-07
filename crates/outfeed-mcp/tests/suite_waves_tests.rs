#![allow(clippy::expect_used, clippy::unwrap_used)]

//! Integration tests for topological release wave partitioning.

use outfeed_mcp::orchestrator::suite_plan::{
    SuitePlanAction, SuiteReleasePlan, SuiteRepoPlanEntry,
};
use outfeed_mcp::orchestrator::suite_waves::{SUITE_RELEASE_WAVES, partition_release_waves};

fn entry(repo_name: &str, action: SuitePlanAction) -> SuiteRepoPlanEntry {
    SuiteRepoPlanEntry {
        repo_name: repo_name.to_string(),
        current_version: "v0.1.0-beta.1".to_string(),
        target_version: Some("v0.1.0-beta.2".to_string()),
        action,
        unreleased_commits: 1,
        referenced_tasks: Vec::new(),
        plan: None,
    }
}

fn plan_of(entries: Vec<SuiteRepoPlanEntry>) -> SuiteReleasePlan {
    SuiteReleasePlan { entries }
}

fn names(wave: &[&SuiteRepoPlanEntry]) -> Vec<String> {
    wave.iter().map(|entry| entry.repo_name.clone()).collect()
}

#[test]
fn test_wave_table_covers_all_eleven_members_once() {
    let flat: Vec<&str> = SUITE_RELEASE_WAVES
        .iter()
        .flat_map(|wave| wave.iter().copied())
        .collect();
    assert_eq!(
        flat.len(),
        11,
        "every suite member must appear exactly once"
    );
    for expected in [
        "common",
        "miter",
        "storypole",
        "sawhorse",
        "chalkline",
        "dado",
        "batten",
        "joiner",
        "benchwork",
        "rabbet",
        "outfeed",
    ] {
        assert_eq!(
            flat.iter().filter(|name| **name == expected).count(),
            1,
            "{expected} must be scheduled exactly once"
        );
    }
}

#[test]
fn test_partition_groups_by_wave_table_and_orders_within_wave_by_plan() {
    // Deliberately reversed plan order: wave *membership* must follow the dependency table while
    // *position inside a wave* follows plan order, keeping reporting deterministic.
    let plan = plan_of(
        [
            "outfeed",
            "benchwork",
            "joiner",
            "batten",
            "chalkline",
            "rabbet",
            "dado",
            "sawhorse",
            "storypole",
            "miter",
        ]
        .into_iter()
        .map(|name| entry(name, SuitePlanAction::ReleasePatch))
        .collect(),
    );

    let waves = partition_release_waves(&plan);
    let shape: Vec<Vec<String>> = waves.iter().map(|wave| names(wave)).collect();

    assert_eq!(
        shape,
        vec![
            vec!["miter".to_string()],
            vec![
                "rabbet".to_string(),
                "dado".to_string(),
                "sawhorse".to_string(),
                "storypole".to_string()
            ],
            vec!["batten".to_string(), "chalkline".to_string()],
            vec!["joiner".to_string()],
            vec!["outfeed".to_string(), "benchwork".to_string()],
        ]
    );
}

#[test]
fn test_partition_skips_clean_entries_and_keeps_unknown_repositories() {
    let plan = plan_of(vec![
        entry("miter", SuitePlanAction::ReleaseMinor),
        entry("storypole", SuitePlanAction::SkipClean),
        entry("raffle", SuitePlanAction::PublishPending),
        entry("sawhorse", SuitePlanAction::ReleasePatch),
    ]);

    let waves = partition_release_waves(&plan);
    let shape: Vec<Vec<String>> = waves.iter().map(|wave| names(wave)).collect();

    assert_eq!(
        shape,
        vec![
            vec!["miter".to_string()],
            vec!["sawhorse".to_string()],
            // Unknown members are released last so nothing is silently dropped.
            vec!["raffle".to_string()],
        ],
        "clean repositories are skipped while unmapped ones trail the schedule"
    );
}

#[test]
fn test_partition_executes_push_pending_entries() {
    // GB-153: a repository that is clean but ahead of its remote must reach the schedule, otherwise
    // the release pass reports success while the tag stays on one machine.
    let plan = plan_of(vec![
        entry("miter", SuitePlanAction::SkipClean),
        entry("storypole", SuitePlanAction::PushPending),
    ]);

    let waves = partition_release_waves(&plan);
    let shape: Vec<Vec<String>> = waves.iter().map(|wave| names(wave)).collect();

    assert_eq!(
        shape,
        vec![vec!["storypole".to_string()]],
        "a pending push is work, not a skip"
    );
    assert_eq!(SuitePlanAction::PushPending.as_str(), "Push (Ahead)");
}

#[test]
fn test_partition_empty_and_all_clean_plans_produce_no_waves() {
    assert!(partition_release_waves(&plan_of(Vec::new())).is_empty());
    let all_clean = plan_of(vec![
        entry("miter", SuitePlanAction::SkipClean),
        entry("outfeed", SuitePlanAction::SkipClean),
    ]);
    assert!(partition_release_waves(&all_clean).is_empty());
}
