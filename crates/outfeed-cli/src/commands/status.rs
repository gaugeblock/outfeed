//! Command handler for `outfeed status`.

use std::path::Path;
use anyhow::Result;
use outfeed_core::git::{get_commits_between, get_latest_tag};
use outfeed_core::manifest::get_repo_version;

use crate::args::StatusArgs;
use super::resolve_repo_dir;

pub fn handle_status(root: &Path, args: StatusArgs) -> Result<()> {
    let repos = if args.all {
        vec![
            "storypole", "sawhorse", "chalkline", "dado", "batten", "joiner", "benchwork",
            "rabbet", "miter", "outfeed",
        ]
    } else {
        vec![args.repo.as_str()]
    };

    println!("{:<14} | {:<12} | {:<14} | {:<18}", "REPOSITORY", "VERSION", "LATEST TAG", "UNRELEASED COMMITS");
    println!("--------------------------------------------------------------------------------");

    for r in repos {
        let repo_dir = resolve_repo_dir(root, r);
        if !repo_dir.is_dir() {
            continue;
        }

        let ver = get_repo_version(&repo_dir).unwrap_or_else(|_| "unknown".to_string());
        let tag = get_latest_tag(&repo_dir).unwrap_or(None).unwrap_or_else(|| "none".to_string());
        let commits = get_commits_between(&repo_dir, if tag == "none" { None } else { Some(&tag) }, "HEAD")
            .unwrap_or_default();

        let count = commits.len();
        let status_str = if count > 0 {
            format!("⚠️  {count} commit(s)")
        } else {
            "✅ clean".to_string()
        };

        println!("{:<14} | {:<12} | {:<14} | {:<18}", r, ver, tag, status_str);
    }

    Ok(())
}

