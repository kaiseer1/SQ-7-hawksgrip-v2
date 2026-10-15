# Consolidated test report: Phases 0–3

**Date:** 2026-10-15 · **Commit under test:** `59d1eca` (Phase 3, verified buildable state)
**Machine:** Intel i5-12400F (12 threads), 15.8 GiB RAM, RTX 3070 (CC 8.6, 7.1 / 8.2 GiB VRAM free), Windows 11
**Toolchain:** rustc 1.99.0, Python 3.12.10, numpy 2.5.4, torch 2.11.0 + CUDA 12.8

## Summary

| Check | Result |
|---|---|
| Environment check (`scripts/check_env.py`) | READY |
| Formatting (`cargo fmt --check`) | Clean |
| Lint (`cargo clippy --all-targets -- -D warnings`) | No issues |
| Rust tests, release build | **94 / 94 passed** (5 s) |
| Rust golden-hash tests, debug build | **15 / 15 passed**; same fingerprints as release |
| Python tests (incl. GPU) | **54 / 54 passed** (5.8 s) |
| Scenario runs with bit-exact replay verification | **6 / 6 verified** |
| **Fresh clone** of the four commits: Rust build + tests | **94 / 94 passed** (clean build 44 s) |
| **Fresh clone**: wheel built from the clone, Python tests | **54 / 54 passed** |

No test failed, no test was skipped, and the GPU tests ran on the RTX 3070.

## Rust suites (release)

| Suite | Tests | What it proves |
|---|---|---|
| Unit tests (`src/`) | 63 | Clock without drift; RNG reference vectors and stream independence; FNV-1a vectors; kinematic limits under 200 000 adversarial commands; orbit closed form; bilinear and Lipschitz bounds; noise; sector shapes and slope limits; LoS soundness vs dense reference; keep-out SDF and segment exactness; optics (Koschmieder, closed-form vs numerical optical depth, band switch); sweep tiling, R2 coverage, per-cycle permutation; frustum and saccade |
| `determinism.rs` | 7 | Same seed → same bits; seed sensitivity; rayon == sequential; replay checkpoints; tamper detection pinpoints divergence; batch identical on 1 vs 12 threads; golden fingerprint |
| `world_behaviour.rs` | 8 | Orbit held exactly; station keeping < 0.5 m; breach/ground/out-of-bounds rules; capacity limits; orders accepted/rejected; terrain only settable before tick 0 |
| `terrain_world.rs` | 8 | Golden terrain fingerprints for all 3 sectors; orbit clearance enforced; 0 ground impacts in terrain-following raids; wadi masking from the orbit; LoS vs dense reference on real sectors; keep-out and structure occlusion |
| `sensing_world.rs` | 8 | Kill-chain timeline order; "SHE LOOKS AT YOU" burst then sentinels; 40-drone saturation keeps sentinels and gives every hostile eyes; sandstorm band switch and shorter ranges; wildlife filter (all false alarms cleared); replay with storm + birds; sweep revisit ≤ 6 s (measured 2.87 s); Kalman estimates stay physical |

## Python suites

| File | Tests | Covers |
|---|---|---|
| `test_bridge.py` | 9 | Slot layout, clock, zero-allocation snapshots, buffer validation, events, orders, **GIL released during `step()`**, config errors, terrain install |
| `test_config.py` | 6 | Rust defaults as source of truth, partial overrides, typo rejection, shipped scenarios load and run |
| `test_determinism.py` | 5 | Hash equality, seed sensitivity, parallel bit-identity, replay tamper detection, batch vs single run |
| `test_driver.py` | 6 | Exact tick count over 1 h at 60 fps, jitter, overrun backlog dropping, time scale, real session |
| `test_gpu.py` | 3 | VRAM budget accounting, CUDA report, terrain tensor < 10 MB |
| `test_optics.py` | 7 | Band physics, sandstorm key, eyes/tracks/timelines bindings, CPU coverage flags, **GPU coverage agrees with Rust (> 99.9 %, clear and Shamal)**, GPU revisit accumulator |
| `test_terrain.py` | 18 | Terrain export and sampler agreement, sector character, LoS scalar vs batch, refinery queries, margin sweep, orbit clearance, procedural replay, DEM loader (.npy/.tif, north-up flip), **GPU vs CPU LoS (no GPU-blocked/CPU-visible case)** |

## Scenario replays

No interception exists in Phases 0–3, so every drone is *expected* to reach the
refinery. The breach counters confirm the rules are wired, and the zero
ground-impact counts confirm terrain following.

| Scenario | Breaches | Ground impacts | Speed | Replay |
|---|---|---|---|---|
| single_raid (flat) | 6 / 6 | 0 | 2 219× real time | verified, 240 checkpoints |
| waves (flat) | 48 / 48 | 0 | 1 037× | verified, 420 |
| sector1_sabkha | 16 / 16 | 0 | 1 505× | verified, 360 |
| sector2_redsea | 16 / 16 | 0 | 1 230× | verified, 360 |
| sector3_wadi | 16 / 16 | 0 | 1 216× | verified, 360 |
| shamal_birds (LWIR, 20 birds) | 16 / 16 | 0 | 1 117× | verified, 360 |

## Performance re-check (against `docs/claims.md`)

| Measurement | Today | Recorded | Note |
|---|---|---|---|
| Python → Rust call | 47 ns | 31–33 ns | Higher today, still < 1 % of a tick; recorded value kept, range noted |
| Tick, 64 threats + 24 eyes, flat | 15.3 µs (546×) | 15.1 µs (551×) | Consistent |
| Sabkha / Red Sea / Highland | 17.0 / 22.1 / 24.2 µs | 16.8 / 21.8 / 24.4 µs | Consistent (within 2 %) |

GPU coverage, LoS throughput and the detection study were measured in Phases 2–3
(`results/bench/*.json`) and were not re-run here, so the committed result files
are unchanged.

## What these tests do *not* establish

- **Cross-OS determinism:** engineered (pure-Rust `libm` for every transcendental
  in the state path) and identical in debug/release, but only ever executed on
  Windows. A Linux run of the golden tests is still needed.
- **Physical validity of SWEPT parameters:** tests prove the code implements
  the models; they do not prove the parameter values (docs/claims.md lists
  each as DESIGN, SWEPT, SOURCE or TO CITE).
- **Data association:** detections use truth association by design (ADR-0011).
- **Interception and defence outcomes:** out of scope for Phases 0–3.
- **One machine only:** all timings are from the reference machine above.

## Reproduce

```bash
cargo test --workspace --release
```

```bash
.venv/Scripts/python -m pytest
```

```bash
.venv/Scripts/python -m hawksgrip.app --scenario scenarios/shamal_birds.toml
```

## Addendum: showroom HMI and persona (2026-10-15)

| Check | Result |
|---|---|
| Rust tests, release (incl. 6 persona unit tests, persona invariance, event-name round-trip) | **102 / 102** |
| Golden fingerprint and persona invariance, debug build | pass (state hash unchanged by the showroom work) |
| Python tests (incl. 11 HMI tests) | **65 / 65** |
| HMI: drawn frustums vs engine detection frustums (20 000 random points) | ≥ 99.95 % agreement |
| HMI: live window smoke test | renders a non-blank 960 × 540 frame; `T`, `S`, `R`, `K`, `G`, `C`, `Space` and Shift+click behave as specified |
| Persona invariance through Python (voice toggled every 7 calls) | identical state hash |
| Showroom frame rate, 10 drones + 12 birds (+ GPU layer, + Shamal) | median 5.5 / 8.1 / 8.8 ms, p95 11.0 / 14.1 / 13.1 ms; ~3.5 % of frames (the 2 Hz HUD refresh) over 16.7 ms |
