//! The trace a gate run belongs to (`GB-243`).
//!
//! A gate run is one unit of work spanning many child processes, so the trace id is minted once per
//! gate process and handed to every child as `GAUGEBLOCK_TRACE_ID`. A [`OnceLock`] rather than a value
//! threaded through every signature: the id is constant for the process, and the spawn site in
//! `gate::execute` is the only reader.
//!
//! An id already in the environment wins, so a gate invoked by an instrumented parent - or by a task
//! runner that has its own trace - extends that trace instead of starting a second, unrelated one.

use std::sync::OnceLock;

use uuid::Uuid;

/// Trace id shared with child processes.
pub const TRACE_ENV: &str = "GAUGEBLOCK_TRACE_ID";

static TRACE: OnceLock<Uuid> = OnceLock::new();

/// Returns this process's trace id, minting it on first use.
#[must_use]
pub fn trace_id() -> Uuid {
    *TRACE.get_or_init(|| {
        std::env::var(TRACE_ENV)
            .ok()
            .and_then(|raw| Uuid::parse_str(raw.trim()).ok())
            .unwrap_or_else(Uuid::new_v4)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_step_of_a_run_shares_one_trace() {
        // The point of the OnceLock: two calls from the same process must not produce two traces, or the
        // waterfall would render each step as an unrelated root.
        assert_eq!(trace_id(), trace_id());
    }

    #[test]
    fn the_trace_is_a_usable_v4_when_nothing_ambient_was_set() {
        assert_eq!(trace_id().get_version_num(), 4);
    }
}
