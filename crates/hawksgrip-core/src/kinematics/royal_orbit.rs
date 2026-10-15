//! Kindah's orbit: a constant-speed horizontal circle around the refinery.
//!
//! Her state is a closed-form function of time, not an integrated one, so no
//! code path can perturb it and no numerical error can accumulate. Whether
//! the orbit is flyable (turn rate V/R and centripetal acceleration V^2/R
//! within her limits) is checked when the configuration is loaded.

use crate::kinematics::KState;
use crate::vec3::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoyalOrbit {
    pub center: Vec3,
    pub radius: f64,
    /// Signed angular rate (rad/s); negative is clockwise seen from above.
    pub omega: f64,
    pub phase0: f64,
}

impl RoyalOrbit {
    /// `center.z` is the absolute orbit altitude.
    pub fn new(center: Vec3, radius: f64, speed: f64, phase0: f64, clockwise: bool) -> Self {
        let w = speed / radius;
        Self {
            center,
            radius,
            omega: if clockwise { -w } else { w },
            phase0,
        }
    }

    pub fn state_at(&self, t: f64) -> KState {
        let th = self.phase0 + self.omega * t;
        let (s, c) = crate::math::sin_cos(th);
        let r = self.radius;
        let w = self.omega;
        KState {
            p: self.center + Vec3::new(r * c, r * s, 0.0),
            v: Vec3::new(-r * w * s, r * w * c, 0.0),
            a: Vec3::new(-r * w * w * c, -r * w * w * s, 0.0),
        }
    }

    pub fn period_s(&self) -> f64 {
        std::f64::consts::TAU / self.omega.abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_speed_radius_and_altitude() {
        let o = RoyalOrbit::new(Vec3::new(0.0, 0.0, 1500.0), 3000.0, 25.0, 0.3, false);
        for i in 0..1000 {
            let s = o.state_at(i as f64 * 1.7);
            assert!(((s.p - o.center).norm() - 3000.0).abs() < 1e-6);
            assert!((s.v.norm() - 25.0).abs() < 1e-9);
            assert!((s.a.norm() - 25.0 * 25.0 / 3000.0).abs() < 1e-12);
            assert_eq!(s.p.z, 1500.0);
        }
    }

    #[test]
    fn velocity_is_derivative_of_position() {
        let o = RoyalOrbit::new(Vec3::ZERO, 3000.0, 25.0, 0.0, true);
        let h = 1e-4;
        let fd = (o.state_at(10.0 + h).p - o.state_at(10.0 - h).p) / (2.0 * h);
        assert!((fd - o.state_at(10.0).v).norm() < 1e-6);
    }
}
