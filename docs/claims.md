# Claims register

Every number the simulation uses or the thesis states, with its status:

- **DESIGN**: a chosen parameter of the simulated system (not a claim about the world).
- **SWEPT**: a placeholder that Phase 8 varies; conclusions must hold across the sweep.
- **SOURCE**: backed by a cited public reference.
- **TO CITE**: used in text; needs a reference before the thesis is submitted.
- **MEASURED**: produced by this repository's benchmarks or tests (script named).

## Simulation parameters (Rust `SimConfig` defaults)

| Parameter | Value | Status | Note |
|---|---|---|---|
| `tick_hz` | 120 Hz | DESIGN | ADR-0001 |
| `world.half_extent_m` | 7 500 m (15 × 15 km) | DESIGN | ADR-0004 |
| `world.cell_m` | 10 m | DESIGN | ADR-0004 |
| `world.refinery_half_size_m` | 1 000 m (2 × 2 km) | DESIGN | synthetic complex |
| `world.refinery_breach_ceiling_m` | 150 m AGL | SWEPT | |
| `queen.orbit_radius_m` | 3 000 m | DESIGN | |
| `queen.orbit_alt_agl_m` | 1 500 m | DESIGN | |
| `queen.airspeed_mps` | 25 m/s | SWEPT | |
| `queen.limits` | v 35, a 3, j 5, ω 0.25 | SWEPT | orbit needs ω = 0.0083 rad/s, a = 0.21 m/s² |
| `seven.limits` | v 50 m/s, a 25 m/s², j 150 m/s³, ω 2.5 rad/s | SWEPT | replace from the author's reference diagrams |
| `seven.station_radius_m` / `below_m` | 150 m / 60 m | DESIGN | |
| `seven.station_kp`, `kd` | 0.8, 1.6 (ζ ≈ 0.89) | DESIGN | |
| `threat.limits` | v 50, a 8, j 30, ω 0.6 | SWEPT | generic fixed-wing intruder class |
| `threat.speed_*_mps` | 25–45 m/s | SWEPT | |
| `threat.spawn_radius_m` | 7 000 m | DESIGN | |
| `threat.spawn_alt_agl_*_m` | 50–400 m | SWEPT | |
| `threat.capacity` | 64 simultaneous | DESIGN | fixed slots (ADR-0006) |

### Phase 2 additions

| Parameter | Value | Status | Note |
|---|---|---|---|
| `terrain.sector` | `flat` (tests) / `eastern_sabkha` / `red_sea_corridor` / `highland_wadi` | DESIGN | archetypes, not maps of real places (ADR-0009) |
| `terrain.seed` | 1 | DESIGN | independent of the run seed |
| `terrain.queen_min_clearance_m` | 300 m | DESIGN | orbit annulus vs highest ground |
| Sector 1 relief / shoreline / dunes | 0–21 m; shoreline x ≈ 3.8 km ± 450 m; dune ridges ≤ 7 m, NNW–SSE | SWEPT | |
| Sector 2 escarpment / canyons | +1550 m over ~3 km; 4 canyons, wall slope 1.0, concave profile exp 2.5 | SWEPT | steepest edge 60° (test-guarded) |
| Sector 3 plateau / basin / wadis | ~1950–2350 m plateau; basin 1350 m; 5 wadis, wall slope 1.0 | SWEPT | |
| Refinery pad apron / blend | 200 m / 350 m | DESIGN | |
| Keep-out margins | tank 60, column 40, process 40, flare 80, control 25 m | SWEPT | `refinery.keepout_margin_scale` multiplies all |
| Synthetic layout | 28 tanks, 4 columns (70 m), 2 flares (110 m), 3 process units, 1 control room | DESIGN | generic; not any real facility |
| `threat.terrain_follow_agl_m` / `terrain_lookahead_s` / `dive_radius_m` | 40 m / 8 s / 1 500 m | SWEPT | |
| LoS grazing tolerance ε | 0.5 m | DESIGN | ADR-0010 |
| `environment.ambient_temp_c` | 40 °C (validated −20..60) | SWEPT | consumed in Phase 4 |
| `environment.visibility_m` | 20 km (clear) | SWEPT | Shamal case < 1 km; consumed in Phase 3 |
| `environment.wind_from_deg` / `wind_speed_mps` | 330° (NNW, Shamal direction) / 6 m/s | SWEPT | TO CITE: Shamal climatology |

### Phase 3 additions (sensors; all SWEPT unless noted)

| Parameter | Value | Status | Note |
|---|---|---|---|
| Eyes / frustum / gimbal | 24 / 12° × 9° / 600 °/s | SWEPT (24 = DESIGN) | author's specification fixes 24 |
| Frame rate / dwell | 30 Hz / 2 frames | SWEPT | |
| Search region | −90° to +5° elevation | DESIGN | P3-11 |
| VIS / SWIR / LWIR IFOV | 0.109 / 0.327 / 0.327 mrad | SWEPT | 12° over 1920 / 640 / 640 px |
| Inherent contrast / threshold | VIS 0.30/0.02, SWIR 0.25/0.02, LWIR 0.15/0.01 | SWEPT | electric drones run cool |
| Dust extinction vs VIS | SWIR 0.6, LWIR 0.35 | SWEPT, TO CITE | dust scatters less at longer wavelengths |
| Koschmieder constant | 3.912 | SOURCE | 2 % contrast threshold (standard meteorology) |
| Dust scale height | 800 m | SWEPT, TO CITE | Shamal boundary layer |
| Detection criterion | 2 px across target | SOURCE / TO CITE | Johnson criteria (detection ≈ 1 cycle) |
| Pd_max per eye-frame / logistic sharpness | 0.9 / 8 | SWEPT | |
| Target sizes | drone 2.0 m, bird 0.6 m | SWEPT | |
| Confirmation | 2 of 3 frames | DESIGN | |
| Classifier | error 0.5·exp(−px/3), min 2 px, 6 hits, bias 0.8 toward drone | SWEPT | abstraction of a temporal-signature classifier |
| Second look | evidence 40 eye·px | SWEPT | |
| Allocation | 4 sentinels, ≤ 6 classification stares, ≥ 2 eyes per hostile, TDM slot 0.25 s, burst 0.5 s (lone hostile) | SWEPT | |
| Tracking noise | 0.2 mrad/eye; range 10 % passive, 1 m laser rangefinder when ≥ 2 eyes locked | SWEPT | |
| Kalman process noise | 6 m/s² | DESIGN | |
| Birds | 8–15 m/s, 30–300 m AGL, wander radius 6.5 km | SWEPT | |

## Economics

| Claim | Value | Status | Note |
|---|---|---|---|
| Kindah unit cost | $10 000 | DESIGN | COTS prototype tier (author's specification) |
| Each of the Seven | $2 000 – $3 000 | DESIGN | COTS prototype tier |
| Directed-energy cost per engagement | $48 000 | TO CITE | inherited from the v0.1 README, source not given |
| Patriot PAC-3 / Stinger per engagement | $3 M / $500 k | TO CITE | inherited from the v0.1 README |
| Sacrificial-frame advantage vs directed energy | 24× | derived | $48 000 / $2 000; v0.1 wrongly said 17× |
| Reusable intercept cost | electricity only | Phase 8 | computed from the Phase 4 energy ledger, not asserted |

## Performance (MEASURED, scripts/bench_bridge.py, i5-12400F + RTX 3070, 2026-10-14)

| Claim | Value |
|---|---|
| Python→Rust call overhead | 31–33 ns |
| One simulation tick, 64 live threats | ≈ 4.1 µs (Phase 1; 6.2–7.5 µs with Phase 2 terrain following) |
| Headless real-time factor, 64 threats | ≈ 2 100× |
| Monte Carlo batch speed-up, 12 threads vs 1 | 6.6–6.9× |
| GPU per-tick round trip | 66–77 µs |
| GPU dense 98 k rays × 256 samples | 1.9–2.3 ms |

## Phase 2 measurements (scripts/bench_terrain.py, scripts/render_sectors.py, 2026-10-15)

| Claim | Value |
|---|---|
| Sector build time (generate + world) | 39 / 91 / 95 ms (Sectors 1 / 2 / 3) |
| Terrain VRAM | 8.6 MiB per sector |
| Authoritative LoS, single ray | 2.0–3.7 µs |
| Authoritative LoS, batch on 12 threads | 18.0 / 7.7 / 6.3 M rays/s |
| GPU fixed-step LoS (PyTorch eager) | 0.37 M rays/s |
| GPU Lipschitz LoS (PyTorch eager) | 6.9 / 0.30 / 0.14 M rays/s |
| GPU vs CPU LoS agreement | 99.96–100 % |
| Area hidden from ≥ half of Kindah's orbit (30 m AGL) | 0 / 5.0 / 17.8 % |
| Area hidden from her entire orbit (30 m AGL) | 0 / 1.4 / 3.1 % |
| Ground impacts, 288 terrain-following threat flights | 0 |
| Tick cost, 64 live terrain-following threats | 6.2 (flat) / 6.7 / 6.8 / 7.5 µs |
| Headless real-time factor, 64 threats | 1 343 (flat) / 1 244 / 1 229 / 1 107× |

## Phase 3 measurements (scripts/bench_optics.py, scripts/render_optics.py, 2026-10-15)

| Claim | Value |
|---|---|
| Tick with 64 threats + 24 eyes | 15.1 (flat) / 16.8 / 21.8 / 24.4 µs (336–551× real time) |
| Worst sweep revisit, 1 296 directions, 20 s | 2.87 s |
| Sweep revisit at 150 m AGL (in range), 30 s | median 1.3 s, p95 2.1 s |
| GPU vs CPU coverage, 0.1 / 0.4 / 1.6 M voxels | 1.4× / 2.5× / 2.7×, agreement 100.000 % |
| Median spawn → lock, clear (Sectors 1 / 2 / 3) | 9.3 / 12.5 / 9.2 s (p95 63 / 59 / 50 s) |
| Drones never locked before breach, clear | 0 / 384 |
| Median spawn → lock, Shamal 800 m | 83.0 / 78.5 / 88.6 s |
| Drones never locked before breach, Shamal 800 m | 184 / 384 (~48 %) |
| Same, Shamal, orbit radius 3.0 / 1.5 / 0.8 km (Sector 1) | 50 / 16 / 2 % |
| Bird false alarms cleared by the second look | 89 / 89 |
| Drones finally misclassified as birds (clear) | 0 / 384 (18 early misreads, all recovered) |

## HUD claims policy (ADR-0008)

- Never print "N× faster than a human" without a cited human baseline (TO CITE:
  simple visual reaction time ≈ 250 ms; operator decision cycles of seconds).
- Never print a bare "0.00 %" survival probability; print the rule-of-three
  95 % upper bound 3/n.
- Show decision latency (measured compute) and physical latency (sensor frame,
  comms, input tick, flight) separately.
