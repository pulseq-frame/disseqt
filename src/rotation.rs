/// For simple pulses, we can assume a constant rotation axis and only need to
/// track the amplitude of rotation. This allows us to detect rotations > 180°,
/// useful for off-resonant pulses (Bloch-Siegert), saturation pulses etc.
///
/// For complex pulses, we can only try to execute the whole rotation and look
/// at the direction a spin points afterwards. This lets us know the smallest
/// possible rotation to achieve the effect of the pulse (ignore wrap arounds).
///
/// This struct tries to do the former and switches to the latter if the phase
/// of the pulse (rotation axis) turns out to be not constant.
#[derive(Clone, Copy, Debug)]
pub struct Rotation(RotRepr);

#[derive(Clone, Copy, Debug)]
enum RotRepr {
    Identity,
    Simple { angle: f64, phase: f64 },
    Complex { dir: [f64; 3] },
}

impl Rotation {
    pub fn new() -> Self {
        Self(RotRepr::Identity)
    }

    pub fn angle(&self) -> f64 {
        match self.0 {
            RotRepr::Identity => 0.0,
            RotRepr::Simple { angle, .. } => angle,
            RotRepr::Complex { dir: spin } => {
                // Normalize because error can build up during rotations
                let norm = (spin[0] * spin[0] + spin[1] * spin[1] + spin[2] * spin[2]).sqrt();
                (spin[2] / norm).acos()
            }
        }
    }

    pub fn phase(&self) -> f64 {
        match self.0 {
            RotRepr::Identity => 0.0,
            RotRepr::Simple { phase, .. } => phase,
            RotRepr::Complex { dir: spin } => {
                // We want the phase of the applied rotation, not of the spin itself
                let tmp = f64::atan2(spin[1], spin[0]) + std::f64::consts::FRAC_PI_2;
                // Map to the range [0, 2*pi]
                if tmp < 0.0 {
                    tmp + std::f64::consts::TAU
                } else {
                    tmp
                }
            }
        }
    }

    pub fn rotate(&self, angle: f64, phase: f64) -> Self {
        self.then(Self(RotRepr::Simple { angle, phase }))
    }

    pub fn then(&self, rot: Self) -> Self {
        Self(match (self.0, rot.0) {
            (RotRepr::Identity, rot) | (rot, RotRepr::Identity) => rot,
            (
                RotRepr::Simple {
                    angle: a1,
                    phase: p1,
                },
                RotRepr::Simple {
                    angle: a2,
                    phase: p2,
                },
            ) if p1.abs() == p2.abs() => {
                // TODO: proper sign logic!
                RotRepr::Simple {
                    angle: a1 + p2.signum() * a2,
                    phase: p1,
                }
            }
            _ => complex_rotation(self.0, rot.0),
        })
    }
}

/// If rotations are joined that differ in their rotation matrix, we can't
/// track the accumulated angle anymore but actually have to join them and see
/// what the final rotation looks like. We do this by applying them to a vector
/// and see where it points at afterwards. This is done in this function.
fn complex_rotation(first: RotRepr, second: RotRepr) -> RotRepr {
    let vec_rotated_by_first = match first {
        RotRepr::Identity => [0.0, 0.0, 1.0],
        RotRepr::Simple { angle, phase } => RotMat::new(angle, phase).apply([0.0, 0.0, 1.0]),
        RotRepr::Complex { dir } => dir,
    };

    let rot_mat_representing_second = match second {
        RotRepr::Identity => RotMat::new(0.0, 0.0),
        RotRepr::Simple { angle, phase } => RotMat::new(angle, phase),
        RotRepr::Complex { .. } => RotMat::new(Rotation(second).angle(), Rotation(second).phase()),
    };

    RotRepr::Complex {
        dir: rot_mat_representing_second.apply(vec_rotated_by_first),
    }
}

/// Helper for applying a rotation (angle, phase) to a vector
struct RotMat([[f64; 3]; 3]);

impl RotMat {
    fn new(angle: f64, phase: f64) -> Self {
        Self([
            [
                angle.cos() * phase.sin().powi(2) + phase.cos().powi(2),
                (1.0 - angle.cos()) * phase.sin() * phase.cos(),
                angle.sin() * phase.sin(),
            ],
            [
                (1.0 - angle.cos()) * phase.sin() * phase.cos(),
                angle.cos() * phase.cos().powi(2) + phase.sin().powi(2),
                -angle.sin() * phase.cos(),
            ],
            [
                -angle.sin() * phase.sin(),
                angle.sin() * phase.cos(),
                angle.cos(),
            ],
        ])
    }

    fn apply(&self, dir: [f64; 3]) -> [f64; 3] {
        let x = self.0[0][0] * dir[0] + self.0[0][1] * dir[1] + self.0[0][2] * dir[2];
        let y = self.0[1][0] * dir[0] + self.0[1][1] * dir[1] + self.0[1][2] * dir[2];
        let z = self.0[2][0] * dir[0] + self.0[2][1] * dir[1] + self.0[2][2] * dir[2];
        [x, y, z]
    }
}

#[cfg(test)]
mod tests {
    use super::Rotation;
    use assert2::check;

    #[test]
    fn random_rot() {
        for _ in 0..1000 {
            let angle = rand::random::<f64>() * std::f64::consts::PI;
            let phase = rand::random::<f64>() * std::f64::consts::TAU;

            let mut rot = Rotation::new();
            rot = rot.rotate(angle, phase);

            check!((rot.angle() - angle).abs() < 1e-6);
            check!((rot.phase() - phase).abs() < 1e-6);
        }
    }

    #[test]
    fn random_multi_rot() {
        for _ in 0..1000 {
            let angle = rand::random::<f64>() * std::f64::consts::PI;
            let phase = rand::random::<f64>() * std::f64::consts::TAU;

            let mut rot = Rotation::new();
            let subsamples = rand::random::<u32>() % 100 + 1;
            for _ in 0..subsamples {
                rot = rot.rotate(angle / subsamples as f64, phase);
            }

            check!((rot.angle() - angle).abs() < 1e-6);
            check!((rot.phase() - phase).abs() < 1e-6);
        }
    }
}
