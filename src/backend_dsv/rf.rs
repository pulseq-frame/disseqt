use std::path::Path;

use super::{
    helpers::{decompress_shape, DsvFile},
    Error,
};

pub struct Rf {
    /// Rf amplitude in volts
    pub amplitude: Vec<f64>,
    /// Rf phase in radians
    pub phase: Vec<f64>,
    /// Sample time step in seconds
    pub time_step: f64,
    /// Frequency in Hz
    pub frequency: f64,
}

impl Rf {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let amplitude = RfRaw::load(&path, "RFD")?;
        let mut phase = RfRaw::load(path, "RFP")?;

        // TODO: return errors instead of panicking
        assert_eq!(amplitude.data.len(), phase.data.len());
        assert_eq!(amplitude.time_step, phase.time_step);
        assert_eq!(amplitude.frequency, phase.frequency);

        // Convert degrees to radians
        for x in &mut phase.data {
            *x = *x * std::f64::consts::PI / 180.0;
        }

        Ok(Self {
            amplitude: amplitude.data,
            phase: phase.data,
            time_step: amplitude.time_step,
            frequency: amplitude.frequency,
        })
    }
}

struct RfRaw {
    /// Can be amplitude in volts or phase in degrees
    data: Vec<f64>,
    time_step: f64,
    frequency: f64,
}
impl RfRaw {
    pub fn load<P: AsRef<Path>>(path: P, which_dsv: &str) -> Result<Self, Error> {
        let dsv = DsvFile::load(&path, which_dsv)?;

        // TODO: don't unwrap but return the parse errors
        // TODO: do the same with key errors (currently panics)
        let num_samples: usize = dsv.definitions["SAMPLES"].parse().unwrap();
        let time_step = dsv.definitions["HORIDELTA"].parse::<f64>().unwrap() * 1e-6;
        let volt_step = 1.0 / dsv.definitions["VERTFACTOR"].parse::<f64>().unwrap();
        let frequency = dsv.definitions["NOMINALFREQUENCY"].parse::<f64>().unwrap();

        let data: Vec<f64> = decompress_shape(dsv.values, num_samples)
            .into_iter()
            .map(|x| x as f64 * volt_step)
            .collect();

        Ok(Self {
            data,
            time_step,
            frequency,
        })
    }
}
