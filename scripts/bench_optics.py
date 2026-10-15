"""Phase 3 benchmarks: the 24-eye array's cost, GPU vs CPU coverage, and the
time-to-detection study across sectors and weather. Every Phase 3 number in
the Red-Team log and defence notes comes from this script.

    .venv/Scripts/python scripts/bench_optics.py [--quick]

Writes results/bench/phase03_optics.json and prints markdown tables.
"""

from __future__ import annotations

import argparse
import json
import statistics
import time
from pathlib import Path

import numpy as np

from hawksgrip import Session
from hawksgrip.config import config_json

ROOT = Path(__file__).resolve().parents[1]
SECTORS = ["eastern_sabkha", "red_sea_corridor", "highland_wadi"]
WEATHER = {"clear": 20_000.0, "shamal": 800.0}


def session(sector: str, seed: int, vis: float | None = None) -> Session:
    s = Session(config_json({"terrain": {"sector": sector}}), seed)
    if vis is not None and vis != 20_000.0:
        s.set_visibility(vis)
    return s


def tick_cost(sector: str, rounds: int) -> float:
    out = []
    for r in range(rounds):
        s = session(sector, r)
        s.random_raid(64)
        s.step(1)
        t0 = time.perf_counter_ns()
        s.step(120 * 60)
        out.append((time.perf_counter_ns() - t0) / (120 * 60) / 1e3)
        assert s.active_mask()[8:].sum() == 64, "timing invalid: threats left the sky"
    return statistics.median(out)


def airspace(n: int) -> np.ndarray:
    """~n voxels: a square grid over the sector at 8 heights (30-1200 m)."""
    k = int(round((n / 8) ** 0.5))
    xs = np.linspace(-7400, 7400, k)
    zs = np.linspace(30, 1200, 8)
    return np.stack(np.meshgrid(xs, xs, zs, indexing="ij"), axis=-1).reshape(-1, 3)


def coverage_bench(sizes, reps: int) -> list[dict]:
    try:
        import torch
    except ImportError:
        return []
    if not torch.cuda.is_available():
        return []
    from hawksgrip.gpu.coverage import CoverageGPU

    s = session("eastern_sabkha", 2)
    s.random_raid(8)
    s.step(120 * 10)
    out = []
    for n in sizes:
        vox = airspace(n)
        s.set_coverage_voxels(vox)
        s.coverage_resident()  # warm-up
        t0 = time.perf_counter()
        for _ in range(reps):
            cpu = s.coverage_resident()
        t_cpu = (time.perf_counter() - t0) / reps
        cov = CoverageGPU(s, vox)
        cov.frame()
        torch.cuda.synchronize()
        t0 = time.perf_counter()
        for _ in range(reps):
            g = cov.frame()
        torch.cuda.synchronize()
        t_gpu = (time.perf_counter() - t0) / reps
        t0 = time.perf_counter()
        for _ in range(reps):
            g_host = cov.frame().cpu().numpy()
        t_gpu_host = (time.perf_counter() - t0) / reps
        out.append(
            {
                "voxels": len(vox),
                "cpu_rayon_ms": t_cpu * 1e3,
                "gpu_ms": t_gpu * 1e3,
                "gpu_with_download_ms": t_gpu_host * 1e3,
                "speedup_gpu_resident": t_cpu / t_gpu,
                "speedup_gpu_download": t_cpu / t_gpu_host,
                "agreement": float((g_host == cpu).mean()),
            }
        )
        cov.release()
        del cov, g
        torch.cuda.empty_cache()
    return out


def detection_study(seeds: int) -> dict:
    """Raid of 16 drones + 10 birds per run; drone kill-chain timelines."""
    res = {}
    for sector in SECTORS:
        for wname, vis in WEATHER.items():
            rows = []
            counters = []
            for seed in range(seeds):
                s = session(sector, 100 + seed, vis)
                s.random_raid(16)
                s.spawn_birds(10)
                while s.tick < 120 * 420 and s.counters()["refinery_breaches"] < 16:
                    s.step(120)
                    s.drain_events()
                for t in s.timelines():
                    if t["class"] == "drone":
                        rows.append(t)
                counters.append(s.sensor_counters())
            hz = 120.0

            def secs(key, start="spawn"):
                v = [(r[key] - r[start]) / hz for r in rows if r[key] is not None and r[start] is not None]
                return v

            det = secs("detected")
            lat = secs("detected", "first_opportunity")
            lock = secs("locked")
            n = len(rows)
            ended_before_lock = sum(1 for r in rows if r["locked"] is None)
            never_hostile = sum(1 for r in rows if r["classified_as"] != "drone")
            pct = lambda v, q: float(np.percentile(v, q)) if v else None  # noqa: E731
            res[f"{sector}/{wname}"] = {
                "drones": n,
                "detect_after_spawn_s_median": pct(det, 50),
                "detect_after_spawn_s_p95": pct(det, 95),
                "pointing_latency_s_median": pct(lat, 50),
                "pointing_latency_s_p95": pct(lat, 95),
                "lock_after_spawn_s_median": pct(lock, 50),
                "lock_after_spawn_s_p95": pct(lock, 95),
                "never_locked": ended_before_lock,
                "final_verdict_not_drone": never_hostile,
                "bird_false_alarms": sum(c["false_alarms"] for c in counters),
                "bird_false_alarms_cleared": sum(c["false_alarms_cleared"] for c in counters),
                "drones_misread_then_recovered": sum(c["missed_drones_recovered"] for c in counters),
                "detect_s": det,
                "lock_s": lock,
            }
    return res


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--quick", action="store_true")
    a = ap.parse_args()
    q = a.quick
    res = {
        "tick_cost_64_threats_us": {sec: tick_cost(sec, 2 if q else 5) for sec in ["flat", *SECTORS]},
        "coverage": coverage_bench([100_000, 400_000] if q else [100_000, 400_000, 1_600_000], 5 if q else 20),
        "detection": detection_study(2 if q else 8),
    }
    out = ROOT / "results" / "bench" / "phase03_optics.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(res, indent=2), encoding="utf-8")

    print("| sector | tick, 64 threats + 24 eyes | real-time factor |\n|---|---|---|")
    for k, v in res["tick_cost_64_threats_us"].items():
        print(f"| {k} | {v:.1f} us | {1e6 / (v * 120):.0f}x |")
    print("\n| voxels | CPU rayon (12 thr) | GPU resident | GPU + download | speed-up (resident / download) | agreement |")
    print("|---|---|---|---|---|---|")
    for c in res["coverage"]:
        print(
            f"| {c['voxels']:,} | {c['cpu_rayon_ms']:.2f} ms | {c['gpu_ms']:.2f} ms | {c['gpu_with_download_ms']:.2f} ms | "
            f"{c['speedup_gpu_resident']:.1f}x / {c['speedup_gpu_download']:.1f}x | {100 * c['agreement']:.3f} % |"
        )
    print("\n| sector / weather | drones | detect after spawn (med / p95) | pointing latency (med / p95) | lock after spawn (med / p95) | never locked | bird FA (cleared) |")
    print("|---|---|---|---|---|---|---|")
    f = lambda x: "-" if x is None else f"{x:.1f} s"  # noqa: E731
    for k, v in res["detection"].items():
        print(
            f"| {k} | {v['drones']} | {f(v['detect_after_spawn_s_median'])} / {f(v['detect_after_spawn_s_p95'])} | "
            f"{f(v['pointing_latency_s_median'])} / {f(v['pointing_latency_s_p95'])} | "
            f"{f(v['lock_after_spawn_s_median'])} / {f(v['lock_after_spawn_s_p95'])} | {v['never_locked']} | "
            f"{v['bird_false_alarms']} ({v['bird_false_alarms_cleared']}) |"
        )
    print(f"\nsaved {out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
