"""Lockstep driver: maps wall-clock time onto fixed 120 Hz ticks (ADR-0001).

The renderer calls ``advance(wall_dt_ns)`` once per frame (about 60 fps). The
driver works out how many whole ticks are due and steps the Rust engine that
many times. The simulation never sees wall-clock time, only tick counts, so
frame-rate hiccups cannot change its results.

All bookkeeping is in integer nanoseconds, so after one hour at 60 fps
exactly 432 000 ticks have run. A float accumulator would drift.

If a frame overruns (window drag, GC pause), at most ``max_ticks_per_frame``
ticks are run and the rest of the backlog is *dropped*, i.e. the simulation
briefly runs slower than real time instead of spiralling. Dropped ticks are
counted and shown on the HUD; they never alter the simulation's outcome.
"""

from __future__ import annotations

from typing import Protocol

NS_PER_S = 1_000_000_000


class Steppable(Protocol):
    tick_hz: int

    def step(self, n: int) -> int: ...


class LockstepDriver:
    def __init__(self, session: Steppable, time_scale: float = 1.0, max_ticks_per_frame: int | None = None):
        if time_scale <= 0:
            raise ValueError("time_scale must be positive")
        self.session = session
        self.hz = int(session.tick_hz)
        self.time_scale = time_scale
        # Default catch-up cap: a quarter second of simulation per frame.
        self.max_ticks_per_frame = max_ticks_per_frame or max(1, self.hz // 4)
        self._sim_ns = 0  # scaled wall time owed to the simulation
        self._ticks_due_base = 0  # ticks already accounted for (run or dropped)
        self.ticks_run = 0
        self.ticks_dropped = 0

    def advance(self, wall_dt_ns: int) -> int:
        """Run the ticks due for ``wall_dt_ns`` of wall time. Returns ticks run."""
        if wall_dt_ns < 0:
            raise ValueError("wall time cannot go backwards")
        if self.time_scale == 1.0:
            self._sim_ns += int(wall_dt_ns)
        else:
            self._sim_ns += round(wall_dt_ns * self.time_scale)
        due = self._sim_ns * self.hz // NS_PER_S - self._ticks_due_base
        n = min(due, self.max_ticks_per_frame)
        self.ticks_dropped += due - n
        self._ticks_due_base += due
        if n:
            self.session.step(n)
            self.ticks_run += n
        return n

    @property
    def alpha(self) -> float:
        """Fraction of the way to the next tick, for render interpolation."""
        return (self._sim_ns * self.hz % NS_PER_S) / NS_PER_S
