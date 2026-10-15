# ADR-0006: Per-frame state crosses the bridge by copy into caller-owned buffers

**Status:** Accepted · **Implemented:** Phase 1

## Context

The renderer needs positions and velocities of all bodies every frame. Options:

| Option | Description |
|---|---|
| A. Aliased "zero-copy" view | numpy array pointing straight into Rust memory |
| B. **Copy into caller-owned buffers** (chosen) | Python allocates once; `snapshot_into(pos, vel)` fills them |
| C. Allocate fresh arrays per call | `positions()` returns a new array |

## Evidence (72 bodies, measured)

| Call | Cost |
|---|---|
| `snapshot_into` (B) | 0.32 µs |
| `positions()` (C) | 0.18 µs |

At this size, **allocation is not the bottleneck**. numpy's borrow-checking in
B costs more than C's allocation. So the decision is **not** justified by
speed, and the thesis must not claim it is.

## Decision and real reasons

Option B is kept for **safety and stability**, not speed:

1. **No aliasing across a released GIL.** `step()` releases the GIL (ADR-0001).
   An aliased view (A) could be read by another Python thread while Rust
   writes it. That is a data race and undefined behaviour in Rust.
2. **Stable buffer addresses.** Phase 3/7 will upload these buffers to the GPU
   and the renderer; fixed addresses allow pinned-memory uploads without
   per-frame registration.
3. **Fixed entity slots** (Queen 0, Seven 1–7, threats 8+) mean shapes never
   change during a run.

Passing the same buffer twice is refused (tested), and so are wrong shapes,
dtypes and non-contiguous views.

## Revisit when

Batch Monte Carlo (Phase 6/8) moves ~10⁴–10⁶ samples. There, true bulk
transfer matters, and results will come back as one array per batch call.
