# ADR-0011: Kindah's 24-eye compound optical array

**Status:** Accepted · **Implemented:** Phase 3 (`crates/hawksgrip-core/src/sensing/`, `python/hawksgrip/gpu/coverage.py`)

## Decision summary

| Element | Decision | Why |
|---|---|---|
| Eye | 12° × 9° level frustum, 600 °/s gimbal, 30 Hz frames; contributes only when settled (< 0.5° from aim) | Mid-saccade frames are smeared |
| Search region | Every direction from −90° to +5° elevation (she looks *down*) | Threats fly low; −85° left a nadir hole (P3-11) |
| Sweep (State 1) | Region split into equal-area sectors, one per sweeping eye. Each sector is tiled into footprint-sized cells, visited once per cycle in a per-cycle permuted order (coprime stride, R2 offset) with R2 jitter | Guarantees every direction is revisited within 2 cycles while staying randomised and non-repeating. Raw R2 points left a 6.7 s blind spot (P3-1) |
| Optics | VIS / SWIR / LWIR. Johnson 2-px detection criterion; Koschmieder extinction (σ = 3.912/V) with a dust layer of 800 m scale height, closed-form slant optical depth; logistic Pd | Standard EO practice. Band switching *emerges* from the physics (LWIR wins below ~1.7 km visibility) |
| Detect → confirm | Combined Pd 1 − (1 − pd)ⁿ over the n eyes on target; tentative track, 2-of-3 confirmation stare | M-of-N rejects single-frame noise |
| Classify (bird vs drone) | Temporal-signature classifier (the optical analogue of micro-Doppler); error 0.5·exp(−px/3), **biased toward "drone"** (bias 0.8) | Missing a drone costs the refinery; a false alarm costs a few eyes for a moment (P3-8) |
| "SHE LOOKS AT YOU" | A **lone** hostile triggers a 0.5 s burst: all 24 eyes. With several hostiles there is no total burst | Back-to-back bursts starved every other track (P3-6) |
| Second look | Eyes on a hostile accumulate evidence Σ(eyes × px). At 40, the verdict is re-decided with error 0.5·exp(−E/3) ≈ 10⁻⁶ | One decisive look clears false alarms. Repeated weak looks would eventually stand down a real drone (P3-9) |
| Allocation | Sentinels (4) always sweep → classification stares (≤ 6) → hostiles share the rest by time-to-refinery priority, with TDM when over-subscribed | She never loses awareness, and new tracks are classified quickly (P3-10) |
| Tracking | Constant-velocity Kalman filter per axis on noisy fixes (0.2 mrad per eye / √n; range 10 % passive, 1 m laser rangefinder once locked); restart after > 3 s gaps | The α–β filter diverged to 10¹³ m on sparse fixes (P3-7) |
| Opportunity metric | First frame with line-of-sight and pd ≥ 5 %, or the detection itself | "Time to detection" measured from when detection became possible |
| GPU | Dense coverage/visibility volume (frustums + Pd per voxel) and revisit accumulator; Rust/rayon reference with resident voxels | Elementwise geometry is the GPU's strength (amended ADR-0003). **Measured 2.5–2.9× at ≥ 400 k voxels, 100 % agreement** |

## Consequences

- Every per-frame decision draws from the Sensor RNG stream in a fixed order;
  sensor state (eyes, tracks, RNG) is in the state hash, so replays stay bit-exact.
- Shamal at 800 m visibility is the system's weakest condition: ~50 % of drones
  are never locked before breach with a 3 km orbit (see the Phase 3 Red-Team
  log, P3-12). A tighter storm orbit is the measured mitigation, and an
  **author decision** because it touches the "unbroken orbit" persona.
- Out of scope, recorded: detections use truth association (no clutter
  data-association problem).
