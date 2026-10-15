# Phase 1 Red-Team attack log: deterministic core loop

`[AGENT: ADVERSARIAL BREAKER (RED-TEAM)]`: *"'Deterministic' is the word your committee will
test first. 'Real-time' is the second. I attacked both, and your own
benchmark, before they could."*

## Findings

| # | Attack | Evidence | Fix | Status |
|---|---|---|---|---|
| P1-1 | **"Your benchmark is lying."** Timing loops of 20 000 ticks ran past the moment threats breach and despawn, so "64 threats" partly measured an empty sky | First run: 1.02 µs/tick. Corrected run: **4.08 µs/tick** (4× higher). Real-time factor 10 700× → **2 100×** | `time_ticks()` uses a fresh session per round, keeps each round under 120 simulated seconds, and **asserts all 64 threats are still alive** after timing | Fixed |
| P1-2 | "Throw rayon at the tick loop: 12 threads must be faster." | Per-tick rayon: **15.2 µs** vs sequential **4.1 µs** (3.7× *slower*). Fork/join overhead dwarfs 71 bodies × ~55 ns | Sequential per tick by default. Parallelism goes **across independent worlds**: 48-run batch **6.6× faster on 12 threads** (4.1× on 6). Rayon per-tick kept behind `set_parallel()` only to prove bit-identity | Fixed |
| P1-3 | "Do LoS on the GPU every tick." (Red-Team #1, now measured) | GPU round trip **66–77 µs** = 16–19 entire CPU ticks; empty launch + sync 19–26 µs. Dense 98 k rays × 256 samples: **1.9–2.3 ms** | Confirms ADR-0003: CPU-authoritative LoS, GPU for dense grids | Confirmed |
| P1-4 | "'Zero-copy saves time' is folklore at 72 bodies." | `snapshot_into` (pre-allocated) **0.32 µs** vs allocating `positions()` **0.18 µs** | Claim withdrawn. ADR-0006 keeps copy-into-buffers for **safety** (no aliasing while the GIL is released) and stable addresses, not speed | Corrected |
| P1-5 | "Your replays only replay on *your* PC." sin/cos come from the platform C library and can differ in the last bit between MSVC, glibc and macOS | Orbit, station slots and raid azimuths all used `f64::sin_cos` | State-path trig moved to pure-Rust `libm` (`math.rs`), agreeing with std to ≤ 2 ulp. **Golden fingerprint** `0x02CB1F3F57C3C200` pinned; identical in debug and release | Fixed (Linux run open) |
| P1-6 | "`t += dt` drifts." | Test: 432 000 × (1/120) summed in f64 ≠ 3600.0 | Time = tick / hz; driver bookkeeping in integer ns (exact over 1 h at 60 fps, tested) | Fixed |
| P1-7 | "Your turn-rate test passes for the wrong reason." | First version: at 40 m/s, a_max (0.625 rad/s) binds before ω_max = 2.5 rad/s, so it never tested ω_max, and it failed | Rewritten at 5 m/s, where ω·v = 12.5 < a_max: heading rate reaches **≥ 99 % of ω_max and never exceeds it** | Fixed |
| P1-8 | "Adversarial commands will break your limiter." | 200 000 random, discontinuous commands up to 400 m/s² | Speed, acceleration and heading-rate limits never exceeded (proof in ADR-0002: speed projection is non-expansive) | Verified |
| P1-9 | "A late input changes history." | Python calls arrive at arbitrary wall times | Inputs apply at the next tick boundary and are logged with it; a 10⁻⁹ change to one input is detected by replay verification at the first following checkpoint | Verified |
| P1-10 | "Queen orbit deviation 0.00 m is a tautology." | Her state is closed-form, so the deviation is 0 by construction | Counter kept for theatre and as a tripwire; `aegis_interventions` and `threats_reaching_queen` counters added beside it (ADR-0008 R6) | Mitigated |
| P1-11 | "If Python stalls, events are lost silently." | Ring buffer of 4 096 | Overflow drops the *oldest* and counts it (`events_dropped` on the HUD); steady state never reallocates (tested) | Mitigated |
| P1-12 | "GIL release is a claim, not a fact." | | Test: a Python thread completes > 1 000 iterations during one `step()` call | Verified |

## Open items (carried to ROADMAP)

- **Cross-OS run:** the golden hash is engineered to be portable but has only run on Windows.
- **Hold-point chatter:** the √(2·a·d) arrival profile has infinite slope at d = 0.
  Test shows |v| < 1 m/s at hold, but a linear terminal zone should replace it in Phase 6.
- **Jerk yield rate:** how often the jerk limit gives way to re-projection is not yet measured (Phase 4).
- **Orbit terrain clearance:** orbit altitude is set from ground height at the centre only (Phase 2 must check the whole circle).

## Verdict

Phase 1 exit criteria met: bit-identical replays (verified with checkpoints and
a golden hash) and a headless real-time factor of ≈ 2 100× with 64 live
threats, against a target of ≥ 500×. Two marketing-style numbers (P1-1, P1-4)
were caught and corrected before they reached the thesis.
