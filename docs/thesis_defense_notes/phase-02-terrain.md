# Viva Defence Cheat-Sheet: Phase 2 (Saudi terrain, line-of-sight, refinery)

`[AGENT: THESIS DEFENSE COACH & EXPLAINER]` with `[AGENT: CUDA & OPTICS]` and
`[AGENT: KINEMATICS & PHYSICS]`

## 1. Plain-English pitch

The simulator now builds three kinds of Saudi landscape around the synthetic
refinery: a flat Gulf coast, a Red Sea coast under steep canyon-cut mountains,
and highland wadis. It can answer one question exactly and fast: *can Kindah
see that drone, or is a hill in the way?* It answers six million of those
questions a second, and it is mathematically guaranteed never to "see through"
a hill.

## 2. Mathematical core

**(a) Terrain = a smooth surface with a known maximum slope.** The grid
(1501 × 1501, 10 m) is interpolated bilinearly. Inside each cell the slope is
a blend of edge slopes, so |∇h| ≤ L everywhere, with L = √(Lx² + Ly²) taken
from the steepest grid edges.

**(b) Lipschitz ray-march (ADR-0010).** Along a sightline, clearance c(s)
changes no faster than |m| + L, where m is the ray's own slope. So from
clearance c the ray can safely jump

```
Δs = (c − ε) / (|m| + L)
```

without touching the ground. Stop with "blocked" when c < 2ε.
**Theorem:** visible ⇒ clearance ≥ ε along the whole line.
High rays take huge steps; grazing rays take small ones.

**(c) Procedural sectors (ADR-0009):**
- Gradient noise, fBm and ridged multifractal (deterministic integer hashing).
- Wadis carved as meandering valleys with concave long-profiles,
  floor(u) = end + (start − end)(1 − u)^p.
- A levelled refinery pad.
- Guarded by tests: steepest slope < 61°, pad flat to 1 cm, wadi floors descending.

**(d) Keep-out volumes.** Each structure is inflated by its safety margin
(tanks 60 m, flares 80 m, …; swept). Queries use exact signed distance (for
Phase 5's cost function) and exact segment intersection (for Phase 6's
interception paths).

**(e) Terrain-following threats.** Look 8 s ahead along the track. The climb
needed to reach ground + 40 m by each look-ahead time τ is (floor − z)/τ; fly
the largest. Measured result: 0 ground impacts in 288 raids across the sectors.

## 3. Key results (measured)

| | Sabkha | Red Sea | Highland |
|---|---|---|---|
| Relief | 0–21 m | 0–1847 m | 1350–2348 m |
| Build time | 39 ms | 91 ms | 95 ms |
| LoS, CPU 12 threads | 18.0 M rays/s | 7.7 M rays/s | 6.3 M rays/s |
| Area hidden from ≥ ½ of the orbit (drone at 30 m AGL) | 0 % | 5.0 % | **17.8 %** |
| Area hidden from the *whole* orbit | 0 % | 1.4 % | 3.1 % |

## 4. Committee Q&A prep

**Toughest question:** *"Your specification says the 3D terrain raycasting runs
on the RTX 3070 with CUDA. Your line-of-sight runs on the CPU. Didn't you fail
to deliver the GPU part?"*

**Winning answer:**

"I built both and measured them on the same 100 000 sightlines per sector,
and the CPU won. The GPU versions are in the repository and tested to 99.96–100 %
agreement with the CPU. But the multithreaded Rust Lipschitz march does 6 to 18
million rays per second, against 0.37 million for a brute-force GPU march.
The reason is algorithmic:
- The adaptive march resolves 95 % of rays in under 64 steps, where fixed
  sampling needs 2 000.
- Without hand-written CUDA kernels, which my two-language constraint rules
  out, PyTorch can't fuse those steps.

So I use the GPU where I can *measure* that it wins: dense frustum geometry
and Monte Carlo. A thesis that puts everything on the GPU because it sounds
faster would be making a claim I can disprove with my own benchmark."

**Follow-up:** *"Procedural terrain isn't real Saudi terrain. Why should I trust results on it?"*
- "The three sectors are *archetypes* that isolate one defensive problem
  each: no cover, an escarpment with canyons, and deep wadis.
- Every amplitude is a swept parameter in my claims register, and terrain
  seeds can be held out for evaluation.
- For realism there's a DEM loader that takes a public elevation tile.
  I deliberately don't ship real facility coordinates."

**Follow-up:** *"Isn't 3 % of the area hidden from the entire orbit a fatal blind spot?"*
- "It's the most important number Phase 2 produced. It's in my Red-Team log
  as an open design problem rather than buried.
- A drone flying a wadi floor can stay hidden from every point of Kindah's
  orbit for part of its route.
- Phases 3 and 5 must measure *time-to-detection* rather than assume
  instant detection. I'll test the obvious counters: pickets from the Seven
  over the wadi mouths, and orbit choice.
- Altitude is exactly what shrinks that number, which is why Phase 8
  compares her against a ground mast on the same terrain."
