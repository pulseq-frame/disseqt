use std::path::Path;

use crate::{types::*, util, Sequence};
use pulseq_rs::Gradient;

mod helpers;

pub struct PulseqSequence {
    // elements contain block start time
    pub blocks: Vec<(f32, pulseq_rs::Block)>,
    pub raster: pulseq_rs::TimeRaster,
}

impl PulseqSequence {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, pulseq_rs::Error> {
        let seq = pulseq_rs::Sequence::from_file(path)?;
        let blocks = seq
            .blocks
            .into_iter()
            .scan(0.0, |t_start, block| {
                let tmp = *t_start;
                *t_start += block.duration;
                Some((tmp, block))
            })
            .collect();

        Ok(Self {
            blocks,
            raster: seq.time_raster,
        })
    }
}

impl Sequence for PulseqSequence {
    fn duration(&self) -> f32 {
        self.blocks.iter().map(|(_, b)| b.duration).sum()
    }

    fn next_block(&self, t_start: f32, ty: EventType) -> Option<(f32, f32)> {
        let idx_start = match self
            .blocks
            .binary_search_by(|(block_start, _)| block_start.total_cmp(&t_start))
        {
            Ok(idx) => idx,             // start with the exact match
            Err(idx) => idx.max(1) - 1, // start before the insertion point
        };

        for (block_start, block) in &self.blocks[idx_start..] {
            let t = match ty {
                EventType::RfPulse => block
                    .rf
                    .as_ref()
                    .map(|rf| (rf.delay, rf.duration(self.raster.rf))),
                EventType::Adc => block.adc.as_ref().map(|adc| (adc.delay, adc.duration())),
                EventType::Gradient(channel) => match channel {
                    GradientChannel::X => block.gx.as_ref(),
                    GradientChannel::Y => block.gy.as_ref(),
                    GradientChannel::Z => block.gz.as_ref(),
                }
                .map(|grad| (grad.delay(), grad.duration(self.raster.grad))),
            };

            if let Some((delay, dur)) = t {
                if block_start + delay >= t_start {
                    return Some((block_start + delay, block_start + dur));
                }
            }
        }

        None
    }

    fn next_poi(&self, t_start: f32, ty: EventType) -> Option<f32> {
        let idx_start = match self
            .blocks
            .binary_search_by(|(block_start, _)| block_start.total_cmp(&t_start))
        {
            Ok(idx) => idx,             // start with the exact match
            Err(idx) => idx.max(1) - 1, // start before the insertion point
        };

        for (block_start, block) in &self.blocks[idx_start..] {
            // We sample in between samples, so for e.g., a shape of len=10
            // there will be 0..=10 -> 11 samples.
            let t = t_start - block_start;
            let t = match ty {
                EventType::RfPulse => block.rf.as_ref().map(|rf| {
                    let idx = ((t - rf.delay) / self.raster.rf)
                        .clamp(0.0, rf.amp_shape.0.len() as f32)
                        .ceil();
                    rf.delay + idx * self.raster.rf
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
                        let idx = ((t - delay) / self.raster.grad)
                            .clamp(0.0, shape.0.len() as f32)
                            .ceil();
                        delay + idx * self.raster.grad
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
                if t + block_start >= t_start {
                    return Some(t + block_start);
                }
            }
        }

        None
    }

    fn integrate(&self, t_start: f32, t_end: f32) -> (PulseMoment, GradientMoment) {
        assert!(t_start < t_end);

        let idx_start = match self
            .blocks
            .binary_search_by(|(block_start, _)| block_start.total_cmp(&t_start))
        {
            Ok(idx) => idx,             // start with the exact match
            Err(idx) => idx.max(1) - 1, // start before the insertion point
        };

        let mut spin = util::Spin::relaxed();
        let mut grad = GradientMoment {
            gx: 0.0,
            gy: 0.0,
            gz: 0.0,
        };
        for (block_start, block) in &self.blocks[idx_start..] {
            if *block_start >= t_end {
                break;
            }
            if let Some(gx) = block.gx.as_ref() {
                grad.gx += helpers::integrate_grad(
                    gx.as_ref(),
                    t_start,
                    t_end,
                    *block_start,
                    self.raster.grad,
                );
            }
            if let Some(gy) = block.gy.as_ref() {
                grad.gy += helpers::integrate_grad(
                    gy.as_ref(),
                    t_start,
                    t_end,
                    *block_start,
                    self.raster.grad,
                );
            }
            if let Some(gz) = block.gz.as_ref() {
                grad.gz += helpers::integrate_grad(
                    gz.as_ref(),
                    t_start,
                    t_end,
                    *block_start,
                    self.raster.grad,
                );
            }
            if let Some(rf) = block.rf.as_ref() {
                helpers::integrate_rf(rf, &mut spin, t_start, t_end, *block_start, self.raster.rf);
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
            .blocks
            .binary_search_by(|(block_start, _)| block_start.total_cmp(&t))
        {
            Ok(idx) => idx,             // sample is exactly at beginning of block
            Err(idx) => idx.max(1) - 1, // sample is somewhere in the block
        };
        let (block_start, block) = &self.blocks[block_idx];

        let pulse_sample = if let Some(rf) = &block.rf {
            let index = ((t - block_start - rf.delay) / self.raster.rf - 0.5).ceil() as usize;
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
            helpers::sample_grad(t - block_start, gx.as_ref(), self.raster.grad)
        });
        let y = block.gy.as_ref().map_or(0.0, |gy| {
            helpers::sample_grad(t - block_start, gy.as_ref(), self.raster.grad)
        });
        let z = block.gz.as_ref().map_or(0.0, |gz| {
            helpers::sample_grad(t - block_start, gz.as_ref(), self.raster.grad)
        });

        let adc_sample = if let Some(adc) = &block.adc {
            if block_start + adc.delay <= t
                && t <= block_start + adc.delay + adc.num as f32 * adc.dwell
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
