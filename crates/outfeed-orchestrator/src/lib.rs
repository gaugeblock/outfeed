//! Release orchestration: the layer that composes the rest of the suite (`GB-204`, `GB-285`, `GB-286`).
//!
//! This lived inside `outfeed-core`, which meant the core named storypole's and sawhorse's capabilities
//! and carried a lateral domain-to-domain edge to each. Orchestration genuinely belongs above the core:
//! planning, semver, manifests and deterministic changelog formatting stay in `outfeed-core`, and the part
//! that walks a workspace, reads a backlog and calls an inference backend lives here, in an integrator
//! crate that is allowed to depend on all of them.

pub mod backlog;
pub mod release;
pub mod suite_exec;
pub mod suite_plan;
pub mod suite_waves;
pub mod synthesis;
