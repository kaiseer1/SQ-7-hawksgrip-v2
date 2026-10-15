# Phase 2 Red-Team attack log: terrain, line-of-sight, refinery

`[AGENT: ADVERSARIAL BREAKER (RED-TEAM)]`: *"Procedural mountains are easy to
make pretty and hard to make honest. I attacked the generators with slope
tests, the line-of-sight with brute force, and your GPU plan with a stopwatch."*

## Findings

| # | Attack | Evidence | Fix | Status |
|---|---|---|---|---|
| P2-1 | **"Your line-of-sight sees through hills."** | First march stepped c/(\|m\|+L), stopping at c < ε. Dense reference found a "visible" ray with **0.492 m** clearance, below ε = 0.5 m. That step only guarantees clearance > 0 *between* samples | Step (c − ε)/(\|m\|+L), stop at c < 2ε. **Proved** in ADR-0010: visible ⇒ clearance ≥ ε everywhere. Random-terrain soundness test (3 000 rays) plus a real-sector test | Fixed and proved |
| P2-2 | **"There's a cliff at the head of every canyon."** | Lipschitz bound **41.7** (an ~88° wall) in Sector 2. Cause: wadi fade-in implemented as "+10⁴ m × (1 − fade)", a near-vertical ramp | Replaced with a true blend, h + fade·(min(h, carved) − h); concave floor profile; 700–900 m fades. `steepest_edge()` diagnostic added | Fixed |
| P2-3 | **"Ridged noise stacks slopes."** | After P2-2, L = **4.09**. Each ridged octave doubles frequency and halves amplitude, so its *slope* contribution is constant and 5 octaves add up | Escarpment ridging capped at 3 octaves, amplitude 260 → 120 m. Steepest real edge now **1.70 (60°)**, L = 2.03. Test: steepest edge < 1.8, L < 2.2, all sectors | Fixed |
| P2-4 | **"The GPU will crush line-of-sight."** (the plan's assumption) | 100 000 rays: CPU Lipschitz on 12 threads **6.3–18.0 M rays/s**; GPU fixed-step **0.37 M/s** (17–49× slower); GPU Lipschitz port **0.14–6.9 M/s**; GPU 64-iteration cap + CPU tail ~1.5 M/s | ADR-0003 amended: dense LoS maps default to the CPU. GPU paths kept, tested (99.96–100 % agreement), and documented as the fused-kernel baseline. **Needs the author's decision** (spec asked for GPU raycasting) | Decided pending author |
| P2-5 | **"Straight-line threats fly into the escarpment."** | Phase 1 threats fly straight at the target; in Sectors 2–3 the line passes through 1.5 km of rock | Terrain following: 8-sample, 8 s look-ahead floor at 40 m AGL; floor released inside the 1.5 km dive radius. **0 ground impacts in 288 threat flights** (3 sectors × 3 seeds × 32) | Fixed |
| P2-6 | **"Kindah's orbit could hit a mountain."** (Phase 1 open item) | Orbit altitude was set from the ground at the centre only | World refuses to build unless the orbit annulus (R ± ring radius) clears the highest ground by ≥ 300 m; tested with a 400 m-AGL orbit over the highlands | Fixed |
| P2-7 | **"The 24 eyes are blind in the wadis."** | Masking map (`scripts/render_sectors.py`, 509 796 rays per sector), drone at 30 m AGL vs 36 orbit positions: **Sector 3: 17.8 % of the area is hidden from ≥ half the orbit, 3.1 % from all of it. Sector 2: 5.0 % / 1.4 %. Sector 1: 0 %** | Not a bug: this is the thesis problem. Carried to Phases 3/5 as a requirement: (a) the time-to-detection distribution must be reported, not assumed; (b) evaluate the Seven as forward pickets over wadi mouths and/or orbit choice | Open (design) |
| P2-8 | "Golden hash changed: is determinism broken?" | Phase 1 golden `0x02CB…C200` → `0x765F…0E1B` | Deliberate: terrain following changes flat-ground descent. History recorded in the test. Debug == release still holds | Explained |
| P2-9 | "Refinery pad height disagrees with itself." | 8.9013395 vs 8.9013399 m | Heights are stored as f32 by design (ADR-0004); comparisons use a 1 mm tolerance | Explained |
| P2-10 | "Custom DEM replays can't be verified." | `verify_replay` regenerates procedural terrain only | Documented; replay stores the terrain fingerprint and refuses mismatched ground. Passing the raster to `verify_replay` is an open item | Open (minor) |
| P2-11 | "Low sightlines ignore the refinery itself." | Distillation columns stand 70 m tall | Authoritative LoS includes structure occlusion (cylinders and boxes, exact segment tests); tested | Fixed |
| P2-13 | **"Terrain following tripled your tick cost."** | With 64 threats: 4.1 µs → **11.7 µs** per tick (9 terrain lookups per threat per tick); real-time factor 2 100× → 714× | Look-ahead refreshed in staggered slots at 30 Hz and cached (a stale climb rate can only overstate the climb, so it stays conservative); floor underneath still checked every tick; the cache is now part of the state hash. **6.2–7.5 µs per tick, 1 100–1 340× real time** across the sectors; still 0 ground impacts | Fixed |
| P2-12 | "A north-up GeoTIFF will load upside down." | Raster row 0 is north; the grid's row 0 is south | Loader flips rows; test with an off-centre hill lands it in the correct quadrant | Fixed |

## Verdict

Phase 2 exit criteria met:
- Terrain uses **8.6 MiB** of VRAM (< 10 MB).
- The LoS soundness and keep-out tests pass.
- Three sectors build in 39–95 ms.
- Raids cross all of them with zero ground impacts.
- The loop runs 1 100–1 340× real time with 64 live threats (target 500×).

The biggest result is P2-4. On this hardware and under the two-language
constraint, a better algorithm beat the GPU by an order of magnitude for
terrain line-of-sight. The author should decide whether to pursue a
Python-authored fused kernel or redirect GPU effort.
