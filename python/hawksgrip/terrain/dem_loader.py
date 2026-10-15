"""Load an external elevation model onto the sector grid.

No DEM data ships with the project, and no real facility coordinates are
used. The loader takes any square elevation raster the author supplies
(e.g. a public Copernicus GLO-30 tile cropped by hand) and maps it onto the
15 x 15 km, 10 m grid (ADR-0004). The synthetic refinery is then placed at
the grid centre, as with the procedural sectors.

Replays recorded on custom terrain store its fingerprint; verifying them
needs the same raster (``verify_replay`` regenerates only procedural sectors).
"""

from __future__ import annotations

from pathlib import Path

import numpy as np

PLAUSIBLE_RANGE_M = (-500.0, 9_000.0)


def resample_bilinear(src: np.ndarray, n: int) -> np.ndarray:
    """Resample a 2-D array to (n, n) with bilinear interpolation, treating
    the source as covering exactly the same square (corner-aligned)."""
    src = np.asarray(src, dtype=np.float64)
    sy, sx = src.shape
    gy = np.linspace(0.0, sy - 1, n)
    gx = np.linspace(0.0, sx - 1, n)
    iy = np.clip(np.floor(gy).astype(int), 0, max(sy - 2, 0))
    ix = np.clip(np.floor(gx).astype(int), 0, max(sx - 2, 0))
    ty = (gy - iy)[:, None]
    tx = (gx - ix)[None, :]
    iy1 = np.minimum(iy + 1, sy - 1)
    ix1 = np.minimum(ix + 1, sx - 1)
    h00 = src[np.ix_(iy, ix)]
    h10 = src[np.ix_(iy, ix1)]
    h01 = src[np.ix_(iy1, ix)]
    h11 = src[np.ix_(iy1, ix1)]
    a = h00 + (h10 - h00) * tx
    b = h01 + (h11 - h01) * tx
    return a + (b - a) * ty


def load_dem(path: str | Path, *, half_extent_m: float = 7_500.0, cell_m: float = 10.0, nodata: float | None = None) -> np.ndarray:
    """Read a raster (.npy, .tif, .tiff), fill gaps and resample to the
    sector grid. Returns float32 of shape (n, n), row 0 = south edge.

    The raster's first row is taken as its **north** edge (the GeoTIFF
    convention) and flipped, because the simulation grid's row index grows
    northward.
    """
    path = Path(path)
    suffix = path.suffix.lower()
    if suffix == ".npy":
        raw = np.load(path)
    elif suffix in (".tif", ".tiff"):
        import tifffile

        raw = tifffile.imread(path)
    else:
        raise ValueError(f"unsupported DEM format {suffix!r} (use .npy, .tif or .tiff)")
    raw = np.asarray(raw, dtype=np.float64)
    if raw.ndim != 2 or min(raw.shape) < 2:
        raise ValueError(f"DEM must be a 2-D raster, got shape {raw.shape}")
    if nodata is not None:
        raw = np.where(raw == nodata, np.nan, raw)
    valid = np.isfinite(raw)
    if not valid.any():
        raise ValueError("DEM has no valid samples")
    if not valid.all():
        raw = np.where(valid, raw, np.nanmin(raw))  # conservative: holes become low ground
    lo, hi = float(raw.min()), float(raw.max())
    if lo < PLAUSIBLE_RANGE_M[0] or hi > PLAUSIBLE_RANGE_M[1]:
        raise ValueError(f"DEM heights {lo:.0f}..{hi:.0f} m outside the plausible range {PLAUSIBLE_RANGE_M}")
    n = round(2 * half_extent_m / cell_m) + 1
    out = resample_bilinear(raw[::-1, :], n)  # north-up raster -> row index grows north
    return out.astype(np.float32)


def install_dem(session, heights: np.ndarray, *, half_extent_m: float = 7_500.0, cell_m: float = 10.0) -> None:
    """Install loaded heights into a Session before its first tick."""
    session.set_terrain(np.ascontiguousarray(heights, dtype=np.float32), cell_m, -half_extent_m, -half_extent_m)
