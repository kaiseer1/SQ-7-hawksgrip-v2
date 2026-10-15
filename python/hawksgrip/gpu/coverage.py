"""GPU dense coverage of the airspace by Kindah's 24 eyes (Phase 3).

For a fixed voxel grid resident in VRAM, each camera frame computes:

- ``IN_VIEW``: voxel direction inside at least one eye's frustum. Three small
  matrix products (N,3)@(3,24) give the forward, right and up components;
  the test |right| <= tan_h * fwd and |up| <= tan_v * fwd avoids division.
- ``DETECTABLE``: pd >= pd_max / 2 for a drone at that voxel. This is the same
  closed-form dust optical depth, pixel margin and logistics as the Rust
  optics (sensing/optics.rs), evaluated elementwise.

and optionally accumulates **revisit statistics**: the longest gap between
two frames in which each voxel was both in view and detectable. This is the
"no blind spots" metric made spatial.

This is the GPU's job under the amended ADR-0003: pure elementwise geometry,
no terrain gathers. The speed-up over the Rust/rayon reference
(``Session.coverage_resident``) is measured in scripts/bench_optics.py, and
the agreement is tested in tests/test_optics.py.
"""

from __future__ import annotations

import numpy as np

from hawksgrip.gpu.device import VramBudget

IN_VIEW = 1
DETECTABLE = 2
SHARPNESS = 8.0  # must match sensing/optics.rs


def eye_bases(dirs: np.ndarray):
    """Right and up vectors of level frustums (same convention as eyes.rs)."""
    f = np.asarray(dirs, dtype=np.float64)
    up_w = np.array([0.0, 0.0, 1.0])
    r = np.cross(f, up_w)
    n = np.linalg.norm(r, axis=1, keepdims=True)
    r = np.where(n < 1e-9, np.array([1.0, 0.0, 0.0]), r / np.maximum(n, 1e-12))
    u = np.cross(r, f)
    return f, r, u


class CoverageGPU:
    def __init__(self, session, voxels: np.ndarray, budget: VramBudget | None = None, chunk: int = 1 << 19, device: str = "cuda"):
        import torch

        self.t = torch
        self.session = session
        self.budget = budget or VramBudget()
        vox = np.ascontiguousarray(voxels, dtype=np.float32)
        n = len(vox)
        self.chunk = min(chunk, n)
        # Resident voxels + flags + revisit state + per-chunk scratch (~8 (chunk, 24) f32 temporaries).
        self.budget.reserve("coverage_voxels", vox.nbytes + n * (1 + 4 + 4))
        self.budget.reserve("coverage_scratch", self.chunk * 24 * 4 * 8)
        self.vox = torch.from_numpy(vox).to(device)
        self.device = self.vox.device
        self.n = n
        self.last_seen = torch.full((n,), float("nan"), device=self.device)
        self.max_gap = torch.zeros(n, device=self.device)
        self.refresh_params()

    def refresh_params(self):
        """Re-read optics (band/visibility can change with [S])."""
        p = self.session.optics_params()
        info = self.session.sensor_info()
        self.p = p
        self.tan_h, self.tan_v = info["tan_h"], info["tan_v"]

    def frame(self, q=None, eye_dirs=None):
        """Coverage flags (uint8 tensor on the GPU) for the current eyes."""
        t = self.t
        if q is None:
            q = self.session.positions()[0]
        if eye_dirs is None:
            eye_dirs = self.session.eyes()["dirs"]
        f, r, u = eye_bases(eye_dirs)
        F = t.as_tensor(f.T, dtype=t.float32, device=self.device)
        R = t.as_tensor(r.T, dtype=t.float32, device=self.device)
        U = t.as_tensor(u.T, dtype=t.float32, device=self.device)
        qt = t.as_tensor(np.asarray(q, dtype=np.float32), device=self.device)
        p = self.p
        H = p["dust_scale_height_m"]
        ha = float(q[2]) - p["ground_ref_z"]
        ea = np.exp(-ha / H)
        out = t.empty(self.n, dtype=t.uint8, device=self.device)
        for i in range(0, self.n, self.chunk):
            d = self.vox[i : i + self.chunk] - qt
            rng = t.linalg.vector_norm(d, dim=1).clamp(min=1e-6)
            uu = d / rng[:, None]
            fz = uu @ F
            inside = (fz > 0) & ((uu @ R).abs() <= self.tan_h * fz) & ((uu @ U).abs() <= self.tan_v * fz)
            in_view = inside.any(dim=1)
            # Detection margins (same algebra as Optics::pd).
            px = p["size_m"] / (rng * p["ifov_mrad"] * 1e-3)
            hb = self.vox[i : i + self.chunk, 2] - p["ground_ref_z"]
            dh = hb - ha
            flat = dh.abs() < 1e-6
            tau = t.where(
                flat,
                p["sigma_ground"] * rng * ea,
                p["sigma_ground"] * rng * H * (ea - t.exp(-hb / H)) / t.where(flat, t.ones_like(dh), dh),
            )
            c_margin = p["contrast"] * t.exp(-tau) / p["contrast_threshold"]
            pd = p["pd_max"] * t.sigmoid(SHARPNESS * t.log(px / p["detect_px"])) * t.sigmoid(SHARPNESS * t.log(c_margin))
            det = pd >= 0.5 * p["pd_max"]
            out[i : i + self.chunk] = in_view.to(t.uint8) * IN_VIEW + det.to(t.uint8) * DETECTABLE
        return out

    def accumulate(self, flags, t_s: float):
        """Update revisit statistics with one frame's flags at time t_s."""
        t = self.t
        seen = flags == (IN_VIEW | DETECTABLE)
        prev = self.last_seen
        gap = t.where(seen & ~t.isnan(prev), t_s - prev, t.zeros_like(prev))
        self.max_gap = t.maximum(self.max_gap, gap)
        self.last_seen = t.where(seen, t.full_like(prev, t_s), prev)

    def revisit(self, t_end: float):
        """(max gap per voxel incl. the open gap at t_end, never-seen mask)."""
        t = self.t
        never = t.isnan(self.last_seen)
        open_gap = t.where(never, t.full_like(self.last_seen, float("inf")), t_end - self.last_seen)
        return t.maximum(self.max_gap, open_gap), never

    def release(self):
        self.budget.release("coverage_voxels")
        self.budget.release("coverage_scratch")
