use pulseq_rs::{Gradient, Rf, Shape};

use crate::util::{Rotation, Spin};

pub fn integrate_grad(
    gx: &Gradient,
    t_start: f32,
    t_end: f32,
    block_start: f32,
    grad_raster: f32,
) -> f32 {
    match gx {
        Gradient::Free { amp, delay, shape } => {
            amp * integrate_free(
                t_start - block_start - delay,
                t_end - block_start - delay,
                shape,
                grad_raster,
            )
        }
        Gradient::Trap {
            amp,
            rise,
            flat,
            fall,
            delay,
        } => {
            amp * integrate_trap(
                t_start - block_start - delay,
                t_end - block_start - delay,
                *rise,
                *flat,
                *fall,
            )
        }
    }
}

// TODO: change spin + rotation matrix to a unified rotation struct (matrix or quaternion etc.)
// that is returned from this function
pub fn integrate_rf(
    rf: &Rf,
    spin: &mut Spin,
    t_start: f32,
    t_end: f32,
    block_start: f32,
    rf_raster: f32,
) {
    for i in 0..rf.amp_shape.0.len() {
        let dwell = rf_raster;
        // Start time of the sample number i
        let t = block_start + rf.delay + i as f32 * dwell;

        // Skip samples before t_start, quit when reaching t_end
        if t + dwell < t_start {
            continue;
        }
        if t_end <= t {
            break;
        }

        // We could do the clamping for all samples, but when integrating
        // over many samples, it seems to be very sensitive to accumulating
        // errors. Only doing it in the edge cases is much more robust.
        let dur = if t_start <= t && t + dwell <= t_end {
            dwell
        } else {
            // Clamp the sample intervall to the integration intervall
            let t0 = f32::max(t_start, t);
            let t1 = f32::min(t_end, t + dwell);
            t1 - t0
        };

        *spin *= Rotation::new(
            rf.amp * rf.amp_shape.0[i] * dur * std::f32::consts::TAU,
            rf.phase + rf.phase_shape.0[i] * std::f32::consts::TAU,
        );
    }
}

pub fn sample_grad(t: f32, grad: &Gradient, grad_raster: f32) -> f32 {
    match grad {
        pulseq_rs::Gradient::Free { amp, delay, shape } => {
            let index = ((t - delay) / grad_raster - 0.5).ceil() as usize;
            shape.0.get(index).map_or(0.0, |x| amp * x)
        }
        pulseq_rs::Gradient::Trap {
            amp,
            rise,
            flat,
            fall,
            delay,
        } => amp * trap_sample(t - delay, *rise, *flat, *fall),
    }
}

pub fn trap_sample(t: f32, rise: f32, flat: f32, fall: f32) -> f32 {
    if t < 0.0 {
        0.0
    } else if t < rise {
        t / rise
    } else if t < rise + flat {
        1.0
    } else if t < rise + flat + fall {
        ((rise + flat + fall) - t) / fall
    } else {
        0.0
    }
}

pub fn integrate_trap(t_start: f32, t_end: f32, rise: f32, flat: f32, fall: f32) -> f32 {
    let integral = |t| {
        if t <= rise {
            0.5 * t * t / rise
        } else if t <= rise + flat {
            (0.5 * rise) + (t - rise)
        } else {
            let rev_t = rise + flat + fall - t;
            (0.5 * rise) + flat + (0.5 * (fall - rev_t * rev_t / fall))
        }
    };
    integral(t_end.min(rise + flat + fall)) - integral(t_start.max(0.0))
}

pub fn integrate_free(t_start: f32, t_end: f32, shape: &Shape, dwell: f32) -> f32 {
    let mut integrated = 0.0;

    for i in 0..shape.0.len() {
        // Start time of the sample number i
        let t = i as f32 * dwell;

        // Skip samples before t_start, quit when reaching t_end
        if t + dwell <= t_start {
            continue;
        }
        if t_end <= t {
            break;
        }

        // We could do the clamping for all samples, but when integrating
        // over many samples, it seems to be very sensitive to accumulating
        // errors. Only doing it in the edge cases is much more robust.
        let dur = if t_start <= t && t + dwell <= t_end {
            dwell
        } else {
            // Clamp the sample intervall to the integration intervall
            let t0 = f32::max(t_start, t);
            let t1 = f32::min(t_end, t + dwell);
            t1 - t0
        };

        integrated += shape.0[i] * dur;
    }

    integrated
}
