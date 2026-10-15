//! Authoritative terrain line-of-sight (ADR-0003, ADR-0010).
//!
//! **Algorithm: Lipschitz ray-marching** (sphere tracing for heightfields).
//! Parametrise the segment by horizontal distance s, with ray height
//! z(s) = z0 + m s. Clearance c(s) = z(s) - h(x(s), y(s)) changes at most at
//! rate |m| + L, where L bounds |grad h| (heightfield.rs). So from a point
//! with clearance c > 0, the ray cannot touch the ground before
//! s + c / (|m| + L). Stepping exactly that far is **safe**: it can never step
//! over a ridge.
//!
//! To guarantee clearance >= eps (not just > 0) along the whole segment, the
//! step is (c - eps) / (|m| + L), and the march stops with "occluded" when
//! c < 2 eps. Result:
//! - **visible ⇒ clearance ≥ eps everywhere** (sound: never sees through ground);
//! - **occluded ⇒ clearance < 2 eps somewhere** (grazing rays count as blocked).
//!
//! Far above the terrain the steps are long; near it they shrink. The cost
//! adapts to the geometry instead of sampling every 5 m.

use crate::terrain::Heightfield;
use crate::vec3::Vec3;

/// Grazing tolerance: closer than this to the ground counts as occluded.
pub const DEFAULT_EPS_M: f64 = 0.5;

/// Points within this horizontal distance of the end are not tested, so a
/// target sitting on the ground is not "occluded by itself".
const END_TOLERANCE_M: f64 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LosStats {
    pub steps: u32,
}

/// True if the segment a→b clears the terrain by at least `eps` everywhere.
/// False means some point is closer than 2 eps (or below ground).
pub fn visible(hf: &Heightfield, a: Vec3, b: Vec3, eps: f64) -> bool {
    visible_with_stats(hf, a, b, eps).0
}

pub fn visible_with_stats(hf: &Heightfield, a: Vec3, b: Vec3, eps: f64) -> (bool, LosStats) {
    let d = b - a;
    let len_h = d.horizontal().norm();
    let mut st = LosStats::default();
    if len_h < END_TOLERANCE_M {
        // (Near-)vertical: visible iff the lower end is above the ground.
        let low = if a.z < b.z { a } else { b };
        return (low.z - hf.height(low.x, low.y) >= eps, st);
    }
    let (ux, uy) = (d.x / len_h, d.y / len_h);
    let m = d.z / len_h;
    let rate = m.abs() + hf.lipschitz();
    let s_end = len_h - END_TOLERANCE_M;
    // Every step is at least eps / rate long (c >= 2 eps whenever we step).
    let max_steps = (s_end / (eps / rate.max(1e-9))).ceil() as u64 + 16;
    let mut s = 0.0;
    for _ in 0..max_steps {
        let (x, y, z) = (a.x + ux * s, a.y + uy * s, a.z + m * s);
        let c = z - hf.height(x, y);
        st.steps += 1;
        if c < 2.0 * eps {
            return (false, st);
        }
        if rate <= 0.0 {
            return (true, st); // flat ground, level ray, already clear
        }
        s += (c - eps) / rate;
        if s >= s_end {
            return (true, st);
        }
    }
    (false, st) // unreachable in exact arithmetic; conservative if it happens
}

/// Brute-force reference: sample every `step` metres. Used only by tests and
/// for cross-validating the GPU (ADR-0003).
pub fn visible_reference(hf: &Heightfield, a: Vec3, b: Vec3, eps: f64, step: f64) -> (bool, f64) {
    let d = b - a;
    let len_h = d.horizontal().norm();
    let s_end = (len_h - END_TOLERANCE_M).max(0.0);
    let n = (s_end / step).ceil() as usize;
    let mut min_c = f64::INFINITY;
    for k in 0..=n {
        let s = (k as f64 * step).min(s_end);
        let p = a + d * (s / len_h.max(1e-12));
        min_c = min_c.min(p.z - hf.height(p.x, p.y));
    }
    (min_c >= eps, min_c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::rng::Rng;

    /// A single sharp ridge 100 m high along x = 500.
    fn ridge() -> Heightfield {
        let (nx, ny, cell) = (101, 11, 10.0);
        let mut d = vec![0f32; nx * ny];
        for iy in 0..ny {
            for ix in 0..nx {
                let x = ix as f64 * cell;
                d[iy * nx + ix] = (100.0 - (x - 500.0).abs()).max(0.0) as f32;
            }
        }
        Heightfield::new(nx, ny, cell, 0.0, 0.0, d).unwrap()
    }

    #[test]
    fn flat_ground_is_transparent() {
        let hf = Heightfield::flat(1000.0);
        assert!(visible(
            &hf,
            Vec3::new(-900.0, 0.0, 10.0),
            Vec3::new(900.0, 30.0, 2.0),
            0.5
        ));
        assert!(!visible(
            &hf,
            Vec3::new(-900.0, 0.0, 10.0),
            Vec3::new(900.0, 0.0, -5.0),
            0.5
        ));
    }

    #[test]
    fn ridge_blocks_low_rays_and_passes_high_ones() {
        let hf = ridge();
        let a = Vec3::new(100.0, 50.0, 20.0);
        assert!(!visible(&hf, a, Vec3::new(900.0, 50.0, 20.0), 0.5));
        assert!(visible(&hf, a, Vec3::new(900.0, 50.0, 200.0), 0.5));
        // Cresting with 0.3 m to spare: below eps, so it must be blocked.
        assert!(!visible(
            &hf,
            Vec3::new(100.0, 50.0, 100.3),
            Vec3::new(900.0, 50.0, 100.3),
            0.5
        ));
        // Exactly cresting: blocked.
        assert!(!visible(
            &hf,
            Vec3::new(100.0, 50.0, 100.0),
            Vec3::new(900.0, 50.0, 100.0),
            0.5
        ));
    }

    #[test]
    fn target_on_the_ground_is_not_self_occluded() {
        let hf = Heightfield::flat(1000.0);
        assert!(visible(
            &hf,
            Vec3::new(0.0, 0.0, 500.0),
            Vec3::new(300.0, 0.0, 0.0),
            0.5
        ));
    }

    #[test]
    fn vertical_rays() {
        let hf = ridge();
        assert!(visible(
            &hf,
            Vec3::new(500.0, 50.0, 300.0),
            Vec3::new(500.0, 50.0, 150.0),
            0.5
        ));
        assert!(!visible(
            &hf,
            Vec3::new(500.0, 50.0, 300.0),
            Vec3::new(500.0, 50.0, 50.0),
            0.5
        ));
    }

    #[test]
    fn never_sees_through_terrain_random_rough_ground() {
        // Rough random terrain, many random rays: whenever the fast march says
        // "visible", dense sampling must find clearance >= eps everywhere.
        let mut rng = Rng::from_seed(11);
        let (nx, ny, cell) = (80, 80, 10.0);
        let d: Vec<f32> = (0..nx * ny)
            .map(|_| rng.uniform(0.0, 12.0) as f32)
            .collect();
        let hf = Heightfield::new(nx, ny, cell, 0.0, 0.0, d).unwrap();
        let (mut vis, mut agree) = (0, 0);
        for _ in 0..3_000 {
            let a = Vec3::new(
                rng.uniform(0.0, 790.0),
                rng.uniform(0.0, 790.0),
                rng.uniform(5.0, 40.0),
            );
            let b = Vec3::new(
                rng.uniform(0.0, 790.0),
                rng.uniform(0.0, 790.0),
                rng.uniform(5.0, 40.0),
            );
            let fast = visible(&hf, a, b, 0.5);
            let (slow, min_c) = visible_reference(&hf, a, b, 0.5, 0.05);
            if fast {
                vis += 1;
                assert!(
                    slow,
                    "fast march saw through terrain: min clearance {min_c}"
                );
            }
            if fast == slow {
                agree += 1;
            } else {
                // Only allowed disagreement: the conservative band [eps, 2 eps).
                assert!(
                    min_c < 2.0 * 0.5 + 0.05,
                    "blocked a clear ray (min clearance {min_c})"
                );
            }
        }
        assert!(vis > 300, "test needs both outcomes, got {vis} visible");
        // Rays skimming 0-12 m noise are often inside the conservative band;
        // on real sectors the agreement is far higher (see integration tests).
        assert!(agree >= 2_850, "agreement {agree}/3000");
    }
}
