//! This file contains the whole public API. It is designed in a way to be as
//! minimalistic as possible while providing all the tools necessary to plot,
//! simulate, ... MRI sequences. It does not expose the internal storage of the
//! sequence but a series of functions to sample it. This makes the usesrs of
//! this API independent of implementation details of, e.g. pulseq.

use std::iter::once;

use crate::util::Rotation;

mod util;

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
pub struct Sequence(pulseq_rs::Sequence);

impl Sequence {
    /// Create a `Sequence` by parsing a pulseq .seq file.
    /// Returns an error if parsing fails.
    pub fn from_pulseq_file(source: &str) -> Result<Self, pulseq_rs::Error> {
        pulseq_rs::Sequence::from_source(source).map(Self)
    }

    /// Calculate the duration of the MRI sequence. It is guaranteed that there
    /// are no POIs outside of the time range `[0, duration()]`
    pub fn duration(&self) -> f32 {
        self.0.blocks.iter().map(|b| b.duration).sum()
    }

    /// Return the next Point of Interest of the given type after the given
    /// point in time. Returns `None` if there is none.
    pub fn next(&self, t_start: f32, poi: Poi) -> Option<f32> {
        for block in &self.0.blocks {
            // Check if block is too early - we could directly start with the
            // right block by doing a binary search in the start times, but to
            // guarantee correctness, this first version is as simple as possible.
            if t_start > block.t_start + block.duration {
                continue;
            }

            let t = match poi {
                Poi::PulseStart => block.rf.as_ref().map(|rf| block.t_start + rf.delay),
                Poi::PulseSample => todo!(),
                Poi::PulseEnd => block
                    .rf
                    .as_ref()
                    .map(|rf| block.t_start + rf.duration(self.0.time_raster.rf)),
                Poi::GradientStart => todo!(),
                Poi::GradientSample => todo!(),
                Poi::GradientEnd => todo!(),
                Poi::AdcStart => todo!(),
                Poi::AdcSample => todo!(),
                Poi::AdcEnd => todo!(),
            };
            // Only return the POI if it's actually after t_start
            if let Some(t) = t {
                if t >= t_start {
                    return Some(t);
                }
            }
        }

        None
    }

    /// Calculate the pulse and gradient moment for a given time range.
    /// # Panics
    /// If `t_start >= t_end`
    pub fn integrate(&self, t_start: f32, t_end: f32) -> (PulseMoment, GradientMoment) {
        assert!(t_start < t_end);

        let idx_start = match self
            .0
            .blocks
            .binary_search_by(|probe| probe.t_start.total_cmp(&t_start))
        {
            Ok(idx) => idx,             // start searching beginning with the exact match
            Err(idx) => idx.max(1) - 1, // start searching before the insertion point
        };
        let idx_end = match self
            .0
            .blocks
            .binary_search_by(|probe| probe.t_start.total_cmp(&t_end))
        {
            Ok(idx) => idx,  // end searching before the exact match
            Err(idx) => idx, // end searching before the insertion point
        };

        let mut grad = GradientMoment {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };

        // let mut spin = util::Spin::relaxed();
        // println!("{idx_start}..{idx_end}");
        // for block in &self.0.blocks[idx_start..idx_end] {
        //     if let Some(rf) = &block.rf {
        //         let sample_time: Vec<f32> = match &rf.time_shape {
        //             Some(shape) => shape.0.clone(),
        //             None => (0..rf.amp_shape.0.len()).map(|i| i as f32).collect(),
        //         };
        //         let sample_dur: Vec<f32> = match &rf.time_shape {
        //             Some(shape) => shape
        //                 .0
        //                 .windows(2)
        //                 .map(|ab| {
        //                     let [a, b] = ab else { unreachable!() };
        //                     b - a
        //                 })
        //                 .chain(once(1.0))
        //                 .collect(),
        //             None => vec![1.0; rf.amp_shape.0.len()],
        //         };

        //         for i in 0..rf.amp_shape.0.len() {
        //             let sample_start =
        //                 block.t_start + rf.delay + sample_time[i] * self.0.time_raster.rf;
        //             let sample_end = sample_start + sample_dur[i] * self.0.time_raster.rf;

        //             // Sample is before the integration window
        //             if sample_end <= t_start {
        //                 continue;
        //             }
        //             // Sample has passed the integration window
        //             if t_end <= sample_start {
        //                 break;
        //             }

        //             // Sample overlaps integration window
        //             let amp = rf.amp * rf.amp_shape.0[i];
        //             let phase = rf.phase + rf.phase_shape.0[i] * std::f32::consts::PI;

        //             // Calculate the overlap of sample and integration window
        //             let dur = f32::min(sample_end, t_end) - f32::max(sample_start, t_start);
        //             // dbg!(dur);
        //             spin *= Rotation::new(amp * dur, phase);
        //         }
        //     }
        // }

        // Basic first impl: integrate over whole pulse, ignore t_start, t_end.
        // In addition, we ignore time shapes

        let mut spin = util::Spin::relaxed();
        for block in &self.0.blocks[idx_start..idx_end] {
            let Some(rf) = &block.rf else { continue };

            for (amp_sample, phase_sample) in rf.amp_shape.0.iter().zip(&rf.phase_shape.0) {
                let sample_dur = self.0.time_raster.rf;
                spin *= Rotation::new(
                    rf.amp * amp_sample * sample_dur * std::f32::consts::TAU,
                    rf.phase + phase_sample * std::f32::consts::TAU,
                );
            }
        }

        (
            PulseMoment {
                angle: spin.angle(),
                phase: spin.phase(),
            },
            grad,
        )
    }

    /// Returns the amplitudes and phases that are applied at time point `t`.
    pub fn sample(&self, t: f32) -> (PulseSample, GradientSample, AdcBlockSample) {
        todo!()
    }
}
