// TODO: Everything here and in the pulseq-rs crate should use f64!
// Individual samples can go down to sub-microsecond scale while sequence durations
// often are in seconds or minutes. Single-precision f32 is not sufficient in the
// general case. It probably is enough for simulation with absolute times, but
// errors accumulate too quickly when generating sequences -> use double precision!

// TODO: (Evaluate first if this idea is good) the Trait should only contain the
// functions that return lists of POIs or Samples, which are then implemented by
// the backend. Functions like load_pulseq then return a wrapper (newtype around
// Box<dyn Sequence>) that implements functions that only return a single poi etc.
// This has one advantage: taking e.g. an impl RangeBounds is not trait save, but
// can be implemented in the wrapper (for time ranges) without problems.

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

/// This trait is implemented by all backends and provides the basic functions
/// on which the public disseqt API is built upon
trait Backend: Send {
    /// Return the FOV of the Sequence, if it is available
    fn fov(&self) -> Option<(f32, f32, f32)>;

    /// Duration of the MRI sequence: no samples, blocks, etc. exist outside
    /// of the time range [0, duration()]
    fn duration(&self) -> f32;

    /// Returns all events of the given type in the given duration.
    /// t_start is inclusive, t_end is exclusive. If a max_count is given and
    /// reached, there might be more events in the time span that are not returned.
    fn events(&self, ty: EventType, t_start: f32, t_end: f32, max_count: usize) -> Vec<f32>;

    /// Returns the time range of the next encounter of the given type.
    /// If `t_start` is inside of a block, this block is not returned: only
    /// blocks **starting** after (or exactly on) `t_start` are considered.
    fn encounter(&self, t_start: f32, ty: EventType) -> Option<(f32, f32)>;

    /// Samples the sequence at the given time points
    fn sample(&self, time: &[f32]) -> Vec<Sample>;

    /// Integrates over the n-1 time intervalls given by the list of n time points.
    fn integrate(&self, time: &[f32]) -> Vec<Moment>;
}
