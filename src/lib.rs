// TODO: Everything here and in the pulseq-rs crate should use f64!
// Individual samples can go down to sub-microsecond scale while sequence durations
// often are in seconds or minutes. Single-precision f64 is not sufficient in the
// general case. It probably is enough for simulation with absolute times, but
// errors accumulate too quickly when generating sequences -> use double precision!

mod backend_dsv;
mod backend_pulseq;
mod sequence;
mod types;
mod util;

pub use sequence::Sequence;
use std::path::Path;
pub use types::*;

pub fn load_pulseq<P: AsRef<Path>>(path: P) -> Result<Sequence, pulseq_rs::Error> {
    Ok(Sequence(Box::new(backend_pulseq::PulseqSequence::load(
        path,
    )?)))
}

pub fn load_dsv<P: AsRef<Path>>(path: P) -> Result<Sequence, backend_dsv::Error> {
    Ok(Sequence(Box::new(backend_dsv::DsvSequence::load(path)?)))
}

pub fn load_pulseq_str(source: &str) -> Result<Sequence, pulseq_rs::Error> {
    Ok(Sequence(Box::new(
        backend_pulseq::PulseqSequence::load_str(source)?,
    )))
}

/// This trait is implemented by all backends and provides the basic functions
/// on which the public disseqt API is built upon
trait Backend: Send {
    /// Return the FOV of the Sequence, if it is available
    fn fov(&self) -> Option<(f64, f64, f64)>;

    /// Duration of the MRI sequence: no samples, blocks, etc. exist outside
    /// of the time range [0, duration()]
    fn duration(&self) -> f64;

    /// Returns all events of the given type in the given duration.
    /// t_start is inclusive, t_end is exclusive. If a max_count is given and
    /// reached, there might be more events in the time span that are not returned.
    fn events(&self, ty: EventType, t_start: f64, t_end: f64, max_count: usize) -> Vec<f64>;

    /// Returns the time range of the next encounter of the given type.
    /// If `t_start` is inside of a block, this block is not returned: only
    /// blocks **starting** after (or exactly on) `t_start` are considered.
    /// TODO: EventType should be the first parameter
    fn encounter(&self, t_start: f64, ty: EventType) -> Option<(f64, f64)>;

    /// Samples the sequence at the given time points
    fn sample(&self, time: &[f64]) -> Vec<Sample>;

    /// Integrates over the n-1 time intervalls given by the list of n time points.
    fn integrate(&self, time: &[f64]) -> Vec<Moment>;
}
