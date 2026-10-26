//! MCP Tool invocation dispatcher for `outfeed`.

use outfeed_core::engine::{apply_release, plan_release};
use outfeed_core::install::{install_all, install_tool, SUITE_TOOLS};
use outfeed_core::model::{InstallOpts, PublishOpts, PushOpts};
use outfeed_core::publish::publish_repo;
use outfeed_core::push::push_repo;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn resolve_repo_dir(root_dir: &Path, raw_repo: Option<&str>) -> PathBuf {
    match raw_repo {
        Some(r) => {
            let direct = PathBuf::from(r);
            if direct.is_dir() {
                direct
            } else {
                root_dir.join(r)
            }
        }
        None => root_dir.to_path_buf(),
    }
}

/// Dispatches an MCP tool call to the corresponding outfeed engine function.
pub async fn dispatch_tool(
    name: &str,
    args: &Value,
    root_dir: &Path,
) -> Result<Value, String> {
    let repo_str = args.get("repo").and_then(|v| v.as_str());
    let repo_dir = resolve_repo_dir(root_dir, repo_str);
    let override_ver = args.get("override_version").and_then(|v| v.as_str());

    match name {
        "outfeed_plan" => {
            let plan = plan_release(&repo_dir, override_ver, false)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(&plan).map_err(|e| e.to_string())
        }
        "outfeed_apply" => {
            let verify_tier4 = args
                .get("verify_tier4")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            let plan = plan_release(&repo_dir, override_ver, false)
                .await
                .map_err(|e| e.to_string())?;

            apply_release(&repo_dir, &plan, verify_tier4, false)
                .map_err(|e| e.to_string())?;

            Ok(json!({
                "success": true,
                "repo": plan.repo_name,
                "version": plan.target_version,
                "tag": format!("v{}", plan.target_version),
                "summary": format!("Successfully released v{} for {}", plan.target_version, plan.repo_name)
            }))
        }
        "outfeed_install" => {
            let tool_name = args.get("tool").and_then(|v| v.as_str()).unwrap_or("all");
            let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
            let opts = InstallOpts { debug: false, force };

            if tool_name == "all" {
                let results = install_all(root_dir, &opts).map_err(|e| e.to_string())?;
                Ok(json!({
                    "success": true,
                    "installed_tools": results.len(),
                    "details": results.iter().map(|(n, o)| json!({ "name": n, "outcome": format!("{o:?}") })).collect::<Vec<_>>()
                }))
            } else {
                let tool = SUITE_TOOLS
                    .iter()
                    .find(|t| t.name == tool_name || t.bin_name == tool_name)
                    .ok_or_else(|| format!("Unknown tool: {tool_name}"))?;
                let outcome = install_tool(root_dir, tool, &opts).map_err(|e| e.to_string())?;
                Ok(json!({
                    "success": true,
                    "tool": tool.bin_name,
                    "outcome": format!("{outcome:?}")
                }))
            }
        }
        "outfeed_push" => {
            let remote = args.get("remote").and_then(|v| v.as_str()).unwrap_or("origin");
            let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);
            let opts = PushOpts { remote: remote.to_string(), dry_run };
            let outcome = push_repo(&repo_dir, &opts).map_err(|e| e.to_string())?;
            Ok(json!({
                "success": true,
                "remote": outcome.remote,
                "branch": outcome.branch,
                "tag": outcome.tag,
                "dry_run": outcome.dry_run,
            }))
        }
        "outfeed_publish" => {
            let dry_run = args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false);
            let opts = PublishOpts { registry: None, dry_run, delay_secs: 15 };
            let published = publish_repo(&repo_dir, &opts).map_err(|e| e.to_string())?;
            Ok(json!({
                "success": true,
                "dry_run": dry_run,
                "published_crates": published,
            }))
        }
        _ => Err(format!("Unknown outfeed MCP tool: {name}")),
    }
}

