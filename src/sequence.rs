use crate::{EventType, Moment, Sample};

/// TODO: Document very well, this is the type the user works with!

/// A disseqt sequence. This opaque type on purpose does not expose the sequence data,
/// but provides a simple interface which makes it possible to build importers and more
/// that efficiently work with all supported MRI file formats.
pub struct Sequence(pub(crate) Box<dyn super::Backend>);

// Largely just forwards the trait impls, but also adds convenicence functions.
impl Sequence {
    pub fn fov(&self) -> Option<(f32, f32, f32)> {
        self.0.fov()
    }

    pub fn duration(&self) -> f32 {
        self.0.duration()
    }

    pub fn encounter(&self, t_start: f32, ty: EventType) -> Option<(f32, f32)> {
        self.0.encounter(t_start, ty)
    }

    // t_end is exclusive, so following up with a new call where t_start == t_end will not overlap
    /// Useful default values for `t_start`, `t_end` and `max_count` that will not limit the returned events:
    /// ```
    /// let t_start = f32::NEG_INFINITY;
    /// let t_end = f32::INFINITY;
    /// let max_count = usize::MAX;
    /// ```
    pub fn events(&self, ty: EventType, t_start: f32, t_end: f32, max_count: usize) -> Vec<f32> {
        self.0.events(ty, t_start, t_end, max_count)
    }

    pub fn next_event(&self, t_start: f32, ty: EventType) -> Option<f32> {
        self.events(ty, t_start, f32::INFINITY, 1).last().cloned()
    }

    pub fn sample(&self, time: &[f32]) -> Vec<Sample> {
        self.0.sample(time)
    }

    pub fn sample_one(&self, t: f32) -> Sample {
        self.sample(&[t])[0]
    }

    pub fn integrate(&self, time: &[f32]) -> Vec<Moment> {
        self.0.integrate(time)
    }

    pub fn integrate_one(&self, t_start: f32, t_end: f32) -> Moment {
        self.integrate(&[t_start, t_end])[0]
    }
}
