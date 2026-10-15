//! The Synthetic Refinery Complex: a generic layout of the kinds of
//! structure every refinery has, scaled to the configured footprint and
//! standing on the levelled pad. It is not modelled on any real facility.
//!
//! Each structure carries a **keep-out margin**: the stand-off within which no
//! interception may happen (debris and blast hazard over volatile plant).
//! Margins are SWEPT (docs/claims.md). Phase 5's cost function and Phase 6's
//! guidance query the inflated volumes.

use crate::refinery::shapes::Solid;
use crate::vec3::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    StorageTank,
    DistillationColumn,
    ProcessUnit,
    FlareStack,
    ControlRoom,
}

impl Kind {
    /// Default keep-out margin in metres.
    pub fn keepout_margin(self) -> f64 {
        match self {
            Kind::StorageTank => 60.0,
            Kind::DistillationColumn => 40.0,
            Kind::ProcessUnit => 40.0,
            Kind::FlareStack => 80.0,
            Kind::ControlRoom => 25.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Structure {
    pub kind: Kind,
    pub solid: Solid,
    pub keepout: Solid,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Refinery {
    pub center: Vec3,
    pub half_size: f64,
    pub structures: Vec<Structure>,
}

impl Refinery {
    /// Build the layout. `pad_z` is the levelled ground height; `margin_scale`
    /// multiplies every keep-out margin (for sensitivity sweeps).
    pub fn synthetic(center: Vec3, half_size: f64, pad_z: f64, margin_scale: f64) -> Self {
        // Template in a 2 x 2 km frame (+-1000 m), scaled to the footprint.
        let k = half_size / 1_000.0;
        let at = |x: f64, y: f64| (center.x + x * k, center.y + y * k);
        let mut out = Vec::new();
        let mut cyl = |kind: Kind, x: f64, y: f64, r: f64, h: f64| {
            let (cx, cy) = at(x, y);
            out.push((
                kind,
                Solid::Cylinder {
                    cx,
                    cy,
                    r,
                    z0: pad_z,
                    z1: pad_z + h,
                },
            ));
        };
        // Tank farm (west): 4 x 7 floating-roof tanks.
        for x in [-850.0, -700.0, -550.0, -400.0] {
            for y in [-720.0, -480.0, -240.0, 0.0, 240.0, 480.0, 720.0] {
                cyl(Kind::StorageTank, x, y, 35.0, 18.0);
            }
        }
        // Process area (centre-east): four tall columns.
        for (x, y) in [
            (150.0, -100.0),
            (150.0, 100.0),
            (260.0, -100.0),
            (260.0, 100.0),
        ] {
            cyl(Kind::DistillationColumn, x, y, 4.0, 70.0);
        }
        // Flare stacks at the far east corners.
        for (x, y) in [(870.0, -820.0), (870.0, 820.0)] {
            cyl(Kind::FlareStack, x, y, 2.0, 110.0);
        }
        let mut boxes = Vec::new();
        for y in [-450.0, 0.0, 450.0] {
            boxes.push((Kind::ProcessUnit, 480.0, y, 60.0, 40.0, 25.0));
        }
        boxes.push((Kind::ControlRoom, -150.0, -880.0, 20.0, 15.0, 8.0));
        for (kind, x, y, hx, hy, h) in boxes {
            let (cx, cy) = at(x, y);
            out.push((
                kind,
                Solid::Box {
                    min: Vec3::new(cx - hx, cy - hy, pad_z),
                    max: Vec3::new(cx + hx, cy + hy, pad_z + h),
                },
            ));
        }
        let structures = out
            .into_iter()
            .map(|(kind, solid)| Structure {
                kind,
                solid,
                keepout: solid.inflate(kind.keepout_margin() * margin_scale),
            })
            .collect();
        Refinery {
            center,
            half_size,
            structures,
        }
    }

    /// Smallest signed distance to any keep-out volume (negative = inside).
    pub fn keepout_distance(&self, p: Vec3) -> f64 {
        self.structures
            .iter()
            .map(|s| s.keepout.signed_distance(p))
            .fold(f64::INFINITY, f64::min)
    }

    pub fn in_keepout(&self, p: Vec3) -> bool {
        self.structures.iter().any(|s| s.keepout.contains(p))
    }

    /// True if the straight path a→b stays outside every keep-out volume.
    pub fn path_clear_of_keepout(&self, a: Vec3, b: Vec3) -> bool {
        !self
            .structures
            .iter()
            .any(|s| s.keepout.intersects_segment(a, b))
    }

    /// True if a→b is blocked by a physical structure (for line-of-sight).
    pub fn occludes(&self, a: Vec3, b: Vec3) -> bool {
        self.structures
            .iter()
            .any(|s| s.solid.intersects_segment(a, b))
    }

    pub fn tallest(&self) -> f64 {
        self.structures
            .iter()
            .map(|s| s.solid.top())
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r() -> Refinery {
        Refinery::synthetic(Vec3::ZERO, 1_000.0, 5.0, 1.0)
    }

    #[test]
    fn layout_fits_footprint_and_stands_on_pad() {
        let r = r();
        assert_eq!(r.structures.len(), 28 + 4 + 2 + 3 + 1);
        for s in &r.structures {
            let (lo, hi) = match s.solid {
                Solid::Cylinder { cx, cy, r, z0, .. } => {
                    assert_eq!(z0, 5.0);
                    ((cx - r).min(cy - r), (cx + r).max(cy + r))
                }
                Solid::Box { min, max } => {
                    assert_eq!(min.z, 5.0);
                    (min.x.min(min.y), max.x.max(max.y))
                }
            };
            assert!(
                lo >= -1_000.0 && hi <= 1_000.0,
                "{:?} outside footprint",
                s.kind
            );
        }
        assert!(
            r.tallest() - 5.0 < 150.0,
            "all structures below the breach ceiling"
        );
    }

    #[test]
    fn structures_do_not_overlap() {
        let r = r();
        for (i, a) in r.structures.iter().enumerate() {
            for b in &r.structures[i + 1..] {
                // Sample a's footprint centre: it must not be inside b.
                let c = match a.solid {
                    Solid::Cylinder { cx, cy, z0, .. } => Vec3::new(cx, cy, z0 + 1.0),
                    Solid::Box { min, max } => (min + max) * 0.5,
                };
                assert!(!b.solid.contains(c), "{:?} overlaps {:?}", a.kind, b.kind);
            }
        }
    }

    #[test]
    fn keepout_queries() {
        let r = r();
        // Directly above a tank, inside its 60 m margin.
        assert!(r.in_keepout(Vec3::new(-850.0, -720.0, 5.0 + 18.0 + 30.0)));
        // High above everything: clear.
        assert!(!r.in_keepout(Vec3::new(0.0, 0.0, 400.0)));
        assert!(r.keepout_distance(Vec3::new(0.0, 0.0, 400.0)) > 100.0);
        // A low pass over the tank farm crosses keep-out; a high one does not.
        assert!(
            !r.path_clear_of_keepout(Vec3::new(-990.0, 0.0, 30.0), Vec3::new(-300.0, 0.0, 30.0))
        );
        assert!(
            r.path_clear_of_keepout(Vec3::new(-990.0, 0.0, 300.0), Vec3::new(-300.0, 0.0, 300.0))
        );
        // Margin scaling widens the zone.
        let wide = Refinery::synthetic(Vec3::ZERO, 1_000.0, 5.0, 2.0);
        let p = Vec3::new(-850.0, -720.0 + 35.0 + 90.0, 10.0);
        assert!(!r.in_keepout(p) && wide.in_keepout(p));
    }

    #[test]
    fn columns_occlude_low_sightlines() {
        let r = r();
        assert!(r.occludes(
            Vec3::new(150.0, -400.0, 40.0),
            Vec3::new(150.0, 400.0, 40.0)
        ));
        assert!(!r.occludes(
            Vec3::new(150.0, -400.0, 140.0),
            Vec3::new(150.0, 400.0, 140.0)
        ));
    }
}
