//! Crates.io rate-limit detection, `Retry-After` parsing and cooldown retries.

use std::thread::sleep;
use std::time::Duration;

use chrono::DateTime;

use crate::error::OutfeedError;
use crate::progress::ProgressReporter;

/// Default cooldown applied when a 429 carries no usable `Retry-After` hint.
pub const DEFAULT_COOLDOWN: Duration = Duration::from_secs(30);

/// Maximum publication attempts per crate once rate limiting is involved.
pub const MAX_ATTEMPTS: u32 = 3;

/// Interval between countdown progress updates during a cooldown.
const COUNTDOWN_STEP: Duration = Duration::from_secs(30);

/// Returns true when captured cargo/crates.io output indicates rate limiting.
pub fn is_rate_limited(output: &str) -> bool {
    let lowered = output.to_ascii_lowercase();
    lowered.contains("429")
        || lowered.contains("too many requests")
        || lowered.contains("rate limit")
}

/// Parses an HTTP `Retry-After` value.
///
/// Accepts integer seconds (`120`), the RFC 2822 form crates.io emits
/// (`Wed, 21 Oct 2026 07:28:00 GMT`) and `now`-relative dates that are already in the past
/// (treated as zero).
pub fn parse_retry_after(value: &str, now: DateTime<chrono::Utc>) -> Option<Duration> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    // RFC 2822 / RFC 1123 date form.
    if let Ok(deadline) = DateTime::parse_from_rfc2822(trimmed) {
        let delta = deadline.signed_duration_since(now);
        return Some(delta.to_std().unwrap_or(Duration::ZERO));
    }
    None
}

/// Resolves a cooldown from captured output, falling back to [`DEFAULT_COOLDOWN`].
///
/// Recognises `Retry-After: <value>`, `retry after <n> seconds` and `try again in <n>s`.
pub fn parse_cooldown(output: &str, now: DateTime<chrono::Utc>) -> Duration {
    let lowered = output.to_ascii_lowercase();

    // 1. Explicit `Retry-After` token/header: take the first whitespace-delimited value, then fall
    //    back to a full date that may itself contain spaces.
    if let Some(idx) = lowered.find("retry-after") {
        let rest = output[idx + "retry-after".len()..].trim_start_matches([':', ' ', '\t']);
        let token: String = rest
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '\r' && *c != '\n')
            .collect();
        if let Some(cooldown) = parse_retry_after(&token, now) {
            return cooldown;
        }
        if let Some(cooldown) = parse_date_in_text(rest, now) {
            return cooldown;
        }
    }

    // 2. Prose deadlines. crates.io reports new-crate limits only in the body, e.g.
    //    "Please try again after Wed, 28 Oct 2026 15:30:25 GMT and see …/rate-limits". The date must
    //    be parsed *before* any bare integer, because a day-of-month reads as a small number of
    //    seconds otherwise.
    if let Some(idx) = lowered.find("try again after")
        && let Some(cooldown) = parse_date_in_text(&output[idx..], now)
    {
        return cooldown;
    }

    for marker in ["retry after", "try again in"] {
        if let Some(idx) = lowered.find(marker) {
            let rest = &output[idx + marker.len()..];
            if let Some(seconds) = first_integer(rest) {
                return Duration::from_secs(seconds);
            }
            if let Some(cooldown) = parse_date_in_text(rest, now) {
                return cooldown;
            }
        }
    }

    // 3. Last resort: any RFC 2822 timestamp anywhere in the output.
    if let Some(cooldown) = parse_date_in_text(output, now) {
        return cooldown;
    }

    DEFAULT_COOLDOWN
}

/// Finds an RFC 2822 timestamp inside free-form text and converts it into a cooldown.
///
/// RFC 2822 dates occupy five whitespace-separated tokens (`Wed, 28 Oct 2026 15:30:25 GMT`), so the
/// scan slides a five-token window across the text and keeps the first parseable date.
fn parse_date_in_text(text: &str, now: DateTime<chrono::Utc>) -> Option<Duration> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for window in tokens.windows(5) {
        let candidate = window.join(" ");
        if let Some(cooldown) = parse_retry_after(&candidate, now) {
            return Some(cooldown);
        }
    }
    None
}

/// Extracts the first run of ASCII digits from `text`.
fn first_integer(text: &str) -> Option<u64> {
    let digits: String = text
        .trim_start_matches(|c: char| !c.is_ascii_digit())
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Runs `attempt` up to [`MAX_ATTEMPTS`] times, sleeping out rate-limit cooldowns in between.
///
/// `classify` receives each captured output and decides whether the failure was rate limiting; the
/// closure is responsible for the actual command invocation. Countdown updates are reported through
/// `reporter` so the operator sees why the release is waiting.
pub fn run_with_rate_limit_retry<F>(
    label: &str,
    reporter: &dyn ProgressReporter,
    mut attempt: F,
) -> Result<crate::cmd::CommandOutput, OutfeedError>
where
    F: FnMut() -> Result<crate::cmd::CommandOutput, OutfeedError>,
{
    let mut attempt_number = 1;
    loop {
        let output = attempt()?;
        if output.success || !is_rate_limited(&output.combined_output()) {
            return Ok(output);
        }
        if attempt_number >= MAX_ATTEMPTS {
            return Ok(output);
        }

        let cooldown = parse_cooldown(&output.combined_output(), chrono::Utc::now());
        report_countdown(label, reporter, cooldown);
        attempt_number += 1;
        reporter.on_update(
            label,
            &format!("retrying after rate limit (attempt {attempt_number}/{MAX_ATTEMPTS})"),
        );
    }
}

/// Sleeps `cooldown`, emitting periodic progress updates while waiting.
fn report_countdown(label: &str, reporter: &dyn ProgressReporter, cooldown: Duration) {
    let mut remaining = cooldown;
    while remaining > Duration::ZERO {
        reporter.on_update(
            label,
            &format!("crates.io rate limit: retrying in {}s", remaining.as_secs()),
        );
        let step = remaining.min(COUNTDOWN_STEP);
        sleep(step);
        remaining = remaining.saturating_sub(step);
    }
}
