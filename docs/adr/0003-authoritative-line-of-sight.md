# ADR-0003: CPU-authoritative line-of-sight, GPU for dense work

**Status:** Accepted, **amended Phase 2** (see bottom) · **Implemented:** Phase 2

## Context

Per-tick detection needs line-of-sight (LoS) from Kindah's 24 cameras to a
handful of targets over the terrain heightfield: a few hundred rays. Running
that on the GPU inside the loop means upload → kernel → sync → download every tick.

## Decision

- **Authoritative detection LoS runs in Rust on the CPU** (deterministic
  heightfield ray-march, Phase 2). Only CPU results feed simulation state.
- **The GPU does dense, non-authoritative work** where it wins: per-camera ray
  grids (24 × 64 × 64 ≈ 98 k rays) for blind-spot coverage maps, sandstorm
  visibility fields, viewsheds, and batched Monte Carlo escape envelopes.
- GPU results are cross-validated against the CPU reference on sampled rays
  (`tests/test_gpu_vs_cpu_los.py`, Phase 3).

## Evidence (scripts/bench_bridge.py, RTX 3070, measured 2026-10-14)

| Measurement | Value |
|---|---|
| Entire CPU tick, 64 threats (all kinematics + rules) | **≈ 4 µs** |
| GPU round trip, 72×3 positions up, one op, result down | **66–77 µs** |
| GPU empty kernel launch + sync | 19–26 µs |
| GPU dense 98 k rays × 256 samples | 1.9–2.3 ms |

A single GPU round trip costs **16–19 full simulation ticks**. Doing LoS there
would make the GPU the bottleneck of a loop it barely helps. The dense grid,
by contrast, is ~50 million ray samples in ~2 ms, about 14 % of a 60 fps frame.
That is real GPU work.

## Consequences

- Determinism (ADR-0007) is not exposed to GPU floating-point reduction order.
- The GPU stage runs at render rate (60 Hz) or slower, decoupled from the 120 Hz loop.
- "CUDA-accelerated" in the thesis refers to coverage, visibility and Monte Carlo,
  each with a measured speed-up over the CPU reference.

---

## Amendment (Phase 2, 2026-10-15): measured CPU vs GPU for dense LoS

The original decision assumed the GPU would win the *dense* maps. Phase 2
measured it (ADR-0010, `scripts/bench_terrain.py`, 100 000 rays per sector):

| Implementation | Sabkha | Red Sea | Highland | Agreement with CPU |
|---|---|---|---|---|
| **CPU Lipschitz march, rayon, 12 threads** | **18.0 M/s** | **7.7 M/s** | **6.3 M/s** | (authoritative) |
| GPU fixed-step, PyTorch eager | 0.37 M/s | 0.37 M/s | 0.37 M/s | 99.96–100 % |
| GPU Lipschitz port, PyTorch eager | 6.9 M/s | 0.30 M/s | 0.14 M/s | 99.997–100 % |
| GPU Lipschitz, capped at 64 iterations (+ CPU tail) | — | ~1.4 M/s | ~1.7 M/s | — |

**Why the GPU loses here:**
- The fixed-step march does ~2 000 samples per ray. The adaptive march resolves ≥ 95 % of rays within 64 steps (measured: 99.7 % Red Sea, 95.1 % Highland, 100 % Sabkha within 16).
- The GPU Lipschitz port is limited by a long tail of grazing rays (up to 760
  lockstep iterations). Each iteration costs several Python-launched kernels.
- In eager mode every intermediate makes a round trip through VRAM.

A fused custom kernel would change this, but the language constraint (Rust and
Python only) rules out CUDA C++. A Python-authored fused kernel
(`torch.compile` / Triton) is the remaining option, and needs Triton on Windows.

**Amended decision:**
- Authoritative per-tick LoS stays on the CPU (unchanged).
- **Dense LoS maps also default to the CPU** (`Session.los_batch`, GIL released, all cores).
- The GPU LoS paths remain implemented and cross-validated (tests) as the
  baseline for a fused-kernel attempt.
- GPU effort moves to workloads still expected to win: frustum geometry for
  24 × 64 × 64 ray grids (no terrain gathers), sandstorm attenuation fields,
  and batched Monte Carlo. **Each needs a measured speed-up before it is claimed.**

**Phase 3 outcome (option (b) approved by the author, 2026-10-15):** the GPU
dense coverage/visibility volume measured **1.4× / 2.5× / 2.7×** faster than
the 12-thread Rust reference at 0.1 / 0.4 / 1.6 M voxels, with 100.000 %
agreement (`scripts/bench_optics.py`, ADR-0011). That is the GPU claim the
thesis makes, at exactly those sizes.
