//! Procedural Saudi sector generators (ADR-0009).
//!
//! Each sector is a pure function height(x, y; seed) evaluated on the
//! 1501 x 1501 grid (ADR-0004), in parallel by rows. The shapes are
//! *regional archetypes*, not maps of real places, and contain no real
//! facility coordinates. Every amplitude and length below is SWEPT
//! (docs/claims.md).
//!
//! | Sector | Archetype | Defensive problem it creates |
//! |---|---|---|
//! | 1 Eastern Coastal Sabkha | Gulf coast: sea to the east, near-flat salt flats, low dunes | Low marine approach with no terrain to hide behind, but no high ground to see from |
//! | 2 Western Red Sea Corridor | Red Sea to the west, coastal plain, steep escarpment to the east cut by wadis | Threats drop off the escarpment through canyons |
//! | 3 Highland Escarpment / Wadi | Plateau ~2 km high, refinery in a basin, deep sinuous wadis | Terrain masking: threats fly the wadi floors and pop up at the rim |

use crate::terrain::Heightfield;
use crate::terrain::noise::{fbm, ridged, smoothstep};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sector {
    /// Flat ground at z = 0 (unit tests and the Phase 1 golden run).
    #[default]
    Flat,
    EasternSabkha,
    RedSeaCorridor,
    HighlandWadi,
}

impl Sector {
    pub fn label(self) -> &'static str {
        match self {
            Sector::Flat => "Flat test ground",
            Sector::EasternSabkha => "Sector 1: Eastern Coastal Sabkha",
            Sector::RedSeaCorridor => "Sector 2: Western Red Sea Corridor",
            Sector::HighlandWadi => "Sector 3: Highland Escarpment / Wadi",
        }
    }
}

/// A sinuous valley: a straight axis from `start` to `end` with a lateral
/// sine meander, carved as a flat bed with straight walls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wadi {
    pub start: [f64; 2],
    pub end: [f64; 2],
    pub meander_amp: f64,
    pub meander_len: f64,
    pub phase: f64,
    /// Floor height (absolute) at start and end.
    pub floor_start: f64,
    pub floor_end: f64,
    /// Floor profile: floor = end + (start - end) * (1 - u)^exp along the
    /// axis (u = 0 at start). exp > 1 gives the concave long-profile of real
    /// drainage: steep near the head, flattening downstream.
    pub floor_exp: f64,
    pub half_bed: f64,
    /// Wall steepness (rise per metre of lateral distance).
    pub wall_slope: f64,
    /// Length over which the carve fades in at the head and out at the mouth.
    pub fade_m: f64,
}

impl Wadi {
    fn axis(&self) -> ([f64; 2], [f64; 2], f64) {
        let (dx, dy) = (self.end[0] - self.start[0], self.end[1] - self.start[1]);
        let len = (dx * dx + dy * dy).sqrt();
        ([dx / len, dy / len], [-dy / len, dx / len], len)
    }

    fn meander(&self, t: f64) -> f64 {
        self.meander_amp * crate::math::sin_cos(t / self.meander_len + self.phase).0
    }

    fn floor_at(&self, u: f64) -> f64 {
        self.floor_end
            + (self.floor_start - self.floor_end)
                * crate::math::pow(1.0 - u.clamp(0.0, 1.0), self.floor_exp)
    }

    /// Apply this wadi to terrain height `h` at (x, y).
    ///
    /// The carved surface is floor + wall_slope * (lateral distance - half bed);
    /// the result is min(h, carved), blended in with weight `fade` near the head
    /// and mouth so the valley has no cliff ends. Lateral distance is measured
    /// from the meandering centreline, which is accurate while the meander
    /// slope amp/len stays below about 0.5.
    pub fn apply(&self, x: f64, y: f64, h: f64) -> f64 {
        let (dir, perp, len) = self.axis();
        let (px, py) = (x - self.start[0], y - self.start[1]);
        let t = px * dir[0] + py * dir[1];
        let fade = smoothstep(0.0, self.fade_m, t) * (1.0 - smoothstep(len - self.fade_m, len, t));
        if fade <= 0.0 {
            return h;
        }
        let lat = (px * perp[0] + py * perp[1] - self.meander(t)).abs();
        let carved = self.floor_at(t / len) + self.wall_slope * (lat - self.half_bed).max(0.0);
        h + fade * (carved.min(h) - h)
    }

    /// Centreline as a polyline (for rendering and scenario routes).
    pub fn centreline(&self, step: f64) -> Vec<[f64; 3]> {
        let (dir, perp, len) = self.axis();
        let n = (len / step).ceil() as usize;
        (0..=n)
            .map(|k| {
                let t = len * k as f64 / n as f64;
                let m = self.meander(t);
                [
                    self.start[0] + dir[0] * t + perp[0] * m,
                    self.start[1] + dir[1] * t + perp[1] * m,
                    self.floor_at(t / len),
                ]
            })
            .collect()
    }
}

/// Everything a generator produces besides the heights.
#[derive(Clone, Debug, PartialEq)]
pub struct SectorInfo {
    pub sector: Sector,
    pub seed: u64,
    /// Refinery pad elevation (absolute).
    pub pad_z: f64,
    pub wadis: Vec<Wadi>,
    /// Shoreline x as a function of y is sampled here for rendering.
    pub coastline: Vec<[f64; 2]>,
}

pub struct Generated {
    pub heightfield: Heightfield,
    pub info: SectorInfo,
}

/// Geometry shared by all sectors.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub half_extent: f64,
    pub cell: f64,
    pub center: [f64; 2],
    pub refinery_half: f64,
}

const PAD_APRON: f64 = 200.0;
const PAD_BLEND: f64 = 350.0;

pub fn generate(sector: Sector, seed: u64, f: Frame) -> Result<Generated, String> {
    if sector == Sector::Flat {
        return Ok(Generated {
            heightfield: Heightfield::flat(f.half_extent),
            info: SectorInfo {
                sector,
                seed,
                pad_z: 0.0,
                wadis: vec![],
                coastline: vec![],
            },
        });
    }
    let n = (2.0 * f.half_extent / f.cell).round() as usize + 1;
    let origin = -f.half_extent;
    let model = Model::new(sector, seed, f);

    let mut data = vec![0f32; n * n];
    data.par_chunks_mut(n).enumerate().for_each(|(iy, row)| {
        let y = origin + iy as f64 * f.cell;
        for (ix, h) in row.iter_mut().enumerate() {
            let x = origin + ix as f64 * f.cell;
            *h = model.height(x, y) as f32;
        }
    });
    let heightfield = Heightfield::new(n, n, f.cell, origin, origin, data)?;
    let coastline = match model.coast {
        Some(_) => (0..=60)
            .map(|k| {
                let y = origin + 2.0 * f.half_extent * k as f64 / 60.0;
                [model.coast_x(y), y]
            })
            .collect(),
        None => vec![],
    };
    Ok(Generated {
        heightfield,
        info: SectorInfo {
            sector,
            seed,
            pad_z: model.pad_z,
            wadis: model.wadis.clone(),
            coastline,
        },
    })
}

/// Coast: which side the sea is on, and the mean shoreline x.
#[derive(Clone, Copy, Debug)]
struct Coast {
    sea_east: bool,
    mean_x: f64,
    wiggle: f64,
}

struct Model {
    sector: Sector,
    seed: u64,
    f: Frame,
    coast: Option<Coast>,
    wadis: Vec<Wadi>,
    pad_z: f64,
}

impl Model {
    fn new(sector: Sector, seed: u64, f: Frame) -> Self {
        let coast = match sector {
            Sector::EasternSabkha => Some(Coast {
                sea_east: true,
                mean_x: 3_800.0,
                wiggle: 450.0,
            }),
            Sector::RedSeaCorridor => Some(Coast {
                sea_east: false,
                mean_x: -5_200.0,
                wiggle: 300.0,
            }),
            _ => None,
        };
        let mut m = Model {
            sector,
            seed,
            f,
            coast,
            wadis: vec![],
            pad_z: 0.0,
        };
        m.wadis = m.make_wadis();
        m.pad_z = m.raw(f.center[0], f.center[1]);
        m
    }

    fn coast_x(&self, y: f64) -> f64 {
        let c = self.coast.expect("coast");
        c.mean_x + c.wiggle * fbm(y / 3_000.0, 0.37, self.seed ^ 0xC0A5, 3)
    }

    /// Signed inland distance (positive on land), or +inf without a coast.
    fn inland(&self, x: f64, y: f64) -> f64 {
        match self.coast {
            None => f64::INFINITY,
            Some(c) => {
                let cx = self.coast_x(y);
                if c.sea_east { cx - x } else { x - cx }
            }
        }
    }

    fn make_wadis(&self) -> Vec<Wadi> {
        let s = self.seed;
        let jitter = |k: u64, a: f64| a * fbm(k as f64 * 1.7 + 0.3, 0.71, s ^ 0x3AD1, 2);
        match self.sector {
            Sector::RedSeaCorridor => {
                // Canyons draining west off the escarpment onto the plain.
                [-5_200.0, -2_100.0, 2_300.0, 5_400.0]
                    .iter()
                    .enumerate()
                    .map(|(k, &y0)| Wadi {
                        start: [7_400.0, y0 + jitter(k as u64, 600.0)],
                        end: [1_200.0, y0 * 0.85],
                        meander_amp: 260.0,
                        meander_len: 520.0,
                        phase: k as f64 * 1.3,
                        floor_start: 1_650.0,
                        floor_end: 40.0,
                        floor_exp: 2.5,
                        half_bed: 35.0,
                        wall_slope: 1.0,
                        fade_m: 900.0,
                    })
                    .collect()
            }
            Sector::HighlandWadi => {
                // Tributaries from the plateau rim converging on the refinery basin.
                let basin = 1_350.0;
                [
                    (7_400.0, 3_900.0),
                    (-7_400.0, 2_600.0),
                    (-3_500.0, -7_400.0),
                    (4_200.0, -7_400.0),
                    (-6_800.0, -5_200.0),
                ]
                .iter()
                .enumerate()
                .map(|(k, &(x0, y0))| {
                    let (x0, y0) = (
                        x0 + jitter(k as u64, 400.0),
                        y0 + jitter(k as u64 + 9, 400.0),
                    );
                    // End at the edge of the refinery apron, pointing at the centre.
                    let r = (x0 * x0 + y0 * y0).sqrt();
                    let stop = self.f.refinery_half + PAD_APRON + 150.0;
                    Wadi {
                        start: [x0, y0],
                        end: [x0 / r * stop, y0 / r * stop],
                        meander_amp: 220.0,
                        meander_len: 650.0,
                        phase: k as f64 * 2.1,
                        floor_start: 2_050.0,
                        floor_end: basin + 15.0,
                        floor_exp: 1.6,
                        half_bed: 30.0,
                        wall_slope: 1.0,
                        fade_m: 700.0,
                    }
                })
                .collect()
            }
            _ => vec![],
        }
    }

    /// Terrain before the refinery pad is levelled.
    fn raw(&self, x: f64, y: f64) -> f64 {
        let s = self.seed;
        let h = match self.sector {
            Sector::Flat => 0.0,
            Sector::EasternSabkha => {
                let d = self.inland(x, y);
                if d <= 0.0 {
                    return 0.0; // sea surface
                }
                // Gentle westward rise (~1.2 m per km), salt flats near the coast,
                // and low NNW-SSE dune ridges (Shamal-aligned) further inland.
                let base = 0.6 + 0.0012 * d + 1.5 * fbm(x / 2_500.0, y / 2_500.0, s, 4);
                let flat = smoothstep(500.0, 3_500.0, d);
                let (ca, sa) = (0.94, 0.34); // ~20 degrees from north
                let (u, v) = (ca * x + sa * y, -sa * x + ca * y);
                let dunes = 7.0
                    * ridged(u / 140.0, v / 900.0, s ^ 0xD00E, 3)
                    * smoothstep(1_500.0, 6_000.0, d);
                (base * (0.35 + 0.65 * flat) + dunes).max(0.3)
            }
            Sector::RedSeaCorridor => {
                let d = self.inland(x, y);
                if d <= 0.0 {
                    return 0.0;
                }
                let plain = 0.018 * d + 6.0 * fbm(x / 1_800.0, y / 1_800.0, s, 4);
                let foot = 3_000.0 + 500.0 * fbm(y / 4_000.0, 0.13, s ^ 0xF007, 3);
                let e = smoothstep(foot, foot + 3_000.0, x);
                let band = 4.0 * e * (1.0 - e);
                let wx = x + 300.0 * fbm(x / 1_500.0, y / 1_500.0, s ^ 0x77, 2);
                // Ridged slopes add per octave (each octave doubles frequency and
                // halves amplitude), so octaves are capped at 3 to keep the
                // steepest slopes physical (test: slopes_are_physical).
                let rugged = 120.0 * ridged(wx / 900.0, y / 900.0, s ^ 0xE5C, 3) * (band + 0.4 * e);
                let h =
                    plain + 1_550.0 * e + rugged + 60.0 * e * fbm(x / 700.0, y / 700.0, s ^ 0x5, 3);
                self.carve(x, y, h)
            }
            Sector::HighlandWadi => {
                let plateau = 1_950.0 + 220.0 * fbm(x / 5_000.0, y / 5_000.0, s, 3);
                let wx = x + 500.0 * fbm(x / 3_000.0, y / 3_000.0, s ^ 0x91, 2);
                let wy = y + 500.0 * fbm(x / 3_000.0 + 7.1, y / 3_000.0, s ^ 0x92, 2);
                let ridges = 330.0 * ridged(wx / 2_400.0, wy / 2_400.0, s ^ 0xA1, 5);
                let rough = 40.0 * fbm(x / 400.0, y / 400.0, s ^ 0xA2, 3);
                // Bowl-shaped basin around the refinery.
                let r = ((x - self.f.center[0]).powi(2) + (y - self.f.center[1]).powi(2)).sqrt();
                let bowl = smoothstep(1_300.0, 3_200.0, r);
                let h = 1_350.0 + (plateau + ridges + rough - 1_350.0) * bowl;
                self.carve(x, y, h)
            }
        };
        h.max(0.0)
    }

    fn carve(&self, x: f64, y: f64, h: f64) -> f64 {
        self.wadis.iter().fold(h, |h, w| w.apply(x, y, h))
    }

    /// Final height: raw terrain with the refinery pad levelled.
    fn height(&self, x: f64, y: f64) -> f64 {
        let raw = self.raw(x, y);
        let cheb = (x - self.f.center[0])
            .abs()
            .max((y - self.f.center[1]).abs());
        let inner = self.f.refinery_half + PAD_APRON;
        let t = smoothstep(inner, inner + PAD_BLEND, cheb);
        self.pad_z + (raw - self.pad_z) * t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: Frame = Frame {
        half_extent: 7_500.0,
        cell: 10.0,
        center: [0.0, 0.0],
        refinery_half: 1_000.0,
    };

    fn make(s: Sector) -> Generated {
        generate(s, 1, F).unwrap()
    }

    fn pad_flatness(g: &Generated) -> f64 {
        let hf = &g.heightfield;
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for i in 0..=40 {
            for j in 0..=40 {
                let (x, y) = (-1_000.0 + 50.0 * i as f64, -1_000.0 + 50.0 * j as f64);
                let h = hf.height(x, y);
                lo = lo.min(h);
                hi = hi.max(h);
            }
        }
        hi - lo
    }

    #[test]
    fn grid_matches_adr_0004() {
        let g = make(Sector::EasternSabkha);
        assert_eq!(g.heightfield.dims(), (1501, 1501));
        assert_eq!(g.heightfield.data().len() * 4, 9_012_004);
    }

    #[test]
    fn generation_is_deterministic_and_seeded() {
        let a = generate(Sector::HighlandWadi, 3, F)
            .unwrap()
            .heightfield
            .fingerprint();
        let b = generate(Sector::HighlandWadi, 3, F)
            .unwrap()
            .heightfield
            .fingerprint();
        let c = generate(Sector::HighlandWadi, 4, F)
            .unwrap()
            .heightfield
            .fingerprint();
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn refinery_pad_is_level_in_every_sector() {
        for s in [
            Sector::EasternSabkha,
            Sector::RedSeaCorridor,
            Sector::HighlandWadi,
        ] {
            let g = make(s);
            assert!(pad_flatness(&g) < 0.01, "{s:?} pad not level");
            assert!((g.heightfield.height(0.0, 0.0) - g.info.pad_z).abs() < 0.01);
        }
    }

    #[test]
    fn sector1_sabkha_shape() {
        let g = make(Sector::EasternSabkha);
        let hf = &g.heightfield;
        assert_eq!(hf.height(7_000.0, 0.0), 0.0, "sea in the east");
        assert!(hf.height(-7_000.0, 0.0) > 3.0, "land rises westward");
        let (_, max) = hf.min_max();
        assert!(max < 40.0, "coastal plain stays low, max {max}");
    }

    #[test]
    fn sector2_red_sea_shape() {
        let g = make(Sector::RedSeaCorridor);
        let hf = &g.heightfield;
        assert_eq!(hf.height(-7_000.0, 0.0), 0.0, "sea in the west");
        assert!(g.info.pad_z < 200.0, "refinery on the coastal plain");
        let top = (-7..=7)
            .map(|k| hf.height(7_000.0, k as f64 * 1_000.0))
            .fold(0.0, f64::max);
        assert!(top > 1_400.0, "escarpment crest {top}");
        // Wadi floors cut far below the surrounding escarpment.
        let w = &g.info.wadis[1];
        let mid = w.centreline(50.0)[20];
        let floor = hf.height(mid[0], mid[1]);
        let wall = hf
            .height(mid[0], mid[1] + 600.0)
            .max(hf.height(mid[0], mid[1] - 600.0));
        assert!(wall - floor > 150.0, "canyon depth {}", wall - floor);
    }

    #[test]
    fn sector3_basin_and_wadis() {
        let g = make(Sector::HighlandWadi);
        let hf = &g.heightfield;
        let pad = g.info.pad_z;
        assert!((pad - 1_350.0).abs() < 1.0);
        let rim = hf.max_on_circle(0.0, 0.0, 5_000.0, 20.0);
        assert!(rim - pad > 500.0, "basin depth {}", rim - pad);
        for w in &g.info.wadis {
            // Floors descend monotonically toward the basin.
            let line = w.centreline(100.0);
            let floors: Vec<f64> = line.iter().map(|p| hf.height(p[0], p[1])).collect();
            assert!(floors.first().unwrap() > floors.last().unwrap());
            // Away from the faded head and mouth, the ground on the centreline
            // is the carved floor (bilinear sampling of the bed adds < 1 m).
            let skip = (w.fade_m / 100.0).ceil() as usize;
            let mid = line.len().saturating_sub(2 * skip);
            for (p, h) in line.iter().zip(&floors).skip(skip).take(mid) {
                assert!(*h <= p[2] + 1.0, "wadi floor {h} above design {}", p[2]);
            }
        }
    }

    #[test]
    fn slopes_are_physical() {
        // The steepest grid edge must stay below tan(61 deg) = 1.8 (steep
        // escarpment faces, but no cliffs or seams). The Lipschitz bound
        // combines the worst x and y edges from anywhere in the grid, so it is
        // looser by up to sqrt(2); it must stay below 2.2 for LoS efficiency.
        for s in [
            Sector::EasternSabkha,
            Sector::RedSeaCorridor,
            Sector::HighlandWadi,
        ] {
            let hf = make(s).heightfield;
            let (l, edge) = (hf.lipschitz(), hf.steepest_edge());
            assert!(edge.2 < 1.8, "{s:?} steepest edge {edge:?}");
            assert!(l < 2.2, "{s:?} lipschitz {l}");
        }
    }
}
