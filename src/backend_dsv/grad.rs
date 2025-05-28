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
        let time_step = dsv.time_step();
        let amp_step = dsv.amp_step(None);

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

    pub fn duration(&self) -> f64 {
        self.time_step * self.amplitude.len() as f64
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
            (i_end + 1) as f64 * self.time_step,
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

    pub fn integrate(&self, t_start: f64, t_end: f64) -> f64 {
        // TODO: this is not performant for integrations over long time periods
        // because it will sum up all zeros of the empty space between pulses
        let i_start = (t_start / self.time_step).floor() as usize;
        let mut grad = 0.0;

        for i in i_start..self.amplitude.len() {
            let t = i as f64 * self.time_step;

            // Skip samples before t_start, quit when reaching t_end
            if t + self.time_step < t_start {
                continue;
            }
            if t_end <= t {
                break;
            }

            // We could do the clamping for all samples, but when integrating
            // over many samples, it seems to be very sensitive to accumulating
            // errors. Only doing it in the edge cases is much more robust.
            let dur = if t_start <= t && t + self.time_step <= t_end {
                self.time_step
            } else {
                // Clamp the sample intervall to the integration intervall
                let t0 = t.clamp(t_start, t_end);
                let t1 = (t + self.time_step).clamp(t_start, t_end);
                t1 - t0
            };

            // TODO: units?
            grad += self.amplitude[i] * dur;
        }

        grad
    }
}
