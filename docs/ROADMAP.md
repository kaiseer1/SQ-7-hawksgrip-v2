# SQ-7 Hawksgrip v2 Roadmap

Each phase ends with: code → `cargo test` + `pytest` → Red-Team attack log
(`docs/redteam/`) → Viva Defence Cheat-Sheet (`docs/thesis_defense_notes/`)
→ status report to the author.

Decisions are recorded in [`docs/adr/`](adr/); every number used is in
[`docs/claims.md`](claims.md).

| Phase | Builds | Exit criterion | State |
|---|---|---|---|
| **0 · Toolchain & skeleton** | Rust workspace, maturin/PyO3, CUDA PyTorch check, v0.1 → `legacy/`, benchmark harness | `hawksgrip._core.step()` works from Python; bridge cost measured | ✅ 2026-10-14 |
| **1 · Deterministic core loop** | 120 Hz fixed tick, named RNG streams, SoA world with fixed slots, 3-DOF point mass with limits, Kindah's closed-form orbit, Seven station keeping, threat waypoint flight, event ring, counters, replay + checkpoints, rayon batch runner | Bit-identical replays; ≥ 500× real time headless | ✅ 2026-10-14 (≈ 2 100× with 64 threats) |
| **2 · Terrain & refinery** | Sector 1 Eastern Coastal Sabkha, Sector 2 Western Red Sea Corridor, Sector 3 Highland Escarpment/Wadi generators; optional public-DEM loader; CPU LoS reference ray-march; refinery layout with keep-out volumes; climate parameters (ambient °C, Shamal visibility) | Terrain < 10 MB VRAM; LoS and keep-out collision tests | ✅ 2026-10-15 (8.6 MiB; LoS proved sound; 1 100×+ real time) |
| **3 · Compound optical array** | 24 frustums; low-discrepancy sweep (coverage-guaranteed, not pure random); detection with CPU LoS; saccade with angular-rate limit; time-division multiplexing + sentinel eyes for multiple targets; micro-Doppler classifier abstraction; Beer–Lambert sandstorm attenuation with SWIR/thermal switch; GPU dense coverage grids | GPU coverage matches the CPU reference; blind-spot % measured | ✅ 2026-10-15 (100 % agreement, 2.5–2.7×; worst revisit 2.87 s) |
| **4 · All-electric power** *(next)* | Battery equivalent circuit (R_int(T), Arrhenius), K_t·I motor torque, momentum-theory hover power, cell-temperature derating at 50 °C ambient, Wh/J/SoC ledger, Royal Trophallaxis docking with transfer efficiency η, ground-nest fallback | Energy conservation asserted every tick; endurance as an output (ADR-0008 R3) | |
| **5 · Sovereign auction & comms** | Cost function (time-to-intercept, energy, keep-out penalty); assignment (Hungarian vs Bertsekas auction, compared); Royal Aegis override; Ed25519 signing (`ed25519-dalek`); rayon parallel bids; RF + NIR photodiode channel model | Auction latency benchmarked; signatures verified; jammed-RF scenario works (R2, R4) | |
| **6 · Engagement & escape envelope** | Lead pursuit / proportional navigation, terrain- and keep-out-aware; geometric capture event; target vs interceptor reachable sets; survival probability by Monte Carlo (GPU batched) | Survival probability with rule-of-three bound (R5) | |
| **7 · Showroom HMI** *(Phases 0–3 scope built: ADR-0012)* | Single 3D viewport (terrain, 24 sightline cones, swarm); HUD (kill-chain stopwatch with split latencies, counters); `RoyalPersonaEngine` (theatrical/clinical `[T]`); Red-Team hotkeys (click spawn, `[J]`, `[S]`); event-driven audio pool | 60 fps under a 10-threat swarm; persona-invariance test (same hashes in both modes) | ✅ for Phases 0–3 (2026-10-15): 114–181 fps median, p95 ≤ 14 ms; persona invariance proven. Not built: `[J]` (no comms model yet), audio pool |
| **8 · Evaluation** | Monte Carlo batches; ablations (Aegis on/off, RF vs NIR, clear vs Shamal, sector 1/2/3, airborne vs ground mast); economic exchange ratio; v0.1 baseline comparison; thesis figures | All figures regenerate from one script; bootstrap 95 % CIs | |
| **9 · Hardening & demo** | Red-Team edge cases (decoy saturation, terrain masking, VRAM spikes, thread stalls), demo script, packaged build, Linux CI for the golden hash | 10-minute showroom run with no crash | |

## Open items carried forward

| Item | From | Phase |
|---|---|---|
| Run the golden-hash test on Linux (cross-OS determinism) | P1 Red-Team | 9 (CI) or earlier if WSL available |
| Hold-point guidance uses a √(2ad) profile with infinite slope at the target, so check for limit cycles | P1 Red-Team | 6 |
| Measure how often the jerk limit yields to the re-projection | P1 Red-Team | 4 |
| ~~Orbit-altitude terrain clearance~~ (fixed in Phase 2, P2-6) | P1 Red-Team | done |
| **Decide GPU raycasting direction**: CPU won dense LoS 17–49× (P2-4). Options: try a Python-authored fused kernel (`torch.compile`/Triton on Windows), or redirect GPU to frustum geometry and Monte Carlo | P2 Red-Team | author decision, before Phase 3 GPU work |
| Terrain masking: up to 17.8 % of Sector 3 is hidden from ≥ half the orbit. ~~Report time-to-detection~~ (done, Phase 3); evaluate the Seven as wadi-mouth pickets | P2 Red-Team | 5, 8 |
| **Shamal protocol decision:** ~48 % of drones never locked at 800 m visibility with a 3 km orbit; a 0.8 km storm orbit cuts it to 2 % but leaves ~35 s lock→breach and changes the "unbroken orbit". Alternatives: Seven as thermal pickets, larger LWIR optics | P3-12 | author decision before Phase 5 |
| Tick cost rose to 15–24 µs with the eyes (336–551×): per-frame LoS cache | P3-14 | 9 |
| ~~Decide GPU raycasting direction~~: option (b) approved; GPU coverage measured 2.5–2.7× | P2-4 | done |
| `verify_replay` for custom-DEM runs (pass the raster in) | P2 Red-Team | 8 |
| Join local git history with GitHub `origin/main` (author's decision) | P0 Red-Team | any |
| User's reference diagrams → replace SWEPT airframe limits | Planning | 7 (geometry refinement) |
