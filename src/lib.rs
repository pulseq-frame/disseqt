//! This file contains the whole public API. It is designed in a way to be as
//! minimalistic as possible while providing all the tools necessary to plot,
//! simulate, ... MRI sequences. It does not expose the internal storage of the
//! sequence but a series of functions to sample it. This makes the usesrs of
//! this API independent of implementation details of, e.g. pulseq.

// TODO: Everything here and in the pulseq-rs crate should use f64!
// Individual samples can go down to sub-microsecond scale while sequence durations
// often are in seconds or minutes. Single-precision f32 is not sufficient in the
// general case. It probably is enough for simulation with absolute times, but
// errors accumulate too quickly when generating sequences -> use double precision!

// TODO: (Evaluate first if this idea is good) the Trait should only contain the
// functions that return lists of POIs or Samples, which are then implemented by
// the backend. Functions like load_pulseq then return a wrapper (newtype around
// Box<dyn Sequence>) that implements functions that only return a single poi etc.
// This has one advantage: taking e.g. an impl RangeBounds is not trait save, but
// can be implemented in the wrapper (for time ranges) without problems.

mod backend_pulseq;
mod types;
mod util;

use std::{ops::RangeBounds, path::Path};
pub use types::*;

pub fn load_pulseq<P: AsRef<Path>>(path: P) -> Result<Box<dyn Sequence>, pulseq_rs::Error> {
    Ok(Box::new(backend_pulseq::PulseqSequence::load(path)?))
}

pub trait Sequence: Send {
    /// Return the FOV of the Sequence, if available
    fn fov(&self) -> Option<(f32, f32, f32)>;

    /// Duration of the MRI sequence: no samples, blocks, etc. exist outside
    /// of the time range [0, duration()]
    fn duration(&self) -> f32;

    /// Returns the next time range of the next block of the given type.
    /// If `t_start` is inside of a block, this block is not returned: only
    /// blocks *starting* after `t_start` are considered.
    fn next_block(&self, t_start: f32, ty: EventType) -> Option<(f32, f32)>;

    /// Returns the next Point of Interest. The internal structure of the sequence is
    /// intentionally hidden, which might be a bit annoying but means that applications
    /// using disseqt will work with any sequence, even if file formats update or
    /// additional file formats are implemented etc.
    /// A POI is a point where the given event type changes - this is usually _in between_
    /// samples - so you want to either integrate from one POI to the next or sample
    /// exactly between two (or do multiple samples if they are too far apart).
    /// For continuously changing things (maybe we support analytical definitions in the
    /// future?) the next POI might always equal t_start, so you should not try to always
    /// handle every single POI.
    fn next_poi(&self, t_start: f32, ty: EventType) -> Option<f32>;

    /// Return all POIs in the given time range. This is an often used operation,
    /// and this function allows for a more efficient backend implementation.
    fn pois(&self, time_range: &dyn TimeRange, ty: EventType) -> Vec<f32> {
        // NOTE: The indirection by using a trait object seems to be neglectable in terms of
        // performance, although it makes the API a bit worse, as the time range that is
        // usually only constructed for the function call now needs a reference.
        let mut t = time_range.start();
        let mut pois = Vec::new();
        while let Some(t_next) = self.next_poi(t, ty) {
            if !time_range.contains(t_next) {
                break;
            }
            pois.push(t_next);
            t = t_next + 1e-6;
        }

        pois
    }

    // NOTE: This is probably the final signature we want, no other functions than this
    fn get_POIs(&self, t_start: f32, t_end: f32, max_count: usize) -> Vec<f32> {
        // We only implement this function in the (pulseq or other) backend, the wrapper
        // that is accessible to the user then provides a get_POIs(range: impl RangeBounds, max_count: Option<usize>)
        // and a get_POI(t_start) { get_POIs(t_start, INFINITY, usize::MAX)} function.
        // -> But look into if this function signature can be implemented efficiently.
        // It is okay if the single-POI function has bit worse performance bc. of not being implented directly,
        // (e.g. bc. of the Vec return type), because getting multiple POIs at once will always be more performant.
        todo!()
    }

    // Length of returned moments will be time.len() - 1
    fn integrate_n(&self, time: &[f32]) -> Vec<Moment> {
        let mut moments = Vec::new();
        for t in time.windows(2) {
            let (pulse, gradient) = self.integrate(t[0], t[1]);
            moments.push(Moment { pulse, gradient });
        }
        moments
    }

    fn sample_n(&self, time: &[f32]) -> Vec<Sample> {
        time.into_iter().map(|t| {
            let (pulse, gradient, adc) = self.sample(*t);
            Sample {
                pulse,
                gradient,
                adc,
            }
        }).collect()
    }

    /// Calculate the pulse and gradient moment for a given time range.
    /// # Panics
    /// If `t_start >= t_end`
    fn integrate(&self, t_start: f32, t_end: f32) -> (PulseMoment, GradientMoment);

    /// Returns the amplitudes and phases that are applied at time point `t`.
    fn sample(&self, t: f32) -> (PulseSample, GradientSample, AdcBlockSample);
}

/// For whatever reason, std::ops::RangeBounds is not object save, so we use our own
/// trait which just wraps RangeBounds. (The reason is probably is the ?Sized bound on T)
pub trait TimeRange {
    fn start(&self) -> f32;
    fn contains(&self, t: f32) -> bool;
}

impl<R: RangeBounds<f32>> TimeRange for R {
    fn start(&self) -> f32 {
        // We don't differentiate between included and excluded, as for the
        // intended use case with f32 time ranges, you can't really exclude the start.
        // In addition, the Rust ranges never exclude the start, so excluded should not match.
        match self.start_bound() {
            std::ops::Bound::Included(t) => *t,
            std::ops::Bound::Excluded(t) => *t,
            std::ops::Bound::Unbounded => f32::NEG_INFINITY,
        }
    }
    fn contains(&self, t: f32) -> bool {
        self.contains(&t)
    }
}
