//! Conventional Commit parser and SemVer calculation engine.

use crate::error::OutfeedError;
use semver::{Prerelease, Version};

mod commit;

pub use commit::parse_commit;

use crate::model::{BumpType, CommitInfo, ConventionalKind};

/// Evaluates commits and returns the recommended `BumpType` and rationale.
pub fn calculate_bump(
    current_ver: &str,
    commits: &[CommitInfo],
) -> Result<(BumpType, String), OutfeedError> {
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
            Ok((
                BumpType::Major,
                "Incompatible breaking change detected; bumping MAJOR version".to_string(),
            ))
        }
    } else if has_feat {
        if !v.pre.is_empty() {
            if v.pre.as_str().starts_with("alpha") {
                Ok((
                    BumpType::PreReleaseAlpha,
                    "New feature detected; incrementing alpha pre-release".to_string(),
                ))
            } else {
                Ok((
                    BumpType::PreReleasePatch,
                    "New feature detected; incrementing beta pre-release".to_string(),
                ))
            }
        } else if v.major == 0 {
            Ok((
                BumpType::Minor,
                "New feature detected in 0.x; bumping MINOR".to_string(),
            ))
        } else {
            Ok((
                BumpType::Minor,
                "Backwards-compatible feature detected; bumping MINOR".to_string(),
            ))
        }
    } else if !v.pre.is_empty() {
        if v.pre.as_str().starts_with("alpha") {
            Ok((
                BumpType::PreReleaseAlpha,
                "Fixes/refactors; incrementing alpha pre-release".to_string(),
            ))
        } else {
            Ok((
                BumpType::PreReleasePatch,
                "Fixes/refactors; incrementing beta pre-release".to_string(),
            ))
        }
    } else {
        Ok((
            BumpType::Patch,
            "Backwards-compatible fixes; bumping PATCH".to_string(),
        ))
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
        let num: u64 = parts[1]
            .parse()
            .map_err(|_| OutfeedError::SemVer(format!("Invalid pre number: {}", parts[1])))?;
        Ok(format!("{prefix}.{}", num + 1))
    } else {
        Ok(format!("{default_prefix}.1"))
    }
}
