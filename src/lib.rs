pub struct Pulse {
    /// Unit: `Hz`
    pub amplitude: f32,
    /// Unit: `rad`
    pub phase: f32,
    /// Unit: `Hz`
    pub frequency: f32,
}

pub struct Gradient {
    /// Unit: `Hz / m`
    pub x: f32,
    /// Unit: `Hz / m`
    pub y: f32,
    /// Unit: `Hz / m`
    pub z: f32,
}

pub enum Adc {
    Inactive,
    Active {
        /// Unit: `rad`
        phase: f32,
        /// Unit: `Hz`
        frequency: f32,
    },
}

pub struct PulseMoment {
    /// Unit: `rad`
    pub angle: f32,
    /// Unit: `rad`
    pub phase: f32,
}

pub struct GradientMoment {
    /// Unit: `rad / m`
    pub x: f32,
    /// Unit: `rad / m`
    pub y: f32,
    /// Unit: `rad / m`
    pub z: f32,
}

pub enum Poi {
    PulseStart,
    PulseSample,
    PulseEnd,
    GradientStart,
    GradientSample,
    GradientEnd,
    AdcStart,
    AdcSample,
    AdcEnd,
}

pub struct SeqParser {
    block_start_times: Vec<f32>,
    sequence: pulseq_rs::sequence::Sequence,
}

impl SeqParser {
    pub fn new(source: &str) -> Result<Self, pulseq_rs::parsers::common::ParseError> {
        let sequence = pulseq_rs::parse_file(source)?;

        let block_start_times = sequence
            .blocks
            .iter()
            .scan(0.0, |acc, b| {
                *acc += b.duration;
                Some(*acc)
            })
            .collect();

        Ok(Self {
            block_start_times,
            sequence,
        })
    }

    pub fn time_range(&self) -> (f32, f32) {
        (0.0, self.sequence.blocks.iter().map(|b| b.duration).sum())
    }

    pub fn next(&self, t_start: f32, poi: Poi) -> f32 {
        todo!()
    }

    pub fn integrate(&self, t0: f32, t1: f32) -> (PulseMoment, GradientMoment) {
        todo!()
    }

    pub fn sample(&self, t: f32) -> (Pulse, Gradient, Adc) {
        todo!()
    }
}
