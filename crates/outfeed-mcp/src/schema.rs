//! MCP Tool and resource schemas for `outfeed`.

use serde_json::{json, Value};

/// Returns the list of tools provided by the `outfeed` MCP server.
pub fn list_tools() -> Value {
    json!([
        {
            "name": "outfeed_plan",
            "description": "Evaluate unreleased commits and generate a proposed SemVer release plan and changelog",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Target repository path or name (defaults to current dir)"
                    },
                    "override_version": {
                        "type": "string",
                        "description": "Optional explicit version string to use instead of computed bump"
                    }
                }
            }
        },
        {
            "name": "outfeed_apply",
            "description": "Execute a release plan: bump manifests, update changelog, run verification, commit, tag, and update Storypole",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Target repository path or name (defaults to current dir)"
                    },
                    "override_version": {
                        "type": "string",
                        "description": "Optional explicit version string to release"
                    },
                    "verify_tier4": {
                        "type": "boolean",
                        "description": "Whether to run full Tier 4 cargo verification before committing (default true)"
                    }
                }
            }
        },
        {
            "name": "outfeed_install",
            "description": "Compile and install suite CLI binaries into global PATH (~/.cargo/bin)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tool": {
                        "type": "string",
                        "description": "Target tool name or 'all' for complete suite (default: all)"
                    },
                    "force": {
                        "type": "boolean",
                        "description": "Force rebuild and install even if versions match"
                    }
                }
            }
        },
        {
            "name": "outfeed_push",
            "description": "Push release branch and tag to Git remote",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Target repository path or name (defaults to current dir)"
                    },
                    "remote": {
                        "type": "string",
                        "description": "Git remote name (default: origin)"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Simulate push without modifying remote"
                    }
                }
            }
        },
        {
            "name": "outfeed_publish",
            "description": "Publish member crates to crates.io in topological order with --dry-run validation",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "repo": {
                        "type": "string",
                        "description": "Target repository path or name (defaults to current dir)"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Simulate packaging without uploading to registry"
                    }
                }
            }
        },
        {
            "name": "outfeed_clean",
            "description": "Clean target compilation artifacts while preserving release binaries, with dry-run size audit",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "all": {
                        "type": "boolean",
                        "description": "Full clean of entire target/ directory"
                    },
                    "doc": {
                        "type": "boolean",
                        "description": "Clean documentation artifacts (target/doc)"
                    },
                    "release": {
                        "type": "boolean",
                        "description": "Clean release artifacts (target/release)"
                    },
                    "dry_run": {
                        "type": "boolean",
                        "description": "Report disk usage and reclaimable space without deleting (default true)"
                    }
                }
            }
        }
    ])
}

