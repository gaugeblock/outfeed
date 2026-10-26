//! Manifest synchronization and version bumping using `toml_edit`.

use crate::error::OutfeedError;
use crate::model::ManifestDiff;
use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item, value};

/// Reads the current version string from a repository root `Cargo.toml`.
pub fn get_repo_version(repo_dir: &Path) -> Result<String, OutfeedError> {
    let cargo_path = repo_dir.join("Cargo.toml");
    let content = fs::read_to_string(&cargo_path)
        .map_err(|e| OutfeedError::Manifest(cargo_path.display().to_string(), e.to_string()))?;

    let doc: DocumentMut = content.parse().map_err(|e: toml_edit::TomlError| {
        OutfeedError::Manifest(cargo_path.display().to_string(), e.to_string())
    })?;

    if let Some(workspace) = doc.get("workspace").and_then(|w| w.get("package"))
        && let Some(ver) = workspace.get("version").and_then(|v| v.as_str())
    {
        return Ok(ver.to_string());
    }
    if let Some(pkg) = doc.get("package")
        && let Some(ver) = pkg.get("version").and_then(|v| v.as_str())
    {
        return Ok(ver.to_string());
    }

    Err(OutfeedError::Manifest(
        cargo_path.display().to_string(),
        "No version found in Cargo.toml".to_string(),
    ))
}

/// Computes planned manifest diffs across a repository workspace without modifying files.
pub fn plan_manifest_updates(
    repo_dir: &Path,
    target_version: &str,
) -> Result<Vec<ManifestDiff>, OutfeedError> {
    let mut diffs = Vec::new();
    let root_cargo = repo_dir.join("Cargo.toml");

    if root_cargo.exists() {
        let original = fs::read_to_string(&root_cargo)
            .map_err(|e| OutfeedError::Manifest(root_cargo.display().to_string(), e.to_string()))?;
        let mut doc: DocumentMut = original.parse().map_err(|e: toml_edit::TomlError| {
            OutfeedError::Manifest(root_cargo.display().to_string(), e.to_string())
        })?;

        if let Some(workspace) = doc.get_mut("workspace").and_then(|w| w.get_mut("package")) {
            workspace["version"] = value(target_version);
        } else if let Some(pkg) = doc.get_mut("package") {
            pkg["version"] = value(target_version);
        }

        update_member_dep_versions(&mut doc, target_version);
        let proposed = doc.to_string();

        if proposed != original {
            diffs.push(ManifestDiff {
                relative_path: "Cargo.toml".to_string(),
                original_content: original,
                proposed_content: proposed,
            });
        }
    }

    // Inspect sub-crates
    let crates_dir = repo_dir.join("crates");
    if crates_dir.is_dir() {
        for entry in fs::read_dir(&crates_dir)? {
            let entry = entry?;
            let sub_cargo = entry.path().join("Cargo.toml");
            if sub_cargo.is_file() {
                let original = fs::read_to_string(&sub_cargo).map_err(|e| {
                    OutfeedError::Manifest(sub_cargo.display().to_string(), e.to_string())
                })?;
                let mut doc: DocumentMut =
                    original.parse().map_err(|e: toml_edit::TomlError| {
                        OutfeedError::Manifest(sub_cargo.display().to_string(), e.to_string())
                    })?;

                update_member_dep_versions(&mut doc, target_version);
                let proposed = doc.to_string();

                if proposed != original {
                    let rel = sub_cargo.strip_prefix(repo_dir).unwrap_or(&sub_cargo);
                    diffs.push(ManifestDiff {
                        relative_path: rel.display().to_string(),
                        original_content: original,
                        proposed_content: proposed,
                    });
                }
            }
        }
    }

    // Web package.json if present
    let pkg_json = repo_dir.join("web").join("package.json");
    if pkg_json.is_file() {
        let original = fs::read_to_string(&pkg_json)?;
        let mut val: serde_json::Value = serde_json::from_str(&original)?;
        if let Some(obj) = val.as_object_mut() {
            obj.insert(
                "version".to_string(),
                serde_json::Value::String(target_version.to_string()),
            );
            let proposed = format!("{}\n", serde_json::to_string_pretty(&val)?);
            if proposed != original {
                diffs.push(ManifestDiff {
                    relative_path: "web/package.json".to_string(),
                    original_content: original,
                    proposed_content: proposed,
                });
            }
        }
    }

    Ok(diffs)
}

fn update_member_dep_versions(doc: &mut DocumentMut, target_ver: &str) {
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(Item::Table(table)) = doc.get_mut(section) {
            for (_dep_name, dep_item) in table.iter_mut() {
                if let Item::Value(toml_edit::Value::InlineTable(inline)) = dep_item
                    && inline.contains_key("path")
                    && inline.contains_key("version")
                {
                    inline.insert("version", target_ver.into());
                }
            }
        }
    }
}

/// Applies planned manifest diffs to disk.
pub fn apply_manifest_diffs(repo_dir: &Path, diffs: &[ManifestDiff]) -> Result<(), OutfeedError> {
    for diff in diffs {
        let target = repo_dir.join(&diff.relative_path);
        fs::write(&target, &diff.proposed_content)
            .map_err(|e| OutfeedError::Manifest(target.display().to_string(), e.to_string()))?;
    }
    Ok(())
}

/// Reverts manifest modifications using original content.
pub fn rollback_manifest_diffs(
    repo_dir: &Path,
    diffs: &[ManifestDiff],
) -> Result<(), OutfeedError> {
    for diff in diffs {
        let target = repo_dir.join(&diff.relative_path);
        if target.exists() {
            fs::write(&target, &diff.original_content)
                .map_err(|e| OutfeedError::Manifest(target.display().to_string(), e.to_string()))?;
        }
    }
    Ok(())
}
