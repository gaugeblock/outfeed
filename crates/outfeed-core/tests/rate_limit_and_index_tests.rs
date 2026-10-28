//! Regression tests for crates.io rate limiting and sparse-index helpers (GB-125).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::atomic::{AtomicU32, Ordering};

use outfeed_core::cmd::CommandOutput;
use outfeed_core::index_poll::{
    index_body_has_version, latest_index_version, sparse_index_path, wait_for_index_version,
};
use outfeed_core::progress::NoopProgress;
use outfeed_core::rate_limit::{
    DEFAULT_COOLDOWN, MAX_ATTEMPTS, is_rate_limited, parse_cooldown, parse_retry_after,
    run_with_rate_limit_retry,
};
use std::time::Duration;

fn failed(text: &str) -> CommandOutput {
    CommandOutput {
        success: false,
        code: Some(1),
        stdout: String::new(),
        stderr: text.to_string(),
    }
}

fn ok() -> CommandOutput {
    CommandOutput {
        success: true,
        code: Some(0),
        stdout: "published".to_string(),
        stderr: String::new(),
    }
}

#[test]
fn test_rate_limit_detection() {
    assert!(is_rate_limited(
        "error: failed to publish\nHTTP 429 Too Many Requests"
    ));
    assert!(is_rate_limited("crates.io returned too many requests"));
    assert!(is_rate_limited("rate limit exceeded, slow down"));
    assert!(!is_rate_limited("error: package already uploaded"));
    assert!(!is_rate_limited("error[E0432]: unresolved import"));
}

#[test]
fn test_parse_retry_after_integer_seconds() {
    let now = chrono::Utc::now();
    assert_eq!(
        parse_retry_after("120", now),
        Some(Duration::from_secs(120))
    );
    assert_eq!(parse_retry_after(" 0 ", now), Some(Duration::ZERO));
    assert_eq!(parse_retry_after("", now), None);
    assert_eq!(parse_retry_after("soon", now), None);
}

#[test]
fn test_parse_retry_after_rfc2822_date() {
    let now = chrono::DateTime::parse_from_rfc2822("Wed, 21 Oct 2026 07:00:00 GMT")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let parsed = parse_retry_after("Wed, 21 Oct 2026 07:05:00 GMT", now).expect("parsed");
    assert_eq!(parsed, Duration::from_secs(300));

    // A deadline in the past collapses to zero rather than underflowing.
    let past = parse_retry_after("Wed, 21 Oct 2026 06:55:00 GMT", now).expect("parsed");
    assert_eq!(past, Duration::ZERO);
}

#[test]
fn test_parse_cooldown_variants_and_default() {
    let now = chrono::Utc::now();
    assert_eq!(
        parse_cooldown("server said Retry-After: 45", now),
        Duration::from_secs(45)
    );
    assert_eq!(
        parse_cooldown("please retry after 12 seconds", now),
        Duration::from_secs(12)
    );
    assert_eq!(
        parse_cooldown("try again in 7s", now),
        Duration::from_secs(7)
    );
    assert_eq!(
        parse_cooldown("429 without guidance", now),
        DEFAULT_COOLDOWN
    );
}

#[test]
fn test_sparse_index_shard_paths() {
    assert_eq!(sparse_index_path("a"), "1/a");
    assert_eq!(sparse_index_path("ab"), "2/ab");
    assert_eq!(sparse_index_path("abc"), "3/a/abc");
    assert_eq!(
        sparse_index_path("Gaugeblock-Core"),
        "ga/ug/gaugeblock-core"
    );
}

#[test]
fn test_index_body_version_detection_and_latest() {
    let body = concat!(
        "{\"name\":\"outfeed\",\"vers\":\"0.1.0-beta.6\",\"deps\":[]}\n",
        "{\"name\":\"outfeed\",\"vers\":\"0.1.0-beta.7\",\"deps\":[]}\n",
    );
    assert!(index_body_has_version(body, "0.1.0-beta.7"));
    assert!(index_body_has_version(body, "v0.1.0-beta.6"));
    assert!(!index_body_has_version(body, "0.1.0-beta.8"));
    assert_eq!(latest_index_version(body).as_deref(), Some("0.1.0-beta.7"));
    assert_eq!(latest_index_version("not json"), None);
}

#[test]
fn test_retry_wrapper_retries_then_succeeds() {
    let calls = AtomicU32::new(0);
    let out = run_with_rate_limit_retry("outfeed-core", &NoopProgress, || {
        let attempt = calls.fetch_add(1, Ordering::SeqCst);
        if attempt == 0 {
            Ok(failed("HTTP 429 Too Many Requests\nRetry-After: 0"))
        } else {
            Ok(ok())
        }
    })
    .expect("retry");

    assert!(out.success, "second attempt succeeds");
    assert_eq!(calls.load(Ordering::SeqCst), 2, "exactly one retry");
}

#[test]
fn test_retry_wrapper_gives_up_after_max_attempts() {
    let calls = AtomicU32::new(0);
    let out = run_with_rate_limit_retry("outfeed-core", &NoopProgress, || {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(failed("HTTP 429 Too Many Requests\nRetry-After: 0"))
    })
    .expect("retry");

    assert!(
        !out.success,
        "rate-limited output is surfaced to the caller"
    );
    assert_eq!(calls.load(Ordering::SeqCst), MAX_ATTEMPTS);
}

#[test]
fn test_retry_wrapper_does_not_retry_ordinary_failures() {
    let calls = AtomicU32::new(0);
    let out = run_with_rate_limit_retry("outfeed-core", &NoopProgress, || {
        calls.fetch_add(1, Ordering::SeqCst);
        Ok(failed("error: package already uploaded"))
    })
    .expect("retry");

    assert!(!out.success);
    assert_eq!(calls.load(Ordering::SeqCst), 1, "no retry for plain errors");
}

#[test]
fn test_wait_for_index_version_times_out_without_network() {
    // An unregistered crate name can never appear in the index; the helper must return promptly.
    let observed = wait_for_index_version(
        "gaugeblock-never-published-9f3a",
        "0.0.1",
        Duration::from_millis(600),
        &NoopProgress,
    )
    .expect("poll");
    assert!(
        !observed,
        "propagation is not observable and must not block"
    );
}

#[test]
fn test_parse_cooldown_reads_crates_io_prose_deadline() {
    // Verbatim shape observed while reserving namespaces in GB-127: the deadline appears only in
    // the message body, with no `Retry-After` header.
    let observed = "error: failed to publish gaugeblock-mcp v0.1.0-alpha.1 to registry at \
        https://crates.io\n\nCaused by:\n  the remote server responded with an error (status 429 Too \
        Many Requests): You have published too many new crates in a short period of time. Please try \
        again after Wed, 28 Oct 2026 15:30:25 GMT and see \
        https://crates.io/docs/rate-limits for more details.";

    let now = chrono::DateTime::parse_from_rfc2822("Wed, 28 Oct 2026 15:28:25 GMT")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let cooldown = parse_cooldown(observed, now);
    assert_eq!(
        cooldown,
        Duration::from_secs(120),
        "the prose deadline must override the 30s default"
    );
    assert!(cooldown > DEFAULT_COOLDOWN);
}

#[test]
fn test_parse_cooldown_prefers_explicit_seconds_over_prose() {
    let now = chrono::DateTime::parse_from_rfc2822("Wed, 28 Oct 2026 15:28:25 GMT")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let text =
        "HTTP 429 Retry-After: 7 (server said try again after Wed, 28 Oct 2026 15:30:25 GMT)";
    assert_eq!(parse_cooldown(text, now), Duration::from_secs(7));
}
