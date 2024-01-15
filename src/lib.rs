//! This file contains the whole public API. It is designed in a way to be as
//! minimalistic as possible while providing all the tools necessary to plot,
//! simulate, ... MRI sequences. It does not expose the internal storage of the
//! sequence but a series of functions to sample it. This makes the usesrs of
//! this API independent of implementation details of, e.g. pulseq.

// TODO: Everything here and in the pulseq-rs crate should use f64!
// Individual samples can go down to sub-microsecond scale while sequence durations
// often are in seconds or minutes. Single-precision f32 is not sufficient in the
// general case. It probably is enough for simulation with absolute times, but
// errors accumulate too quickly when generating sequences -> use double precision!

use pulseq_rs::{Gradient, Shape};

use crate::util::Rotation;

mod util;

/// Contains the RF Pulse state for a single point in time.
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
    pub gx: f32,
    /// Unit: `rad / m`
    pub gy: f32,
    /// Unit: `rad / m`
    pub gz: f32,
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
        // TODO: Performance can be improved by using binary search.
        for block in &self.0.blocks {
            // We are too early and try beginning with the next block
            if t_start > block.t_start + block.duration {
                continue;
            }

            let t = match poi {
                Poi::PulseStart => block.rf.as_ref().map(|rf| block.t_start + rf.delay),
                Poi::PulseSample => block.rf.as_ref().map(|rf| {
                    // Get the index of the next pulse sample
                    let index =
                        ((t_start - block.t_start - rf.delay) / self.0.time_raster.rf - 0.5).ceil();
                    // Clip to the actual number of samples. If the result is before
                    // t_start, it is handled by the check below.
                    let index = (index as usize).min(rf.amp_shape.0.len() - 1);
                    // Convert back to time
                    block.t_start + rf.delay + (index as f32 + 0.5) * self.0.time_raster.rf
                }),
                Poi::PulseEnd => block
                    .rf
                    .as_ref()
                    .map(|rf| block.t_start + rf.duration(self.0.time_raster.rf)),
                Poi::GradientStart => todo!(),
                Poi::GradientSample => todo!(),
                Poi::GradientEnd => todo!(),
                Poi::AdcStart => block.adc.as_ref().map(|adc| block.t_start + adc.delay),
                Poi::AdcSample => block.adc.as_ref().map(|adc| {
                    // Get the index of the next pulse sample
                    let index = ((t_start - block.t_start - adc.delay) / adc.dwell - 0.5).ceil();
                    // Clip to the actual number of samples. If the result is before
                    // t_start, it is handled by the check below.
                    let index = (index as usize).min(adc.num as usize - 1);
                    // Convert back to time
                    block.t_start + adc.delay + (index as f32 + 0.5) * adc.dwell
                }),
                Poi::AdcEnd => block.adc.as_ref().map(|adc| block.t_start + adc.duration()),
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
            gx: 0.0,
            gy: 0.0,
            gz: 0.0,
        };
        for block in &self.0.blocks[idx_start..idx_end] {
            if let Some(gx) = block.gx.as_ref() {
                grad.gx += integrate_grad(
                    gx.as_ref(),
                    t_start,
                    t_end,
                    block.t_start,
                    self.0.time_raster.grad,
                );
            }
            if let Some(gy) = block.gy.as_ref() {
                grad.gy += integrate_grad(
                    gy.as_ref(),
                    t_start,
                    t_end,
                    block.t_start,
                    self.0.time_raster.grad,
                );
            }
            if let Some(gz) = block.gz.as_ref() {
                grad.gz += integrate_grad(
                    gz.as_ref(),
                    t_start,
                    t_end,
                    block.t_start,
                    self.0.time_raster.grad,
                );
            }
        }

        let mut spin = util::Spin::relaxed();
        for block in &self.0.blocks[idx_start..idx_end] {
            let Some(rf) = &block.rf else { continue };

            for i in 0..rf.amp_shape.0.len() {
                let dwell = self.0.time_raster.rf;
                // Start time of the sample number i
                let t = block.t_start + rf.delay + i as f32 * dwell;

                // Skip samples before t_start, quit when reaching t_end
                if t + dwell < t_start {
                    continue;
                }
                if t_end <= t {
                    break;
                }

                // We could do the clamping for all samples, but when integrating
                // over many samples, it seems to be very sensitive to accumulating
                // errors. Only doing it in the edge cases is much more robust.
                let dur = if t_start <= t && t + dwell <= t_end {
                    dwell
                } else {
                    // Clamp the sample intervall to the integration intervall
                    let t0 = f32::max(t_start, t);
                    let t1 = f32::min(t_end, t + dwell);
                    t1 - t0
                };

                spin *= Rotation::new(
                    rf.amp * rf.amp_shape.0[i] * dur * std::f32::consts::TAU,
                    rf.phase + rf.phase_shape.0[i] * std::f32::consts::TAU,
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
        let block_idx = match self
            .0
            .blocks
            .binary_search_by(|probe| probe.t_start.total_cmp(&t))
        {
            Ok(idx) => idx,             // sample is exactly at beginning of block
            Err(idx) => idx.max(1) - 1, // sample is somewhere in the block
        };
        let block = &self.0.blocks[block_idx];

        let pulse_sample = if let Some(rf) = &block.rf {
            let index =
                ((t - block.t_start - rf.delay) / self.0.time_raster.rf - 0.5).ceil() as usize;
            if index < rf.amp_shape.0.len() {
                PulseSample {
                    amplitude: rf.amp * rf.amp_shape.0[index],
                    phase: rf.phase + rf.phase_shape.0[index] * std::f32::consts::TAU,
                    frequency: rf.freq,
                }
            } else {
                PulseSample::default()
            }
        } else {
            PulseSample::default()
        };

        let x = block.gx.as_ref().map_or(0.0, |gx| {
            sample_grad(t - block.t_start, gx.as_ref(), self.0.time_raster.grad)
        });
        let y = block.gy.as_ref().map_or(0.0, |gy| {
            sample_grad(t - block.t_start, gy.as_ref(), self.0.time_raster.grad)
        });
        let z = block.gz.as_ref().map_or(0.0, |gz| {
            sample_grad(t - block.t_start, gz.as_ref(), self.0.time_raster.grad)
        });

        let adc_sample = if let Some(adc) = &block.adc {
            if block.t_start + adc.delay <= t
                && t <= block.t_start + adc.delay + adc.num as f32 * adc.dwell
            {
                AdcBlockSample::Active {
                    phase: adc.phase,
                    frequency: adc.freq,
                }
            } else {
                AdcBlockSample::Inactive
            }
        } else {
            AdcBlockSample::Inactive
        };

        (pulse_sample, GradientSample { x, y, z }, adc_sample)
    }
}

fn integrate_grad(
    gx: &Gradient,
    t_start: f32,
    t_end: f32,
    block_start: f32,
    grad_raster: f32,
) -> f32 {
    match gx {
        Gradient::Free { amp, delay, shape } => {
            amp * integrate_free(
                t_start - block_start - delay,
                t_end - block_start - delay,
                shape,
                grad_raster,
            )
        }
        Gradient::Trap {
            amp,
            rise,
            flat,
            fall,
            delay,
        } => {
            amp * integrate_trap(
                t_start - block_start - delay,
                t_end - block_start - delay,
                *rise,
                *flat,
                *fall,
            )
        }
    }
}

fn sample_grad(t: f32, grad: &Gradient, grad_raster: f32) -> f32 {
    match grad {
        pulseq_rs::Gradient::Free { amp, delay, shape } => {
            let index = ((t - delay) / grad_raster - 0.5).ceil() as usize;
            shape.0.get(index).map_or(0.0, |x| amp * x)
        }
        pulseq_rs::Gradient::Trap {
            amp,
            rise,
            flat,
            fall,
            delay,
        } => amp * trap_sample(t - delay, *rise, *flat, *fall),
    }
}

fn trap_sample(t: f32, rise: f32, flat: f32, fall: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t < rise {
        t / rise
    } else if t < rise + flat {
        1.0
    } else if t < rise + flat + fall {
        ((rise + flat + fall) - t) / fall
    } else {
        0.0
    }
}

fn integrate_trap(t_start: f32, t_end: f32, rise: f32, flat: f32, fall: f32) -> f32 {
    let integral = |t| {
        if t <= rise {
            0.5 * t * t / rise
        } else if t <= rise + flat {
            (0.5 * rise) + (t - rise)
        } else {
            let rev_t = rise + flat + fall - t;
            (0.5 * rise) + flat + (0.5 * (fall - rev_t * rev_t / fall))
        }
    };
    integral(t_end.min(rise + flat + fall)) - integral(t_start.max(0.0))
}

fn integrate_free(t_start: f32, t_end: f32, shape: &Shape, dwell: f32) -> f32 {
    let mut integrated = 0.0;

    for i in 0..shape.0.len() {
        // Start time of the sample number i
        let t = i as f32 * dwell;

        // Skip samples before t_start, quit when reaching t_end
        if t + dwell < t_start {
            continue;
        }
        if t_end <= t {
            break;
        }

        // We could do the clamping for all samples, but when integrating
        // over many samples, it seems to be very sensitive to accumulating
        // errors. Only doing it in the edge cases is much more robust.
        let dur = if t_start <= t && t + dwell <= t_end {
            dwell
        } else {
            // Clamp the sample intervall to the integration intervall
            let t0 = f32::max(t_start, t);
            let t1 = f32::min(t_end, t + dwell);
            t1 - t0
        };

        integrated += shape.0[i] * dur;
    }

    integrated
}
