//! Minimal f64 3-vector in ENU metres (x = East, y = North, z = Up).
//!
//! Hand-rolled instead of pulling in a linear-algebra crate so every
//! floating-point operation in the hot path is visible and in a fixed order.

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "[f64; 3]", into = "[f64; 3]")]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn norm_sq(self) -> f64 {
        self.dot(self)
    }

    pub fn norm(self) -> f64 {
        self.norm_sq().sqrt()
    }

    /// Unit vector, or zero for a (near-)zero input.
    pub fn normalized_or_zero(self) -> Vec3 {
        let n = self.norm();
        if n > 1e-12 { self / n } else { Vec3::ZERO }
    }

    /// Scale down to `max` length if longer; never scales up.
    /// This is the Euclidean projection onto a ball, which is non-expansive.
    pub fn clamp_norm(self, max: f64) -> Vec3 {
        let n = self.norm();
        if n > max && n > 0.0 {
            self * (max / n)
        } else {
            self
        }
    }

    pub fn horizontal(self) -> Vec3 {
        Vec3::new(self.x, self.y, 0.0)
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn to_array(self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }
}

impl From<[f64; 3]> for Vec3 {
    fn from(a: [f64; 3]) -> Self {
        Vec3::new(a[0], a[1], a[2])
    }
}

impl From<Vec3> for [f64; 3] {
    fn from(v: Vec3) -> Self {
        v.to_array()
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f64) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }
}

impl Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, k: f64) -> Vec3 {
        Vec3::new(self.x / k, self.y / k, self.z / k)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

impl AddAssign for Vec3 {
    fn add_assign(&mut self, o: Vec3) {
        *self = *self + o;
    }
}

impl SubAssign for Vec3 {
    fn sub_assign(&mut self, o: Vec3) {
        *self = *self - o;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_norm_only_shrinks() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert_eq!(v.clamp_norm(10.0), v);
        let c = v.clamp_norm(1.0);
        assert!((c.norm() - 1.0).abs() < 1e-15);
        assert!((c.x - 0.6).abs() < 1e-15);
    }

    #[test]
    fn serde_as_array() {
        let v = Vec3::new(1.0, 2.0, 3.0);
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "[1.0,2.0,3.0]");
        let back: Vec3 = serde_json::from_str(&s).unwrap();
        assert_eq!(back, v);
    }
}
