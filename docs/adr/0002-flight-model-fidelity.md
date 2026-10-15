# ADR-0002: 3-DOF point-mass flight model

**Status:** Accepted (user decision Q2-A, 2026-10-14) · **Implemented:** Phase 1

## Decision

Each body is a point with position **p**, velocity **v** and acceleration **a**
(ENU metres). Guidance produces a *commanded* acceleration; the airframe model
in `kinematics/point_mass.rs` decides what is actually achievable:

1. **Turn rate:** split a into tangential and normal parts relative to v;
   heading rate is |aₙ|/|v|, so clamp |aₙ| ≤ ω_max·|v| (skipped below
   `v_turn_eps`, since hover-capable bodies can accelerate sideways from rest).
2. **Acceleration:** |a| ≤ a_max (projection onto a ball).
3. **Jerk:** |a − a_prev| ≤ j_max·dt (motor/ESC response).
4. **Re-project** onto 1–2 (v changed since a_prev). Airframe limits are hard;
   jerk gives way in that rare case.
5. **Semi-implicit Euler:** v′ = clamp(v + a·dt, v_max); p′ = p + v′·dt.
   The stored acceleration is the effective one, (v′ − v)/dt.

**Why the speed clamp cannot break the acceleration limit (proof):** Projection P onto a closed
convex set is non-expansive: |P(x) − P(y)| ≤ |x − y|. Since |v| ≤ v_max,
P(v) = v, so |v′ − v| = |P(v + a·dt) − P(v)| ≤ |a|·dt ≤ a_max·dt. ∎
A 200 000-step adversarial random-command test confirms speed, acceleration
and heading-rate limits are never exceeded.

The Queen is the exception: her state is a closed-form orbit (ADR-0007), and
her orbit's feasibility (V/R ≤ ω_max, V²/R ≤ a_max, V ≤ v_max) is checked when
the configuration loads.

## Alternatives

| Model | Why not |
|---|---|
| 6-DOF rigid body (attitude, rotor thrust) | ~10× compute per body; needs inertia and aero data we do not have for COTS frames; the thesis contribution is allocation, sensing and energy, not attitude control |
| Pure kinematic (teleport to commanded velocity) | No acceleration or turn limits, so interception and escape-envelope maths would be meaningless |
| Explicit Euler | Less stable for oscillatory station keeping; same cost |

## Consequences

- Escape envelopes (Phase 6) are reachable sets of this model: closed-form bounds exist.
- The energy model (Phase 4) consumes |a|, |v| and climb rate from this state.
- Limits for the Seven and the threats are **SWEPT** placeholders until the
  user's reference diagrams arrive (see claims.md).
