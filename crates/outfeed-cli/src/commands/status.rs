//! Command handler for `outfeed status`.

use anyhow::Result;
use outfeed_core::are_repo_crates_published;
use outfeed_core::git::{get_commits_between, get_latest_tag};
use outfeed_core::manifest::get_repo_version;
use std::path::Path;

use super::resolve_repo_dir;
use crate::args::StatusArgs;

pub fn handle_status(root: &Path, args: StatusArgs) -> Result<()> {
    let repos = if args.all {
        vec![
            "storypole",
            "sawhorse",
            "chalkline",
            "dado",
            "batten",
            "joiner",
            "benchwork",
            "rabbet",
            "miter",
            "outfeed",
        ]
    } else {
        vec![args.repo.as_str()]
    };

    println!(
        "{:<14} | {:<12} | {:<14} | {:<25}",
        "REPOSITORY", "VERSION", "LATEST TAG", "STATUS"
    );
    println!(
        "---------------------------------------------------------------------------------------"
    );

    for r in repos {
        let repo_dir = resolve_repo_dir(root, r);
        if !repo_dir.is_dir() {
            continue;
        }

        let ver = get_repo_version(&repo_dir).unwrap_or_else(|_| "unknown".to_string());
        let tag = get_latest_tag(&repo_dir)
            .unwrap_or(None)
            .unwrap_or_else(|| "none".to_string());
        let commits = get_commits_between(
            &repo_dir,
            if tag == "none" { None } else { Some(&tag) },
            "HEAD",
        )
        .unwrap_or_default();

        let count = commits.len();
        let status_str = if count > 0 {
            format!("⚠️  {count} unreleased commit(s)")
        } else if tag != "none" && !are_repo_crates_published(&repo_dir, &tag).unwrap_or(true) {
            "⚠️  unpublished (crates.io)".to_string()
        } else {
            "✅ clean".to_string()
        };

        println!("{:<14} | {:<12} | {:<14} | {:<25}", r, ver, tag, status_str);
    }

    Ok(())
}
