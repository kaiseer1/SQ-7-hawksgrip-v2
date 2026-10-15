"""Per-frame state buffers, allocated once and refilled in place (ADR-0006)."""

from __future__ import annotations

import numpy as np

from hawksgrip._core import Session


class FrameBuffers:
    def __init__(self, session: Session):
        n = session.n_entities
        self.session = session
        self.pos = np.zeros((n, 3), dtype=np.float64)
        self.vel = np.zeros((n, 3), dtype=np.float64)
        self.roles = session.roles()  # fixed for the whole run
        self.active = session.active_mask()

    def refresh(self) -> None:
        self.session.snapshot_into(self.pos, self.vel)
        self.active = self.session.active_mask()
