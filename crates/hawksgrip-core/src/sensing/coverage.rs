//! Dense coverage of the airspace by the eyes: the CPU reference that the
//! GPU implementation (python/hawksgrip/gpu/coverage.py) is measured and
//! validated against (ADR-0003 amendment: every GPU speed-up is proven
//! against the best CPU version, not a strawman).
//!
//! For every voxel centre v and the current eye directions:
//! - `IN_VIEW`: inside at least one eye's frustum;
//! - `DETECTABLE`: pd(band, Kindah -> v) >= pd_max / 2 (range and dust);
//! - both bits set: an object at v would probably be seen this frame
//!   (terrain occlusion excluded; it is a separate, static layer).

use crate::sensing::eyes;
use crate::sensing::optics::{Band, Optics};
use crate::vec3::Vec3;
use rayon::prelude::*;

pub const IN_VIEW: u8 = 1;
pub const DETECTABLE: u8 = 2;

/// The array's pose for one frame: Kindah's position and her eyes.
pub struct ArrayView<'a> {
    pub q: Vec3,
    pub eye_dirs: &'a [Vec3],
    pub tan_h: f64,
    pub tan_v: f64,
}

pub fn coverage(
    view: &ArrayView,
    optics: &Optics,
    band: Band,
    size: f64,
    voxels: &[Vec3],
) -> Vec<u8> {
    let (q, tan_h, tan_v) = (view.q, view.tan_h, view.tan_v);
    // Each eye's frame is computed once, not once per voxel.
    let frames: Vec<(Vec3, Vec3, Vec3)> = view
        .eye_dirs
        .iter()
        .map(|&f| {
            let (r, u) = eyes::basis(f);
            (f, r, u)
        })
        .collect();
    let half = 0.5 * optics.pd_max;
    voxels
        .par_iter()
        .map(|&v| {
            let d = v - q;
            let range = d.norm();
            if range < 1e-6 {
                return 0;
            }
            let u = d / range;
            let in_view = frames.iter().any(|(f, r, up)| {
                let fz = u.dot(*f);
                fz > 0.0 && (u.dot(*r) / fz).abs() <= tan_h && (u.dot(*up) / fz).abs() <= tan_v
            });
            let det = optics.pd(band, q, v, size) >= half;
            ((in_view as u8) * IN_VIEW) | ((det as u8) * DETECTABLE)
        })
        .collect()
}
