//! Progress reporting factory and terminal management.

pub mod multi;
pub mod stream;

use std::io::IsTerminal;
use std::sync::Arc;
use outfeed_core::{NoopProgress, ProgressReporter};
pub use multi::MultiProgressReporter;
pub use stream::StreamReporter;

/// Constructs the appropriate progress reporter based on TTY presence and CLI flags.
pub fn create_reporter(quiet: bool, json: bool) -> Arc<dyn ProgressReporter> {
    if quiet || json {
        Arc::new(NoopProgress)
    } else if std::io::stderr().is_terminal() {
        Arc::new(MultiProgressReporter::new())
    } else {
        Arc::new(StreamReporter)
    }
}

