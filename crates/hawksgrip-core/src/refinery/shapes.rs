//! Primitive solids for refinery structures and keep-out volumes: vertical
//! cylinders (tanks, columns, flare stacks) and axis-aligned boxes (process
//! units, buildings). Each supports point containment, an exact signed
//! distance, and segment intersection.

use crate::vec3::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum Solid {
    /// Vertical cylinder: axis at (cx, cy), from z0 up to z1.
    Cylinder {
        cx: f64,
        cy: f64,
        r: f64,
        z0: f64,
        z1: f64,
    },
    /// Axis-aligned box.
    Box { min: Vec3, max: Vec3 },
}

impl Solid {
    /// The same solid grown by `m` metres in every direction (sides and top).
    /// Downward growth is not needed: everything stands on the ground.
    pub fn inflate(&self, m: f64) -> Solid {
        match *self {
            Solid::Cylinder { cx, cy, r, z0, z1 } => Solid::Cylinder {
                cx,
                cy,
                r: r + m,
                z0,
                z1: z1 + m,
            },
            Solid::Box { min, max } => Solid::Box {
                min: Vec3::new(min.x - m, min.y - m, min.z),
                max: Vec3::new(max.x + m, max.y + m, max.z + m),
            },
        }
    }

    /// Exact Euclidean signed distance: negative inside, positive outside.
    pub fn signed_distance(&self, p: Vec3) -> f64 {
        match *self {
            Solid::Cylinder { cx, cy, r, z0, z1 } => {
                let dr = ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt() - r;
                let dz = (z0 - p.z).max(p.z - z1);
                let outside = (dr.max(0.0).powi(2) + dz.max(0.0).powi(2)).sqrt();
                outside + dr.max(dz).min(0.0)
            }
            Solid::Box { min, max } => {
                let c = (min + max) * 0.5;
                let h = (max - min) * 0.5;
                let q = Vec3::new(
                    (p.x - c.x).abs() - h.x,
                    (p.y - c.y).abs() - h.y,
                    (p.z - c.z).abs() - h.z,
                );
                let outside = Vec3::new(q.x.max(0.0), q.y.max(0.0), q.z.max(0.0)).norm();
                outside + q.x.max(q.y).max(q.z).min(0.0)
            }
        }
    }

    pub fn contains(&self, p: Vec3) -> bool {
        self.signed_distance(p) <= 0.0
    }

    /// Does the closed segment a→b touch the solid?
    pub fn intersects_segment(&self, a: Vec3, b: Vec3) -> bool {
        let d = b - a;
        // Parameter interval [t0, t1] within [0, 1] where the segment is inside.
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut slab = |p: f64, dp: f64, lo: f64, hi: f64| -> bool {
            if dp.abs() < 1e-12 {
                return p >= lo && p <= hi;
            }
            let (mut ta, mut tb) = ((lo - p) / dp, (hi - p) / dp);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            t0 = t0.max(ta);
            t1 = t1.min(tb);
            t0 <= t1
        };
        match *self {
            Solid::Box { min, max } => {
                slab(a.x, d.x, min.x, max.x)
                    && slab(a.y, d.y, min.y, max.y)
                    && slab(a.z, d.z, min.z, max.z)
            }
            Solid::Cylinder { cx, cy, r, z0, z1 } => {
                if !slab(a.z, d.z, z0, z1) {
                    return false;
                }
                // Horizontal circle: |(a_xy - c) + t d_xy|^2 <= r^2.
                let (ox, oy) = (a.x - cx, a.y - cy);
                let qa = d.x * d.x + d.y * d.y;
                let qb = 2.0 * (ox * d.x + oy * d.y);
                let qc = ox * ox + oy * oy - r * r;
                if qa < 1e-12 {
                    return qc <= 0.0 && t0 <= t1;
                }
                let disc = qb * qb - 4.0 * qa * qc;
                if disc < 0.0 {
                    return false;
                }
                let sq = disc.sqrt();
                let (ta, tb) = ((-qb - sq) / (2.0 * qa), (-qb + sq) / (2.0 * qa));
                t0.max(ta) <= t1.min(tb)
            }
        }
    }

    pub fn top(&self) -> f64 {
        match *self {
            Solid::Cylinder { z1, .. } => z1,
            Solid::Box { max, .. } => max.z,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::rng::Rng;

    fn cyl() -> Solid {
        Solid::Cylinder {
            cx: 10.0,
            cy: -5.0,
            r: 4.0,
            z0: 0.0,
            z1: 20.0,
        }
    }

    fn bx() -> Solid {
        Solid::Box {
            min: Vec3::new(-3.0, -2.0, 0.0),
            max: Vec3::new(5.0, 2.0, 6.0),
        }
    }

    #[test]
    fn known_distances() {
        let c = cyl();
        assert!((c.signed_distance(Vec3::new(20.0, -5.0, 10.0)) - 6.0).abs() < 1e-12);
        assert!((c.signed_distance(Vec3::new(10.0, -5.0, 10.0)) + 4.0).abs() < 1e-12);
        assert!((c.signed_distance(Vec3::new(10.0, -5.0, 23.0)) - 3.0).abs() < 1e-12);
        // Corner region: distance to the rim edge.
        assert!((c.signed_distance(Vec3::new(17.0, -5.0, 24.0)) - 5.0).abs() < 1e-12);
        let b = bx();
        assert!((b.signed_distance(Vec3::new(1.0, 0.0, 3.0)) + 2.0).abs() < 1e-12);
        assert!((b.signed_distance(Vec3::new(8.0, 6.0, 3.0)) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn distance_is_1_lipschitz_and_matches_containment() {
        let mut rng = Rng::from_seed(3);
        for s in [cyl(), bx(), cyl().inflate(7.0), bx().inflate(2.0)] {
            for _ in 0..20_000 {
                let p = Vec3::new(
                    rng.uniform(-20.0, 30.0),
                    rng.uniform(-20.0, 20.0),
                    rng.uniform(-5.0, 35.0),
                );
                let q = p + Vec3::new(
                    rng.uniform(-1.0, 1.0),
                    rng.uniform(-1.0, 1.0),
                    rng.uniform(-1.0, 1.0),
                );
                assert!(
                    (s.signed_distance(p) - s.signed_distance(q)).abs() <= (p - q).norm() + 1e-9
                );
                assert_eq!(s.contains(p), s.signed_distance(p) <= 0.0);
            }
        }
    }

    #[test]
    fn segment_intersection_matches_dense_sampling() {
        let mut rng = Rng::from_seed(4);
        for s in [cyl(), bx()] {
            for _ in 0..5_000 {
                let a = Vec3::new(
                    rng.uniform(-20.0, 30.0),
                    rng.uniform(-20.0, 20.0),
                    rng.uniform(-5.0, 30.0),
                );
                let b = Vec3::new(
                    rng.uniform(-20.0, 30.0),
                    rng.uniform(-20.0, 20.0),
                    rng.uniform(-5.0, 30.0),
                );
                let exact = s.intersects_segment(a, b);
                let min_sd = (0..=4000)
                    .map(|k| s.signed_distance(a + (b - a) * (k as f64 / 4000.0)))
                    .fold(f64::INFINITY, f64::min);
                if min_sd < -0.05 {
                    assert!(exact, "missed a penetrating segment");
                }
                if min_sd > 0.05 {
                    assert!(!exact, "false hit, min distance {min_sd}");
                }
            }
        }
    }

    #[test]
    fn inflation_grows_distance_field_by_margin() {
        let p = Vec3::new(30.0, -5.0, 10.0);
        assert!(
            (cyl().signed_distance(p) - cyl().inflate(5.0).signed_distance(p) - 5.0).abs() < 1e-12
        );
    }
}
