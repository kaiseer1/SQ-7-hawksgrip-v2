# SQ-7 Hawksgrip (v2)

**SQ-7 = the Sky Queen and her Seven.** An open-source, locally hosted
**Critical Infrastructure Protection (CIP) simulation** for a Master's thesis
in Artificial Intelligence. It models the defence of a *synthetic* oil-refinery
perimeter against unauthorised aerial intrusions: an airborne command node
("Kindah") with a 24-eye compound optical array commands seven reusable
all-electric interceptor nodes through a cryptographically signed auction.

Everything is simulation. Interception is a geometric capture event, terrain is
procedural or public DEM data, and no real facility is modelled.

| Layer | Language | Role |
|---|---|---|
| `crates/hawksgrip-core` | Rust | Deterministic 120 Hz loop, 3-DOF kinematics, energy, auction, Ed25519, telemetry (no Python dependency) |
| `crates/hawksgrip-py` | Rust (PyO3) | Thin bindings exposed as `hawksgrip._core` |
| `python/hawksgrip` | Python | Lockstep driver, CUDA sensing (PyTorch), HMI, audio, analysis |

## Status

| Phase | Scope | State |
|---|---|---|
| 0 | Toolchain, workspace, bridge, benchmarks, legacy baseline | ✅ |
| 1 | Deterministic core loop, kinematics, royal orbit, events, replay, batch | ✅ |
| 2 | Three Saudi sectors, Lipschitz line-of-sight, refinery keep-out, terrain-following threats | ✅ |
| 3 | Kindah's 24-eye array: guaranteed sweep, VIS/SWIR/LWIR physics, bird filter, "SHE LOOKS AT YOU", sentinels/TDM, Kalman tracks, GPU coverage | ✅ |
| 4 – 9 | Energy, auction, engagement, HMI, evaluation, demo | see [docs/ROADMAP.md](docs/ROADMAP.md) |

## Build and run (Windows, RTX 3070 reference machine)

Requirements: Rust (stable, verified on 1.99), MSVC Build Tools 2022, Python ≥ 3.10.

```bash
python -m venv .venv
.venv/Scripts/pip install maturin numpy pytest torch --index-url https://download.pytorch.org/whl/cu128 --extra-index-url https://pypi.org/simple
.venv/Scripts/maturin develop --release
```

```bash
cargo test --workspace --release
```

```bash
.venv/Scripts/python -m pytest
```

```bash
.venv/Scripts/python -m hawksgrip.app --scenario scenarios/single_raid.toml
```

```bash
.venv/Scripts/python -m hawksgrip.app --scenario scenarios/sector3_wadi.toml
```

```bash
.venv/Scripts/python -m hawksgrip.app --scenario scenarios/shamal_birds.toml
```

```bash
.venv/Scripts/python scripts/render_sectors.py
```

```bash
.venv/Scripts/python scripts/bench_bridge.py
```

## Documentation

- [Roadmap](docs/ROADMAP.md) · [Architecture decisions](docs/adr/) · [Claims register](docs/claims.md)
- [Red-Team attack logs](docs/redteam/) · [Viva defence notes](docs/thesis_defense_notes/)
- [Earlier design concept](docs/design.md) (pre-v2; see ADR-0000 for what carried over)

## Economics (corrected from v0.1)

v2 fields **seven** interceptors (v0.1 had six). The v0.1 README's "17× cheaper"
line was wrong. Per engagement, $48,000 (directed energy) ÷ $2,000 (one
sacrificial interceptor frame) = **24×**. A *reusable* intercept costs only
electricity, and that ratio is computed by the simulation in Phase 8 rather
than asserted here. Unit costs used: Kindah $10,000; each of the Seven
$2,000–$3,000 (see [claims register](docs/claims.md)).

## Legacy baseline

[`legacy/v0_1_pygame/`](legacy/v0_1_pygame/) is the original pure-Python v0.1,
kept unchanged as the performance and behaviour baseline for the thesis.

## Author

Basil Abdullah Alzahrani · Al Baha University · Apache-2.0
