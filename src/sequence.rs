use crate::{EventType, Moment, MomentVec, Sample, SampleVec};

/// TODO: Document very well, this is the type the user works with!

/// A disseqt sequence. This opaque type on purpose does not expose the sequence data,
/// but provides a simple interface which makes it possible to build importers and more
/// that efficiently work with all supported MRI file formats.
pub struct Sequence(pub(crate) Box<dyn super::Backend>);

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
        self.0.sample(&[t])[0]
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
