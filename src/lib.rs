mod backend_dsv;
mod backend_pulseq;
mod types;
mod util;

use std::path::Path;
pub use types::*;

pub fn load_pulseq<P: AsRef<Path>>(path: P) -> Result<Sequence, pulseq_rs::Error> {
    Ok(Sequence(Box::new(backend_pulseq::PulseqSequence::load(
        path,
    )?)))
}

pub fn load_dsv<P: AsRef<Path>>(
    path: P,
    resolution: Option<usize>,
    ref_voltage: f64,
) -> Result<Sequence, backend_dsv::Error> {
    Ok(Sequence(Box::new(backend_dsv::DsvSequence::load(
        path,
        resolution,
        ref_voltage,
    )?)))
}

/// A disseqt sequence. This opaque type on purpose does not expose the sequence data,
/// but provides a simple interface which makes it possible to build importers and more
/// that efficiently work with all supported MRI file formats.
pub struct Sequence(pub(crate) Box<dyn Backend>);

// Largely just forwards the trait impls, but also adds convenicence functions.
impl Sequence {
    pub fn fov(&self) -> Option<(f64, f64, f64)> {
        self.0.fov()
    }

    pub fn duration(&self) -> f64 {
        self.0.duration()
    }
    /// TODO: EventType should be the first parameter
    pub fn encounter(&self, t_start: f64, ty: EventType) -> Option<(f64, f64)> {
        self.0.encounter(t_start, ty)
    }

    pub fn events(&self, ty: EventType, t_start: f64, t_end: f64, max_count: usize) -> Vec<f64> {
        self.0.events(ty, t_start, t_end, max_count)
    }
    /// TODO: EventType should be the first parameter
    pub fn next_event(&self, t_start: f64, ty: EventType) -> Option<f64> {
        self.events(ty, t_start, f64::INFINITY, 1).last().cloned()
    }

    pub fn sample(&self, time: &[f64]) -> SampleVec {
        // TODO: We do a AoS -> SoA conversion here, which should be moved into the backend
        // so the data can be emitted directly in the desired format
        self.0.sample(time).into()
    }

    pub fn sample_one(&self, t: f64) -> Sample {
        self.0.sample(&[t])[0].clone()
    }

    pub fn integrate(&self, time: &[f64]) -> MomentVec {
        // TODO: We do a AoS -> SoA conversion here, which should be moved into the backend
        // so the data can be emitted directly in the desired format
        self.0.integrate(time).into()
    }

    pub fn integrate_one(&self, t_start: f64, t_end: f64) -> Moment {
        self.0.integrate(&[t_start, t_end])[0]
    }
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
