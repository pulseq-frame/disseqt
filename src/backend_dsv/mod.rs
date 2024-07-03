use crate::Backend;
use helpers::DsvFile;
use std::{collections::HashMap, path::Path};
use thiserror::Error;

mod helpers;
mod rf;
mod trigger;

#[derive(Error, Debug)]
pub enum Error {}

pub struct DsvSequence {
    rf: rf::Rf,
}

impl DsvSequence {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Ok(Self {
            // rf_amplitude: load_rfds(path)?,
            rf: rf::Rf::load(path)?,
        })
    }
}

impl Backend for DsvSequence {
    fn fov(&self) -> Option<(f64, f64, f64)> {
        // Can be found in the .pro protocol XML file
        Some((0.22, 0.22, 0.04))
    }

    fn duration(&self) -> f64 {
        todo!()
    }

    fn events(&self, ty: crate::EventType, t_start: f64, t_end: f64, max_count: usize) -> Vec<f64> {
        match ty {
            crate::EventType::RfPulse => self.rf.events(t_start, t_end, max_count),
            crate::EventType::Adc => Vec::new(),
            crate::EventType::Gradient(_) => Vec::new(),
        }
    }

    fn encounter(&self, t_start: f64, ty: crate::EventType) -> Option<(f64, f64)> {
        match ty {
            crate::EventType::RfPulse => self.rf.encounter(t_start),
            crate::EventType::Adc => None,
            crate::EventType::Gradient(_) => None,
        }
    }

    fn sample(&self, time: &[f64]) -> Vec<crate::Sample> {
        // TODO: look if this rounding is correct / where is the center of a sample?

        // TODO: maybe the current backend trait is suboptimal; It would be much
        // nicer if we could create the Vec types here directly.
        // Maybe provide both sample and sample_vec in the trait, with blanket impls?

        time.iter()
            .map(|&t| {
                let index = (t / self.rf.time_step).round() as usize;

                let pulse = crate::RfPulseSample {
                    amplitude: self.rf.amplitude[index],
                    phase: self.rf.phase[index],
                    frequency: self.rf.frequency,
                };

                let gradient = crate::GradientSample {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                };
                let adc = crate::AdcBlockSample {
                    active: false,
                    phase: 0.0,
                    frequency: 0.0,
                };

                crate::Sample {
                    pulse,
                    gradient,
                    adc,
                }
            })
            .collect()
    }

    fn integrate(&self, time: &[f64]) -> Vec<crate::Moment> {
        todo!()
    }
}

// TODO: replace all the unwraps with errors
