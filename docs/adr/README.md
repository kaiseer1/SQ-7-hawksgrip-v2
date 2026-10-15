# Architecture Decision Records

Each record states one decision, the alternatives considered, and the evidence.
Status values: **Accepted**, **Superseded by ADR-NNNN**, **Proposed**.

| ADR | Decision | Status |
|---|---|---|
| [0000](0000-relationship-to-design-md.md) | What carries over from the pre-v2 `docs/design.md` concept | Accepted |
| [0001](0001-loop-ownership.md) | Python drives `core.step(n)` in lockstep at 120 Hz; render decoupled at 60 fps | Accepted |
| [0002](0002-flight-model-fidelity.md) | 3-DOF point mass with acceleration, turn-rate, jerk and speed limits | Accepted |
| [0003](0003-authoritative-line-of-sight.md) | Authoritative LoS on the CPU in Rust; GPU for dense work, **amended:** dense LoS maps also CPU (measured faster) | Accepted, amended P2 |
| [0004](0004-world-scale-and-frames.md) | 15 × 15 km sector, 10 m grid, ENU metres, f64 state, f32 terrain | Accepted |
| [0005](0005-repository-layout.md) | Cargo workspace (pure core + thin bindings), maturin mixed layout, legacy baseline | Accepted |
| [0006](0006-bridge-data-exchange.md) | Per-frame state copied into caller-owned numpy buffers, not aliased | Accepted |
| [0007](0007-determinism-strategy.md) | Bit-exact determinism: tick time, named RNG streams, ordered parallelism, libm, hashes | Accepted |
| [0008](0008-planning-redteam-requirements.md) | Planning-phase Red-Team fixes are binding requirements for Phases 4, 5, 7 | Accepted |
| [0009](0009-terrain-generation.md) | Terrain generated in Rust from (sector, terrain seed); shared via Arc; DEM loader in Python | Accepted |
| [0010](0010-lipschitz-line-of-sight.md) | Line-of-sight by Lipschitz ray-marching, with soundness proof | Accepted |
| [0011](0011-compound-optical-array.md) | Kindah's 24-eye array: guaranteed sweep, EO physics, biased classifier + decisive second look, sentinel/TDM allocation, Kalman tracks, GPU coverage | Accepted |
