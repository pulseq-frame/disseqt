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
mod types;
mod util;

use std::path::Path;
pub use types::*;

pub fn load_pulseq<P: AsRef<Path>>(path: P) -> Result<Box<dyn Sequence>, pulseq_rs::Error> {
    Ok(Box::new(backend_pulseq::PulseqSequence::load(path)?))
}

pub trait Sequence: Send {
    /// Duration of the MRI sequence: no samples, blocks, etc. exist outside
    /// of the time range [0, duration()]
    fn duration(&self) -> f32;

    /// Returns the next time range of the next block of the given type.
    /// If `t_start` is inside of a block, this block is not returned: only
    /// blocks *starting* after `t_start` are considered.
    fn next_block(&self, t_start: f32, ty: EventType) -> Option<(f32, f32)>;

    /// Returns the next Point of Interest. The internal structure of the sequence is
    /// intentionally hidden, which might be a bit annoying but means that applications
    /// using disseqt will work with any sequence, even if file formats update or
    /// additional file formats are implemented etc.
    /// A POI is a point where the given event type changes - this is usually _in between_
    /// samples - so you want to either integrate from one POI to the next or sample
    /// exactly between two (or do multiple samples if they are too far apart).
    /// For continuously changing things (maybe we support analytical definitions in the
    /// future?) the next POI might always equal t_start, so you should not try to always
    /// handle every single POI.
    fn next_poi(&self, t_start: f32, ty: EventType) -> Option<f32>;

    /// Calculate the pulse and gradient moment for a given time range.
    /// # Panics
    /// If `t_start >= t_end`
    fn integrate(&self, t_start: f32, t_end: f32) -> (PulseMoment, GradientMoment);

    /// Returns the amplitudes and phases that are applied at time point `t`.
    fn sample(&self, t: f32) -> (PulseSample, GradientSample, AdcBlockSample);
}
