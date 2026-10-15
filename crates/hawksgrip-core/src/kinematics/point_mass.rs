//! 3-DOF point-mass kinematics with acceleration, turn-rate, jerk and speed
//! limits (ADR-0002).
//!
//! Per tick:
//! 1. **Turn-rate limit.** Split the commanded acceleration into a part along
//!    the velocity (speed change) and a part across it (direction change).
//!    Heading changes at rate |a_n| / |v|, so |a_n| <= omega_max * |v|.
//! 2. **Acceleration limit.** |a| <= a_max (projection onto a ball).
//! 3. **Jerk limit.** |a - a_prev| <= j_max * dt (models motor/ESC response).
//! 4. Re-project onto steps 1–2, because the velocity has changed since
//!    `a_prev` was produced. Airframe limits (1, 2) are hard; jerk is the
//!    smoothing limit that gives way in that rare case.
//! 5. **Semi-implicit Euler:** v' = clamp(v + a dt, v_max), p' = p + v' dt.
//!    The stored acceleration is the *effective* one, (v' - v) / dt.
//!    Projection onto the speed ball is non-expansive and v is already inside
//!    it, so |effective a| <= |a| <= a_max. The speed clamp never breaks the
//!    acceleration limit.

use crate::vec3::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Maximum speed (m/s).
    pub v_max: f64,
    /// Maximum acceleration magnitude (m/s^2).
    pub a_max: f64,
    /// Maximum rate of change of acceleration (m/s^3).
    pub j_max: f64,
    /// Maximum heading rate (rad/s).
    pub omega_max: f64,
    /// Below this speed the turn-rate limit is not applied (hover-capable).
    pub v_turn_eps: f64,
}

impl Limits {
    pub fn validate(&self, name: &str) -> Result<(), String> {
        for (field, v) in [
            ("v_max", self.v_max),
            ("a_max", self.a_max),
            ("j_max", self.j_max),
            ("omega_max", self.omega_max),
        ] {
            if !(v.is_finite() && v > 0.0) {
                return Err(format!("{name}.{field} must be positive and finite"));
            }
        }
        if !(self.v_turn_eps.is_finite() && self.v_turn_eps >= 0.0) {
            return Err(format!("{name}.v_turn_eps must be >= 0"));
        }
        Ok(())
    }
}

/// Kinematic state of one body.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct KState {
    pub p: Vec3,
    pub v: Vec3,
    pub a: Vec3,
}

/// Limit the across-velocity acceleration to omega_max * |v|.
pub fn clamp_turn(v: Vec3, a: Vec3, lim: &Limits) -> Vec3 {
    let speed = v.norm();
    if speed < lim.v_turn_eps || speed == 0.0 {
        return a;
    }
    let t = v / speed;
    let a_t = a.dot(t);
    let a_n = a - t * a_t;
    let max_n = lim.omega_max * speed;
    let n = a_n.norm();
    if n > max_n {
        t * a_t + a_n * (max_n / n)
    } else {
        a
    }
}

/// Turn the commanded acceleration into one the airframe can actually produce.
pub fn constrain_accel(v: Vec3, a_prev: Vec3, a_cmd: Vec3, lim: &Limits, dt: f64) -> Vec3 {
    let target = clamp_turn(v, a_cmd, lim).clamp_norm(lim.a_max);
    let da = target - a_prev;
    let a = a_prev + da.clamp_norm(lim.j_max * dt);
    clamp_turn(v, a, lim).clamp_norm(lim.a_max)
}

/// Advance one body by one tick.
pub fn step(s: KState, a_cmd: Vec3, lim: &Limits, dt: f64) -> KState {
    let a = constrain_accel(s.v, s.a, a_cmd, lim, dt);
    let v = (s.v + a * dt).clamp_norm(lim.v_max);
    let p = s.p + v * dt;
    KState {
        p,
        v,
        a: (v - s.v) / dt,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::rng::Rng;

    const LIM: Limits = Limits {
        v_max: 50.0,
        a_max: 25.0,
        j_max: 150.0,
        omega_max: 2.5,
        v_turn_eps: 1.0,
    };
    const DT: f64 = 1.0 / 120.0;

    #[test]
    fn hard_limits_never_violated_under_random_commands() {
        let mut rng = Rng::from_seed(99);
        let mut s = KState::default();
        for i in 0..200_000 {
            // Wild, discontinuous commands: the worst case for a limiter.
            let k = if i % 500 < 250 { 400.0 } else { 3.0 };
            let a_cmd = Vec3::new(rng.uniform(-k, k), rng.uniform(-k, k), rng.uniform(-k, k));
            let prev = s;
            s = step(s, a_cmd, &LIM, DT);

            assert!(s.v.norm() <= LIM.v_max * (1.0 + 1e-12), "speed");
            assert!(
                s.a.norm() <= LIM.a_max * (1.0 + 1e-9),
                "accel {}",
                s.a.norm()
            );

            // Heading rate measured from the actual velocity change.
            let sp = prev.v.norm();
            if sp >= LIM.v_turn_eps {
                let t = prev.v / sp;
                let a_n = s.a - t * s.a.dot(t);
                assert!(
                    a_n.norm() <= LIM.omega_max * sp * (1.0 + 1e-9) + 1e-9,
                    "turn"
                );
            }
            assert!(s.p.is_finite());
        }
    }

    #[test]
    fn jerk_limited_straight_line_ramp() {
        // From rest, command full forward accel: a ramps at exactly j_max.
        let mut s = KState::default();
        let a_cmd = Vec3::new(1000.0, 0.0, 0.0);
        let mut prev_a = 0.0;
        for _ in 0..30 {
            s = step(s, a_cmd, &LIM, DT);
            let da = s.a.x - prev_a;
            assert!(da <= LIM.j_max * DT * (1.0 + 1e-9));
            prev_a = s.a.x;
        }
        // After 20 ticks (1/6 s) at 150 m/s^3 the ramp reaches the 25 m/s^2 cap.
        assert!((s.a.x - LIM.a_max).abs() < 1e-9);
    }

    #[test]
    fn reaches_and_holds_v_max() {
        let mut s = KState::default();
        for _ in 0..(120 * 10) {
            s = step(s, Vec3::new(0.0, 100.0, 0.0), &LIM, DT);
        }
        assert!((s.v.norm() - LIM.v_max).abs() < 1e-9);
        assert!(s.a.norm() < 1e-9);
    }

    #[test]
    fn heading_rate_caps_at_omega_max() {
        // At 5 m/s, omega_max * v = 12.5 m/s^2 < a_max, so the turn-rate limit
        // (not the acceleration limit) binds. Demand a hard turn, always
        // perpendicular to the current velocity: the heading rate must climb
        // to exactly omega_max and never exceed it.
        let mut s = KState {
            p: Vec3::ZERO,
            v: Vec3::new(5.0, 0.0, 0.0),
            a: Vec3::ZERO,
        };
        let mut max_rate: f64 = 0.0;
        for _ in 0..240 {
            let h0 = s.v.y.atan2(s.v.x);
            let left = Vec3::new(-s.v.y, s.v.x, 0.0).normalized_or_zero() * 1000.0;
            s = step(s, left, &LIM, DT);
            let mut dh = s.v.y.atan2(s.v.x) - h0;
            if dh < -std::f64::consts::PI {
                dh += std::f64::consts::TAU;
            }
            max_rate = max_rate.max(dh.abs() / DT);
        }
        assert!(max_rate <= LIM.omega_max * (1.0 + 1e-6), "{max_rate}");
        assert!(max_rate > LIM.omega_max * 0.99, "{max_rate}");
    }
}
