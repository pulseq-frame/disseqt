//! SoA can have much better performance than AoS, especially in when using py-disseqt
//! where we need to extract the info in some loop, which can be very slow.
//! Right now, the implementation is duplicated - maybe there is some better way
//! of structuring it.
//! The non-prefixed structs are emitted by the sample_one and integrate_one functions
//! The ...Vec structs are emitted by sample and integrate.

/// Contains the RF Pulse state for a single point in time.
#[derive(Default, Debug, Clone, Copy)]
pub struct RfPulseSample {
    /// Unit: `Hz`
    pub amplitude: f32,
    /// Unit: `rad`
    pub phase: f32,
    /// Unit: `Hz`
    pub frequency: f32,
}

#[derive(Debug, Clone)]
pub struct RfPulseSampleVec {
    pub amplitude: Vec<f32>,
    pub phase: Vec<f32>,
    pub frequency: Vec<f32>,
}

/// Contains the gradient amplitudes for a single point in time.
#[derive(Default, Debug, Clone, Copy)]
pub struct GradientSample {
    /// Unit: `Hz / m`
    pub x: f32,
    /// Unit: `Hz / m`
    pub y: f32,
    /// Unit: `Hz / m`
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct GradientSampleVec {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub z: Vec<f32>,
}

/// Contains the ADC state for a single point in time. NOTE: this does not
/// indicate if a particular time point is sampled, only that an ADC block is
/// active (or not) at the particular point in time. Use the sequence POI API
/// to fetch the ADC sample locations.
#[derive(Default, Debug, Clone, Copy)]
pub struct AdcBlockSample {
    /// Specifies if the ADC is active, not if this is an ADC sample
    pub active: bool,
    /// Unit: `rad`
    pub phase: f32,
    /// Unit: `Hz`
    pub frequency: f32,
}

#[derive(Debug, Clone)]
pub struct AdcBlockSampleVec {
    pub active: Vec<bool>,
    pub phase: Vec<f32>,
    pub frequency: Vec<f32>,
}

/// See `RfPulseSample`, `GradientSample` and `AdcBlockSample`
#[derive(Default, Debug, Clone, Copy)]
pub struct Sample {
    pub pulse: RfPulseSample,
    pub gradient: GradientSample,
    pub adc: AdcBlockSample,
}

#[derive(Debug, Clone)]
pub struct SampleVec {
    pub pulse: RfPulseSampleVec,
    pub gradient: GradientSampleVec,
    pub adc: AdcBlockSampleVec,
}

impl SampleVec {
    pub fn len(&self) -> usize {
        // TODO: we should check for equal length of all vecs here or on
        // construction. Maybe use boxed slices to enforce invariance?
        self.pulse.amplitude.len()
    }
}

/// Resulting flip angle by integrating an RF pulse over some time period.
#[derive(Default, Debug, Clone, Copy)]
pub struct RfPulseMoment {
    /// Unit: `rad`
    pub angle: f32,
    /// Unit: `rad`
    pub phase: f32,
}

#[derive(Debug, Clone)]
pub struct RfPulseMomentVec {
    pub angle: Vec<f32>,
    pub phase: Vec<f32>,
}

/// Resulting gradient moments by integrating gradients over some time period.
#[derive(Default, Debug, Clone, Copy)]
pub struct GradientMoment {
    /// Unit: `rad / m`
    pub x: f32,
    /// Unit: `rad / m`
    pub y: f32,
    /// Unit: `rad / m`
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct GradientMomentVec {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub z: Vec<f32>,
}

/// See `RfPulseMoment` and `GradientMoment`
#[derive(Default, Debug, Clone, Copy)]
pub struct Moment {
    pub pulse: RfPulseMoment,
    pub gradient: GradientMoment,
}

#[derive(Debug, Clone)]
pub struct MomentVec {
    pub pulse: RfPulseMomentVec,
    pub gradient: GradientMomentVec,
}

/// Used for Block::Gradient(channel)
#[derive(Debug, Clone, Copy)]
pub enum GradientChannel {
    X,
    Y,
    Z,
}

/// Used to fetch the next POI or block time span of the given type.
#[derive(Debug, Clone, Copy)]
pub enum EventType {
    RfPulse,
    Adc,
    Gradient(GradientChannel),
}
