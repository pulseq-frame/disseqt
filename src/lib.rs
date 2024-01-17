//! This file contains the whole public API. It is designed in a way to be as
//! minimalistic as possible while providing all the tools necessary to plot,
//! simulate, ... MRI sequences. It does not expose the internal storage of the
//! sequence but a series of functions to sample it. This makes the usesrs of
//! this API independent of implementation details of, e.g. pulseq.

// TODO: Everything here and in the pulseq-rs crate should use f64!
// Individual samples can go down to sub-microsecond scale while sequence durations
// often are in seconds or minutes. Single-precision f32 is not sufficient in the
// general case. It probably is enough for simulation with absolute times, but
// errors accumulate too quickly when generating sequences -> use double precision!

mod backend_pulseq;
mod frontend;
mod types;
mod util;

use std::path::Path;

pub use frontend::Sequence;
pub use types::*;

pub fn load_pulseq<P: AsRef<Path>>(
    path: P,
) -> Result<Box<dyn frontend::Sequence>, pulseq_rs::Error> {
    Ok(Box::new(backend_pulseq::PulseqSequence::load(path)?))
}
