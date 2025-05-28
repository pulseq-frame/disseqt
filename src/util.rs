use std::ops::MulAssign;

pub struct Spin([f64; 3]);

impl Spin {
    pub fn relaxed() -> Self {
        Self([0.0, 0.0, 1.0])
    }

    pub fn angle(&self) -> f64 {
        // Normalize because error can build up during rotations
        (self.0[2] / self.norm()).acos() as f64
    }

    pub fn phase(&self) -> f64 {
        // We want the phase of the applied rotation, not of the spin itself
        let tmp = f64::atan2(self.0[1], self.0[0]) as f64 + std::f64::consts::FRAC_PI_2;
        // Map to the range [0, 2*pi]
        if tmp < 0.0 {
            tmp + std::f64::consts::TAU
        } else {
            tmp
        }
    }

    fn norm(&self) -> f64 {
        (self.0[0] * self.0[0] + self.0[1] * self.0[1] + self.0[2] * self.0[2]).sqrt()
    }
}

pub struct Rotation([[f64; 3]; 3]);

impl Rotation {
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
}

impl MulAssign<Rotation> for Spin {
    fn mul_assign(&mut self, rhs: Rotation) {
        let x = rhs.0[0][0] * self.0[0] + rhs.0[0][1] * self.0[1] + rhs.0[0][2] * self.0[2];
        let y = rhs.0[1][0] * self.0[0] + rhs.0[1][1] * self.0[1] + rhs.0[1][2] * self.0[2];
        let z = rhs.0[2][0] * self.0[0] + rhs.0[2][1] * self.0[1] + rhs.0[2][2] * self.0[2];
        self.0 = [x, y, z];
    }
}

#[cfg(test)]
mod tests {
    use super::{Rotation, Spin};
    use assert2::check;

    #[test]
    fn random_rot() {
        for _ in 0..1000 {
            let angle = rand::random::<f64>() * std::f64::consts::PI;
            let phase = rand::random::<f64>() * std::f64::consts::TAU;

            let mut spin = Spin::relaxed();
            spin *= Rotation::new(angle, phase);

            check!((spin.angle() - angle).abs() < 1e-6);
            check!((spin.phase() - phase).abs() < 1e-6);
        }
    }

    #[test]
    fn random_multi_rot() {
        for _ in 0..1000 {
            let angle = rand::random::<f64>() * std::f64::consts::PI;
            let phase = rand::random::<f64>() * std::f64::consts::TAU;

            let mut spin = Spin::relaxed();
            let subsamples = rand::random::<u32>() % 100 + 1;
            for _ in 0..subsamples {
                spin *= Rotation::new(angle / subsamples as f64, phase);
            }

            check!((spin.angle() - angle).abs() < 1e-6);
            check!((spin.phase() - phase).abs() < 1e-6);
        }
    }
}
