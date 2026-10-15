"""Pure scene geometry for the showroom (numpy only, no Qt).

Everything is computed in metres and scaled to kilometres by the showroom,
which keeps GPU float precision comfortable for a 15 km world.
"""

from __future__ import annotations

import numpy as np

from hawksgrip.hmi import theme


def ramp(t: np.ndarray, stops: list[str]) -> np.ndarray:
    """Map t in [0, 1] to RGBA along evenly spaced hex colour stops."""
    t = np.clip(np.asarray(t, dtype=np.float64), 0.0, 1.0)
    cols = np.array([theme.rgba(s)[:3] for s in stops])
    xs = np.linspace(0.0, 1.0, len(stops))
    rgb = np.stack([np.interp(t, xs, cols[:, c]) for c in range(3)], axis=-1)
    return np.concatenate([rgb, np.ones(t.shape + (1,))], axis=-1)


def hillshade(z: np.ndarray, cell: float, azimuth_deg: float = 315.0, altitude_deg: float = 40.0) -> np.ndarray:
    """Lambertian hillshade in [0, 1] for a grid z[i, j] at (x_i, y_j)."""
    dzdx, dzdy = np.gradient(z, cell, cell)
    nx, ny, nz = -dzdx, -dzdy, np.ones_like(z)
    n = np.sqrt(nx * nx + ny * ny + nz * nz)
    az, alt = np.radians(azimuth_deg), np.radians(altitude_deg)
    # Light direction (x = east, y = north), azimuth clockwise from north.
    lx, ly, lz = np.cos(alt) * np.sin(az), np.cos(alt) * np.cos(az), np.sin(alt)
    return np.clip((nx * lx + ny * ly + nz * lz) / n, 0.0, 1.0)


def terrain_grid(heights: np.ndarray, cell: float, origin: tuple[float, float], step: int):
    """Subsample the (ny, nx) north-row heightfield into (x, y, z[x, y]) in metres."""
    h = np.asarray(heights, dtype=np.float64)[::step, ::step]
    ny, nx = h.shape
    x = origin[0] + np.arange(nx) * cell * step
    y = origin[1] + np.arange(ny) * cell * step
    return x, y, h.T  # z[i, j] at (x[i], y[j])


def terrain_colors(z: np.ndarray, cell: float, thermal: bool) -> np.ndarray:
    """Elevation ramp x hillshade; sea (z <= 0.05 m) drawn flat."""
    lo, hi = float(z.min()), float(z.max())
    t = (z - lo) / max(hi - lo, 1e-6)
    shade = 0.45 + 0.55 * hillshade(z, cell)
    c = ramp(t, theme.THERMAL_RAMP if thermal else theme.TERRAIN_RAMP)
    c[..., :3] *= shade[..., None]
    sea = z <= 0.05
    if sea.any():
        c[sea] = theme.rgba("#090909" if thermal else theme.SEA)
    return c.astype(np.float32)


def eye_basis(dirs: np.ndarray):
    """Right/up vectors of level frustums: same convention as sensing/eyes.rs."""
    f = np.asarray(dirs, dtype=np.float64)
    r = np.cross(f, np.array([0.0, 0.0, 1.0]))
    n = np.linalg.norm(r, axis=-1, keepdims=True)
    r = np.where(n < 1e-9, np.array([1.0, 0.0, 0.0]), r / np.maximum(n, 1e-12))
    u = np.cross(r, f)
    return f, r, u


CORNERS = np.array([[1, 1], [1, -1], [-1, -1], [-1, 1]], dtype=np.float64)


def frustum_segments(q: np.ndarray, dirs: np.ndarray, tan_h: float, tan_v: float, length: float) -> np.ndarray:
    """Wireframe of each eye's frustum: 4 edges from the apex plus the far
    rectangle at `length` along the boresight. Returns (n_eyes * 16, 3)
    vertex pairs for GL "lines" mode."""
    f, r, u = eye_basis(dirs)
    q = np.asarray(q, dtype=np.float64)
    n = len(f)
    corners = q + length * (
        f[:, None, :] + CORNERS[None, :, 0:1] * tan_h * r[:, None, :] + CORNERS[None, :, 1:2] * tan_v * u[:, None, :]
    )  # (n, 4, 3)
    apex = np.broadcast_to(q, (n, 4, 3))
    edges = np.stack([apex, corners], axis=2)  # (n, 4, 2, 3)
    rim = np.stack([corners, np.roll(corners, -1, axis=1)], axis=2)  # (n, 4, 2, 3)
    return np.concatenate([edges, rim], axis=1).reshape(-1, 3)


def cone_colors(task: np.ndarray, flash: bool, thermal: bool) -> np.ndarray:
    """Per-vertex colours for frustum_segments: sweep / confirm / track."""
    alpha_sweep = 0.45 if not thermal else 0.35
    lut = {
        0: theme.rgba(theme.CONE_SWEEP, alpha_sweep),
        1: theme.rgba(theme.CONE_CONFIRM, 0.95),
        2: theme.rgba(theme.CONE_FLASH if flash else theme.CONE_TRACK, 0.95),
    }
    per_eye = np.array([lut[int(t)] for t in task], dtype=np.float32)
    return np.repeat(per_eye, 16, axis=0)


def circle(center, radius: float, n: int = 256) -> np.ndarray:
    th = np.linspace(0.0, 2 * np.pi, n + 1)
    c = np.asarray(center, dtype=np.float64)
    return np.column_stack([c[0] + radius * np.cos(th), c[1] + radius * np.sin(th), np.full(n + 1, c[2])])


def ring_segments(centers: np.ndarray, radius: float, n: int = 20) -> np.ndarray:
    """Horizontal rings around each centre as GL line pairs: (len(centers) * n * 2, 3)."""
    if len(centers) == 0:
        return np.zeros((0, 3))
    th = np.linspace(0.0, 2 * np.pi, n + 1)
    ring = np.column_stack([radius * np.cos(th), radius * np.sin(th), np.zeros(n + 1)])
    pairs = np.stack([ring[:-1], ring[1:]], axis=1)  # (n, 2, 3)
    return (np.asarray(centers)[:, None, None, :] + pairs[None]).reshape(-1, 3)


class Trails:
    """Ring buffer of recent positions per slot, emitted as GL line pairs."""

    def __init__(self, n_slots: int, length: int):
        self.pos = np.zeros((n_slots, length, 3))
        self.valid = np.zeros((n_slots, length), dtype=bool)
        self.head = 0
        self.length = length

    def push(self, positions: np.ndarray, active: np.ndarray) -> None:
        self.head = (self.head + 1) % self.length
        self.pos[:, self.head] = positions
        self.valid[:, self.head] = active
        # A slot that just became inactive must not connect to its next life.
        self.valid[~active] = False

    def segments(self, mask: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        """(vertices (M*2, 3), age in [0, 1] per vertex) for slots in `mask`."""
        order = (self.head + 1 + np.arange(self.length)) % self.length  # oldest -> newest
        p = self.pos[mask][:, order]
        v = self.valid[mask][:, order]
        ok = v[:, :-1] & v[:, 1:]
        a = np.stack([p[:, :-1], p[:, 1:]], axis=2)[ok]  # (M, 2, 3)
        age = np.broadcast_to(np.arange(self.length - 1) / (self.length - 1), ok.shape)[ok]
        return a.reshape(-1, 3), np.repeat(age, 2)


def ray_plane(origin: np.ndarray, direction: np.ndarray, z: float):
    """Intersection of a ray with the horizontal plane at height z (or None)."""
    o, d = np.asarray(origin, float), np.asarray(direction, float)
    if abs(d[2]) < 1e-12:
        return None
    t = (z - o[2]) / d[2]
    return None if t < 0 else o + t * d


def box_mesh(lo, hi):
    """Vertices and triangle faces of an axis-aligned box."""
    (x0, y0, z0), (x1, y1, z1) = lo, hi
    v = np.array([[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0], [x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], float)
    f = np.array([[0, 1, 2], [0, 2, 3], [4, 6, 5], [4, 7, 6], [0, 5, 1], [0, 4, 5], [1, 6, 2], [1, 5, 6], [2, 7, 3], [2, 6, 7], [3, 4, 0], [3, 7, 4]])
    return v, f


def cylinder_mesh(cx, cy, r, z0, z1, cols: int = 20):
    """Vertices and faces of a closed vertical cylinder."""
    th = np.linspace(0, 2 * np.pi, cols, endpoint=False)
    ring = np.column_stack([cx + r * np.cos(th), cy + r * np.sin(th)])
    v = np.vstack([np.column_stack([ring, np.full(cols, z0)]), np.column_stack([ring, np.full(cols, z1)]), [[cx, cy, z0], [cx, cy, z1]]])
    faces = []
    for i in range(cols):
        j = (i + 1) % cols
        faces += [[i, j, cols + j], [i, cols + j, cols + i], [2 * cols, j, i], [2 * cols + 1, cols + i, cols + j]]
    return v, np.array(faces)


class Dust:
    """Shamal dust particles advected by the wind, wrapped in a box."""

    def __init__(self, n: int, half_extent: float, top: float, seed: int = 0):
        rng = np.random.default_rng(seed)
        self.half = half_extent
        self.top = top
        self.p = np.column_stack([rng.uniform(-half_extent, half_extent, (n, 2)), rng.uniform(0, top, n)])
        self.jitter = rng.normal(0, 1, (n, 3)) * np.array([3.0, 3.0, 0.6])

    def advance(self, dt: float, wind_from_deg: float, speed: float) -> np.ndarray:
        to = np.radians(wind_from_deg + 180.0)
        v = np.array([np.sin(to), np.cos(to), 0.0]) * speed
        self.p += (v + self.jitter) * dt
        self.p[:, :2] = (self.p[:, :2] + self.half) % (2 * self.half) - self.half
        self.p[:, 2] = np.mod(self.p[:, 2], self.top)
        return self.p
