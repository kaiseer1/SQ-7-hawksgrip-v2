"""Phase 3 figure: Kindah's 24-eye array under clear sky and Shamal dust.

(a) Detection range vs visibility for each band (why she switches to LWIR).
(b) Time from spawn to lock, clear vs Shamal, per sector (from
    results/bench/phase03_optics.json; run scripts/bench_optics.py first).
(c) Sweep revisit map: the longest gap between looks at 150 m AGL over 30 s
    (GPU coverage accumulator; CPU fallback).
(d) Shamal mitigation: drones never locked vs orbit radius.

    .venv/Scripts/python scripts/render_optics.py
"""

from __future__ import annotations

import json
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import LinearSegmentedColormap
from matplotlib.patches import Circle, Rectangle

from hawksgrip import Session
from hawksgrip.config import config_json

ROOT = Path(__file__).resolve().parents[1]
SURFACE, INK, INK_2, GRID = "#fcfcfb", "#0b0b0b", "#52514e", "#e4e3de"
# Categorical slots 1-3 of the reference palette (light mode), fixed order.
CAT = ["#2a78d6", "#eb6834", "#1baf7a"]
BLUE_RAMP = ["#cde2fb", "#9ec5f4", "#6da7ec", "#3987e5", "#256abf", "#184f95", "#0d366b"]
SECTORS = ["eastern_sabkha", "red_sea_corridor", "highland_wadi"]
LABEL = {"eastern_sabkha": "Sector 1 Sabkha", "red_sea_corridor": "Sector 2 Red Sea", "highland_wadi": "Sector 3 Highland"}


def style(ax, title):
    ax.set_facecolor(SURFACE)
    ax.set_title(title, color=INK, fontsize=10, loc="left", fontweight="bold")
    ax.tick_params(colors=INK_2, labelsize=7, length=2)
    for sp in ax.spines.values():
        sp.set_color("#d6d5d0")
        sp.set_linewidth(0.6)
    ax.grid(color=GRID, linewidth=0.6)
    ax.set_axisbelow(True)


def panel_bands(ax):
    vis = np.geomspace(300, 30_000, 28)
    ranges = {b: [] for b in ("VIS", "SWIR", "LWIR")}
    for v in vis:
        s = Session(config_json({"terrain": {"sector": "eastern_sabkha"}, "environment": {"visibility_m": float(v)}}), 0)
        for b, r in s.sensor_info()["detection_range_m"].items():
            ranges[b].append(r / 1e3)
    for i, (b, r) in enumerate(ranges.items()):
        ax.plot(vis / 1e3, r, color=CAT[i], lw=2, label=b)
    # Direct labels at the clear-air end; SWIR and LWIR coincide there (both
    # limited by their 640-px resolution), so they share one label.
    end = vis[-1] / 1e3
    ax.annotate("VIS", (end, ranges["VIS"][-1]), xytext=(4, 0), textcoords="offset points", color=INK_2, fontsize=8, va="center")
    ax.annotate("SWIR = LWIR", (end, ranges["LWIR"][-1]), xytext=(4, 0), textcoords="offset points", color=INK_2, fontsize=8, va="center")
    ax.set_xscale("log")
    ax.set_xlabel("ground visibility (km, log scale)", color=INK_2, fontsize=8)
    ax.set_ylabel("detection range for a 2 m drone (km)", color=INK_2, fontsize=8)
    ax.axvspan(0.3, 1.0, color="#f0efec", zorder=0)
    ax.text(0.33, 8.6, "Shamal", color=INK_2, fontsize=7)
    ax.set_xlim(0.28, 60)
    ax.legend(fontsize=7, frameon=False, labelcolor=INK_2, loc="lower right")
    style(ax, "(a) Band switching: LWIR wins below ~1.7 km visibility")


def panel_lock(ax, det):
    for i, sec in enumerate(SECTORS):
        for wx, ls in (("clear", "-"), ("shamal", (0, (4, 2)))):
            d = det.get(f"{sec}/{wx}")
            if not d:
                continue
            lock = np.sort(d["lock_s"])
            n = d["drones"]
            y = np.arange(1, len(lock) + 1) / n * 100
            ax.step(lock, y, where="post", color=CAT[i], lw=2 if wx == "clear" else 1.6, ls=ls,
                    label=f"{LABEL[sec]}, {wx}")
    ax.set_xlabel("seconds from spawn to 24-eye lock", color=INK_2, fontsize=8)
    ax.set_ylabel("% of drones locked", color=INK_2, fontsize=8)
    ax.set_ylim(0, 102)
    ax.set_xlim(0, 200)
    ax.legend(fontsize=6.5, frameon=False, labelcolor=INK_2, loc="lower right", ncol=1)
    style(ax, "(b) Time to lock: clear (solid) vs Shamal 800 m (dashed)")


def panel_revisit(fig, ax):
    s = Session(config_json({"terrain": {"sector": "eastern_sabkha"}}), 7)
    xs = np.linspace(-7400, 7400, 150)
    gx, gy = np.meshgrid(xs, xs)
    gz = np.array([s.ground_height(x, y) for x, y in zip(gx.ravel(), gy.ravel())]) + 150.0
    vox = np.column_stack([gx.ravel(), gy.ravel(), gz])
    frames, dt = 900, 4  # 30 s of camera frames
    # Count only voxels within detection range for the whole window (start and
    # end): voxels at the range edge drift in and out as Kindah moves.
    det_start = (s.coverage_cpu(vox) & 2) > 0
    try:
        import torch

        if not torch.cuda.is_available():
            raise ImportError
        from hawksgrip.gpu.coverage import CoverageGPU

        cov = CoverageGPU(s, vox)
        for _ in range(frames):
            s.step(dt)
            cov.accumulate(cov.frame(), s.time)
        gap, never = cov.revisit(s.time)
        gap = gap.cpu().numpy()
        cov.release()
        how = "GPU"
    except ImportError:
        last = np.full(len(vox), np.nan)
        gap = np.zeros(len(vox))
        s.set_coverage_voxels(vox)
        for _ in range(frames):
            s.step(dt)
            seen = s.coverage_resident() == 3
            g = np.where(seen & ~np.isnan(last), s.time - last, 0.0)
            gap = np.maximum(gap, g)
            last = np.where(seen, s.time, last)
        gap = np.maximum(gap, np.where(np.isnan(last), np.inf, s.time - last))
        how = "CPU"
    det = det_start & ((s.coverage_cpu(vox) & 2) > 0)
    g = gap.reshape(gx.shape)
    shown = np.where(det.reshape(gx.shape), np.minimum(g, 30.0), np.nan)
    cmap = LinearSegmentedColormap.from_list("gap", BLUE_RAMP)
    cmap.set_bad("#f0efec")
    im = ax.imshow(shown, origin="lower", extent=(-7.5, 7.5, -7.5, 7.5), cmap=cmap, vmin=0, vmax=10, interpolation="nearest")
    o = s.orbit_info()
    ax.add_patch(Circle((o["center"][0] / 1e3, o["center"][1] / 1e3), o["radius_m"] / 1e3, fill=False, ec=INK, lw=1.0, ls=(0, (4, 3))))
    ax.add_patch(Rectangle((-1, -1), 2, 2, fill=False, ec=INK, lw=0.8))
    finite = g[det.reshape(gx.shape)]
    med, p95 = np.median(finite), np.percentile(finite, 95)
    ax.set_aspect("equal")
    ax.set_xlabel("East (km)", color=INK_2, fontsize=8)
    ax.set_ylabel("North (km)", color=INK_2, fontsize=8)
    cb = fig.colorbar(im, ax=ax, shrink=0.85, pad=0.02)
    cb.set_label("longest gap between looks (s)", color=INK_2, fontsize=7)
    cb.ax.tick_params(labelsize=7, colors=INK_2)
    cb.outline.set_visible(False)
    style(ax, f"(c) Sweep revisit at 150 m AGL, 30 s ({how}): median {med:.1f} s, p95 {p95:.1f} s")
    ax.grid(False)
    return {"median_s": float(med), "p95_s": float(p95), "max_s": float(finite.max())}


def panel_storm_orbit(ax):
    radii = [3000.0, 1500.0, 800.0]
    never = []
    for r in radii:
        nv = n = 0
        for seed in range(6):
            s = Session(config_json({"terrain": {"sector": "eastern_sabkha"}, "queen": {"orbit_radius_m": r}}), 100 + seed)
            s.set_visibility(800.0)
            s.random_raid(16)
            while s.tick < 120 * 420 and s.counters()["refinery_breaches"] < 16:
                s.step(120)
                s.drain_events()
            for t in s.timelines():
                if t["class"] == "drone":
                    n += 1
                    nv += t["locked"] is None
        never.append(100 * nv / n)
    x = np.arange(len(radii))
    ax.bar(x, never, width=0.5, color=CAT[0])
    for xi, v in zip(x, never):
        ax.text(xi, v + 1.5, f"{v:.0f} %", ha="center", color=INK, fontsize=8)
    ax.set_xticks(x, [f"{r / 1e3:.1f} km" for r in radii])
    ax.set_xlabel("Kindah's orbit radius during the Shamal", color=INK_2, fontsize=8)
    ax.set_ylabel("% of drones never locked before breach", color=INK_2, fontsize=8)
    ax.set_ylim(0, max(never) * 1.25 + 5)
    style(ax, "(d) Shamal 800 m: a tighter orbit closes the gap")
    return dict(zip([str(int(r)) for r in radii], never))


def main() -> int:
    plt.rcParams.update({"font.family": "DejaVu Sans", "figure.facecolor": SURFACE})
    det = json.loads((ROOT / "results" / "bench" / "phase03_optics.json").read_text(encoding="utf-8"))["detection"]
    fig, axes = plt.subplots(2, 2, figsize=(13, 10), constrained_layout=True)
    panel_bands(axes[0, 0])
    panel_lock(axes[0, 1], det)
    revisit = panel_revisit(fig, axes[1, 0])
    storm = panel_storm_orbit(axes[1, 1])
    fig.suptitle("SQ-7 Hawksgrip: Phase 3, Kindah's 24-eye compound optical array", color=INK, fontsize=12, fontweight="bold", x=0.01, ha="left")
    out = ROOT / "results" / "figures" / "phase03_optics.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=150, facecolor=SURFACE)
    js = ROOT / "results" / "bench" / "phase03_figure_extras.json"
    js.write_text(json.dumps({"revisit_150m_agl": revisit, "shamal_never_locked_pct_by_orbit_radius": storm}, indent=2), encoding="utf-8")
    print("revisit:", revisit)
    print("storm orbit:", storm)
    print(f"saved {out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
