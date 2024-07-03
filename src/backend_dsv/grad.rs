use std::path::Path;

use crate::backend_dsv::helpers::DsvFile;

use super::{helpers::decompress_shape, trigger::Trigger, Error};

pub struct Grad {
    // TODO: this is written in the file, should convert it into something else
    /// Currently: mT/m
    amplitude: Vec<f64>,
    /// Sample time step in seconds
    time_step: f64,
    /// Location of gradients
    events: Trigger,
}

// TODO: the impls are very similar to RF - maybe factor out something?

impl Grad {
    pub fn load<P: AsRef<Path>>(path: P, which_dsv: &str) -> Result<Self, Error> {
        let dsv = DsvFile::load(path, which_dsv)?;

        // TODO: don't unwrap but return the parse errors
        // TODO: do the same with key errors (currently panics)
        let num_samples: usize = dsv.definitions["SAMPLES"].parse().unwrap();
        let time_step = dsv.definitions["HORIDELTA"].parse::<f64>().unwrap() * 1e-6;
        let amp_step = 1.0 / dsv.definitions["VERTFACTOR"].parse::<f64>().unwrap();

        let amplitude: Vec<f64> = decompress_shape(dsv.values, num_samples)
            .into_iter()
            .map(|x| x as f64 * amp_step)
            .collect();

        let events = Trigger::new(&amplitude);

        Ok(Self {
            amplitude,
            time_step,
            events,
        })
    }

    pub fn events(&self, t_start: f64, t_end: f64, max_count: usize) -> Vec<f64> {
        // Simple solution: we are on a fixed raster - return that.
        // Could only return events within encounters, but we assume that
        // The user checks where those encounters are themselves.
        let i_start = (t_start / self.time_step).ceil() as usize;
        let i_end = (t_end / self.time_step).ceil() as usize;

        (i_start..i_end)
            .take(max_count)
            .map(|i| i as f64 * self.time_step)
            .collect()
    }

    pub fn encounter(&self, t_start: f64) -> Option<(f64, f64)> {
        let i_start = (t_start / self.time_step).ceil() as usize;
        let (i_start, i_end) = self.events.search(i_start)?;

        Some((
            i_start as f64 * self.time_step,
            i_end as f64 * self.time_step,
        ))
    }

    pub fn sample(&self, t: f64) -> f64 {
        if t < 0.0 {
            0.0
        } else {
            let index = (t / self.time_step).round() as usize;
            self.amplitude.get(index).cloned().unwrap_or(0.0)
        }
    }
}
