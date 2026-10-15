# Viva Defence Cheat-Sheet: Phase 1 (deterministic core loop)

`[AGENT: THESIS DEFENSE COACH & EXPLAINER]` with input from
`[AGENT: KINEMATICS & PHYSICS]` and `[AGENT: SOTA TRENDS & THESIS ARCHITECT]`

## 1. Plain-English pitch

The simulator advances the world in fixed slices of 1/120 of a second, and
every random choice comes from a numbered seed. So any run, including any
failure, can be replayed **exactly**, down to the last bit, and checked
automatically. On my desktop it simulates a 64-drone raid about **2 000 times
faster than real time**, so thousands of experiments take minutes, not days.

## 2. Mathematical core

**(a) Fixed-step time.** tₖ = k / 120. Time is derived from an integer tick,
never accumulated, so after one hour it reads exactly 3600 s. (A test proves
that the naive float sum does *not*.)

**(b) Point-mass airframe with limits (ADR-0002).** Per tick, with commanded
acceleration a_cmd:

```
a_n ≤ ω_max · |v|          (turn rate)
|a| ≤ a_max                (acceleration)
|a − a_prev| ≤ j_max · dt  (jerk / motor response)
v' = Proj_{|v|≤v_max}(v + a·dt),   p' = p + v'·dt     (semi-implicit Euler)
```

*Key lemma:* projection onto a convex ball is **non-expansive**, and v is
already inside it, so |v′ − v| ≤ |a|·dt. Clamping speed can never create an
acceleration above a_max. Verified on 200 000 adversarial random commands.

**(c) Kindah's orbit is closed-form.**

```
p(t) = c + R(cos θ, sin θ, 0),  θ = θ₀ + ωt,  ω = V/R
```

Its feasibility (V/R ≤ ω_max, V²/R ≤ a_max) is checked at config load.
"Unbroken orbit" is a configuration-time guarantee, not a hope.

**(d) Station keeping of the Seven** is PD tracking of a moving slot,
including the slot's rotation with her heading (yaw rate
r = (v × a)_z / |v_h|²): a = k_p·e_p + k_d·e_v + a_ref, with ζ = k_d / (2√k_p) ≈ 0.89.
Measured station error after 2 minutes: < 0.5 m.

**(e) Determinism and verification (ADR-0007):**
- xoshiro256++ RNG with **named streams**, so adding sensor noise cannot shift threat spawns.
- FNV-1a hash of exact bit patterns, with a checkpoint every second.
- Replay = (config, seed, terrain fingerprint, input log).
- A 10⁻⁹ change in one input is detected at the next checkpoint.
- Golden fingerprint `0x02CB1F3F57C3C200` is identical in debug and release builds.

**Why these choices over alternatives:**

| Chosen | Alternative | Reason |
|---|---|---|
| Semi-implicit Euler | Explicit Euler | Same cost, stable for oscillatory station keeping |
| Semi-implicit Euler | RK4 | 4× cost; accuracy dominated by the limits, not the integrator, at dt = 8.3 ms |
| xoshiro256++ | Mersenne Twister | 32 bytes of state vs 2.5 KB; faster; trivially splittable into streams |
| Parallel across worlds | Parallel within a tick | Measured: within-tick rayon is 3.7× *slower*; across worlds is 6.6× faster on 12 threads |

## 3. Committee Q&A prep

**Toughest question:** *"Your simulation is 'deterministic'. Prove it, and tell me where it would break."*

**Winning answer:**

"Determinism here means: same configuration, seed, terrain and input log
gives the same state, bit for bit. I prove it three ways, all automated:
1. **Replay checkpoints:** a state hash every simulated second; re-running
   the input log must reproduce all of them.
2. **Parallel equivalence:** the same run on 1 and on 12 threads gives
   identical hashes.
3. **A golden fingerprint** for a fixed raid, identical in debug and release builds.

As for where it could break:
- **Platform maths libraries.** sin and cos aren't correctly rounded by
  IEEE-754, so I use a pure-Rust libm in the state path.
- **The GPU.** Its reduction order isn't deterministic, so GPU results never
  feed simulation state; they're used for coverage maps and Monte Carlo statistics only.
- **Cross-OS.** I've engineered for it but only executed it on Windows, so a
  Linux run of the golden test is on my roadmap. I'd rather tell you that
  than claim it."

**Likely follow-up:** *"Why is your Queen's orbit deviation always exactly zero? Isn't that a fake metric?"*
→ "By construction, yes, and I say so: her state is a closed-form function of
time, so no code path can perturb it. I show it next to the counters that
*can* move, the Royal Aegis interventions and threats reaching her. The
deviation counter stays as a tripwire: if any future code ever perturbs her,
it stops reading zero."

**Second follow-up:** *"Your HUD says 2 000× real time. Measured how?"*
→ "`scripts/bench_bridge.py`. A fresh session per round, 64 live threats, and
an assertion that all 64 are still airborne when timing ends. My first version
didn't have that check: threats were landing mid-benchmark and the figure was
inflated 5×. My Red-Team log records the correction."
