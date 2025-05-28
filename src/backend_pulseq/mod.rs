use std::path::Path;

use crate::{types::*, util, Backend};
use pulseq_rs::Gradient;

mod helpers;

pub struct PulseqSequence {
    // elements contain block start time
    pub blocks: Vec<(f64, pulseq_rs::Block)>,
    pub raster: pulseq_rs::TimeRaster,
    pub fov: Option<(f64, f64, f64)>,
}

impl PulseqSequence {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, pulseq_rs::Error> {
        let seq = pulseq_rs::Sequence::from_file(path)?;
        Ok(Self::from_seq(seq))
    }

    fn from_seq(seq: pulseq_rs::Sequence) -> Self {
        let blocks = seq
            .blocks
            .into_iter()
            .scan(0.0, |t_start, block| {
                let tmp = *t_start;
                *t_start += block.duration;
                Some((tmp, block))
            })
            .collect();
        // We could check for e.g. lower case fov and if definition is in mm
        let fov = seq
            .fov
            .or_else(|| seq.definitions.get("FOV").and_then(|s| parse_fov(s)));

        Self {
            blocks,
            raster: seq.time_raster,
            fov,
        }
    }
}

fn parse_fov(s: &str) -> Option<(f64, f64, f64)> {
    let splits: Vec<_> = s.split_whitespace().collect();
    if splits.len() == 3 {
        Some((
            splits[0].parse().ok()?,
            splits[1].parse().ok()?,
            splits[2].parse().ok()?,
        ))
    } else {
        None
    }
}

impl Backend for PulseqSequence {
    fn fov(&self) -> Option<(f64, f64, f64)> {
        self.fov
    }

    fn duration(&self) -> f64 {
        self.blocks.iter().map(|(_, b)| b.duration).sum()
    }

    fn events(&self, ty: EventType, t_start: f64, t_end: f64, max_count: usize) -> Vec<f64> {
        // NOTE: The indirection by using a trait object seems to be neglectable in terms of
        // performance, although it makes the API a bit worse, as the time range that is
        // usually only constructed for the function call now needs a reference.
        let mut t = t_start;
        let mut pois = Vec::new();
        // TODO: this currently is based on the PulseqSequence::next_poi function.
        // Replace with a more efficient impl that directly fetches a list of samples
        while let Some(t_next) = self.next_poi(t, ty) {
            // Important: make t_end exclusive so we don't need to advance by some small value
            if t_next >= t_end || pois.len() >= max_count {
                break;
            }
            pois.push(t_next);
            t = t_next + 1e-9;
        }

        pois
    }

    fn encounter(&self, t_start: f64, ty: EventType) -> Option<(f64, f64)> {
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

    fn integrate(&self, time: &[f64]) -> Vec<Moment> {
        let mut moments = Vec::new();
        for t in time.windows(2) {
            let (pulse, gradient) = self.integrate(t[0], t[1]);
            moments.push(Moment { pulse, gradient });
        }
        moments
    }

    fn sample(&self, time: &[f64]) -> Vec<Sample> {
        time.into_iter()
            .map(|t| {
                let (pulse, gradient, adc) = self.sample(*t);
                Sample {
                    pulse,
                    gradient,
                    adc,
                }
            })
            .collect()
    }
}

// The old, inefficient single-element methods are moved into this impl block,
// the trait implementation just loops over it.
// TODO: replace with code that effectively implements the function signatures
// given by the Sequence trait
impl PulseqSequence {
    fn next_poi(&self, t_start: f64, ty: EventType) -> Option<f64> {
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
                        .clamp(0.0, rf.amp_shape.0.len() as f64)
                        .ceil();
                    rf.delay + idx * self.raster.rf
                }),
                EventType::Adc => block.adc.as_ref().map(|adc| {
                    // Here we actually sample in the centers instead of edges because,
                    // well, that's where the ADC samples are!
                    let idx = ((t - adc.delay) / adc.dwell - 0.5)
                        .clamp(0.0, adc.num as f64 - 1.0)
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
                            .clamp(0.0, shape.0.len() as f64)
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

    fn integrate(&self, mut t_start: f64, mut t_end: f64) -> (RfPulseMoment, GradientMoment) {
        let mut sign = 1.0;
        if t_end < t_start {
            // Integrate backwards and flip sign
            std::mem::swap(&mut t_start, &mut t_end);
            sign = -1.0;
        }

        let idx_start = match self
            .blocks
            .binary_search_by(|(block_start, _)| block_start.total_cmp(&t_start))
        {
            Ok(idx) => idx,             // start with the exact match
            Err(idx) => idx.max(1) - 1, // start before the insertion point
        };

        let mut spin = util::Spin::relaxed();
        let mut grad = GradientMoment {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        for (block_start, block) in &self.blocks[idx_start..] {
            if *block_start >= t_end {
                break;
            }
            if let Some(gx) = block.gx.as_ref() {
                grad.x += helpers::integrate_grad(
                    gx.as_ref(),
                    t_start,
                    t_end,
                    *block_start,
                    self.raster.grad,
                );
            }
            if let Some(gy) = block.gy.as_ref() {
                grad.y += helpers::integrate_grad(
                    gy.as_ref(),
                    t_start,
                    t_end,
                    *block_start,
                    self.raster.grad,
                );
            }
            if let Some(gz) = block.gz.as_ref() {
                grad.z += helpers::integrate_grad(
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
            RfPulseMoment {
                angle: sign * spin.angle(),
                phase: sign * spin.phase(),
            },
            GradientMoment {
                x: sign * grad.x,
                y: sign * grad.y,
                z: sign * grad.z,
            },
        )
    }

    fn sample(&self, t: f64) -> (RfPulseSample, GradientSample, AdcBlockSample) {
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
                let shim = rf.shim_shape.as_ref().map(|(mag, phase)| {
                    assert_eq!(mag.0.len(), phase.0.len());
                    mag.0.iter().zip(&phase.0).map(|(&m, &p)| (m, p)).collect()
                });
                RfPulseSample {
                    amplitude: rf.amp * rf.amp_shape.0[index],
                    phase: rf.phase + rf.phase_shape.0[index] * std::f64::consts::TAU,
                    frequency: rf.freq,
                    shim,
                }
            } else {
                RfPulseSample::default()
            }
        } else {
            RfPulseSample::default()
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
                && t <= block_start + adc.delay + adc.num as f64 * adc.dwell
            {
                AdcBlockSample {
                    active: true,
                    phase: adc.phase + adc.freq * (t - *block_start - adc.delay),
                    frequency: adc.freq,
                }
            } else {
                AdcBlockSample::default()
            }
        } else {
            AdcBlockSample::default()
        };

        (pulse_sample, GradientSample { x, y, z }, adc_sample)
    }
}
