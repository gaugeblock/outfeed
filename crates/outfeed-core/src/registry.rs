//! Crates.io registry inspection and published version verification.

use std::path::Path;
use std::process::Command;

use crate::cmd::run_captured;
use crate::error::OutfeedError;
use crate::publish::resolve_publish_order;

/// Queries the latest published version of a crate from crates.io via `cargo search`.
pub fn get_published_crate_version(crate_name: &str) -> Result<Option<String>, OutfeedError> {
    let mut cmd = Command::new("cargo");
    cmd.args(["search", crate_name, "--limit", "1"]);

    let out = match run_captured(&mut cmd) {
        Ok(o) if o.success => o.stdout,
        _ => return Ok(None),
    };

    Ok(parse_cargo_search_version(&out, crate_name))
}

/// Parses the crate version from a `cargo search` output.
///
/// Example: `rabbet = "0.1.0-beta.3"    # AST-anonymized...`
pub fn parse_cargo_search_version(output: &str, crate_name: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some((pkg, rest)) = trimmed.split_once('=')
            && pkg.trim() == crate_name
        {
            let rest = rest.trim();
            if let Some(open) = rest.find('"')
                && let Some(close) = rest[open + 1..].find('"')
            {
                return Some(rest[open + 1..open + 1 + close].to_string());
            }
        }
    }
    None
}

/// Checks whether a specific version of a crate is published on crates.io.
pub fn is_version_published(crate_name: &str, target_version: &str) -> Result<bool, OutfeedError> {
    let target = target_version.trim_start_matches('v');
    let published = get_published_crate_version(crate_name)?;
    Ok(published.as_deref() == Some(target))
}

/// Checks whether all member crates of a repository are published at the target version on crates.io.
pub fn are_repo_crates_published(
    repo_dir: &Path,
    target_version: &str,
) -> Result<bool, OutfeedError> {
    let target = target_version.trim_start_matches('v');
    let members = match resolve_publish_order(repo_dir) {
        Ok(m) => m,
        Err(_) => return Ok(true),
    };

    for member in members {
        if !is_version_published(&member.name, target)? {
            return Ok(false);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cargo_search_version_exact() {
        let sample = "rabbet-core = \"0.1.0-beta.3\"    # Core library for rabbet\n";
        assert_eq!(
            parse_cargo_search_version(sample, "rabbet-core"),
            Some("0.1.0-beta.3".to_string())
        );
    }

    #[test]
    fn test_parse_cargo_search_version_mismatch() {
        let sample = "rabbet = \"0.1.0-beta.3\"    # Telemetry hub\n";
        assert_eq!(parse_cargo_search_version(sample, "rabbet-core"), None);
    }

    #[test]
    fn test_parse_cargo_search_version_empty() {
        assert_eq!(parse_cargo_search_version("", "rabbet"), None);
    }
}
