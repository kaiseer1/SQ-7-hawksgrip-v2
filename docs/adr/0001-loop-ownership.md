# ADR-0001: Python drives the loop in lockstep at 120 Hz

**Status:** Accepted (user decision Q1-A, 2026-10-14) · **Implemented:** Phase 1

## Context

Rust owns the simulation, while Python owns the GPU, the HMI and audio.
Someone has to own the clock.

## Options

| Option | For | Against |
|---|---|---|
| **A. Python calls `step(n)`** (chosen) | One thread of control; trivially replayable; easy to debug; the GPU stage can run between calls | Python call overhead per call (measured: negligible) |
| B. Rust free-runs in its own thread, streams events | Looks "more real-time" | Two clocks; race conditions; hard to replay; GPU results arrive at nondeterministic ticks |

## Decision

- Fixed tick **dt = 1/120 s**. Simulated time is `tick / 120`, never accumulated.
- The renderer runs at ~60 fps. `LockstepDriver` (python/hawksgrip/bridge/driver.py)
  converts wall time into a whole number of ticks using **integer nanoseconds**,
  so one hour at 60 fps gives exactly 432 000 ticks (tested).
- Overruns: at most `hz/4` ticks per frame; the rest of the backlog is dropped
  and counted. The simulation runs briefly slower than real time; it never
  spirals and never changes outcome.
- `step()` **releases the GIL** (`py.detach`), so audio and HMI threads keep running.
  This is tested: a Python thread made >1000 iterations during one `step()` call.
- External inputs (spawns, orders) queue up and take effect at the **start of
  the next tick**, logged with that tick number. Worst-case input latency is one
  tick (8.33 ms); the Phase 5 kill-chain stopwatch must count it.

## Evidence (scripts/bench_bridge.py, i5-12400F)

| | |
|---|---|
| Python→Rust call overhead | 31–33 ns |
| `step(1)`, 64 live threats | 4.1 µs |
| Same tick inside `step(120)` | 3.9 µs |
| Headless real-time factor, 64 threats | ≈ 2 100× |

The call overhead is under 1 % of a tick, so option A costs nothing measurable.
