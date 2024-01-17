use std::path::Path;

use crate::{types::*, util, Sequence};
use pulseq_rs::Gradient;

mod helpers;

pub struct PulseqSequence {
    pub seq: pulseq_rs::Sequence,
}

impl PulseqSequence {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, pulseq_rs::Error> {
        Ok(Self {
            seq: pulseq_rs::Sequence::from_file(path)?,
        })
    }
}

impl Sequence for PulseqSequence {
    fn duration(&self) -> f32 {
        self.seq.blocks.iter().map(|b| b.duration).sum()
    }

    fn next_block(&self, t_start: f32, ty: EventType) -> Option<(f32, f32)> {
        for block in &self.seq.blocks {
            if t_start > block.t_start + block.duration {
                // This can't be the next block if t_start is after it ends
                continue;
            }

            let t = match ty {
                EventType::RfPulse => block
                    .rf
                    .as_ref()
                    .map(|rf| (rf.delay, rf.duration(self.seq.time_raster.rf))),
                EventType::Adc => block.adc.as_ref().map(|adc| (adc.delay, adc.duration())),
                EventType::Gradient(channel) => match channel {
                    GradientChannel::X => block.gx.as_ref(),
                    GradientChannel::Y => block.gy.as_ref(),
                    GradientChannel::Z => block.gz.as_ref(),
                }
                .map(|grad| (grad.delay(), grad.duration(self.seq.time_raster.grad))),
            };

            if let Some((delay, dur)) = t {
                if block.t_start + delay >= t_start {
                    return Some((block.t_start + delay, block.t_start + dur));
                }
            }
        }

        None
    }

    fn next_poi(&self, t_start: f32, ty: EventType) -> Option<f32> {
        for block in &self.seq.blocks {
            if t_start > block.t_start + block.duration {
                // POI can't be in this block if t_start is after it ends.
                continue;
            }

            // We sample in between samples, so for e.g., a shape of len=10
            // there will be 0..=10 -> 11 samples.
            let t = t_start - block.t_start;
            let t = match ty {
                EventType::RfPulse => block.rf.as_ref().map(|rf| {
                    let idx = ((t - rf.delay) / self.seq.time_raster.rf)
                        .clamp(0.0, rf.amp_shape.0.len() as f32)
                        .ceil();
                    rf.delay + idx * self.seq.time_raster.rf
                }),
                EventType::Adc => block.adc.as_ref().map(|adc| {
                    // Here we actually sample in the centers instead of edges because
                    // well, that's where the ADC samples are!
                    let idx = ((t - adc.delay) / adc.dwell - 0.5)
                        .clamp(0.0, adc.num as f32 - 1.0)
                        .ceil();
                    adc.delay + (idx + 0.5) * adc.dwell
                }),
                EventType::Gradient(channel) => match channel {
                    GradientChannel::X => block.gx.as_ref(),
                    GradientChannel::Y => block.gy.as_ref(),
                    GradientChannel::Z => block.gz.as_ref(),
                }
                .map(|grad| match grad.as_ref() {
                    Gradient::Free { delay, shape, .. } => {
                        let idx = ((t - delay) / self.seq.time_raster.grad)
                            .clamp(0.0, shape.0.len() as f32)
                            .ceil();
                        delay + idx * self.seq.time_raster.grad
                    }
                    &Gradient::Trap {
                        rise,
                        flat,
                        fall,
                        delay,
                        ..
                    } => {
                        // The four vertices of the trap are its POIs
                        if t < delay {
                            delay
                        } else if t < rise {
                            delay + rise
                        } else if t < rise + flat {
                            delay + rise + flat
                        } else {
                            // No if bc. of check below and mandatory else branch
                            delay + rise + flat + fall
                        }
                    }
                }),
            };

            if let Some(t) = t {
                if t + block.t_start >= t_start {
                    return Some(t + block.t_start);
                }
            }
        }

        None
    }

    fn integrate(&self, t_start: f32, t_end: f32) -> (PulseMoment, GradientMoment) {
        assert!(t_start < t_end);

        let idx_start = match self
            .seq
            .blocks
            .binary_search_by(|probe| probe.t_start.total_cmp(&t_start))
        {
            Ok(idx) => idx,             // start searching beginning with the exact match
            Err(idx) => idx.max(1) - 1, // start searching before the insertion point
        };
        let idx_end = match self
            .seq
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
        for block in &self.seq.blocks[idx_start..idx_end] {
            if let Some(gx) = block.gx.as_ref() {
                grad.gx += helpers::integrate_grad(
                    gx.as_ref(),
                    t_start,
                    t_end,
                    block.t_start,
                    self.seq.time_raster.grad,
                );
            }
            if let Some(gy) = block.gy.as_ref() {
                grad.gy += helpers::integrate_grad(
                    gy.as_ref(),
                    t_start,
                    t_end,
                    block.t_start,
                    self.seq.time_raster.grad,
                );
            }
            if let Some(gz) = block.gz.as_ref() {
                grad.gz += helpers::integrate_grad(
                    gz.as_ref(),
                    t_start,
                    t_end,
                    block.t_start,
                    self.seq.time_raster.grad,
                );
            }
        }

        let mut spin = util::Spin::relaxed();
        for block in &self.seq.blocks[idx_start..idx_end] {
            let Some(rf) = &block.rf else { continue };

            for i in 0..rf.amp_shape.0.len() {
                let dwell = self.seq.time_raster.rf;
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

                spin *= util::Rotation::new(
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

    fn sample(&self, t: f32) -> (PulseSample, GradientSample, AdcBlockSample) {
        let block_idx = match self
            .seq
            .blocks
            .binary_search_by(|probe| probe.t_start.total_cmp(&t))
        {
            Ok(idx) => idx,             // sample is exactly at beginning of block
            Err(idx) => idx.max(1) - 1, // sample is somewhere in the block
        };
        let block = &self.seq.blocks[block_idx];

        let pulse_sample = if let Some(rf) = &block.rf {
            let index =
                ((t - block.t_start - rf.delay) / self.seq.time_raster.rf - 0.5).ceil() as usize;
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
            helpers::sample_grad(t - block.t_start, gx.as_ref(), self.seq.time_raster.grad)
        });
        let y = block.gy.as_ref().map_or(0.0, |gy| {
            helpers::sample_grad(t - block.t_start, gy.as_ref(), self.seq.time_raster.grad)
        });
        let z = block.gz.as_ref().map_or(0.0, |gz| {
            helpers::sample_grad(t - block.t_start, gz.as_ref(), self.seq.time_raster.grad)
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
