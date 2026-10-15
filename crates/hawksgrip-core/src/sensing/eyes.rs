//! One facet ("eye") of Kindah's compound optical array: a rectangular
//! frustum on a fast gimbal.
//!
//! - **Frustum test.** With forward f, right r = norm(f x up), up' = r x f,
//!   a direction u is inside iff u.f > 0, |u.r / u.f| <= tan(h/2) and
//!   |u.up' / u.f| <= tan(v/2). The frame is kept level with the horizon.
//! - **Saccade.** The pointing direction rotates along the great circle
//!   toward its aim by at most slew_rate * dt per tick. An eye only "sees"
//!   (contributes detections) when settled within `settle_rad` of its aim:
//!   frames captured mid-saccade are motion-smeared.

use crate::vec3::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Task {
    /// State 1: sweeping its sector of the search region.
    Sweep,
    /// Staring at a fresh detection to confirm it (M-of-N).
    Confirm { track: u32 },
    /// Assigned to a track (State 2 convergence, or a TDM share).
    Track { track: u32 },
}

impl Task {
    pub fn code(&self) -> u64 {
        match self {
            Task::Sweep => 0,
            Task::Confirm { track } => (1 << 32) | *track as u64,
            Task::Track { track } => (2 << 32) | *track as u64,
        }
    }

    pub fn track(&self) -> Option<u32> {
        match self {
            Task::Sweep => None,
            Task::Confirm { track } | Task::Track { track } => Some(*track),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eye {
    /// Current pointing direction (unit).
    pub dir: Vec3,
    /// Where it is being driven (unit).
    pub aim: Vec3,
    pub task: Task,
    pub settled: bool,
    /// Index of this eye's sector in the current sweep tiling.
    pub sector: usize,
    /// Position in its R2 sequence and the per-eye random rotation.
    pub sweep_k: u64,
    pub shift: [f64; 2],
    /// Frames left to stare at the current sweep point.
    pub dwell_left: u32,
}

/// Right and up vectors of a level frustum looking along `f` (unit).
pub fn basis(f: Vec3) -> (Vec3, Vec3) {
    let up = Vec3::new(0.0, 0.0, 1.0);
    let mut r = cross(f, up);
    if r.norm() < 1e-9 {
        r = Vec3::new(1.0, 0.0, 0.0); // looking straight up or down
    }
    let r = r.normalized_or_zero();
    (r, cross(r, f))
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}

/// Is unit direction `u` inside the frustum looking along `f`?
pub fn in_frustum(f: Vec3, tan_h: f64, tan_v: f64, u: Vec3) -> bool {
    let fz = u.dot(f);
    if fz <= 0.0 {
        return false;
    }
    let (r, up) = basis(f);
    (u.dot(r) / fz).abs() <= tan_h && (u.dot(up) / fz).abs() <= tan_v
}

/// Angle between unit vectors (radians), robust near 0 and pi.
pub fn angle(a: Vec3, b: Vec3) -> f64 {
    let c = cross(a, b).norm();
    let d = a.dot(b);
    libm::atan2(c, d)
}

/// Rotate unit `dir` toward unit `aim` along the great circle by at most `max_angle`.
pub fn slew(dir: Vec3, aim: Vec3, max_angle: f64) -> Vec3 {
    let th = angle(dir, aim);
    if th <= max_angle {
        return aim;
    }
    let mut w = aim - dir * dir.dot(aim);
    if w.norm() < 1e-12 {
        w = basis(dir).0; // antipodal: any perpendicular will do
    }
    let w = w.normalized_or_zero();
    let (s, c) = crate::math::sin_cos(max_angle);
    (dir * c + w * s).normalized_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frustum_edges() {
        let f = Vec3::new(1.0, 0.0, 0.0);
        let (th, tv) = (6f64.to_radians().tan(), 4.5f64.to_radians().tan());
        assert!(in_frustum(f, th, tv, f));
        let inside = Vec3::new(1.0, 5.9f64.to_radians().tan(), 0.0).normalized_or_zero();
        let outside = Vec3::new(1.0, 6.1f64.to_radians().tan(), 0.0).normalized_or_zero();
        assert!(in_frustum(f, th, tv, inside));
        assert!(!in_frustum(f, th, tv, outside));
        assert!(!in_frustum(f, th, tv, -f));
        // Vertical half-angle is the narrower one.
        let up5 = Vec3::new(1.0, 0.0, 5f64.to_radians().tan()).normalized_or_zero();
        assert!(!in_frustum(f, th, tv, up5));
    }

    #[test]
    fn frustum_looking_straight_down() {
        let f = Vec3::new(0.0, 0.0, -1.0);
        let t = 6f64.to_radians().tan();
        assert!(in_frustum(f, t, t, f));
        assert!(in_frustum(
            f,
            t,
            t,
            Vec3::new(0.05, 0.0, -1.0).normalized_or_zero()
        ));
    }

    #[test]
    fn slew_is_rate_limited_and_converges() {
        let mut d = Vec3::new(1.0, 0.0, 0.0);
        let aim = Vec3::new(0.0, 0.0, -1.0); // 90 degrees away
        let step = 600f64.to_radians() / 120.0; // 600 deg/s at 120 Hz
        let mut ticks = 0;
        while d != aim {
            let nd = slew(d, aim, step);
            assert!(angle(d, nd) <= step + 1e-12);
            assert!((nd.norm() - 1.0).abs() < 1e-12);
            d = nd;
            ticks += 1;
        }
        // 90 deg at 600 deg/s = 0.15 s = 18 ticks (+1 for floating-point residue).
        assert!((18..=19).contains(&ticks), "{ticks}");
        let anti = slew(Vec3::new(1.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0), 0.1);
        assert!((angle(Vec3::new(1.0, 0.0, 0.0), anti) - 0.1).abs() < 1e-9);
    }
}
