# ADR-0009: Terrain is generated in Rust from (sector, terrain seed)

**Status:** Accepted · **Implemented:** Phase 2

## Context

The plan placed the sector generators in Python. But replays must reproduce
the ground exactly, batches must not regenerate a 1501 × 1501 grid per run, and
the authoritative line-of-sight runs in Rust.

## Decision

- **Generators live in Rust** (`terrain/generator.rs`, `terrain/noise.rs`): a
  pure function height(x, y; sector, terrain seed), evaluated on the grid in
  parallel by rows. They use only +, −, ×, floor, integer hashing and `libm`
  (ADR-0007), so the result is identical on every platform.
- **Configured, not passed in:** `[sim.terrain] sector = "...", seed = N`.
  A replay therefore regenerates its own terrain, and its stored fingerprint
  proves it is the same ground.
- **Terrain seed ≠ run seed.** Many raids can fly over one map, and whole maps
  can be held out for evaluation (Phase 8).
- **Shared, immutable:** `Arc<Heightfield>`. A batch builds the map once for
  all runs.
- **Bring-your-own DEM:** Python's `terrain/dem_loader.py` resamples a
  user-supplied raster onto the grid and installs it with `set_terrain`.
  These replays need the raster to verify. No DEM data and no real facility
  coordinates ship with the project.

## The three sectors (all amplitudes SWEPT, docs/claims.md)

| Sector | Shape | Relief (seed 1) | Build time |
|---|---|---|---|
| 1 Eastern Coastal Sabkha | Sea east of an irregular shoreline; ~1.2 m/km rise inland; salt flats near the coast; Shamal-aligned (NNW–SSE) dune ridges inland | 0–21 m | 39 ms |
| 2 Western Red Sea Corridor | Sea west; coastal plain (1.8 %); escarpment rising ~1550 m over ~3 km; 4 canyons with concave long-profiles cut down from the crest | 0–1847 m | 91 ms |
| 3 Highland Escarpment / Wadi | Plateau ~2 km with ridged relief; refinery in a 1350 m basin; 5 sinuous wadis converge on it | 1350–2348 m | 95 ms |

In every sector the refinery pad (2 × 2 km + 200 m apron) is levelled and
blended into the terrain over 350 m.

## Guards (tests)

- Pad level to < 1 cm; the grid is 1501 × 1501 f32 = 9.01 MB.
- **Steepest edge < 1.8** (≈ 61°) and Lipschitz bound < 2.2 in every sector.
  This test caught two generator bugs (Red-Team P2-2, P2-3).
- Wadi floors descend monotonically, and the centreline sits on the carved floor.
- Kindah's orbit clears the highest ground under her annulus by ≥ 300 m
  (`terrain.queen_min_clearance_m`); otherwise the world refuses to build.
- Golden terrain fingerprints per sector (identical in debug and release).
