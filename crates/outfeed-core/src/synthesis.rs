//! Neurosymbolic changelog synthesis powered by `sawhorse-core`.

use crate::changelog::format_release_section;
use crate::error::OutfeedError;
use crate::model::CommitInfo;
use sawhorse_core::config::SawhorseConfig;
use sawhorse_core::router::RoleRouter;
use sawhorse_core::types::{InferenceRequest, ModelRole};

/// Synthesizes high-quality release notes using sawhorse LLM if configured,
/// or falls back cleanly to deterministic Keep-a-Changelog formatting.
pub async fn synthesize_release_notes(
    repo_name: &str,
    version: &str,
    commits: &[CommitInfo],
    task_summaries: &[(String, String, String)],
    no_llm: bool,
) -> Result<String, OutfeedError> {
    if !no_llm
        && let Ok(config) = SawhorseConfig::discover()
        && let Ok(router) = RoleRouter::from_config(config)
    {
        let fut = call_sawhorse(&router, repo_name, version, commits, task_summaries);
        match tokio::time::timeout(std::time::Duration::from_secs(30), fut).await {
            Ok(Ok(notes)) if !notes.trim().is_empty() => return Ok(notes),
            Ok(Err(err)) => {
                tracing::info!(
                    repo = %repo_name,
                    error = %err,
                    "LLM synthesis unavailable; falling back to deterministic changelog"
                );
            }
            Err(_) => {
                tracing::info!(
                    repo = %repo_name,
                    "LLM synthesis timed out (30s); falling back to deterministic changelog"
                );
            }
            _ => {}
        }
    }

    // Deterministic fallback
    Ok(format_release_section(version, commits))
}

async fn call_sawhorse(
    router: &RoleRouter,
    repo_name: &str,
    version: &str,
    commits: &[CommitInfo],
    task_summaries: &[(String, String, String)],
) -> Result<String, OutfeedError> {
    let prompt = build_synthesis_prompt(repo_name, version, commits, task_summaries);
    tracing::debug!(
        repo = %repo_name,
        version = %version,
        commits = commits.len(),
        tasks = task_summaries.len(),
        prompt_chars = prompt.len(),
        max_tokens = 4096,
        "Dispatching changelog synthesis to sawhorse router"
    );
    let mut req = InferenceRequest::new(ModelRole::Planner, prompt);
    req.temperature = 0.2;
    req.max_tokens = 4096;

    let resp = router
        .complete(&req)
        .await
        .map_err(|e| OutfeedError::Synthesis(e.to_string()))?;

    Ok(resp.text.trim().to_string())
}

fn build_synthesis_prompt(
    repo_name: &str,
    version: &str,
    commits: &[CommitInfo],
    task_summaries: &[(String, String, String)],
) -> String {
    let mut commit_text = String::new();
    for c in commits {
        let tickets = if !c.referenced_tickets.is_empty() {
            format!(" [{}]", c.referenced_tickets.join(","))
        } else {
            String::new()
        };
        commit_text.push_str(&format!("- {}: {}{tickets}\n", c.sha, c.summary));
    }

    let mut task_text = String::new();
    for (key, title, kind) in task_summaries {
        task_text.push_str(&format!("- {key} ({kind}): {title}\n"));
    }

    format!(
        "You are an expert systems release engineer generating a changelog entry for {repo_name} v{version}.\n\
         Generate Markdown strictly conforming to Keep a Changelog starting with '## [v{version}] - YYYY-MM-DD'.\n\
         Group changes into: ### Added, ### Fixed, ### Changed, ### Maintenance.\n\
         Synthesize concise, professional bullet points linking task keys (e.g. GB-XX) and crate names.\n\
         Do not include preamble or conversational text.\n\n\
         COMMITS:\n{commit_text}\n\
         STORYPOLE TASKS:\n{task_text}\n"
    )
}
