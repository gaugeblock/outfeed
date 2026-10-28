//! Release plan models, re-exported from the shared contracts crate.
//!
//! The definitions moved to `gaugeblock-contracts` (GB-143); this module keeps the historical
//! `outfeed_core::model::plans` path working so nothing downstream has to change.

pub use gaugeblock_contracts::{BumpType, ManifestDiff, ReleasePlan};
