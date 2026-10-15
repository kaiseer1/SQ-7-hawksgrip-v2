# ADR-0012: The showroom HMI and the persona engine (Phases 0–3 scope)

**Status:** Accepted · **Implemented:** `python/hawksgrip/hmi/`, `crates/hawksgrip-core/src/persona/`

## Decisions

| Element | Decision | Why |
|---|---|---|
| Rendering stack | PySide6 (Qt 6) + pyqtgraph OpenGL, one viewport | Pure Python (language constraint); OpenGL 4.6 on the RTX 3070; Qt widgets for HUD overlays and hotkeys |
| 24 cameras | Drawn as 24 frustum wireframes from the engine's own eye directions and frustum tangents, **not** rendered as 24 views | The plan's VRAM rule. A test proves the drawn frustums are the detection frustums (≥ 99.95 % agreement on 20 000 random points) |
| Truth vs belief | Solid markers = ground truth (drones, birds); rings + labels = Kindah's tracks (estimated positions) | The gap between the two *is* the sensing story (lag, passive-range error, lost tracks) |
| Persona | `RoyalPersonaEngine` in Rust, outside `World`: reads drained events, owns the Persona RNG stream, never writes back | **Persona invariance** (from the pre-v2 design): `tests/persona.rs` toggles the voice every 97 ticks and every per-second state checkpoint still matches; `tests/test_hmi.py` repeats it through Python |
| What Kindah may say | Only what her sensors know: classification lines use the *estimated* class; spawns are not announced to her | A voice that leaks ground truth would misrepresent the system |
| HUD claims | Engine compute per tick (measured, µs) shown separately from physical sensing time (sim seconds); no "× faster than a human"; orbit deviation shown beside threats-reaching-queen; "interception not modelled in Phases 0–3: breaches are expected" | ADR-0008 claims policy |
| Sandstorm | `[S]` toggles 0.7 km ↔ 20 km visibility through the engine's `SetVisibility` input (so it is replayable); the band switch comes from the optics, the visuals follow it (dust particles, thermal grayscale, white-hot targets) | The HMI never decides physics |
| Thermal palette | Grayscale terrain + white-hot drones when the band is SWIR/LWIR | Mirrors the sensor's view |
| GPU coverage layer | `[G]` streams the Phase 3 GPU coverage volume (120 × 120 voxels at 150 m AGL) with time-since-last-look shading; CPU fallback | Shows the measured GPU module live |
| Scripted runs | `--at T:ACTION[:ARG] --frames N --screenshot` with fixed-step pacing | Reproducible screenshots for the thesis and an automated smoke test |

## Measured frame rate (RTX 3070, 1600 × 900, 10 drones + 12 birds; `results/bench/showroom_fps.json`)

| Load | Median frame | p95 | Frames over 16.7 ms |
|---|---|---|---|
| Swarm | 5.5 ms (181 fps) | 11.0 ms | 3.3 % |
| + GPU coverage layer | 8.1 ms (123 fps) | 14.1 ms | 3.8 % |
| + Shamal (dust, thermal) | 8.8 ms (114 fps) | 13.1 ms | 3.4 % |

Any overlay-label change makes Qt re-composite the window over the GL view
(~10–15 ms), so the HUD refreshes at 2 Hz; those refresh frames are the ~3.5 %
over budget. Two earlier variants were measured and rejected: a full HUD
refresh every 6th frame (16.7 % of frames over budget) and one panel per frame
(median 14–18 ms).

## Known limitations

- pyqtgraph 0.14 does not redraw a surface after a colours-only `setData`; the
  showroom forces `meshDataChanged()` (covered by the hotkey smoke test).
- Recreating a GL window in one process invalidated cached shader programs;
  `AA_ShareOpenGLContexts` is set and tests drive a single window.
- Interception, the Seven's engagement and the energy model are not part of
  this showroom; the HUD says so instead of showing placeholders.
