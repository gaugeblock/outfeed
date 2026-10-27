//! Available disk space inspection and headroom pre-flight validation.

use std::fs;
use std::path::Path;
use std::process::Command;

use crate::clean::{dir_size, format_bytes};
use crate::cmd::run_captured;
use crate::error::OutfeedError;

/// Default minimum available bytes required for single repository release (2.5 GB).
pub const DEFAULT_MIN_SINGLE_REPO_DISK_BYTES: u64 = 2_500_000_000;

/// Default minimum available bytes required for full suite release with inter-repo clean (4.0 GB).
pub const DEFAULT_MIN_SUITE_DISK_BYTES: u64 = 4_000_000_000;

/// Default minimum available bytes required for suite release without inter-repo clean (20.0 GB).
pub const DEFAULT_MIN_SUITE_NO_CLEAN_DISK_BYTES: u64 = 20_000_000_000;

/// Queries the available filesystem capacity in bytes for a given path using POSIX df.
pub fn get_available_disk_space(path: &Path) -> Result<u64, OutfeedError> {
    let mut cmd = Command::new("df");
    cmd.args(["-P", "-k"]).arg(path);

    let output = match run_captured(&mut cmd) {
        Ok(out) if out.success => out.stdout,
        _ => {
            // If df is unavailable (e.g. non-Unix or restricted sandbox), return high fallback
            return Ok(u64::MAX);
        }
    };

    parse_df_available_bytes(&output).ok_or_else(|| {
        OutfeedError::Clean(format!(
            "Failed to parse available disk space from 'df' output for {}",
            path.display()
        ))
    })
}

/// Parses available KiB from POSIX 'df -P -k' output line and converts to bytes.
pub fn parse_df_available_bytes(output: &str) -> Option<u64> {
    for line in output.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        // Format: Filesystem 1024-blocks Used Available Capacity Mounted on
        if fields.len() >= 4
            && let Ok(kib) = fields[3].parse::<u64>()
        {
            return Some(kib.saturating_mul(1024));
        }
    }
    None
}

/// Validates that the filesystem containing `path` has at least `required_bytes` available.
pub fn check_disk_space_headroom(path: &Path, required_bytes: u64) -> Result<u64, OutfeedError> {
    let available = get_available_disk_space(path)?;
    if available < required_bytes {
        return Err(OutfeedError::InsufficientDiskSpace {
            available: format_bytes(available),
            required: format_bytes(required_bytes),
        });
    }
    Ok(available)
}

/// Cleans intermediate compilation and packaging artifacts between releases in a suite run.
///
/// Purges `target/debug` and `target/package` to reclaim space while preserving installed
/// binaries and release artifacts.
pub fn clean_repo_target_cache(workspace_root: &Path) -> Result<u64, OutfeedError> {
    let target = workspace_root.join("target");
    if !target.exists() {
        return Ok(0);
    }

    let mut reclaimed: u64 = 0;
    for dir_name in ["debug", "package"] {
        let path = target.join(dir_name);
        if path.exists() {
            let size = dir_size(&path);
            reclaimed = reclaimed.saturating_add(size);
            fs::remove_dir_all(&path).map_err(OutfeedError::Io)?;
        }
    }

    Ok(reclaimed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_df_available_bytes_posix() {
        let sample = "Filesystem   1024-blocks      Used Available Capacity  Mounted on\n\
                      /dev/disk1s1   244912536 212456936  11947184    95%    /System/Volumes/Data\n";
        let bytes = parse_df_available_bytes(sample);
        assert_eq!(bytes, Some(11_947_184 * 1024));
    }

    #[test]
    fn test_parse_df_available_bytes_empty() {
        assert_eq!(parse_df_available_bytes(""), None);
        assert_eq!(parse_df_available_bytes("header only"), None);
    }

    #[test]
    fn test_check_disk_space_headroom_failure() {
        let res = check_disk_space_headroom(Path::new("."), u64::MAX);
        assert!(res.is_err());
        if let Err(OutfeedError::InsufficientDiskSpace {
            available,
            required,
        }) = res
        {
            assert!(!available.is_empty());
            assert!(!required.is_empty());
        } else {
            panic!("Expected InsufficientDiskSpace error");
        }
    }
}
