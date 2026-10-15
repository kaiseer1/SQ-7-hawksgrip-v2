# Phase 3 Red-Team attack log: the 24-eye compound optical array

`[AGENT: ADVERSARIAL BREAKER (RED-TEAM)]`: *"Twenty-four eyes sound
omniscient. I measured where they weren't looking, what they mistook for
birds, and how long a drone flies before she actually locks it."*

All numbers are from `scripts/bench_optics.py`, `scripts/render_optics.py` and
`crates/hawksgrip-core/tests/sensing_world.rs` (i5-12400F + RTX 3070).

## Findings

| # | Attack | Evidence | Fix | Status |
|---|---|---|---|---|
| P3-1 | **"Random sweeping leaves blind spots."** | Raw R2 points per sector: a direction went **6.7 s** unobserved (1 296-direction probe, 20 s) | Footprint-sized cells visited once per cycle in a per-cycle permuted order with R2 jitter. Worst revisit **2.4 s** (2.87 s once the nadir cap was added); at 150 m AGL median 1.3 s, p95 2.1 s | Fixed |
| P3-2 | **"Confirmation stares eat the sentinels."** | 40-drone raid: sweeping eyes fell to **0** | Confirmation and classification stares never take the sweeping count below `sentinel_eyes`; saturation test asserts ≥ 4 sweepers outside a burst | Fixed |
| P3-3 / P3-8 | **"Two-pixel birds look like drones, and vice versa."** | First classifier: 7 of 17 birds declared hostile; 3 of 4 drones first read as birds | Asymmetric classifier (bias toward "drone") + re-examination when seen with 1.5× the pixels + post-lock second look. Clear weather, 384 drones: **0 final misreads**; 18 early misreads all recovered | Fixed |
| P3-4 | "Untestable birds hog eyes." | Far birds (< 2 px) held confirmation eyes indefinitely | A stare is held only while the track is tentative or classifiable | Fixed |
| P3-5 | "A 'bird' verdict is forever." | Misread drones were only re-checked within 2 km of the refinery | Re-examine whenever seen with 1.5× the decision pixels, and near the refinery (≤ 3 per track) | Fixed |
| P3-6 | **"Total convergence under saturation is total blindness."** | Every new hostile took all 24 eyes for 0.5 s; in a 40-drone raid the bursts chained back to back | The burst is reserved for a **lone** hostile; multiple hostiles use sentinels + priority shares + TDM. Every hostile track received eyes (test) | Fixed (spec intent preserved) |
| P3-7 | **"Your tracker explodes."** | α–β gains assumed a fix every frame; sweep-refreshed tracks diverged to **10¹³ m**; then 117 m/s bird velocities from 70 m passive-range noise; then a stale covariance kept a 966 m-old estimate | Constant-velocity Kalman filter weighted by each fix's noise, extrapolation cap, restart after > 3 s gaps, speed clamp. Test: recent tracks within 0.4 × range + 100 m | Fixed |
| P3-9 | "Re-checking weak evidence stands down real drones." | Single weak second looks left 4–7 bird false alarms standing; repeating them would eventually clear a drone | One decisive look after accumulating 40 eye·px of evidence (error ~10⁻⁶). **All 89 bird false alarms in the study cleared**; no drone stood down | Fixed |
| P3-10 | **"New tracks wait forever to be classified."** | Stage timing: confirmed → classified **22.7 s median, 72 s p90**; stares were assigned before last frame's track tasks were cleared | Allocation reordered: clear → burst → classification stares (≤ 6) → hostile shares. Confirmed → classified now **0.2 s median**; spawn → lock **36.5 s → 9.2–12.5 s median** | Fixed |
| P3-11 | **"There's a hole straight under her."** | Search floor −85° left a 5° nadir cone (~120 m disc) never searched | Search region extended to −90° | Fixed |
| P3-12 | **"In a Shamal the eyes cannot protect the refinery."** | 800 m visibility: VIS goes blind looking down; LWIR resolution-limited to ~3 km slant. **~50 % of drones never locked before breach** (58–64 of 128 per sector) | Not a bug: a physical limit. Measured mitigation: orbit radius 3 km → 1.5 km → 0.8 km cuts never-locked from 50 % → 16 % → 2 %, but leaves ~35 s from lock to breach for the Seven. **Author decision** (conflicts with the "unbroken orbit") | Open (design) |
| P3-13 | "The GPU will crush coverage." (after P2-4, prove it) | Coverage of 0.1 / 0.4 / 1.6 M voxels: CPU rayon 1.9 / 6.9 / 27 ms; GPU 1.3 / 2.8 / 10.2 ms | **1.4× / 2.5× / 2.7× speed-up, 100.000 % agreement.** A real, modest GPU win, claimed at exactly that size | Measured |
| P3-14 | "24 eyes cost the real-time budget." | Tick with 64 threats: 6–8 µs → **15–24 µs** (336–551× real time) | Accepted for now; still over 300× real time. Optimisation candidate: per-frame LoS cache | Open (minor) |
| P3-15 | "Phase 1 tests assumed a silent sky." | `test_events_and_counters` expected only spawn events; the eyes now detect on the spawn tick | Test updated to filter by event kind | Fixed |

## Kill chain, measured (128 drones per cell; 16-drone raids + 10 birds; 8 seeds)

| Sector / weather | Detect (median / p95) | Lock (median / p95) | Never locked |
|---|---|---|---|
| Sabkha, clear | 5.7 / 22.3 s | **9.3 / 63.1 s** | 0 |
| Red Sea, clear | 5.2 / 31.4 s | **12.5 / 59.0 s** | 0 |
| Highland, clear | 4.4 / 31.1 s | **9.2 / 50.1 s** | 0 |
| Sabkha, Shamal 800 m | 62.7 / 164.1 s | 83.0 / 163.0 s | 64 |
| Red Sea, Shamal 800 m | 63.5 / 167.3 s | 78.5 / 169.5 s | 62 |
| Highland, Shamal 800 m | 91.0 / 188.7 s | 88.6 / 193.5 s | 58 |

## Verdict

Phase 3 exit criteria met:
- GPU coverage matches the CPU reference (100.000 %), with a measured 2.5–2.7× speed-up.
- Blind spots are measured and bounded (worst revisit 2.87 s).
- Time-to-detection is reported as distributions, not assumed.

The system's real weakness is now quantified: in a Shamal, half the drones go
unlocked under the default orbit. The mitigation is known and measured, but it
is a design choice for the author.
