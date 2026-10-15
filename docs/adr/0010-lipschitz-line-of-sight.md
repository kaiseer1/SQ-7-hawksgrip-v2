# ADR-0010: Line-of-sight by Lipschitz ray-marching

**Status:** Accepted · **Implemented:** Phase 2 (`terrain/los.rs`)

## Decision

Terrain line-of-sight is computed by **Lipschitz ray-marching**, i.e. sphere
tracing specialised to heightfields.

**Lemma (slope bound).** Inside one bilinear cell, ∂h/∂x is a linear blend of
the two x-edge slopes, so |∂h/∂x| ≤ Lx, the largest x-edge slope in the grid
(likewise for y). Hence |∇h| ≤ L = √(Lx² + Ly²) everywhere. L is computed
once when the heightfield is built.

**Theorem (soundness).** Parametrise the segment by horizontal distance s,
with ray height z(s) = z₀ + m·s and clearance c(s) = z(s) − h(x(s), y(s)).
Then |c′(s)| ≤ |m| + L. From a point with clearance c ≥ 2ε, step
Δs = (c − ε)/(|m| + L). Over that step the clearance cannot fall below ε.
Stop with "occluded" as soon as c < 2ε. Therefore:

- **visible ⇒ clearance ≥ ε along the whole segment** (it never sees through terrain);
- **occluded ⇒ some point has clearance < 2ε** (grazing rays count as blocked).

Each step is at least ε/(|m| + L) long, so the march always terminates. With
ε = 0.5 m and the last metre excluded, a target sitting on the ground does not
occlude itself.

## Evidence

- Random rough terrain, 3 000 rays: the march **never** reports visible where
  dense 5 cm sampling finds clearance < ε. All disagreements lie in the [ε, 2ε) band.
- Real sectors: ≤ 1 % disagreement with a dense reference, all inside the band.
- The first implementation stepped c/(|m|+L) and stopped at c < ε. That only
  guarantees clearance > 0 between samples. The test caught a "visible" ray
  with 0.492 m clearance (Red-Team P2-1), and the theorem above is the fix.

## Performance (scripts/bench_terrain.py; 100 000 rays from Kindah's orbit)

| Sector | L | CPU, 1 ray | CPU batch, 12 threads | GPU fixed-step (5 m) | GPU Lipschitz port |
|---|---|---|---|---|---|
| 1 Sabkha | 0.27 | 2.0 µs | **18.0 M rays/s** | 0.37 M rays/s | 6.9 M rays/s |
| 2 Red Sea | 2.03 | 3.4 µs | **7.7 M rays/s** | 0.37 M rays/s | 0.30 M rays/s |
| 3 Highland | 2.10 | 3.7 µs | **6.3 M rays/s** | 0.37 M rays/s | 0.14 M rays/s |

Adaptive stepping resolves ≥ 95 % of rays within 64 steps (100 % within 16 on
the Sabkha; the slowest grazing rays need 400–760), where a fixed 5 m step
needs ~2 000. On these workloads the algorithm beats raw parallel hardware
(see the ADR-0003 amendment).

## Alternatives considered

| Method | Why not |
|---|---|
| Fixed-step sampling | Can step over ridges thinner than the step; 2 000 samples per 10 km ray |
| Exact cell traversal (Amanatides–Woo DDA + bilinear patch intersection) | Exact but complex (cubic per cell); the Lipschitz march is sound with a tunable ε |
| Max-mipmap (quadtree) march | Faster in the worst case; a candidate if a local L ever becomes necessary |
