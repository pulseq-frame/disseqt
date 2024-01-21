use crate::{
    AdcBlockSampleVec, EventType, GradientMomentVec, GradientSampleVec, Moment, MomentVec,
    RfPulseMomentVec, RfPulseSampleVec, Sample, SampleVec,
};

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

    pub fn sample(&self, time: &[f32]) -> SampleVec {
        // TODO: We do a AoS -> SoA conversion here, which should be moved into the backend
        // so the data can be emitted directly in the desired format

        // SoA are better for performance, but the equivalent lenght of all individual
        // samples (pulse, gradient, adc) is not an invariant anymore - can we force this in some
        // way? Maybe making the contents private and providing accessor functions?
        let samples = self.0.sample(time);
        let pulse = RfPulseSampleVec {
            amplitude: samples.iter().map(|s| s.pulse.amplitude).collect(),
            phase: samples.iter().map(|s| s.pulse.phase).collect(),
            frequency: samples.iter().map(|s| s.pulse.frequency).collect(),
        };
        let gradient = GradientSampleVec {
            x: samples.iter().map(|s| s.gradient.x).collect(),
            y: samples.iter().map(|s| s.gradient.y).collect(),
            z: samples.iter().map(|s| s.gradient.z).collect(),
        };
        let adc = AdcBlockSampleVec {
            active: samples.iter().map(|s| s.adc.active).collect(),
            phase: samples.iter().map(|s| s.adc.phase).collect(),
            frequency: samples.iter().map(|s| s.adc.frequency).collect(),
        };
        SampleVec {
            pulse,
            gradient,
            adc,
        }
    }

    pub fn sample_one(&self, t: f32) -> Sample {
        self.0.sample(&[t])[0]
    }

    pub fn integrate(&self, time: &[f32]) -> MomentVec {
        let moments = self.0.integrate(time);
        let pulse = RfPulseMomentVec {
            angle: moments.iter().map(|m| m.pulse.angle).collect(),
            phase: moments.iter().map(|m| m.pulse.phase).collect(),
        };
        let gradient = GradientMomentVec {
            x: moments.iter().map(|m| m.gradient.x).collect(),
            y: moments.iter().map(|m| m.gradient.y).collect(),
            z: moments.iter().map(|m| m.gradient.z).collect(),
        };
        MomentVec { pulse, gradient }
    }

    pub fn integrate_one(&self, t_start: f32, t_end: f32) -> Moment {
        self.0.integrate(&[t_start, t_end])[0]
    }
}
