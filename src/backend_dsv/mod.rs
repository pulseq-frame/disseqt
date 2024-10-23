use crate::{util, Backend, Moment};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use thiserror::Error;

mod adc;
mod grad;
mod helpers;
mod rf;
mod trigger;

#[derive(Error, Debug)]
pub enum Error {
    FileNotFound(PathBuf),
}

// TODO: use thiserror, color_eyre (if compatible with pydisseqt / python) or whatever
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FileNotFound(path_buf) => write!(f, "File not found: {}", path_buf.display()),
        }
    }
}

pub struct DsvSequence {
    rf: rf::Rf,
    gx: grad::Grad,
    gy: grad::Grad,
    gz: grad::Grad,
    adc: adc::Adc,
}

impl DsvSequence {
    pub fn load<P: AsRef<Path>>(path: P, resolution: Option<usize>) -> Result<Self, Error> {
        let rf = rf::Rf::load(&path)?;
        let gx = grad::Grad::load(&path, "GRX")?;
        let gy = grad::Grad::load(&path, "GRY")?;
        let gz = grad::Grad::load(&path, "GRZ")?;
        let adc = adc::Adc::load(path, resolution)?;

        Ok(Self {
            rf,
            gx,
            gy,
            gz,
            adc,
        })
    }
}

impl Backend for DsvSequence {
    fn fov(&self) -> Option<(f64, f64, f64)> {
        // TODO: Can be found in the .pro protocol XML file
        // Some((0.22, 0.22, 0.04))
        None
    }

    fn duration(&self) -> f64 {
        *[
            self.rf.duration(),
            self.gx.duration(),
            self.gy.duration(),
            self.gz.duration(),
            self.adc.duration(),
        ]
        .iter()
        .max_by(|a, b| a.total_cmp(b))
        .unwrap()
    }

    fn events(&self, ty: crate::EventType, t_start: f64, t_end: f64, max_count: usize) -> Vec<f64> {
        match ty {
            crate::EventType::RfPulse => self.rf.events(t_start, t_end, max_count),
            crate::EventType::Adc => self.adc.events(t_start, t_end, max_count),
            crate::EventType::Gradient(channel) => match channel {
                crate::GradientChannel::X => self.gx.events(t_start, t_end, max_count),
                crate::GradientChannel::Y => self.gy.events(t_start, t_end, max_count),
                crate::GradientChannel::Z => self.gz.events(t_start, t_end, max_count),
            },
        }
    }

    fn encounter(&self, t_start: f64, ty: crate::EventType) -> Option<(f64, f64)> {
        match ty {
            crate::EventType::RfPulse => self.rf.encounter(t_start),
            crate::EventType::Adc => self.adc.encounter(t_start),
            crate::EventType::Gradient(channel) => match channel {
                crate::GradientChannel::X => self.gx.encounter(t_start),
                crate::GradientChannel::Y => self.gy.encounter(t_start),
                crate::GradientChannel::Z => self.gz.encounter(t_start),
            },
        }
    }

    fn sample(&self, time: &[f64]) -> Vec<crate::Sample> {
        // TODO: look if this rounding is correct / where is the center of a sample?

        // TODO: maybe the current backend trait is suboptimal; It would be much
        // nicer if we could create the Vec types here directly.
        // Maybe provide both sample and sample_vec in the trait, with blanket impls?

        time.iter()
            .map(|&t| {
                // very much repetition - can we unify shapes somehow?

                // TODO: no out of bounds protection
                let index = (t / self.rf.time_step).round() as usize;

                let pulse = crate::RfPulseSample {
                    amplitude: *self.rf.amplitude.get(index).unwrap_or(&0.0),
                    phase: *self.rf.phase.get(index).unwrap_or(&0.0),
                    frequency: self.rf.frequency,
                };

                let gradient = crate::GradientSample {
                    x: self.gx.sample(t),
                    y: self.gy.sample(t),
                    z: self.gz.sample(t),
                };

                // TODO: no out of bounds protection
                let index = (t / self.adc.time_step).round() as usize;
                let adc = crate::AdcBlockSample {
                    active: *self.adc.active.get(index).unwrap_or(&false),
                    phase: *self.adc.phase.get(index).unwrap_or(&0.0),
                    frequency: self.adc.frequency,
                };

                crate::Sample {
                    pulse,
                    gradient,
                    adc,
                }
            })
            .collect()
    }

    fn integrate(&self, time: &[f64]) -> Vec<Moment> {
        let mut moments = Vec::new();
        for t in time.windows(2) {
            let mut spin = util::Spin::relaxed();
            self.rf.integrate(&mut spin, t[0], t[1]);

            let pulse = crate::RfPulseMoment {
                angle: spin.angle(),
                phase: spin.phase(),
            };
            moments.push(Moment {
                pulse,
                gradient: crate::GradientMoment {
                    x: self.gx.integrate(t[0], t[1]),
                    y: self.gy.integrate(t[0], t[1]),
                    z: self.gz.integrate(t[0], t[1]),
                },
            });
        }
        moments
    }
}

// TODO: replace all the unwraps with errors
