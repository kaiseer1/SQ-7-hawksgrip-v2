"""Phase 2 benchmarks: terrain generation, authoritative CPU line-of-sight,
GPU dense line-of-sight, and their agreement. Every Phase 2 number in the
Red-Team log and defence notes comes from this script.

    .venv/Scripts/python scripts/bench_terrain.py [--quick]

Writes results/bench/phase02_terrain.json and prints a markdown table.
"""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path

import numpy as np

from hawksgrip import Session
from hawksgrip.config import config_json

ROOT = Path(__file__).resolve().parents[1]
SECTORS = ["eastern_sabkha", "red_sea_corridor", "highland_wadi"]


def best_of(fn, k: int = 3) -> float:
    best = float("inf")
    for _ in range(k):
        t0 = time.perf_counter()
        fn()
        best = min(best, time.perf_counter() - t0)
    return best


def queen_rays(s: Session, n: int, seed: int):
    """Rays from Kindah's orbit to targets 20-200 m AGL, 1.8-7 km out."""
    rng = np.random.default_rng(seed)
    oc = s.orbit_info()["center"]
    th = rng.uniform(0, 2 * np.pi, n)
    a = np.column_stack([3000 * np.cos(th), 3000 * np.sin(th), np.full(n, oc[2])])
    r = rng.uniform(1800, 7000, n)
    phi = rng.uniform(0, 2 * np.pi, n)
    b = np.column_stack([r * np.cos(phi), r * np.sin(phi), np.zeros(n)])
    b[:, 2] = [s.ground_height(x, y) for x, y in b[:, :2]] + rng.uniform(20, 200, n)
    return a, b


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--quick", action="store_true")
    args = ap.parse_args()
    n_rays = 20_000 if args.quick else 100_000

    try:
        import torch

        cuda = torch.cuda.is_available()
    except ImportError:
        cuda = False

    res: dict = {"rays": n_rays, "sectors": {}}
    for name in SECTORS:
        cfg = config_json({"terrain": {"sector": name}})
        r: dict = {}
        r["generate_and_build_world_s"] = best_of(lambda: Session(cfg, 0))
        s = Session(cfg, 0)
        info = s.terrain_info()
        r["lipschitz"] = info["lipschitz"]
        r["height_range_m"] = [info["min_m"], info["max_m"]]
        a, b = queen_rays(s, n_rays, 7)

        # Single-ray CPU latency (the per-tick authoritative case).
        k = 2000
        t0 = time.perf_counter()
        for i in range(k):
            s.line_of_sight(tuple(a[i]), tuple(b[i]))
        r["cpu_single_ray_us"] = (time.perf_counter() - t0) / k * 1e6

        cpu_vis = None

        def cpu_batch():
            nonlocal cpu_vis
            cpu_vis = s.los_batch(a, b)

        t = best_of(cpu_batch)
        r["cpu_batch_rays_per_s"] = n_rays / t
        r["visible_fraction"] = float(cpu_vis.mean())

        if cuda:
            from hawksgrip.gpu.terrain import TerrainTensor

            tt = TerrainTensor(s)
            ta = torch.as_tensor(a, dtype=torch.float32, device="cuda")
            tb = torch.as_tensor(b, dtype=torch.float32, device="cuda")
            gpu_vis = None

            def gpu_batch():
                nonlocal gpu_vis
                gpu_vis = tt.los(ta, tb)
                torch.cuda.synchronize()

            gpu_batch()  # warm-up
            t = best_of(gpu_batch)
            r["gpu_bruteforce_rays_per_s"] = n_rays / t
            r["gpu_bruteforce_agreement"] = float((gpu_vis.cpu().numpy() == cpu_vis).mean())

            def gpu_lip():
                nonlocal gpu_vis
                gpu_vis = tt.los_lipschitz(ta, tb)
                torch.cuda.synchronize()

            gpu_lip()
            t = best_of(gpu_lip)
            g = gpu_vis.cpu().numpy()
            r["gpu_rays_per_s"] = n_rays / t
            r["gpu_iterations"] = tt.iterations
            r["gpu_unresolved"] = tt.unresolved
            r["gpu_vram_terrain_mib"] = tt.nbytes / 2**20
            r["gpu_cpu_agreement"] = float((g == cpu_vis).mean())
            r["gpu_blocked_cpu_visible"] = int(np.sum(~g & cpu_vis))
            r["gpu_visible_cpu_blocked"] = int(np.sum(g & ~cpu_vis))
            del tt
            torch.cuda.empty_cache()
        res["sectors"][name] = r

    out = ROOT / "results" / "bench" / "phase02_terrain.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(res, indent=2), encoding="utf-8")

    print(f"rays per sector: {n_rays:,} (Kindah's orbit -> targets 20-200 m AGL)\n")
    print("| sector | build | L | CPU 1 ray | CPU batch (12 thr) | GPU brute force | GPU Lipschitz | agree | visible |")
    print("|---|---|---|---|---|---|---|---|---|")
    for name, r in res["sectors"].items():
        gpu = f"{r['gpu_rays_per_s'] / 1e6:.2f} M/s" if "gpu_rays_per_s" in r else "n/a"
        brute = f"{r['gpu_bruteforce_rays_per_s'] / 1e6:.2f} M/s" if "gpu_rays_per_s" in r else "n/a"
        agree = f"{100 * r['gpu_cpu_agreement']:.2f} %" if "gpu_cpu_agreement" in r else "n/a"
        print(
            f"| {name} | {r['generate_and_build_world_s'] * 1e3:.0f} ms | {r['lipschitz']:.2f} | "
            f"{r['cpu_single_ray_us']:.1f} us | {r['cpu_batch_rays_per_s'] / 1e6:.2f} M/s | {brute} | {gpu} | {agree} | "
            f"{100 * r['visible_fraction']:.1f} % |"
        )
    print(f"\nsaved {out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
