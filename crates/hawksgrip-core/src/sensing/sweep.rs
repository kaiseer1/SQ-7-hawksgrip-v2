//! Stochastic omni-directional sweep (State 1).
//!
//! The search region is every direction from Kindah between `el_min` and
//! `el_max` (mostly *below* her: she flies at 1.5 km and threats fly low).
//! It is split into equal-solid-angle sectors, one per sweeping eye. Equal
//! area in direction space means uniform steps in azimuth and in sin(elevation).
//!
//! Each sector is tiled with **footprint-sized cells**: rows of elevation no
//! taller than the frustum, each row cut into azimuth cells no wider than the
//! frustum's footprint at that elevation (the footprint widens in azimuth by
//! 1/cos(el) toward the nadir). An eye visits *every* cell of its sector once
//! per cycle, so no direction can go unobserved for more than two cycles.
//!
//! The visiting order is randomised and never repeats: cycle c uses the
//! permutation i -> (i * stride + offset_c) mod n. Here stride is coprime with
//! n (a full cycle), and offset_c comes from the R2 low-discrepancy sequence
//! (Roberts 2018, plastic number) with a per-eye random rotation. The aim point
//! also jitters inside its cell. Red-Team P3-1 measured why cells matter:
//! visiting raw R2 points (no cells) left a direction unobserved for 6.7 s.
//!
//! When the number of free eyes changes (convergence, sentinel fallback), the
//! region is re-tiled across whichever eyes are sweeping, so the *whole*
//! region is always covered, just at a lower revisit rate.

use crate::vec3::Vec3;

/// The plastic number: unique real root of x^3 = x + 1.
const PLASTIC: f64 = 1.324_717_957_244_746;
const A1: f64 = 1.0 / PLASTIC;
const A2: f64 = 1.0 / (PLASTIC * PLASTIC);

/// k-th point of the R2 sequence in [0, 1)^2, with a rotation `shift`.
pub fn r2(k: u64, shift: [f64; 2]) -> [f64; 2] {
    let k = k as f64;
    [(shift[0] + A1 * k).fract(), (shift[1] + A2 * k).fract()]
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sector {
    pub az0: f64,
    pub az1: f64,
    /// Bounds in sin(elevation).
    pub s0: f64,
    pub s1: f64,
}

impl Sector {
    /// Unit direction for unit-square coordinates (u, v) in this sector.
    pub fn direction(&self, u: f64, v: f64) -> Vec3 {
        let az = self.az0 + (self.az1 - self.az0) * u;
        let s = self.s0 + (self.s1 - self.s0) * v;
        let c = (1.0 - s * s).max(0.0).sqrt();
        let (sa, ca) = crate::math::sin_cos(az);
        // ENU: azimuth measured counter-clockwise from East.
        Vec3::new(c * ca, c * sa, s)
    }

    pub fn solid_angle(&self) -> f64 {
        (self.az1 - self.az0) * (self.s1 - self.s0)
    }
}

/// A footprint-sized sweep cell: centre (az, el) and size (daz, del), radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub az: f64,
    pub el: f64,
    pub daz: f64,
    pub del: f64,
}

impl Cell {
    /// Direction for offsets (du, dv) in cell units (0 = centre).
    pub fn direction(&self, du: f64, dv: f64) -> Vec3 {
        let az = self.az + du * self.daz;
        let el = self.el + dv * self.del;
        let (se, ce) = crate::math::sin_cos(el);
        let (sa, ca) = crate::math::sin_cos(az);
        Vec3::new(ce * ca, ce * sa, se)
    }
}

/// Tile a sector with cells at most `overlap` times the frustum size.
pub fn cells(sec: &Sector, fov_h: f64, fov_v: f64, overlap: f64) -> Vec<Cell> {
    let (el0, el1) = (libm::asin(sec.s0), libm::asin(sec.s1));
    let rows = (((el1 - el0) / (fov_v * overlap)).ceil() as usize).max(1);
    let del = (el1 - el0) / rows as f64;
    let mut out = Vec::new();
    for r in 0..rows {
        let lo = el0 + del * r as f64;
        let hi = lo + del;
        // The footprint is narrowest in azimuth at the row edge nearest the horizon.
        let cmax = if lo <= 0.0 && hi >= 0.0 {
            1.0
        } else {
            crate::math::sin_cos(lo).1.max(crate::math::sin_cos(hi).1)
        };
        let width = sec.az1 - sec.az0;
        let n_az = ((width * cmax / (fov_h * overlap)).ceil() as usize).max(1);
        let daz = width / n_az as f64;
        for a in 0..n_az {
            out.push(Cell {
                az: sec.az0 + daz * (a as f64 + 0.5),
                el: lo + 0.5 * del,
                daz,
                del,
            });
        }
    }
    out
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// A stride near n / golden ratio that is coprime with n, so i -> i * stride
/// mod n visits every cell once and consecutive visits are far apart.
pub fn stride(n: usize) -> usize {
    if n <= 2 {
        return 1;
    }
    let mut s = ((n as f64 / 1.618_033_988_749_895).round() as usize).max(1);
    while gcd(s, n) != 1 {
        s += 1;
    }
    s
}

/// Cell index visited at step k of an eye's sweep (see module docs).
pub fn visit(k: u64, n: usize, shift: [f64; 2]) -> usize {
    let n64 = n as u64;
    let (cycle, pos) = (k / n64, k % n64);
    let offset = ((shift[0] + A1 * cycle as f64).fract() * n as f64) as u64 % n64;
    ((pos * stride(n) as u64 + offset) % n64) as usize
}

/// Split the region into `n` equal-solid-angle sectors: n_el elevation bands
/// times n_az azimuth slices, with the band count chosen so sectors are
/// roughly square in angle.
pub fn tile(n: usize, el_min: f64, el_max: f64) -> Vec<Sector> {
    assert!(n > 0);
    // libm, not std: sweep geometry feeds simulation state (ADR-0007).
    let (s_lo, s_hi) = (
        crate::math::sin_cos(el_min).0,
        crate::math::sin_cos(el_max).0,
    );
    // The region spans 2*pi of azimuth and about (el_max - el_min) of elevation.
    let aspect = std::f64::consts::TAU / (el_max - el_min);
    let ideal_el = (n as f64 / aspect).sqrt();
    let n_el = (1..=n)
        .filter(|d| n % d == 0)
        .min_by(|a, b| {
            let da = (*a as f64 - ideal_el).abs();
            let db = (*b as f64 - ideal_el).abs();
            da.partial_cmp(&db).unwrap()
        })
        .unwrap();
    let n_az = n / n_el;
    let mut out = Vec::with_capacity(n);
    for j in 0..n_el {
        for i in 0..n_az {
            let tau = std::f64::consts::TAU;
            out.push(Sector {
                az0: tau * i as f64 / n_az as f64,
                az1: tau * (i + 1) as f64 / n_az as f64,
                s0: s_lo + (s_hi - s_lo) * j as f64 / n_el as f64,
                s1: s_lo + (s_hi - s_lo) * (j + 1) as f64 / n_el as f64,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_cover_region_with_equal_area() {
        let (lo, hi) = (-85f64.to_radians(), 5f64.to_radians());
        for n in [1, 4, 7, 20, 24, 32] {
            let t = tile(n, lo, hi);
            assert_eq!(t.len(), n);
            let total: f64 = t.iter().map(|s| s.solid_angle()).sum();
            let expect = std::f64::consts::TAU * (hi.sin() - lo.sin());
            assert!((total - expect).abs() < 1e-9);
            for s in &t {
                assert!((s.solid_angle() - expect / n as f64).abs() < 1e-9);
            }
        }
        // 24 eyes over 360 x 90 degrees: 12 azimuth slices x 2 bands is the
        // most nearly square split (30 deg wide sectors).
        let t = tile(24, lo, hi);
        assert!((t[0].az1 - t[0].az0 - std::f64::consts::TAU / 12.0).abs() < 1e-12);
    }

    #[test]
    fn r2_has_low_discrepancy() {
        // 16 x 16 cells. R2 fills every cell within 768 points; uniform random
        // sampling leaves ~13 cells empty on average at that count (measured
        // over 200 seeds), and at 256 points R2 reaches 216 cells vs ~162.
        let cells = |n: u64| {
            let mut hit = [[false; 16]; 16];
            for k in 0..n {
                let [u, v] = r2(k, [0.3, 0.7]);
                hit[(u * 16.0) as usize][(v * 16.0) as usize] = true;
            }
            hit.iter().flatten().filter(|&&h| h).count()
        };
        assert_eq!(cells(768), 256);
        assert!(cells(256) >= 210);
    }

    #[test]
    fn every_cycle_visits_every_cell_once_in_a_new_order() {
        for n in [1usize, 2, 7, 12, 13, 30] {
            let mut prev: Vec<usize> = Vec::new();
            for cycle in 0..4u64 {
                let order: Vec<usize> = (0..n as u64)
                    .map(|p| visit(cycle * n as u64 + p, n, [0.4, 0.9]))
                    .collect();
                let mut sorted = order.clone();
                sorted.sort();
                assert_eq!(sorted, (0..n).collect::<Vec<_>>(), "n={n} cycle={cycle}");
                if n > 2 && cycle > 0 {
                    assert_ne!(order, prev, "order repeated");
                }
                prev = order;
            }
        }
    }

    #[test]
    fn cells_cover_their_sector_without_gaps() {
        let (fh, fv) = (12f64.to_radians(), 9f64.to_radians());
        for s in tile(24, -85f64.to_radians(), 5f64.to_radians()) {
            let c = cells(&s, fh, fv, 0.85);
            // Every cell fits inside one frustum at its own elevation.
            for cell in &c {
                let worst_cos = (cell.el - cell.del / 2.0)
                    .cos()
                    .max((cell.el + cell.del / 2.0).cos());
                assert!(cell.del <= fv * 0.85 + 1e-12);
                assert!(cell.daz * worst_cos.min(1.0) <= fh * 0.85 + 1e-12);
            }
            let area: f64 = c
                .iter()
                .map(|k| k.daz * ((k.el + k.del / 2.0).sin() - (k.el - k.del / 2.0).sin()))
                .sum();
            assert!((area - s.solid_angle()).abs() < 1e-9);
        }
    }

    #[test]
    fn directions_are_unit_and_inside_band() {
        let t = tile(24, -85f64.to_radians(), 5f64.to_radians());
        for s in &t {
            for k in 0..50 {
                let [u, v] = r2(k, [0.1, 0.2]);
                let d = s.direction(u, v);
                assert!((d.norm() - 1.0).abs() < 1e-12);
                assert!(d.z >= s.s0 - 1e-12 && d.z <= s.s1 + 1e-12);
            }
        }
    }
}
