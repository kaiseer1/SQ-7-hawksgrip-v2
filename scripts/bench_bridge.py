"""Phase 0/1 benchmarks on the reference machine. Every number in the
Phase 0/1 Red-Team log and defence notes comes from this script.

    .venv/Scripts/python scripts/bench_bridge.py [--quick]

Writes results/bench/phase01_bridge.json and prints a markdown table.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import statistics
import time
from pathlib import Path

import numpy as np

from hawksgrip import Session, _core, run_batch
from hawksgrip.bridge import FrameBuffers

ROOT = Path(__file__).resolve().parents[1]


def timeit(fn, reps: int, rounds: int = 5) -> float:
    """Median seconds per call over `rounds` rounds of `reps` calls."""
    out = []
    for _ in range(rounds):
        t0 = time.perf_counter_ns()
        for _ in range(reps):
            fn()
        out.append((time.perf_counter_ns() - t0) / reps / 1e9)
    return statistics.median(out)


def time_ticks(threats: int, ticks_per_call: int, calls: int, parallel: bool = False, rounds: int = 5) -> float:
    """Median seconds per *tick*. A fresh session each round keeps every
    threat airborne while timing: total simulated time per round is kept
    under 120 s, and the fastest threat needs ~155 s to reach the refinery."""
    assert ticks_per_call * calls <= 120 * 120
    out = []
    for _ in range(rounds):
        s = loaded_session(threats, parallel)
        t0 = time.perf_counter_ns()
        for _ in range(calls):
            s.step(ticks_per_call)
        out.append((time.perf_counter_ns() - t0) / (calls * ticks_per_call) / 1e9)
        alive = int(s.active_mask()[8:].sum())
        assert alive == threats, f"timing invalid: only {alive}/{threats} threats alive"
    return statistics.median(out)


def loaded_session(threats: int, parallel: bool = False) -> Session:
    s = Session(seed=1)
    s.set_parallel(parallel)
    if threats:
        s.random_raid(threats)
    s.step(1)
    return s


def bench_cpu(quick: bool) -> dict:
    r: dict = {}
    k = 0.2 if quick else 1.0
    r["noop_call_ns"] = timeit(_core.noop, int(200_000 * k)) * 1e9

    for threats in (0, 64):
        r[f"step1_{threats}thr_us"] = time_ticks(threats, 1, int(10_000 * k)) * 1e6
        per_tick = time_ticks(threats, 120, int(100 * k))
        r[f"tick_in_batch120_{threats}thr_us"] = per_tick * 1e6
        r[f"realtime_factor_{threats}thr"] = 1.0 / (per_tick * 120)

    r["step1_64thr_rayon_us"] = time_ticks(64, 1, int(5_000 * k), parallel=True) * 1e6

    s = loaded_session(64)
    fb = FrameBuffers(s)
    r["snapshot_into_us"] = timeit(lambda: s.snapshot_into(fb.pos, fb.vel), int(20_000 * k)) * 1e6
    r["positions_alloc_us"] = timeit(s.positions, int(20_000 * k)) * 1e6
    r["frame_refresh_us"] = timeit(fb.refresh, int(20_000 * k)) * 1e6

    seeds = list(range(48))
    ticks = 120 * 30
    batch = {}
    for th in (1, 6, 12):
        best = float("inf")
        for _ in range(3):
            t0 = time.perf_counter()
            run_batch(seeds, ticks, 32, threads=th)
            best = min(best, time.perf_counter() - t0)
        batch[th] = best
    r["batch_48x30s_seconds"] = batch
    r["batch_speedup_6_vs_1"] = batch[1] / batch[6]
    r["batch_speedup_12_vs_1"] = batch[1] / batch[12]
    r["batch_sim_seconds_per_wall_second_12thr"] = len(seeds) * 30 / batch[12]
    return r


def bench_gpu(quick: bool) -> dict:
    """The Red-Team #1 measurement: what a per-tick GPU round trip costs."""
    try:
        import torch
    except ImportError:
        return {"skipped": "torch not installed"}
    if not torch.cuda.is_available():
        return {"skipped": "CUDA not available"}
    dev = torch.device("cuda")
    k = 0.2 if quick else 1.0
    s = loaded_session(64)
    fb = FrameBuffers(s)
    fb.refresh()
    torch.zeros(1, device=dev)  # create context
    torch.cuda.synchronize()

    def round_trip():
        # Upload 72x3 positions, run a tiny op, bring the answer back.
        t = torch.from_numpy(fb.pos).to(dev, non_blocking=False)
        d = torch.linalg.vector_norm(t, dim=1)
        return d.cpu()

    def kernel_sync_only():
        torch.empty(1, device=dev).add_(1.0)
        torch.cuda.synchronize()

    r = {
        "device": torch.cuda.get_device_name(0),
        "round_trip_72x3_us": timeit(round_trip, int(2_000 * k)) * 1e6,
        "launch_plus_sync_us": timeit(kernel_sync_only, int(5_000 * k)) * 1e6,
    }
    # The workload the GPU is actually for: a dense frustum ray grid.
    rays = torch.randn(24 * 64 * 64, 3, device=dev)
    torch.cuda.synchronize()

    def dense():
        n = torch.nn.functional.normalize(rays, dim=1)
        steps = torch.linspace(0, 5000, 256, device=dev)
        pts = n[:, None, :] * steps[None, :, None]  # 98k rays x 256 samples
        hit = (pts[..., 2] < 0).any(dim=1)
        torch.cuda.synchronize()
        return hit

    r["dense_98k_rays_x256_ms"] = timeit(dense, int(50 * k) or 5) * 1e3
    r["dense_rays_per_second"] = 24 * 64 * 64 / (r["dense_98k_rays_x256_ms"] / 1e3)
    del rays
    torch.cuda.empty_cache()
    return r


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--quick", action="store_true")
    a = ap.parse_args()
    res = {
        "machine": {"cpu": platform.processor(), "threads": os.cpu_count(), "python": platform.python_version()},
        "core_version": _core.__version__,
        "cpu": bench_cpu(a.quick),
        "gpu": bench_gpu(a.quick),
    }
    out = ROOT / "results" / "bench" / "phase01_bridge.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(res, indent=2), encoding="utf-8")

    c, g = res["cpu"], res["gpu"]
    print("| measurement | value |\n|---|---|")
    print(f"| Python->Rust call overhead (noop) | {c['noop_call_ns']:.0f} ns |")
    print(f"| step(1), 0 threats | {c['step1_0thr_us']:.2f} us |")
    print(f"| step(1), 64 threats | {c['step1_64thr_us']:.2f} us |")
    print(f"| step(1), 64 threats, rayon per-tick | {c['step1_64thr_rayon_us']:.2f} us |")
    print(f"| tick cost inside step(120), 64 threats | {c['tick_in_batch120_64thr_us']:.2f} us |")
    print(f"| headless real-time factor, 64 threats | {c['realtime_factor_64thr']:.0f}x |")
    print(f"| snapshot_into (preallocated) | {c['snapshot_into_us']:.2f} us |")
    print(f"| FrameBuffers.refresh (snapshot + active mask) | {c['frame_refresh_us']:.2f} us |")
    print(f"| positions() (allocating) | {c['positions_alloc_us']:.2f} us |")
    b = c["batch_48x30s_seconds"]
    print(f"| batch 48 runs x 30 s: 1 / 6 / 12 threads | {b[1]:.3f} / {b[6]:.3f} / {b[12]:.3f} s |")
    print(f"| batch speed-up 6 / 12 threads vs 1 | {c['batch_speedup_6_vs_1']:.2f}x / {c['batch_speedup_12_vs_1']:.2f}x |")
    if "skipped" not in g:
        print(f"| GPU round trip (72x3 up, op, down) | {g['round_trip_72x3_us']:.1f} us |")
        print(f"| GPU empty launch + sync | {g['launch_plus_sync_us']:.1f} us |")
        print(f"| GPU dense 98k rays x 256 samples | {g['dense_98k_rays_x256_ms']:.2f} ms |")
    else:
        print(f"| GPU | skipped: {g['skipped']} |")
    print(f"\nsaved {out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
