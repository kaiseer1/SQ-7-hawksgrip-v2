//! Phase-1 guidance laws. Each turns a goal into a *commanded* acceleration;
//! `point_mass::step` then decides what the airframe can actually do.
//!
//! Pursuit guidance (lead pursuit / proportional navigation) arrives in Phase 6.

use crate::kinematics::KState;
use crate::terrain::Heightfield;
use crate::vec3::Vec3;

/// PD tracking of a moving reference (station keeping in the Queen's frame):
/// a = kp (p_ref - p) + kd (v_ref - v) + a_ref.
pub fn track(s: &KState, reference: &KState, kp: f64, kd: f64) -> Vec3 {
    (reference.p - s.p) * kp + (reference.v - s.v) * kd + reference.a
}

/// Fly toward `target` at `speed`. With `arrive`, slow so the body can stop
/// at the target: |v_des| <= sqrt(2 a_brake d), the stopping-distance bound.
pub fn waypoint(s: &KState, target: Vec3, speed: f64, kv: f64, arrive: bool, a_brake: f64) -> Vec3 {
    let d = target - s.p;
    let dist = d.norm();
    let mut sp = speed;
    if arrive {
        sp = sp.min((2.0 * a_brake * dist).sqrt());
    }
    let v_des = d.normalized_or_zero() * sp;
    (v_des - s.v) * kv
}

/// Station slot `i` of the Seven: a ring under the Queen, fixed in her
/// heading frame so the formation turns with her.
pub fn station_slot(queen: &KState, index: usize, count: usize, radius: f64, below: f64) -> KState {
    let heading = queen.v.horizontal().normalized_or_zero();
    let (fx, fy) = if heading == Vec3::ZERO {
        (1.0, 0.0)
    } else {
        (heading.x, heading.y)
    };
    let ang = std::f64::consts::TAU * index as f64 / count as f64;
    let (s, c) = crate::math::sin_cos(ang);
    // Body frame: forward = heading, left = heading rotated +90 degrees.
    let off = Vec3::new(
        radius * (c * fx - s * fy),
        radius * (c * fy + s * fx),
        -below,
    );
    // The slot rotates about the Queen at her yaw rate r = (v x a)_z / |v_h|^2,
    // which adds r z_hat x off to its velocity and -r^2 off_h to its acceleration.
    let vh2 = queen.v.horizontal().norm_sq();
    let yaw = if vh2 > 1e-9 {
        (queen.v.x * queen.a.y - queen.v.y * queen.a.x) / vh2
    } else {
        0.0
    };
    let v_off = Vec3::new(-yaw * off.y, yaw * off.x, 0.0);
    let a_off = off.horizontal() * (-yaw * yaw);
    KState {
        p: queen.p + off,
        v: queen.v + v_off,
        a: queen.a + a_off,
    }
}

/// The ground and the terrain-following parameters, bundled.
#[derive(Clone, Copy, Debug)]
pub struct FollowEnv<'a> {
    pub hf: &'a Heightfield,
    pub tf: TerrainFollow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainFollow {
    pub agl: f64,
    pub lookahead_s: f64,
    pub dive_radius: f64,
    pub kz: f64,
}

/// Straight-line flight to `target` that never lets the ground ahead come
/// closer than `agl`.
///
/// It samples the ground along the horizontal track for the next
/// `lookahead_s` seconds. For each sample at time tau, the climb rate needed
/// to be at floor = ground + agl by then is (floor - z) / tau; the largest of
/// these (and kz * (floor - z) for the point underneath) becomes the vertical
/// velocity command. Total speed is preserved by trading horizontal speed for
/// climb. Within `dive_radius` of the aim point the floor is released.
///
/// The 8-sample look-ahead is the expensive part (9 terrain lookups). It is
/// recomputed only when `refresh` is set (the world staggers this at 30 Hz)
/// and cached in `lookahead_vz` in between; the floor directly underneath is
/// checked every tick. Between refreshes the cached climb rate was computed
/// from a slightly lower altitude, so it can only overstate the climb needed
/// (conservative).
pub fn waypoint_terrain_following(
    s: &KState,
    target: Vec3,
    speed: f64,
    kv: f64,
    env: &FollowEnv<'_>,
    lookahead_vz: &mut f64,
    refresh: bool,
) -> Vec3 {
    let (hf, tf) = (env.hf, &env.tf);
    let d = target - s.p;
    let dh = d.horizontal();
    let dist_h = dh.norm();
    let mut v_des = d.normalized_or_zero() * speed;
    if dist_h > tf.dive_radius {
        let dir_h = dh / dist_h;
        let vh = v_des.horizontal().norm().max(1.0);
        let floor_here = hf.height(s.p.x, s.p.y) + tf.agl;
        if refresh || *lookahead_vz == f64::NEG_INFINITY {
            const SAMPLES: u32 = 8;
            let mut ahead = f64::NEG_INFINITY;
            for k in 1..=SAMPLES {
                let tau = tf.lookahead_s * k as f64 / SAMPLES as f64;
                let q = s.p + dir_h * (vh * tau);
                let floor = hf.height(q.x, q.y) + tf.agl;
                ahead = ahead.max((floor - s.p.z) / tau);
            }
            *lookahead_vz = ahead;
        }
        let vz_req = (tf.kz * (floor_here - s.p.z)).max(*lookahead_vz);
        if vz_req > v_des.z {
            let vz = vz_req.min(0.8 * speed);
            let vh_new = (speed * speed - vz * vz).max(0.0).sqrt();
            v_des = dir_h * vh_new + Vec3::new(0.0, 0.0, vz);
        }
    }
    (v_des - s.v) * kv
}
