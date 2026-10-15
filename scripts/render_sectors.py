"""Phase 2 figure: the three Saudi sectors and where a drone can hide from Kindah.

Top row: shaded relief with coastline, wadi centrelines, refinery keep-out
volumes and Kindah's orbit. Bottom row: for a drone 30 m above the ground at
each point, the fraction of Kindah's orbit (36 positions) from which it is
*hidden* (authoritative CPU line-of-sight, terrain + refinery structures).

    .venv/Scripts/python scripts/render_sectors.py

Writes results/figures/phase02_sectors.png and results/bench/phase02_masking.json.
"""

from __future__ import annotations

import json
import time
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.colors import LightSource, LinearSegmentedColormap
from matplotlib.patches import Circle, Rectangle

from hawksgrip import Session
from hawksgrip.config import config_json

ROOT = Path(__file__).resolve().parents[1]
SECTORS = ["eastern_sabkha", "red_sea_corridor", "highland_wadi"]

# Reference palette tokens (dataviz skill, references/palette.md).
SURFACE = "#fcfcfb"
INK = "#0b0b0b"
INK_2 = "#52514e"
BLUE_RAMP = ["#cde2fb", "#9ec5f4", "#6da7ec", "#3987e5", "#256abf", "#184f95", "#0d366b"]
# Second sequential context takes the next categorical hue (orange), one hue light->dark.
ORANGE_RAMP = ["#fde6d9", "#f8c3a4", "#f19a6e", "#eb6834", "#c94f1f", "#9a3a15", "#6b280e"]

ELEV = LinearSegmentedColormap.from_list("elev_orange", ORANGE_RAMP)
HIDDEN = LinearSegmentedColormap.from_list("hidden_blue", BLUE_RAMP)

AGL = 30.0
ORBIT_POSITIONS = 36
GRID_STEP = 125.0


def hidden_fraction(s: Session):
    """Fraction of orbit positions with no line-of-sight to each grid point."""
    o = s.orbit_info()
    cx, cy, cz = o["center"]
    r = o["radius_m"]
    xs = np.arange(-7400.0, 7400.0 + 1, GRID_STEP)
    gx, gy = np.meshgrid(xs, xs)
    pts = np.column_stack([gx.ravel(), gy.ravel()])
    pz = np.array([s.ground_height(x, y) for x, y in pts]) + AGL
    targets = np.column_stack([pts, pz])
    th = np.linspace(0, 2 * np.pi, ORBIT_POSITIONS, endpoint=False)
    orbit = np.column_stack([cx + r * np.cos(th), cy + r * np.sin(th), np.full_like(th, cz)])
    a = np.repeat(orbit, len(targets), axis=0)
    b = np.tile(targets, (ORBIT_POSITIONS, 1))
    t0 = time.perf_counter()
    vis = s.los_batch(a, b).reshape(ORBIT_POSITIONS, len(targets))
    dt = time.perf_counter() - t0
    hidden = 1.0 - vis.mean(axis=0)
    return xs, hidden.reshape(gx.shape), len(a), dt


def overlays(ax, s: Session, info: dict, keepout: bool):
    if info["coastline"]:
        c = np.array(info["coastline"])
        ax.plot(c[:, 0] / 1e3, c[:, 1] / 1e3, color=INK_2, lw=1.2, label="coastline")
    for i, w in enumerate(info["wadis"]):
        w = np.array(w)
        ax.plot(w[:, 0] / 1e3, w[:, 1] / 1e3, color=BLUE_RAMP[5], lw=1.0, label="wadi centreline" if i == 0 else None)
    o = s.orbit_info()
    ax.add_patch(Circle((o["center"][0] / 1e3, o["center"][1] / 1e3), o["radius_m"] / 1e3, fill=False, ec=INK, lw=1.2, ls=(0, (4, 3)), label="Kindah's orbit"))
    ax.add_patch(Rectangle((-1, -1), 2, 2, fill=False, ec=INK, lw=1.0, label="refinery footprint"))
    if keepout:
        for st in s.refinery_structures():
            k = st["keepout"]
            if k["shape"] == "cylinder":
                ax.add_patch(Circle((k["cx"] / 1e3, k["cy"] / 1e3), k["r"] / 1e3, fill=False, ec=INK, lw=0.4))
            else:
                (x0, y0, _), (x1, y1, _) = k["min"], k["max"]
                ax.add_patch(Rectangle((x0 / 1e3, y0 / 1e3), (x1 - x0) / 1e3, (y1 - y0) / 1e3, fill=False, ec=INK, lw=0.4))


def style(ax):
    ax.set_facecolor(SURFACE)
    ax.set_xlim(-7.5, 7.5)
    ax.set_ylim(-7.5, 7.5)
    ax.set_aspect("equal")
    ax.tick_params(colors=INK_2, labelsize=7, length=2)
    for sp in ax.spines.values():
        sp.set_color("#d6d5d0")
        sp.set_linewidth(0.6)
    ax.set_xlabel("East (km)", color=INK_2, fontsize=7)


def main() -> int:
    plt.rcParams.update({"font.family": "DejaVu Sans", "figure.facecolor": SURFACE})
    fig, axes = plt.subplots(2, 3, figsize=(13.5, 9.6), constrained_layout=True)
    ls = LightSource(azdeg=315, altdeg=40)
    stats = {}
    hid_im = None
    for col, name in enumerate(SECTORS):
        s = Session(config_json({"terrain": {"sector": name}}), 0)
        info = s.terrain_info()
        h = s.terrain_heights().astype(np.float64)
        lo, hi = float(h.min()), float(h.max())

        ax = axes[0, col]
        norm_h = (h - lo) / max(hi - lo, 1e-9)
        rgb = ls.shade_rgb(ELEV(norm_h)[..., :3], h, vert_exag=0.25 if hi - lo > 200 else 25.0, blend_mode="soft", dx=10, dy=10)
        ax.imshow(rgb, origin="lower", extent=(-7.5, 7.5, -7.5, 7.5), interpolation="bilinear")
        overlays(ax, s, info, keepout=True)
        style(ax)
        ax.set_title(info["label"], color=INK, fontsize=10, loc="left", fontweight="bold")
        sm = plt.cm.ScalarMappable(cmap=ELEV, norm=plt.Normalize(lo, hi))
        cb = fig.colorbar(sm, ax=ax, shrink=0.78, pad=0.02)
        cb.set_label("elevation (m), own scale per sector", color=INK_2, fontsize=7)
        cb.ax.tick_params(labelsize=7, colors=INK_2)
        cb.outline.set_visible(False)
        if col == 0:
            ax.set_ylabel("North (km)", color=INK_2, fontsize=7)
        if col == 1:  # the only sector with every overlay (coast, wadis, orbit, refinery)
            ax.plot([], [], color=INK, lw=0.4, label="keep-out volumes")
            ax.legend(loc="lower left", fontsize=6.5, frameon=True, facecolor=SURFACE, edgecolor="#d6d5d0", labelcolor=INK_2)

        xs, hidden, n_rays, dt = hidden_fraction(s)
        outside = np.maximum(*np.meshgrid(np.abs(xs), np.abs(xs))) > 1_200
        partly = float((hidden[outside] > 0).mean())
        mostly = float((hidden[outside] >= 0.5).mean())
        fully = float((hidden[outside] >= 1.0).mean())
        stats[name] = {
            "rays": n_rays,
            "cpu_seconds": dt,
            "area_hidden_from_some_of_orbit": partly,
            "area_hidden_from_half_or_more": mostly,
            "area_hidden_from_entire_orbit": fully,
        }
        ax = axes[1, col]
        hid_im = ax.imshow(hidden * 100, origin="lower", extent=(-7.5 - 0.0625, 7.5 + 0.0625) * 2, cmap=HIDDEN, vmin=0, vmax=100, interpolation="nearest")
        overlays(ax, s, info, keepout=False)
        style(ax)
        ax.set_title(
            f"hidden from ≥ half the orbit: {100 * mostly:.1f} % of area\nhidden from the entire orbit: {100 * fully:.1f} %",
            color=INK_2,
            fontsize=8,
            loc="left",
        )
        if col == 0:
            ax.set_ylabel("North (km)", color=INK_2, fontsize=7)

    cb = fig.colorbar(hid_im, ax=axes[1, :], shrink=0.6, pad=0.01, location="right")
    cb.set_label(f"% of Kindah's orbit from which a drone {AGL:.0f} m AGL is hidden", color=INK_2, fontsize=8)
    cb.ax.tick_params(labelsize=7, colors=INK_2)
    cb.outline.set_visible(False)
    fig.suptitle(
        "SQ-7 Hawksgrip: Phase 2 sectors, and where terrain can hide a low drone from Kindah",
        color=INK,
        fontsize=12,
        fontweight="bold",
        x=0.01,
        ha="left",
    )

    out = ROOT / "results" / "figures" / "phase02_sectors.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(out, dpi=150, facecolor=SURFACE)
    js = ROOT / "results" / "bench" / "phase02_masking.json"
    js.parent.mkdir(parents=True, exist_ok=True)
    js.write_text(json.dumps({"agl_m": AGL, "orbit_positions": ORBIT_POSITIONS, "grid_step_m": GRID_STEP, "sectors": stats}, indent=2), encoding="utf-8")
    for k, v in stats.items():
        print(
            f"{k:18s} rays {v['rays']:,} in {v['cpu_seconds']:.2f} s | hidden from some {100 * v['area_hidden_from_some_of_orbit']:.1f} % "
            f"| >= half {100 * v['area_hidden_from_half_or_more']:.1f} % | entire orbit {100 * v['area_hidden_from_entire_orbit']:.1f} %"
        )
    print(f"saved {out.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
