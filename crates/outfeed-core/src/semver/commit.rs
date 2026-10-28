//! Conventional Commit parsing: header classification, scope, and ticket extraction.

use crate::model::{CommitInfo, ConventionalKind};

/// Parses a raw commit into structured `CommitInfo`.
pub fn parse_commit(sha: &str, summary: &str, body: Option<&str>) -> CommitInfo {
    let (kind, scope, is_breaking_summary, clean_summary) = parse_header(summary);
    let mut is_breaking = is_breaking_summary;
    let mut breaking_desc = None;

    if let Some(b) = body
        && (b.contains("BREAKING CHANGE:") || b.contains("BREAKING-CHANGE:"))
    {
        is_breaking = true;
        let desc = b
            .lines()
            .find(|l| l.contains("BREAKING CHANGE:") || l.contains("BREAKING-CHANGE:"))
            .map(|l| l.trim().to_string());
        breaking_desc = desc;
    }

    let referenced_tickets = extract_ticket_keys(summary, body);

    CommitInfo {
        sha: sha.to_string(),
        summary: clean_summary,
        body: body.map(|b| b.to_string()),
        kind,
        scope,
        is_breaking,
        breaking_description: breaking_desc,
        referenced_tickets,
    }
}

fn parse_header(header: &str) -> (ConventionalKind, Option<String>, bool, String) {
    let header = header.trim();
    let colon_idx = match header.find(':') {
        Some(idx) => idx,
        None => {
            return (
                ConventionalKind::Other("unknown".to_string()),
                None,
                false,
                header.to_string(),
            );
        }
    };

    let prefix = &header[..colon_idx].trim();
    let clean_summary = header[colon_idx + 1..].trim().to_string();

    let is_breaking = prefix.ends_with('!');
    let prefix = if is_breaking {
        &prefix[..prefix.len() - 1].trim()
    } else {
        prefix
    };

    let (type_str, scope) = if let Some(open) = prefix.find('(') {
        if let Some(close) = prefix.find(')') {
            let t = &prefix[..open];
            let s = &prefix[open + 1..close];
            (t.trim(), Some(s.trim().to_string()))
        } else {
            (*prefix, None)
        }
    } else {
        (*prefix, None)
    };

    let kind = match type_str.to_lowercase().as_str() {
        "feat" => ConventionalKind::Feat,
        "fix" => ConventionalKind::Fix,
        "refactor" => ConventionalKind::Refactor,
        "perf" => ConventionalKind::Perf,
        "docs" => ConventionalKind::Docs,
        "style" => ConventionalKind::Style,
        "chore" => ConventionalKind::Chore,
        "ci" => ConventionalKind::Ci,
        "build" => ConventionalKind::Build,
        "test" => ConventionalKind::Test,
        other => ConventionalKind::Other(other.to_string()),
    };

    (kind, scope, is_breaking, clean_summary)
}

fn extract_ticket_keys(summary: &str, body: Option<&str>) -> Vec<String> {
    let mut keys = Vec::new();
    let mut scan_text = summary.to_string();
    if let Some(b) = body {
        scan_text.push(' ');
        scan_text.push_str(b);
    }

    for word in scan_text.split_whitespace() {
        let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
        if (clean.starts_with("GB-") || clean.starts_with("SP-"))
            && !keys.contains(&clean.to_string())
        {
            keys.push(clean.to_string());
        }
    }
    keys
}
