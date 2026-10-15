//! Regular-grid elevation model Z = f(X, Y) with bilinear sampling.
//!
//! Heights are f32 (decimetre precision is far beyond DEM accuracy);
//! everything computed from them is f64. Sector generators live in
//! `generator.rs`, line-of-sight in `los.rs` (ADR-0003, ADR-0004, ADR-0009).
//!
//! **Slope bound.** Inside a bilinear cell, dh/dx is a linear blend of the
//! two x-edge slopes, so |dh/dx| <= Lx = the largest x-edge slope in the grid
//! (and likewise for y). Hence |grad h| <= L = sqrt(Lx^2 + Ly^2) everywhere,
//! a global Lipschitz constant computed once at construction. Line-of-sight
//! ray-marching uses it to take provably safe steps.

use crate::sim::hash::Fnv64;

#[derive(Clone, Debug, PartialEq)]
pub struct Heightfield {
    nx: usize,
    ny: usize,
    cell: f64,
    /// World coordinate of sample (0, 0).
    origin_x: f64,
    origin_y: f64,
    /// Row-major: index = iy * nx + ix.
    data: Vec<f32>,
    /// Global Lipschitz bound of the bilinear interpolant (m per m).
    lipschitz: f64,
}

impl Heightfield {
    pub fn new(
        nx: usize,
        ny: usize,
        cell: f64,
        origin_x: f64,
        origin_y: f64,
        data: Vec<f32>,
    ) -> Result<Self, String> {
        if nx < 2 || ny < 2 {
            return Err("heightfield needs at least 2x2 samples".into());
        }
        if data.len() != nx * ny {
            return Err(format!(
                "heightfield data has {} samples, expected {}",
                data.len(),
                nx * ny
            ));
        }
        if !(cell.is_finite() && cell > 0.0) {
            return Err("heightfield cell size must be positive".into());
        }
        if let Some(i) = data.iter().position(|h| !h.is_finite()) {
            return Err(format!("heightfield sample {i} is not finite"));
        }
        let (mut lx, mut ly) = (0.0f64, 0.0f64);
        for iy in 0..ny {
            for ix in 0..nx {
                let h = data[iy * nx + ix] as f64;
                if ix + 1 < nx {
                    lx = lx.max((data[iy * nx + ix + 1] as f64 - h).abs());
                }
                if iy + 1 < ny {
                    ly = ly.max((data[(iy + 1) * nx + ix] as f64 - h).abs());
                }
            }
        }
        let lipschitz = (lx * lx + ly * ly).sqrt() / cell;
        Ok(Self {
            nx,
            ny,
            cell,
            origin_x,
            origin_y,
            data,
            lipschitz,
        })
    }

    /// Global bound on |grad h| (see module docs).
    pub fn lipschitz(&self) -> f64 {
        self.lipschitz
    }

    /// Steepest grid edge: (x, y, slope) at its lower-left sample.
    /// A diagnostic for generator bugs (cliffs, seams).
    pub fn steepest_edge(&self) -> (f64, f64, f64) {
        let mut best = (0.0, 0.0, 0.0);
        for iy in 0..self.ny {
            for ix in 0..self.nx {
                let h = self.at(ix, iy);
                let mut g: f64 = 0.0;
                if ix + 1 < self.nx {
                    g = g.max((self.at(ix + 1, iy) - h).abs());
                }
                if iy + 1 < self.ny {
                    g = g.max((self.at(ix, iy + 1) - h).abs());
                }
                if g / self.cell > best.2 {
                    let x = self.origin_x + ix as f64 * self.cell;
                    let y = self.origin_y + iy as f64 * self.cell;
                    best = (x, y, g / self.cell);
                }
            }
        }
        best
    }

    /// (min, max) sample height.
    pub fn min_max(&self) -> (f64, f64) {
        self.data
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &h| {
                (lo.min(h as f64), hi.max(h as f64))
            })
    }

    /// Highest ground along a horizontal circle, sampled every `step` metres.
    pub fn max_on_circle(&self, cx: f64, cy: f64, radius: f64, step: f64) -> f64 {
        let n = ((std::f64::consts::TAU * radius / step).ceil() as usize).max(16);
        (0..n)
            .map(|k| {
                let (s, c) = crate::math::sin_cos(std::f64::consts::TAU * k as f64 / n as f64);
                self.height(cx + radius * c, cy + radius * s)
            })
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Flat ground at z = 0 covering a square of side 2 * half_extent.
    pub fn flat(half_extent: f64) -> Self {
        let cell = 2.0 * half_extent;
        Self::new(2, 2, cell, -half_extent, -half_extent, vec![0.0; 4]).expect("valid flat terrain")
    }

    pub fn dims(&self) -> (usize, usize) {
        (self.nx, self.ny)
    }

    pub fn cell(&self) -> f64 {
        self.cell
    }

    pub fn origin(&self) -> (f64, f64) {
        (self.origin_x, self.origin_y)
    }

    pub fn data(&self) -> &[f32] {
        &self.data
    }

    fn at(&self, ix: usize, iy: usize) -> f64 {
        self.data[iy * self.nx + ix] as f64
    }

    /// Bilinear height at (x, y); clamps to the edge outside the grid.
    pub fn height(&self, x: f64, y: f64) -> f64 {
        let fx = ((x - self.origin_x) / self.cell).clamp(0.0, (self.nx - 1) as f64);
        let fy = ((y - self.origin_y) / self.cell).clamp(0.0, (self.ny - 1) as f64);
        let ix = (fx.floor() as usize).min(self.nx - 2);
        let iy = (fy.floor() as usize).min(self.ny - 2);
        let tx = fx - ix as f64;
        let ty = fy - iy as f64;
        let h00 = self.at(ix, iy);
        let h10 = self.at(ix + 1, iy);
        let h01 = self.at(ix, iy + 1);
        let h11 = self.at(ix + 1, iy + 1);
        let a = h00 + (h10 - h00) * tx;
        let b = h01 + (h11 - h01) * tx;
        a + (b - a) * ty
    }

    /// Content fingerprint, stored in replays so a replay on a different
    /// terrain is rejected instead of silently diverging.
    pub fn fingerprint(&self) -> u64 {
        let mut h = Fnv64::new();
        h.write_u64(self.nx as u64);
        h.write_u64(self.ny as u64);
        h.write_f64(self.cell);
        h.write_f64(self.origin_x);
        h.write_f64(self.origin_y);
        for v in &self.data {
            h.write_u32(v.to_bits());
        }
        h.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bilinear_reproduces_a_plane_exactly() {
        // z = 2x + 3y + 5 sampled on a grid must be reproduced everywhere.
        let (nx, ny, cell) = (11, 7, 10.0);
        let mut d = Vec::new();
        for iy in 0..ny {
            for ix in 0..nx {
                d.push((2.0 * ix as f64 * cell + 3.0 * iy as f64 * cell + 5.0) as f32);
            }
        }
        let hf = Heightfield::new(nx, ny, cell, 0.0, 0.0, d).unwrap();
        for (x, y) in [
            (0.0, 0.0),
            (13.7, 21.2),
            (99.9, 59.9),
            (100.0, 60.0),
            (55.0, 5.0),
        ] {
            assert!(
                (hf.height(x, y) - (2.0 * x + 3.0 * y + 5.0)).abs() < 1e-3,
                "{x},{y}"
            );
        }
    }

    #[test]
    fn lipschitz_bound_holds_between_random_points() {
        let mut rng = crate::sim::rng::Rng::from_seed(5);
        let (nx, ny, cell) = (40, 30, 10.0);
        let d: Vec<f32> = (0..nx * ny)
            .map(|_| rng.uniform(0.0, 50.0) as f32)
            .collect();
        let hf = Heightfield::new(nx, ny, cell, 0.0, 0.0, d).unwrap();
        let l = hf.lipschitz();
        for _ in 0..20_000 {
            let (x0, y0) = (rng.uniform(-20.0, 420.0), rng.uniform(-20.0, 320.0));
            let (x1, y1) = (x0 + rng.uniform(-15.0, 15.0), y0 + rng.uniform(-15.0, 15.0));
            let dist = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
            assert!((hf.height(x1, y1) - hf.height(x0, y0)).abs() <= l * dist + 1e-9);
        }
    }

    #[test]
    fn clamps_outside_grid() {
        let hf = Heightfield::new(2, 2, 1.0, 0.0, 0.0, vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        assert_eq!(hf.height(-50.0, -50.0), 1.0);
        assert_eq!(hf.height(50.0, 50.0), 4.0);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(Heightfield::new(2, 2, 1.0, 0.0, 0.0, vec![0.0; 3]).is_err());
        assert!(Heightfield::new(2, 2, 1.0, 0.0, 0.0, vec![0.0, f32::NAN, 0.0, 0.0]).is_err());
    }

    #[test]
    fn sector_grid_memory_budget() {
        // ADR-0004: 15 km at 10 m = 1501 x 1501 f32 samples, about 9 MB.
        let n = 1501usize;
        let bytes = n * n * std::mem::size_of::<f32>();
        assert!(bytes < 10 * 1024 * 1024);
    }
}
