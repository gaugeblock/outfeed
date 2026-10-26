//! MCP Tool invocation dispatcher for `outfeed`.

use outfeed_core::engine::{apply_release, plan_release};
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
        _ => Err(format!("Unknown outfeed MCP tool: {name}")),
    }
}

