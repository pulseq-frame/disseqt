/// Contains the RF Pulse state for a single point in time.
/// TODO: Look into if we should universally use Pulse, RfPulse or Rf
#[derive(Default, Debug, Clone, Copy)]
pub struct PulseSample {
    /// Unit: `Hz`
    pub amplitude: f32,
    /// Unit: `rad`
    pub phase: f32,
    /// Unit: `Hz`
    pub frequency: f32,
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

/// Contains the ADC state for a single point in time. NOTE: this does not
/// indicate if a particular time point is sampled, only that an ADC block is
/// active (or not) at the particular point in time. Use the sequence POI API
/// to fetch the ADC sample locations.
#[derive(Default, Debug, Clone, Copy)]
pub enum AdcBlockSample {
    #[default]
    Inactive,
    Active {
        /// Unit: `rad`
        phase: f32,
        /// Unit: `Hz`
        frequency: f32,
    },
}

/// See `PulseSample`, `GradientSample` and `AdcBlockSample`
#[derive(Default, Debug, Clone, Copy)]
pub struct Sample {
    pub pulse: PulseSample,
    pub gradient: GradientSample,
    pub adc: AdcBlockSample,
}

/// Resulting flip angle by integrating an RF pulse over some time period.
#[derive(Default, Debug, Clone, Copy)]
pub struct PulseMoment {
    /// Unit: `rad`
    pub angle: f32,
    /// Unit: `rad`
    pub phase: f32,
}

/// Resulting gradient moments by integrating gradients over some time period.
#[derive(Default, Debug, Clone, Copy)]
pub struct GradientMoment {
    /// Unit: `rad / m`
    pub gx: f32,
    /// Unit: `rad / m`
    pub gy: f32,
    /// Unit: `rad / m`
    pub gz: f32,
}

/// See `PulseMoment` and `GradientMoment`
#[derive(Default, Debug, Clone, Copy)]
pub struct Moment {
    pub pulse: PulseMoment,
    pub gradient: GradientMoment,
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
