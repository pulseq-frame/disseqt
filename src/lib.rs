//! This file contains the whole public API. It is designed in a way to be as
//! minimalistic as possible while providing all the tools necessary to plot,
//! simulate, ... MRI sequences. It does not expose the internal storage of the
//! sequence but a series of functions to sample it. This makes the usesrs of
//! this API independent of implementation details of, e.g. pulseq.

/// Contains the RF Pulse state for a single point in time.
#[derive(Debug, Clone, Copy)]
pub struct PulseSample {
    /// Unit: `Hz`
    pub amplitude: f32,
    /// Unit: `rad`
    pub phase: f32,
    /// Unit: `Hz`
    pub frequency: f32,
}

/// Contains the gradient amplitudes for a single point in time.
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
pub enum AdcBlockSample {
    Inactive,
    Active {
        /// Unit: `rad`
        phase: f32,
        /// Unit: `Hz`
        frequency: f32,
    },
}

/// Resulting flip angle by integrating an RF pulse over some time period.
#[derive(Debug, Clone, Copy)]
pub struct PulseMoment {
    /// Unit: `rad`
    pub angle: f32,
    /// Unit: `rad`
    pub phase: f32,
}

/// Resulting gradient moments by integrating gradients over some time period.
#[derive(Debug, Clone, Copy)]
pub struct GradientMoment {
    /// Unit: `rad / m`
    pub x: f32,
    /// Unit: `rad / m`
    pub y: f32,
    /// Unit: `rad / m`
    pub z: f32,
}

/// Point of Interest: Sequences are continuous in time, arbitary time points
/// can be sampled and arbitrary time periods can be integrated over. Some time
/// points are still of special interest, like ADC samples, RF Pulse start and
/// end or the vertices (samples) of a trapezoidal gradient. The `Poi` struct
/// contains the names for those time points, which can be used in
/// `Sequence::next` to fetch them.
#[derive(Debug, Clone, Copy)]
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

/// A MRI-Sequence black box. The inner structure of the sequence is hidden and
/// might even change in the future if other inputs than pulseq are supported.
/// Use the provided methods to sample and convert the sequence into any format.
pub struct Sequence {
    block_start_times: Vec<f32>,
    sequence: pulseq_rs::Sequence,
}

impl Sequence {
    /// Create a `Sequence` by parsing a pulseq .seq file.
    /// Returns an error if parsing fails.
    pub fn from_pulseq_file(source: &str) -> Result<Self, pulseq_rs::Error> {
        let sequence = pulseq_rs::Sequence::from_source(source)?;

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

    /// Calculate the duration of the MRI sequence. It is guaranteed that there
    /// are no POIs outside of the time range `[0, duration()]`
    pub fn duration(&self) -> f32 {
        self.sequence.blocks.iter().map(|b| b.duration).sum()
    }

    /// Return the next Point of Interest of the given type after the given
    /// point in time. Returns `None` if there is none.
    pub fn next(&self, t_start: f32, poi: Poi) -> Option<f32> {
        let idx_start = match self
            .block_start_times
            .binary_search_by(|t| t.total_cmp(&t_start))
        {
            Ok(idx) => idx,             // start searching beginning with the exact match
            Err(idx) => idx.max(1) - 1, // start searching before the insertion point
        };

        let mut t = t_start;
        for block in &self.sequence.blocks[idx_start..] {
            match poi {
                Poi::PulseStart => {
                    if let Some(rf) = &block.rf {
                        return Some(t + rf.delay);
                    }
                }
                Poi::PulseSample => todo!(),
                Poi::PulseEnd => {
                    if let Some(rf) = &block.rf {
                        return Some(t + rf.delay + rf.duration(self.sequence.time_raster.rf));
                    }
                }
                Poi::GradientStart => todo!(),
                Poi::GradientSample => todo!(),
                Poi::GradientEnd => todo!(),
                Poi::AdcStart => todo!(),
                Poi::AdcSample => todo!(),
                Poi::AdcEnd => todo!(),
            }
            t += block.duration;
        }

        None
    }

    /// Calculate the pulse and gradient moment for a given time range.
    /// # Panics
    /// If `t_start >= t_end`
    pub fn integrate(&self, t_start: f32, t_end: f32) -> (PulseMoment, GradientMoment) {
        assert!(t_start < t_end);

        let idx_start = match self
            .block_start_times
            .binary_search_by(|t| t.total_cmp(&t_start))
        {
            Ok(idx) => idx,             // start searching beginning with the exact match
            Err(idx) => idx.max(1) - 1, // start searching before the insertion point
        };
        let idx_end = match self
            .block_start_times
            .binary_search_by(|t| t.total_cmp(&t_end))
        {
            Ok(idx) => idx,  // end searching before the exact match
            Err(idx) => idx, // end searching before the insertion point
        };

        let mut rf = PulseMoment {
            angle: 0.0,
            phase: 0.0,
        };
        let mut grad = GradientMoment {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        // TODO: integrate over blocks[idx_start..idx_end]

        (rf, grad)
    }

    /// Returns the amplitudes and phases that are applied at time point `t`.
    pub fn sample(&self, t: f32) -> (PulseSample, GradientSample, AdcBlockSample) {
        todo!()
    }
}
