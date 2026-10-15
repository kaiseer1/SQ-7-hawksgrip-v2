//! Deterministic 2-D gradient noise (Perlin-style) and fractal sums.
//!
//! The gradients come from an integer hash of (cell x, cell y, seed), not from
//! a shuffled permutation table, so noise needs no state and no RNG calls.
//! Every value is a pure function of its inputs and is identical on every
//! platform: only +, -, *, floor and integer ops are used.

#[inline]
fn hash(ix: i64, iy: i64, seed: u64) -> u64 {
    let mut z = (ix as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (iy as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ seed.wrapping_mul(0x1656_67B1_9E37_79F9);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

const S: f64 = std::f64::consts::FRAC_1_SQRT_2;
/// Eight unit gradient directions.
const GRADS: [(f64, f64); 8] = [
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (S, S),
    (-S, S),
    (S, -S),
    (-S, -S),
];

#[inline]
fn fade(t: f64) -> f64 {
    // Quintic 6t^5 - 15t^4 + 10t^3: C2-continuous, so slopes have no kinks.
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn corner(ix: i64, iy: i64, dx: f64, dy: f64, seed: u64) -> f64 {
    let (gx, gy) = GRADS[(hash(ix, iy, seed) & 7) as usize];
    gx * dx + gy * dy
}

/// Gradient noise in roughly [-1, 1], zero at integer lattice points.
pub fn perlin(x: f64, y: f64, seed: u64) -> f64 {
    let x0 = x.floor();
    let y0 = y.floor();
    let (ix, iy) = (x0 as i64, y0 as i64);
    let (fx, fy) = (x - x0, y - y0);
    let n00 = corner(ix, iy, fx, fy, seed);
    let n10 = corner(ix + 1, iy, fx - 1.0, fy, seed);
    let n01 = corner(ix, iy + 1, fx, fy - 1.0, seed);
    let n11 = corner(ix + 1, iy + 1, fx - 1.0, fy - 1.0, seed);
    let (u, v) = (fade(fx), fade(fy));
    let a = n00 + (n10 - n00) * u;
    let b = n01 + (n11 - n01) * u;
    // Max of 2-D gradient noise with unit gradients is sqrt(2)/2; rescale.
    (a + (b - a) * v) * std::f64::consts::SQRT_2
}

/// Fractional Brownian motion: octaves of noise, each at twice the frequency
/// and `gain` times the amplitude. Normalised to roughly [-1, 1].
pub fn fbm(x: f64, y: f64, seed: u64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += amp * perlin(x * freq, y * freq, seed.wrapping_add(o as u64 * 101));
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Ridged multifractal in [0, 1]: sharp crests where noise crosses zero,
/// the classic look of eroded mountain ridges.
pub fn ridged(x: f64, y: f64, seed: u64, octaves: u32) -> f64 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        let r = 1.0
            - perlin(x * freq, y * freq, seed.wrapping_add(o as u64 * 131))
                .abs()
                .min(1.0);
        sum += amp * r * r;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Hermite smoothstep from 0 at `e0` to 1 at `e1`.
#[inline]
pub fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_bounded() {
        for i in 0..20_000 {
            let (x, y) = (i as f64 * 0.173 - 900.0, i as f64 * 0.091 + 13.0);
            let a = perlin(x, y, 7);
            assert_eq!(a, perlin(x, y, 7));
            assert!(a.abs() <= 1.0 + 1e-12, "{a}");
            let f = fbm(x, y, 3, 6);
            assert!(f.abs() <= 1.0 + 1e-12);
            let r = ridged(x, y, 3, 5);
            assert!((0.0..=1.0).contains(&r));
        }
    }

    #[test]
    fn zero_on_lattice_and_seed_dependent() {
        assert_eq!(perlin(3.0, -4.0, 1), 0.0);
        let same = (0..1000)
            .filter(|i| {
                let x = *i as f64 * 0.37 + 0.5;
                perlin(x, 0.5, 1) == perlin(x, 0.5, 2)
            })
            .count();
        assert!(same < 50);
    }

    #[test]
    fn roughly_zero_mean() {
        let n = 40_000;
        let m: f64 = (0..n)
            .map(|i| perlin(i as f64 * 0.731, i as f64 * 0.419, 9))
            .sum::<f64>()
            / n as f64;
        assert!(m.abs() < 0.03, "{m}");
    }
}
