# ADR-0004: World scale, coordinate frame and precision

**Status:** Accepted (user decision Q4, 2026-10-14) · **Implemented:** Phase 1 (config), Phase 2 (terrain)

## Decision

| Item | Value |
|---|---|
| Sector | 15 × 15 km square, `half_extent_m = 7500` |
| Terrain grid | 10 m spacing → 1501 × 1501 samples (1500 cells + 1 fencepost) |
| Terrain memory | 1501² × 4 B = **9.01 MB** (f32) in RAM and in VRAM |
| Frame | ENU (x = East, y = North, z = Up), metres, origin at the refinery centre |
| Refinery | 2 × 2 km footprint at the centre; breach = inside footprint below 150 m AGL |
| Kindah's orbit | radius 3 km, 1500 m above ground at the centre, 25 m/s (period ≈ 12.6 min) |
| Threat spawn ring | 7 km from the centre (inside the 7.5 km half-extent) |

**Precision:** dynamic state is **f64** (at 7.5 km, f32 resolves only ~0.5 mm,
and repeated integration in f32 would drift). Terrain heights are **f32**,
since decimetre precision far exceeds any DEM's accuracy; every computation from
them is f64.

## Why 15 km

A threat at 45 m/s crosses 7 km in ~155 s, so the defence has minutes, not
seconds, to work with. The marine approach (Sector 1) and wadi corridors
(Sector 3) fit inside the ring. Kindah's 3 km orbit plus the Seven's 150 m
station ring stays ≥ 4.3 km inside the sector edge (validated at config load).

## Consequences

- VRAM for terrain is ~9 MB of the 1 GiB sim budget (`gpu/device.py`).
- A larger sector later (e.g. 30 km for long-range warning) costs 4× memory, ~36 MB: still trivial.
