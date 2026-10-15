# ADR-0007: Determinism strategy

**Status:** Accepted · **Implemented:** Phase 1

**Claim:** given (config, seed, terrain, input log), the simulation produces
the same state **bit for bit**: across runs, across thread counts, in debug
and release builds, and (by design, see limits below) across operating systems.

## Mechanisms

| Threat to determinism | Mechanism | Test |
|---|---|---|
| Time drift from `t += dt` | Time = tick / hz (integer tick) | `no_drift_after_one_hour` |
| Frame-rate dependence | Lockstep driver in integer ns (ADR-0001) | `test_one_hour_at_60fps_is_exact` |
| RNG coupling between subsystems | xoshiro256++ with **named streams** (Scenario, Sensor, Auction, …) | `streams_are_independent` |
| RNG library upgrades | RNG implemented in-repo (40 lines), SplitMix64 reference vector tested | `splitmix64_reference_vector` |
| Thread scheduling order | rayon only over *independent* bodies or runs; ordered `collect`; rules applied sequentially in slot order | `rayon_parallel_equals_sequential`, batch 1 vs 12 threads |
| Inputs arriving mid-tick | Inputs queue and apply at the next tick boundary, logged with that tick | replay tests |
| Platform libm differences in sin/cos | Pure-Rust `libm` crate in the state path (`math.rs`) | `agrees_with_std_to_an_ulp` |
| Compiler float re-association | No fast-math flags anywhere; Rust guarantees IEEE-754 semantics | debug == release golden hash |
| GPU reduction order | GPU output never feeds state (ADR-0003) | n/a by design |

## Verification

- **State hash:** FNV-1a over the exact bit patterns of every live body's p, v, a and behaviour.
- **Checkpoints:** a hash every 120 ticks (1 s) is stored in the replay. On
  mismatch, `verify` reports the first divergent second.
- **Sensitivity:** changing one input by 1 part in 10⁹ is detected (pytest).
- **Golden fingerprint:** a fixed 2-minute raid must hash to
  `0x02CB1F3F57C3C200`; identical in debug and release on Windows/MSVC.

## Known limits

- Cross-OS identity is engineered (libm, no platform trig) but so far only
  **executed on Windows**. A Linux CI run of the golden test is the open item.
- Replays are tied to `REPLAY_FORMAT = 1` and record `core_version`.
  Intentional behaviour changes move the golden hash and must be committed together.
