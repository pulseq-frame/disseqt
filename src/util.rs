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
pub enum Rotation {
    Identity,
    Simple { angle: f64, phase: f64 },
    Complex { dir: [f64; 3] },
}

impl Rotation {
    pub fn identity() -> Self {
        Self::Identity
    }

    pub fn angle(&self) -> f64 {
        match self {
            Rotation::Identity => 0.0,
            Rotation::Simple { angle, .. } => *angle,
            Rotation::Complex { dir: spin } => {
                // Normalize because error can build up during rotations
                let norm = (spin[0] * spin[0] + spin[1] * spin[1] + spin[2] * spin[2]).sqrt();
                (spin[2] / norm).acos()
            }
        }
    }

    pub fn phase(&self) -> f64 {
        match self {
            Rotation::Identity => 0.0,
            Rotation::Simple { phase, .. } => *phase,
            Rotation::Complex { dir: spin } => {
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

    pub fn rotate(&self, rot: Self) -> Self {
        match (*self, rot) {
            (Rotation::Identity, rot) | (rot, Rotation::Identity) => rot,
            (
                Rotation::Simple {
                    angle: a1,
                    phase: p1,
                },
                Rotation::Simple {
                    angle: a2,
                    phase: p2,
                },
            ) if p1.abs() == p2.abs() => {
                // TODO: proper sign logic!
                Rotation::Simple {
                    angle: a1 + p2.signum() * a2,
                    phase: p1,
                }
            }
            _ => Rotation::Complex {
                dir: rot.as_rot_mat().rotate(self.as_dir_vec()),
            },
        }
    }

    fn as_rot_mat(&self) -> RotMat {
        match self {
            Rotation::Identity => RotMat::new(0.0, 0.0),
            Rotation::Simple { angle, phase } => RotMat::new(*angle, *phase),
            Rotation::Complex { .. } => RotMat::new(self.angle(), self.phase()),
        }
    }

    fn as_dir_vec(&self) -> [f64; 3] {
        match self {
            Rotation::Identity => [0.0, 0.0, 1.0],
            Rotation::Simple { angle, phase } => {
                RotMat::new(*angle, *phase).rotate([0.0, 0.0, 1.0])
            }
            Rotation::Complex { dir } => *dir,
        }
    }
}

pub struct RotMat([[f64; 3]; 3]);

impl RotMat {
    pub fn new(angle: f64, phase: f64) -> Self {
        let angle = angle as f64;
        let phase = phase as f64;
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

    pub fn rotate(&self, dir: [f64; 3]) -> [f64; 3] {
        let x = self.0[0][0] * dir[0] + self.0[0][1] * dir[1] + self.0[0][2] * dir[2];
        let y = self.0[1][0] * dir[0] + self.0[1][1] * dir[1] + self.0[1][2] * dir[2];
        let z = self.0[2][0] * dir[0] + self.0[2][1] * dir[1] + self.0[2][2] * dir[2];
        [x, y, z]
    }
}


#[cfg(test)]
mod tests {
    use super::{Rotation};
    use assert2::check;

    #[test]
    fn random_rot() {
        for _ in 0..1000 {
            let angle = rand::random::<f64>() * std::f64::consts::PI;
            let phase = rand::random::<f64>() * std::f64::consts::TAU;

            let mut rot = Rotation::identity();
            rot = rot.rotate(Rotation::Simple { angle, phase });

            check!((rot.angle() - angle).abs() < 1e-6);
            check!((rot.phase() - phase).abs() < 1e-6);
        }
    }

    #[test]
    fn random_multi_rot() {
        for _ in 0..1000 {
            let angle = rand::random::<f64>() * std::f64::consts::PI;
            let phase = rand::random::<f64>() * std::f64::consts::TAU;

            let mut rot = Rotation::Identity;
            let subsamples = rand::random::<u32>() % 100 + 1;
            for _ in 0..subsamples {
                rot = rot.rotate(Rotation::Simple { angle: angle / subsamples as f64, phase });
            }

            check!((rot.angle() - angle).abs() < 1e-6);
            check!((rot.phase() - phase).abs() < 1e-6);
        }
    }
}
