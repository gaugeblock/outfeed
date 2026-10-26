//! Storypole integration: ledger query, chore ticket creation, and release certification.

use crate::error::OutfeedError;
use std::path::{Path, PathBuf};
use storypole_core::index::query::{get_task_by_id, resolve_task_id};
use storypole_core::model::decision::VerificationReceipt;
use storypole_core::model::task::{TaskKind, TaskScope};
use storypole_core::ops::mutations::{add_task, CreateTaskParams};
use storypole_core::ops::verification::verify_task;
use storypole_core::store::StorypoleStore;

/// Searches upwards from start_dir for the directory containing `.gaugeblock` or `.storypole`.
pub fn find_storypole_root(start_dir: &Path) -> Option<PathBuf> {
    let mut curr = start_dir.to_path_buf();
    loop {
        if curr.join(".gaugeblock").join("storypole").is_dir() || curr.join(".storypole").is_dir() {
            return Some(curr);
        }
        if !curr.pop() {
            return None;
        }
    }
}

/// Fetches summaries for a list of ticket keys (e.g. "GB-41").
pub fn fetch_task_summaries(
    root_dir: &Path,
    ticket_keys: &[String],
) -> Result<Vec<(String, String, String)>, OutfeedError> {
    let store = StorypoleStore::open(root_dir)
        .map_err(|e| OutfeedError::Storypole(e.to_string()))?;
    let mut summaries = Vec::new();

    for key in ticket_keys {
        if let Ok(id) = resolve_task_id(store.conn(), key)
            && let Ok(task) = get_task_by_id(store.conn(), id)
        {
            summaries.push((task.ticket_key, task.title, format!("{:?}", task.kind)));
        }
    }
    Ok(summaries)
}

/// Creates a dedicated Storypole chore ticket representing this release.
pub fn create_release_ticket(
    root_dir: &Path,
    repo: &str,
    version: &str,
    rationale: &str,
    shipped_tickets: &[String],
) -> Result<String, OutfeedError> {
    let mut store = StorypoleStore::open(root_dir)
        .map_err(|e| OutfeedError::Storypole(e.to_string()))?;

    let title = format!("chore(release): release {repo} v{version}");
    let ticket_list = if shipped_tickets.is_empty() {
        "None (maintenance/internal commits only)".to_string()
    } else {
        shipped_tickets.join(", ")
    };
    let desc = format!(
        "Automated release ticket for {repo} v{version}.\n\n\
         Rationale: {rationale}\n\
         Shipped Tasks: {ticket_list}"
    );

    let scope = TaskScope {
        repo: Some(repo.to_string()),
        crate_name: Some(format!("{repo}-core")),
        module_path: None,
        target_files: Vec::new(),
        target_symbols: Vec::new(),
        labels: vec!["release".to_string()],
    };

    let params = CreateTaskParams {
        title,
        description: desc,
        kind: TaskKind::Refactor,
        priority: 1,
        scope,
        acceptance_criteria: vec![
            "Tier 4 cross-repo verification passes".to_string(),
            format!("Manifest version bumped to {version}"),
            "Annotated git tag created".to_string(),
        ],
        status: None,
        edges: Vec::new(),
    };

    let task = add_task(&mut store, params)
        .map_err(|e| OutfeedError::Storypole(e.to_string()))?;
    Ok(task.ticket_key)
}

/// Certifies and marks a release ticket as Verified in Storypole.
pub fn certify_release_ticket(
    root_dir: &Path,
    ticket_key: &str,
    version: &str,
) -> Result<(), OutfeedError> {
    let mut store = StorypoleStore::open(root_dir)
        .map_err(|e| OutfeedError::Storypole(e.to_string()))?;

    let id = resolve_task_id(store.conn(), ticket_key)
        .map_err(|e| OutfeedError::Storypole(e.to_string()))?;

    let receipt = VerificationReceipt {
        verifier: "outfeed".to_string(),
        command: "outfeed apply".to_string(),
        exit_code: Some(0),
        diagnostic_count: Some(0),
        summary: format!("Release v{version} successfully certified and tagged"),
        verified_at_epoch_ms: StorypoleStore::now_ms(),
        git_branch: None,
        base_commit_sha: None,
        head_commit_sha: None,
        commit_shas: Vec::new(),
    };

    verify_task(&mut store, id, Some(receipt), false, false)
        .map_err(|e| OutfeedError::Storypole(e.to_string()))?;
    Ok(())
}
