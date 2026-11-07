#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Delivery-path tests: publish ordering, push, branch enforcement and release orchestration.
//!
//! Split out of a single 260-line file that tripped the hard 250-line ceiling (GB-193).

mod git_and_publish;
