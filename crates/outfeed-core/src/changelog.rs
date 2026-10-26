//! Changelog formatting and Keep-a-Changelog integration.

use crate::error::OutfeedError;
use crate::model::{CommitInfo, ConventionalKind, ManifestDiff};
use chrono::Utc;
use std::fs;
use std::path::Path;

/// Renders a structured Markdown release section from commits.
pub fn format_release_section(version: &str, commits: &[CommitInfo]) -> String {
    let date = Utc::now().format("%Y-%m-%d");
    let mut out = format!("## [v{version}] - {date}\n\n");

    let mut added = Vec::new();
    let mut fixed = Vec::new();
    let mut changed = Vec::new();
    let mut other = Vec::new();

    for c in commits {
        let unmentioned_tickets: Vec<_> = c
            .referenced_tickets
            .iter()
            .filter(|t| !c.summary.contains(t.as_str()))
            .cloned()
            .collect();

        let ticket_suffix = if !unmentioned_tickets.is_empty() {
            format!(" ({})", unmentioned_tickets.join(", "))
        } else {
            String::new()
        };

        let scope_prefix = c
            .scope
            .as_deref()
            .map(|s| format!("`{s}`: "))
            .unwrap_or_default();

        let line = format!("- {scope_prefix}{}{ticket_suffix}", c.summary);

        match c.kind {
            ConventionalKind::Feat => added.push(line),
            ConventionalKind::Fix => fixed.push(line),
            ConventionalKind::Refactor | ConventionalKind::Perf => changed.push(line),
            _ => other.push(line),
        }
    }

    if !added.is_empty() {
        out.push_str("### Added\n");
        for line in added {
            out.push_str(&format!("{line}\n"));
        }
        out.push('\n');
    }

    if !fixed.is_empty() {
        out.push_str("### Fixed\n");
        for line in fixed {
            out.push_str(&format!("{line}\n"));
        }
        out.push('\n');
    }

    if !changed.is_empty() {
        out.push_str("### Changed\n");
        for line in changed {
            out.push_str(&format!("{line}\n"));
        }
        out.push('\n');
    }

    if !other.is_empty() {
        out.push_str("### Maintenance\n");
        for line in other {
            out.push_str(&format!("{line}\n"));
        }
        out.push('\n');
    }

    out.trim_end().to_string()
}

/// Generates a planned diff for `CHANGELOG.md`.
pub fn plan_changelog_update(
    repo_dir: &Path,
    release_markdown: &str,
) -> Result<ManifestDiff, OutfeedError> {
    let changelog_path = repo_dir.join("CHANGELOG.md");
    let original = if changelog_path.exists() {
        fs::read_to_string(&changelog_path)
            .map_err(|e| OutfeedError::Changelog(format!("Failed to read CHANGELOG.md: {e}")))?
    } else {
        initial_changelog_template()
    };

    let proposed = insert_release_section(&original, release_markdown);

    Ok(ManifestDiff {
        relative_path: "CHANGELOG.md".to_string(),
        original_content: original,
        proposed_content: proposed,
    })
}

fn initial_changelog_template() -> String {
    "# Changelog\n\n\
All notable changes to this project will be documented in this file.\n\
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),\n\
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).\n\n\
## [Unreleased]\n"
        .to_string()
}

fn insert_release_section(existing: &str, new_section: &str) -> String {
    if let Some(pos) = existing.find("## [Unreleased]") {
        let after_idx = pos + "## [Unreleased]".len();
        let rest = &existing[after_idx..];
        let leading_newlines = if rest.starts_with("\n\n") {
            "\n\n"
        } else if rest.starts_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        let after_clean = rest.trim_start_matches('\n');
        format!(
            "{}{leading_newlines}{new_section}\n\n{after_clean}",
            &existing[..after_idx]
        )
    } else if let Some(first_header) = existing.find("\n## ") {
        format!(
            "{}\n\n{new_section}\n{}",
            &existing[..first_header],
            &existing[first_header..]
        )
    } else {
        format!("{existing}\n\n{new_section}\n")
    }
}
