use std::path::Path;

use crate::{backend_dsv::trigger::Trigger, util};

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
    /// Location of pulses
    events: Trigger,
}

impl Rf {
    pub fn load<P: AsRef<Path>>(path: P, ref_voltage: f64) -> Result<Self, Error> {
        let amplitude = RfRaw::load(&path, "RFD", Some(ref_voltage))?;

        // Seems like there is not always an RFP file
        let phase = if let Ok(mut phase) = RfRaw::load(path, "RFP", None) {
            // TODO: return errors instead of panicking
            assert_eq!(amplitude.data.len(), phase.data.len());
            assert_eq!(amplitude.time_step, phase.time_step);
            assert_eq!(amplitude.frequency, phase.frequency);

            // Convert degrees to radians
            for x in &mut phase.data {
                *x = *x * std::f64::consts::PI / 180.0;
            }
            phase.data
        } else {
            vec![0.0; amplitude.data.len()]
        };

        let events = Trigger::new(&amplitude.data);
        println!("{events:?}");

        Ok(Self {
            amplitude: amplitude.data,
            phase,
            time_step: amplitude.time_step,
            frequency: amplitude.frequency,
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

    pub fn integrate(&self, spin: &mut util::Spin, t_start: f64, t_end: f64) {
        // TODO: this is not performant for integrations over long time periods
        // because it will sum up all zeros of the empty space between pulses
        let i_start = (t_start / self.time_step).floor() as usize;

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

            *spin *= util::Rotation::new(
                self.amplitude[i] * dur * std::f64::consts::TAU,
                self.phase[i],
            );
        }
    }
}

struct RfRaw {
    /// Can be amplitude in volts or phase in degrees
    data: Vec<f64>,
    time_step: f64,
    frequency: f64,
}
impl RfRaw {
    pub fn load<P: AsRef<Path>>(
        path: P,
        which_dsv: &str,
        ref_voltage: Option<f64>,
    ) -> Result<Self, Error> {
        let dsv = DsvFile::load(&path, which_dsv)?;

        // TODO: don't unwrap but return the parse errors
        // TODO: do the same with key errors (currently panics)
        let num_samples: usize = dsv.definitions["SAMPLES"].parse().unwrap();
        let time_step = dsv.time_step();
        let amp_step = dsv.amp_step(ref_voltage);
        let frequency = dsv.definitions["NOMINALFREQUENCY"].parse::<f64>().unwrap();

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
