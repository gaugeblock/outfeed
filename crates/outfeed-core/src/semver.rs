//! Conventional Commit parser and SemVer calculation engine.

use crate::error::OutfeedError;
use crate::model::{BumpType, CommitInfo, ConventionalKind};
use semver::{Prerelease, Version};

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
        None => return (ConventionalKind::Other("unknown".to_string()), None, false, header.to_string()),
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

/// Evaluates commits and returns the recommended `BumpType` and rationale.
pub fn calculate_bump(current_ver: &str, commits: &[CommitInfo]) -> Result<(BumpType, String), OutfeedError> {
    let v = Version::parse(current_ver).map_err(|e| OutfeedError::SemVer(e.to_string()))?;
    let has_breaking = commits.iter().any(|c| c.is_breaking);
    let has_feat = commits.iter().any(|c| c.kind == ConventionalKind::Feat);

    if has_breaking {
        if v.major == 0 {
            Ok((
                BumpType::PreReleaseMinor,
                "Breaking change detected during pre-release; bumping minor base (e.g. 0.1.0 -> 0.2.0-beta.1)".to_string(),
            ))
        } else {
            Ok((BumpType::Major, "Incompatible breaking change detected; bumping MAJOR version".to_string()))
        }
    } else if has_feat {
        if !v.pre.is_empty() {
            if v.pre.as_str().starts_with("alpha") {
                Ok((BumpType::PreReleaseAlpha, "New feature detected; incrementing alpha pre-release".to_string()))
            } else {
                Ok((BumpType::PreReleasePatch, "New feature detected; incrementing beta pre-release".to_string()))
            }
        } else if v.major == 0 {
            Ok((BumpType::Minor, "New feature detected in 0.x; bumping MINOR".to_string()))
        } else {
            Ok((BumpType::Minor, "Backwards-compatible feature detected; bumping MINOR".to_string()))
        }
    } else if !v.pre.is_empty() {
        if v.pre.as_str().starts_with("alpha") {
            Ok((BumpType::PreReleaseAlpha, "Fixes/refactors; incrementing alpha pre-release".to_string()))
        } else {
            Ok((BumpType::PreReleasePatch, "Fixes/refactors; incrementing beta pre-release".to_string()))
        }
    } else {
        Ok((BumpType::Patch, "Backwards-compatible fixes; bumping PATCH".to_string()))
    }
}

/// Computes the next version string based on current version and `BumpType`.
pub fn apply_bump(current_ver: &str, bump: BumpType) -> Result<String, OutfeedError> {
    let mut v = Version::parse(current_ver).map_err(|e| OutfeedError::SemVer(e.to_string()))?;

    match bump {
        BumpType::Major => {
            v.major += 1;
            v.minor = 0;
            v.patch = 0;
            v.pre = Prerelease::EMPTY;
        }
        BumpType::Minor => {
            v.minor += 1;
            v.patch = 0;
            v.pre = Prerelease::EMPTY;
        }
        BumpType::Patch => {
            v.patch += 1;
            v.pre = Prerelease::EMPTY;
        }
        BumpType::PreReleasePatch => {
            let pre_str = v.pre.as_str();
            let next_pre = increment_pre_identifier(pre_str, "beta")?;
            v.pre = Prerelease::new(&next_pre).map_err(|e| OutfeedError::SemVer(e.to_string()))?;
        }
        BumpType::PreReleaseAlpha => {
            let pre_str = v.pre.as_str();
            let next_pre = increment_pre_identifier(pre_str, "alpha")?;
            v.pre = Prerelease::new(&next_pre).map_err(|e| OutfeedError::SemVer(e.to_string()))?;
        }
        BumpType::PreReleaseMinor => {
            v.minor += 1;
            v.patch = 0;
            v.pre = Prerelease::new("beta.1").map_err(|e| OutfeedError::SemVer(e.to_string()))?;
        }
    }

    Ok(v.to_string())
}

fn increment_pre_identifier(pre: &str, default_prefix: &str) -> Result<String, OutfeedError> {
    if pre.is_empty() {
        return Ok(format!("{default_prefix}.1"));
    }
    let parts: Vec<&str> = pre.split('.').collect();
    if parts.len() == 2 {
        let prefix = parts[0];
        let num: u64 = parts[1].parse().map_err(|_| OutfeedError::SemVer(format!("Invalid pre number: {}", parts[1])))?;
        Ok(format!("{prefix}.{}", num + 1))
    } else {
        Ok(format!("{default_prefix}.1"))
    }
}
