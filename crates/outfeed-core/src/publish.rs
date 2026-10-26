//! Crates.io publication engine with topological sorting and index propagation delay.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use crate::error::OutfeedError;
use crate::model::PublishOpts;

/// Workspace member metadata for publication ordering.
#[derive(Debug, Clone)]
pub struct MemberCrate {
    /// Crate package name.
    pub name: String,
    /// Relative directory path within repository.
    pub rel_path: String,
    /// Intra-workspace package names this crate depends upon.
    pub internal_deps: Vec<String>,
}

/// Discovers member crates in a repository and topologically sorts them.
pub fn resolve_publish_order(repo_dir: &Path) -> Result<Vec<MemberCrate>, OutfeedError> {
    let manifest_path = repo_dir.join("Cargo.toml");
    let content = fs::read_to_string(&manifest_path).map_err(OutfeedError::Io)?;
    let doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e: toml_edit::TomlError| OutfeedError::Manifest(manifest_path.display().to_string(), e.to_string()))?;

    let mut members = Vec::new();
    let mut names = HashSet::new();

    // Check root package
    if let Some(pkg) = doc.get("package") && let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
        members.push((name.to_string(), ".".to_string(), manifest_path.clone()));
        names.insert(name.to_string());
    }

    // Check crates/*
    let crates_dir = repo_dir.join("crates");
    if crates_dir.is_dir() && let Ok(entries) = fs::read_dir(&crates_dir) {
        for entry in entries.flatten() {
            let sub_manifest = entry.path().join("Cargo.toml");
            if sub_manifest.is_file()
                && let Ok(sub_txt) = fs::read_to_string(&sub_manifest)
                && let Ok(sub_doc) = sub_txt.parse::<toml_edit::DocumentMut>()
                && let Some(pkg) = sub_doc.get("package")
                && let Some(name) = pkg.get("name").and_then(|n| n.as_str())
            {
                let rel = format!("crates/{}", entry.file_name().to_string_lossy());
                members.push((name.to_string(), rel, sub_manifest));
                names.insert(name.to_string());
            }
        }
    }

    // Resolve internal dependencies
    let mut crate_map = HashMap::new();
    for (name, rel_path, path) in members {
        let sub_txt = fs::read_to_string(&path).map_err(OutfeedError::Io)?;
        let sub_doc = sub_txt
            .parse::<toml_edit::DocumentMut>()
            .map_err(|e: toml_edit::TomlError| OutfeedError::Manifest(path.display().to_string(), e.to_string()))?;

        let mut internal_deps = Vec::new();
        if let Some(deps) = sub_doc.get("dependencies").and_then(|d| d.as_table()) {
            for (dep_name, _) in deps.iter() {
                if names.contains(dep_name) && dep_name != name {
                    internal_deps.push(dep_name.to_string());
                }
            }
        }
        crate_map.insert(name.clone(), MemberCrate { name, rel_path, internal_deps });
    }

    // Topologically sort: crates with no dependencies first
    let mut ordered = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    fn dfs(
        name: &str,
        crate_map: &HashMap<String, MemberCrate>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        ordered: &mut Vec<MemberCrate>,
    ) -> Result<(), OutfeedError> {
        if visiting.contains(name) {
            return Err(OutfeedError::Publish(format!("Cycle detected in workspace dependencies: {name}")));
        }
        if !visited.contains(name) {
            visiting.insert(name.to_string());
            if let Some(member) = crate_map.get(name) {
                for dep in &member.internal_deps {
                    dfs(dep, crate_map, visited, visiting, ordered)?;
                }
                visited.insert(name.to_string());
                visiting.remove(name);
                ordered.push(member.clone());
            }
        }
        Ok(())
    }

    for name in crate_map.keys() {
        dfs(name, &crate_map, &mut visited, &mut visiting, &mut ordered)?;
    }

    Ok(ordered)
}

/// Publishes all crates within a repository workspace in topological order.
pub fn publish_repo(repo_dir: &Path, opts: &PublishOpts) -> Result<Vec<String>, OutfeedError> {
    let order = resolve_publish_order(repo_dir)?;
    let mut published = Vec::new();

    // 1. Dry run gate across all crates
    for member in &order {
        let mut cmd = Command::new("cargo");
        cmd.arg("publish").arg("--dry-run").arg("-p").arg(&member.name).current_dir(repo_dir);
        if let Some(ref reg) = opts.registry {
            cmd.arg("--registry").arg(reg);
        }
        let status = cmd.status().map_err(|e| OutfeedError::Publish(format!("Dry-run publish failed: {e}")))?;
        if !status.success() {
            return Err(OutfeedError::Publish(format!("cargo publish --dry-run failed for {}", member.name)));
        }
    }

    if opts.dry_run {
        return Ok(order.into_iter().map(|m| m.name).collect());
    }

    // 2. Real publication with index delay
    let total = order.len();
    for (idx, member) in order.into_iter().enumerate() {
        let mut cmd = Command::new("cargo");
        cmd.arg("publish").arg("-p").arg(&member.name).current_dir(repo_dir);
        if let Some(ref reg) = opts.registry {
            cmd.arg("--registry").arg(reg);
        }
        let output = cmd.output().map_err(|e| OutfeedError::Publish(format!("Publish failed: {e}")))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("is already uploaded") || stderr.contains("already uploaded") {
                eprintln!("⏩ {} is already published on registry, skipping...", member.name);
                published.push(member.name);
                continue;
            }
            eprintln!("{}", stderr);
            return Err(OutfeedError::Publish(format!("cargo publish failed for {}", member.name)));
        }
        published.push(member.name);

        // Delay between dependent crates if not the last crate
        if idx + 1 < total && opts.delay_secs > 0 {
            eprintln!("⏳ Waiting {}s for crates.io index propagation...", opts.delay_secs);
            sleep(Duration::from_secs(opts.delay_secs));
        }
    }

    Ok(published)
}
