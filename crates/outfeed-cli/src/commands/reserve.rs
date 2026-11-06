//! Handlers for `outfeed reserve check|scaffold|publish` (`OF-5`).

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, bail};
use outfeed_core::reserve::{
    PLACEHOLDER_VERSION, ReserveCheckReport, ScaffoldOpts, check_names, parent_exclude_hint,
    publish_placeholders, scaffold_placeholders,
};

use crate::args::reserve::{
    ReserveArgs, ReserveCheckArgs, ReserveCommand, ReservePublishArgs, ReserveScaffoldArgs,
};
use crate::progress::create_reporter;

/// Dispatches `outfeed reserve`.
pub fn handle_reserve(args: &ReserveArgs, root_dir: &Path, quiet: bool) -> Result<()> {
    match &args.command {
        ReserveCommand::Check(check) => handle_check(check),
        ReserveCommand::Scaffold(scaffold) => handle_scaffold(scaffold, root_dir, quiet),
        ReserveCommand::Publish(publish) => handle_publish(publish, root_dir, quiet),
    }
}

fn handle_check(args: &ReserveCheckArgs) -> Result<()> {
    let report = check_names(&args.names)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_check_table(&report);
    }

    // Unknown is not free: the registry did not answer, so nothing can be claimed.
    let taken = report.taken();
    let unknown = report.unknown();
    if !taken.is_empty() || !unknown.is_empty() {
        bail!(
            "{} taken, {} unknown, {} free - nothing was published",
            taken.len(),
            unknown.len(),
            report.free().len()
        );
    }
    Ok(())
}

fn print_check_table(report: &ReserveCheckReport) {
    println!("================================================================================");
    println!(" [outfeed] Namespace Reservation Check");
    println!("================================================================================");
    println!(" {:<34} {:<9} VERSION", "CRATE", "STATE");
    println!("--------------------------------------------------------------------------------");
    for entry in &report.entries {
        println!(
            " {:<34} {:<9} {}",
            entry.name,
            entry.state.label(),
            entry.published_version.as_deref().unwrap_or("-")
        );
    }
    println!("--------------------------------------------------------------------------------");
    println!(
        " {} free, {} taken, {} unknown of {} queried",
        report.free().len(),
        report.taken().len(),
        report.unknown().len(),
        report.entries.len()
    );
    println!("================================================================================");
}

fn handle_scaffold(args: &ReserveScaffoldArgs, root_dir: &Path, quiet: bool) -> Result<()> {
    let mut repository_overrides = BTreeMap::new();
    for pair in &args.repo_override {
        let Some((name, url)) = pair.split_once('=') else {
            bail!("--repo-override expects NAME=URL, got `{pair}`");
        };
        repository_overrides.insert(name.to_string(), url.to_string());
    }

    let opts = ScaffoldOpts {
        staging_dir: root_dir.join(&args.staging),
        repository_template: args.repository.clone(),
        repository_overrides,
        ..ScaffoldOpts::default()
    };
    let outcome = scaffold_placeholders(&args.names, &opts)?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "staging": opts.staging_dir.display().to_string(),
                "version": PLACEHOLDER_VERSION,
                "written": outcome.written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
                "skipped": outcome.skipped.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            })
        );
        return Ok(());
    }

    if !quiet {
        println!(
            "[outfeed] scaffolded {} placeholder(s) at {}",
            outcome.written.len(),
            opts.staging_dir.display()
        );
        for path in &outcome.written {
            println!("   + {}", path.display());
        }
        for path in &outcome.skipped {
            println!("   = {} (kept)", path.display());
        }
    }

    // Cargo would enrol the placeholders as members of this workspace without both guards.
    // The hint compares against the parent manifest, which spells the staging directory relative, so
    // it is given the relative form the operator typed rather than the joined absolute path.
    if let Some(hint) = parent_exclude_hint(Path::new(&args.staging), &root_dir.join("Cargo.toml"))
    {
        println!("[outfeed] warning: {hint}");
    }
    Ok(())
}

fn handle_publish(args: &ReservePublishArgs, root_dir: &Path, quiet: bool) -> Result<()> {
    if !args.confirm {
        bail!(
            "refusing to publish without --confirm: a published version can be yanked but never deleted, \
             so this permanently claims {} name(s)",
            args.names.len()
        );
    }

    let staging = root_dir.join(&args.staging);
    let reporter = create_reporter(quiet, args.json);
    let summary = publish_placeholders(
        &staging,
        &args.names,
        args.registry.as_deref(),
        reporter.as_ref(),
    )?;

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "published": summary.published(),
                "already_taken": summary.already_taken(),
                "failed": summary.failed(),
            })
        );
    } else if !quiet {
        println!("[outfeed] reserved {} name(s)", summary.published().len());
        for name in summary.already_taken() {
            println!("   = {name} (already held)");
        }
        for name in summary.failed() {
            println!("   ! {name} (not published)");
        }
    }

    let failed = summary.failed();
    if !failed.is_empty() {
        bail!(
            "{} name(s) remain unpublished: {}",
            failed.len(),
            failed.join(", ")
        );
    }
    Ok(())
}
