use std::path::Path;

use crate::backend_dsv::trigger::Trigger;

use super::{
    helpers::{decompress_shape, DsvFile},
    Error,
};

pub struct Adc {
    /// Adc enabled or not
    pub active: Vec<bool>,
    /// Adc phase in radians
    pub phase: Vec<f64>,
    /// Sample time step in seconds
    pub time_step: f64,
    /// Frequency in Hz
    pub frequency: f64,
    /// Location of adc blocks
    events: Trigger,
}

impl Adc {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let active = AdcRaw::load(&path, "ADC")?;
        let phase = AdcRaw::load(&path, "NC1")?;

        // TODO: return errors instead of panicking
        assert_eq!(active.data.len(), phase.data.len());
        assert_eq!(active.time_step, phase.time_step);

        let events = Trigger::new(&active.data);
        let time_step = active.time_step;
        let frequency = active.frequency.unwrap_or(0.0);
        let phase = phase
            .data
            .into_iter()
            .map(|x| x * std::f64::consts::PI / 180.0)
            .collect();
        let active = active.data.into_iter().map(|x| x > 0.5).collect();

        Ok(Self {
            active,
            phase,
            time_step,
            events,
            frequency,
        })
    }

    pub fn duration(&self) -> f64 {
        self.time_step * self.active.len() as f64
    }

    pub fn encounter(&self, t_start: f64) -> Option<(f64, f64)> {
        let i_start = (t_start / self.time_step).ceil() as usize;
        let (i_start, i_end) = self.events.search(i_start)?;

        Some((
            i_start as f64 * self.time_step,
            (i_end + 1) as f64 * self.time_step,
        ))
    }

    pub fn events(&self, t_start: f64, t_end: f64, max_count: usize) -> Vec<f64> {
        // TODO Naming: the events inside of the Trigger are blocks and ADC events = samples
        let i_start = (t_start / self.time_step).ceil() as usize;
        let i_end = (t_end / self.time_step).floor() as usize;

        let mut samples = Vec::new();
        for event in self.events.events(i_start, i_end) {
            let a = i_start.max(event.0);
            let b = i_end.min(event.1);
            samples.extend(
                (a..=b)
                    .take(max_count - samples.len())
                    .map(|i| i as f64 * self.time_step),
            )
        }

        samples
    }
}

struct AdcRaw {
    data: Vec<f64>,
    time_step: f64,
    frequency: Option<f64>,
}
impl AdcRaw {
    pub fn load<P: AsRef<Path>>(path: P, which_dsv: &str) -> Result<Self, Error> {
        let dsv = DsvFile::load(&path, which_dsv)?;

        // TODO: don't unwrap but return the parse errors
        // TODO: do the same with key errors (currently panics)
        let num_samples: usize = dsv.definitions["SAMPLES"].parse().unwrap();
        let time_step = dsv.definitions["HORIDELTA"].parse::<f64>().unwrap() * 1e-6;
        let amp_step = 1.0 / dsv.definitions["VERTFACTOR"].parse::<f64>().unwrap();

        let frequency = dsv
            .definitions
            .get("NOMINALFREQUENCY")
            .map(|def| def.parse::<f64>().unwrap());

        let data: Vec<f64> = decompress_shape(dsv.values, num_samples)
            .into_iter()
            .map(|x| x as f64 * amp_step)
            .collect();

        Ok(Self {
            data,
            time_step,
            frequency,
        })
    }
}
