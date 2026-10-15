"""GPU-resident terrain and dense, non-authoritative line-of-sight (ADR-0003).

The heightfield lives on the RTX 3070 as one float32 tensor (~9 MB for a
full sector). Two operations run there:

- ``sample(x, y)``: bilinear heights for any number of points, matching the
  Rust sampler to float32 precision;
- ``los_lipschitz(a, b)``: the Rust Lipschitz march, run for all rays in
  lockstep (used for coverage and visibility *maps*);
- ``los(a, b)``: brute-force fixed-step march, kept as the measured baseline
  (Phase 2 Red-Team P2-1: it lost to the CPU by 17-46x).

GPU results never feed simulation state. A fixed-step march can step over a
ridge thinner than its spacing, so it can only err toward "visible". The
CPU Lipschitz march (Rust) is authoritative, and ``tests/test_terrain.py``
measures the agreement between the two.
"""

from __future__ import annotations

import math

import numpy as np

from hawksgrip.gpu.device import VramBudget

MIB = 1024 * 1024


class TerrainTensor:
    def __init__(self, session, budget: VramBudget | None = None, device: str = "cuda"):
        import torch

        self.torch = torch
        info = session.terrain_info()
        heights = session.terrain_heights()  # (ny, nx) float32, row = north
        self.budget = budget or VramBudget()
        self.budget.reserve("terrain", heights.nbytes)
        self.h = torch.from_numpy(heights).to(device)
        self.device = self.h.device
        self.ny, self.nx = heights.shape
        self.cell = float(info["cell_m"])
        self.ox, self.oy = (float(v) for v in info["origin"])
        self.eps = 0.5  # same grazing tolerance as the CPU march
        self.lipschitz = float(info["lipschitz"])

    @property
    def nbytes(self) -> int:
        return self.h.element_size() * self.h.nelement()

    def sample(self, x, y):
        """Bilinear height at tensors x, y (any matching shape), clamped at edges."""
        fx = ((x - self.ox) / self.cell).clamp(0, self.nx - 1)
        fy = ((y - self.oy) / self.cell).clamp(0, self.ny - 1)
        ix = fx.floor().long().clamp(max=self.nx - 2)
        iy = fy.floor().long().clamp(max=self.ny - 2)
        tx, ty = fx - ix, fy - iy
        flat = self.h.reshape(-1)
        base = iy * self.nx + ix
        h00 = flat[base]
        h10 = flat[base + 1]
        h01 = flat[base + self.nx]
        h11 = flat[base + self.nx + 1]
        a = h00 + (h10 - h00) * tx
        b = h01 + (h11 - h01) * tx
        return a + (b - a) * ty

    def los(self, a, b, spacing_m: float = 5.0, chunk_bytes: int = 256 * MIB):
        """Fixed-step LoS for rays a[i] -> b[i] (tensors of shape (N, 3)).
        Visible if every sample is at least eps above ground (last metre
        excluded, as on the CPU). Processes rays in chunks to bound VRAM."""
        t = self.torch
        a = t.as_tensor(a, dtype=t.float32, device=self.device)
        b = t.as_tensor(b, dtype=t.float32, device=self.device)
        d = b - a
        len_h = t.linalg.vector_norm(d[:, :2], dim=1)
        s_end = (len_h - 1.0).clamp(min=0.0)
        k = max(2, math.ceil(float(s_end.max().item()) / spacing_m) + 1)
        # ~6 float32 temporaries of shape (rays, k) per chunk.
        rays_per_chunk = max(1, chunk_bytes // (k * 4 * 6))
        self.budget.reserve("los_scratch", min(len(a), rays_per_chunk) * k * 4 * 6)
        out = t.empty(len(a), dtype=t.bool, device=self.device)
        u = t.linspace(0.0, 1.0, k, device=self.device)
        try:
            for i in range(0, len(a), rays_per_chunk):
                sl = slice(i, i + rays_per_chunk)
                frac = (s_end[sl, None] * u[None, :]) / len_h[sl, None].clamp(min=1e-9)
                p = a[sl, None, :] + d[sl, None, :] * frac[..., None]
                clear = p[..., 2] - self.sample(p[..., 0], p[..., 1])
                out[sl] = (clear >= self.eps).all(dim=1)
        finally:
            self.budget.release("los_scratch")
        return out

    def los_lipschitz(self, a, b, max_iter: int = 2048, check_every: int = 8):
        """Lipschitz ray-march on the GPU: the same algorithm as the
        authoritative Rust march (step (c - eps)/(|m| + L), stop when c < 2 eps),
        run for all rays in lockstep. Finished rays are compacted out every
        `check_every` iterations. Rays still unresolved after `max_iter` are
        reported as blocked (conservative) and counted in `self.unresolved`.

        Returns a bool tensor (True = visible)."""
        t = self.torch
        a = t.as_tensor(a, dtype=t.float32, device=self.device)
        b = t.as_tensor(b, dtype=t.float32, device=self.device)
        d = b - a
        len_h = t.linalg.vector_norm(d[:, :2], dim=1).clamp(min=1e-6)
        u = d[:, :2] / len_h[:, None]
        m = d[:, 2] / len_h
        rate = m.abs() + self.lipschitz
        s_end = len_h - 1.0
        s = t.zeros_like(len_h)
        visible = t.zeros(len(a), dtype=t.bool, device=self.device)
        # Near-vertical rays: lower endpoint above ground (as on the CPU).
        vertical = s_end <= 0
        if vertical.any():
            low = t.where((a[:, 2] < b[:, 2])[:, None], a, b)[vertical]
            visible[vertical] = low[:, 2] - self.sample(low[:, 0], low[:, 1]) >= self.eps
        act = t.nonzero(~vertical).squeeze(1)
        done = t.zeros(len(a), dtype=t.bool, device=self.device)
        self.iterations = 0
        for it in range(max_iter):
            si = s[act]
            x = a[act, 0] + u[act, 0] * si
            y = a[act, 1] + u[act, 1] * si
            z = a[act, 2] + m[act] * si
            c = z - self.sample(x, y)
            blocked = c < 2 * self.eps
            s_new = si + (c - self.eps) / rate[act]
            clear = (~blocked) & (s_new >= s_end[act])
            visible[act] = visible[act] | clear
            done[act] = blocked | clear
            s[act] = s_new
            self.iterations = it + 1
            if (it + 1) % check_every == 0:
                act = act[~done[act]]
                if act.numel() == 0:
                    break
        act = act[~done[act]]
        self.unresolved = int(act.numel())  # left as not visible: conservative
        return visible

    def to_numpy_heights(self) -> np.ndarray:
        return self.h.cpu().numpy()
