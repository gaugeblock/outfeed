//! Command handler for `outfeed release --suite` meta-harness releases.

use anyhow::{Context, Result};
use outfeed_core::{SuiteReleaseOpts, release_suite};
use std::path::Path;

use crate::args::ReleaseArgs;

pub async fn handle_suite_release(root: &Path, args: &ReleaseArgs) -> Result<()> {
    if !args.json {
        println!(
            "================================================================================"
        );
        println!(" [outfeed] Orchestrated Meta-Harness Suite Release");
        if args.dry_run {
            println!(" Mode: DRY RUN (simulating suite.lock generation, changelog update & tag)");
        }
        println!(
            "================================================================================"
        );
    }

    let suite_opts = SuiteReleaseOpts {
        target_version: args.suite_version.clone(),
        dry_run: args.dry_run,
        skip_push: args.skip_push,
    };

    let summary = release_suite(root, &suite_opts).context("Meta-harness suite release failed")?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("✨ Meta-harness suite release complete: {}", summary.tag);
        println!("  - Target version: v{}", summary.target_version);
        println!(
            "  - Pinned tools:   {} member repositories",
            summary.tool_count
        );
        println!("  - Manifest:       {}", summary.lock_path);
        println!(
            "  - Commit & Tag:   {}",
            if summary.committed {
                format!("✅ committed & tagged ({})", summary.tag)
            } else {
                "⏩ skipped (dry run)".to_string()
            }
        );
        println!(
            "  - Remote push:    {}",
            if summary.pushed {
                "✅ pushed"
            } else {
                "⏩ skipped"
            }
        );
    }

    Ok(())
}
