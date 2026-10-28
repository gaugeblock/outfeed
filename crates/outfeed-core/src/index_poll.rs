//! crates.io sparse-index lookups and propagation polling.

use std::time::{Duration, Instant};

use chrono::{TimeZone, Utc};

use crate::error::OutfeedError;
use crate::progress::ProgressReporter;

/// Base URL of the crates.io sparse index.
const SPARSE_INDEX_BASE: &str = "https://index.crates.io";

/// Interval between sparse-index polls while waiting for propagation.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Returns the sparse-index shard path for a crate name.
///
/// crates.io shards by name length: `1/<name>`, `2/<name>`, `3/<first>/<name>`, otherwise
/// `<first two>/<next two>/<name>`.
pub fn sparse_index_path(crate_name: &str) -> String {
    let lowered = crate_name.to_ascii_lowercase();
    match lowered.len() {
        0 => lowered,
        1 => format!("1/{lowered}"),
        2 => format!("2/{lowered}"),
        3 => format!("3/{}/{lowered}", &lowered[..1]),
        _ => format!("{}/{}/{}", &lowered[..2], &lowered[2..4], lowered),
    }
}

/// Returns true when the index body advertises `version`.
pub fn index_body_has_version(body: &str, version: &str) -> bool {
    let target = version.trim_start_matches('v');
    body.lines().any(|line| {
        line.contains(&format!("\"vers\":\"{target}\""))
            || line.contains(&format!("\"vers\": \"{target}\""))
    })
}

/// Returns the highest semantic version advertised by an index body.
pub fn latest_index_version(body: &str) -> Option<String> {
    let mut best: Option<(semver::Version, String)> = None;
    for line in body.lines() {
        let Some(rest) = line.split("\"vers\":\"").nth(1) else {
            continue;
        };
        let Some(raw) = rest.split('"').next() else {
            continue;
        };
        let Ok(parsed) = semver::Version::parse(raw) else {
            continue;
        };
        if best.as_ref().map(|(v, _)| &parsed > v).unwrap_or(true) {
            best = Some((parsed, raw.to_string()));
        }
    }
    best.map(|(_, raw)| raw)
}

/// Fetches the sparse-index body for a crate, returning `None` when it is unreachable.
fn fetch_index_body(crate_name: &str) -> Result<Option<String>, OutfeedError> {
    let url = format!("{SPARSE_INDEX_BASE}/{}", sparse_index_path(crate_name));
    match reqwest::blocking::get(&url) {
        Ok(response) if response.status().is_success() => {
            response.text().map(Some).map_err(|err| {
                OutfeedError::Publish(format!(
                    "failed to read sparse index for {crate_name}: {err}"
                ))
            })
        }
        Ok(response) if response.status() == reqwest::StatusCode::NOT_FOUND => Ok(None),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Reads the highest published version of a crate from the sparse index.
pub fn latest_published_version(crate_name: &str) -> Result<Option<String>, OutfeedError> {
    Ok(fetch_index_body(crate_name)?
        .as_deref()
        .and_then(latest_index_version))
}

/// Polls the sparse index until `version` appears or `timeout` elapses.
///
/// Returns `true` when propagation was observed. A timeout is not an error: crates.io propagation is
/// best-effort and the caller proceeds.
pub fn wait_for_index_version(
    crate_name: &str,
    version: &str,
    timeout: Duration,
    reporter: &dyn ProgressReporter,
) -> Result<bool, OutfeedError> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(body) = fetch_index_body(crate_name)?
            && index_body_has_version(&body, version)
        {
            reporter.on_update(crate_name, &format!("sparse index advertises {version}"));
            return Ok(true);
        }
        if Instant::now() >= deadline {
            reporter.on_update(
                crate_name,
                &format!(
                    "index propagation not observed within {}s; continuing",
                    timeout.as_secs()
                ),
            );
            return Ok(false);
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Adds `seconds` to a fixed instant, used for deterministic `Retry-After` tests.
pub fn instant_plus_seconds(seconds: i64) -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(seconds, 0)
        .single()
        .unwrap_or_else(Utc::now)
}
