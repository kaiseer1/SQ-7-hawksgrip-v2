# ADR-0008: Planning-phase Red-Team fixes are binding requirements

**Status:** Accepted (user, 2026-10-14)

The planning-phase Red-Team audit was accepted in full. These are now
requirements with acceptance tests, not suggestions.

| # | Requirement | Phase | Acceptance test |
|---|---|---|---|
| R1 | CPU-authoritative LoS, GPU for dense grids and Monte Carlo | 2–3 | ADR-0003; GPU vs CPU LoS agreement test |
| R2 | **NIR optical fallback modelled as a dedicated MHz-class photodiode channel** (LoS-only, dust-attenuated), not the navigation cameras. Bandwidth, latency, loss and signature size are explicit in `comms/channel.rs` | 5 | A 64-byte Ed25519 signature + payload's time-on-air is computed from channel bitrate and shown; the jammed-RF scenario still completes |
| R3 | **Energy ledger** in Wh/J with conservation asserted every tick; thermal derating driven by **cell temperature** (~60 °C ceiling) with I²R self-heating against up to 50 °C ambient; internal resistance *falls* with temperature (Arrhenius) | 4 | Conservation test; derating curve test; endurance and refills per sortie reported as outputs |
| R4 | **Claims audit on the HUD:** decision latency (µs, measured compute) shown separately from physical latency (sensor frame + comms + input tick + flight) | 5, 7 | Stopwatch splits sum to the total; no "×-faster-than-human" figure without a cited human baseline |
| R5 | Survival probability reported with the **rule-of-three bound** (0 of n → 95 % upper bound ≈ 3/n) | 6, 8 | HUD never prints a bare "0.00 %" |
| R6 | `QUEEN ORBIT DEVIATION` shown next to **Aegis interventions** and **threats reaching the Queen** (deviation is zero by construction) | 1 (counters), 7 (HUD) | Counters exist in Phase 1 ✅ |
| R7 | v0.1 README numbers corrected: 7 interceptors in v2, 24× (not 17×) | 0 | Root README ✅, legacy README note ✅ |
