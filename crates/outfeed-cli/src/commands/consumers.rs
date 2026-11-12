//! Command handler for `outfeed consumers` (`OF-43`).

use std::path::Path;

use anyhow::Result;
use outfeed_core::consumer_build::{ConsumerReport, check_consumers_with};

use crate::args::LockfileArgs;

/// Compiles every suite member and reports the ones that no longer build.
///
/// This is the cross-repository check the suite did not have: `SP-41` changed a shared contract, every
/// repository that owned the change compiled, and two consumers in other repositories did not.
pub fn handle_consumers(args: &LockfileArgs, workspace_root: &Path) -> Result<()> {
    let root = crate::commands::resolve_repo_dir(workspace_root, &args.root);
    let members = outfeed_core::consumer_build::suite_members(&root);
    if members.is_empty() {
        anyhow::bail!(
            "outfeed: {} declares no suite members, so there is nothing to compile",
            root.display()
        );
    }

    let report = check_consumers_with(&root, &members, |build| {
        // stderr, so `--json` on stdout stays machine-readable.
        match &build.diagnostic {
            None => eprintln!("  {:>12} ok", build.member),
            Some(diagnostic) => eprintln!("  {:>12} FAILED: {diagnostic}", build.member),
        }
    });
    if args.json {
        println!("{}", serde_json::to_string_pretty(&json(&report))?);
    } else {
        print!("{}", format_report(&report));
    }
    if report.is_ok() {
        return Ok(());
    }
    anyhow::bail!(
        "{} member(s) do not build against the workspace as checked out",
        report.failures().len()
    )
}

/// The report as JSON, one object per member.
fn json(report: &ConsumerReport) -> serde_json::Value {
    let builds: Vec<serde_json::Value> = report
        .builds
        .iter()
        .map(|build| {
            serde_json::json!({
                "member": build.member,
                "ok": build.diagnostic.is_none(),
                "diagnostic": build.diagnostic,
            })
        })
        .collect();
    serde_json::json!({ "builds": builds })
}

/// The report as a table, failures first because they are what a reader is looking for.
fn format_report(report: &ConsumerReport) -> String {
    let mut out = String::new();
    if report.is_ok() {
        out.push_str(&format!(
            "CONSUMERS PASSED: all {} member(s) build against the workspace as checked out\n",
            report.builds.len()
        ));
        return out;
    }
    out.push_str("Members that do not build:\n");
    for build in report.failures() {
        out.push_str(&format!("  {:<14} ", build.member));
        out.push_str(build.diagnostic.as_deref().unwrap_or(""));
        out.push('\n');
    }
    out
}
