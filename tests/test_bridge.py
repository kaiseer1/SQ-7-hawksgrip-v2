import threading
import time

import numpy as np
import pytest

import hawksgrip
from hawksgrip import FIRST_SEVEN, FIRST_THREAT, QUEEN, SEVEN, Session
from hawksgrip.bridge import FrameBuffers


def test_layout_constants():
    assert (QUEEN, FIRST_SEVEN, SEVEN, FIRST_THREAT) == (0, 1, 7, 8)
    s = Session()
    roles = s.roles()
    assert roles[0] == hawksgrip.ROLE_QUEEN
    assert np.all(roles[1:8] == hawksgrip.ROLE_SEVEN)
    assert np.all(roles[8:] == hawksgrip.ROLE_THREAT)
    assert s.n_entities == 8 + 64


def test_step_and_clock():
    s = Session(seed=3)
    assert s.tick == 0 and s.tick_hz == 120
    assert s.step(120) == 120
    assert s.time == 1.0
    assert s.dt == pytest.approx(1 / 120)


def test_snapshot_into_fills_preallocated_buffers():
    s = Session()
    s.random_raid(4)
    s.step(10)
    fb = FrameBuffers(s)
    pos_id = id(fb.pos)
    fb.refresh()
    assert id(fb.pos) == pos_id  # filled in place, not reallocated
    np.testing.assert_array_equal(fb.pos, s.positions())
    np.testing.assert_array_equal(fb.vel, s.velocities())
    assert fb.active[:8].all() and fb.active[8:12].all() and not fb.active[12:].any()


def test_snapshot_into_rejects_bad_buffers():
    s = Session()
    n = s.n_entities
    good = np.zeros((n, 3))
    with pytest.raises(ValueError, match="shape"):
        s.snapshot_into(np.zeros((n, 2)), good)
    with pytest.raises(ValueError, match="contiguous"):
        s.snapshot_into(np.zeros((n, 6))[:, ::2], good)
    with pytest.raises((ValueError, TypeError)):
        s.snapshot_into(np.zeros((n, 3), dtype=np.float32), good)
    with pytest.raises(ValueError):  # same buffer twice: aliasing refused
        s.snapshot_into(good, good)


def test_events_and_counters():
    s = Session(seed=1)
    s.random_raid(3)
    s.step(1)
    ev = s.drain_events()
    # Since Phase 3 the eyes may already report DETECTED on the spawn tick.
    spawned = [e for e in ev if e[1] == "THREAT_SPAWNED"]
    assert len(spawned) == 3
    assert all(e[2] >= FIRST_THREAT for e in spawned)
    assert {e[1] for e in ev} <= {"THREAT_SPAWNED", "DETECTED"}
    assert s.drain_events() == []
    c = s.counters()
    assert c["threats_spawned"] == 3
    assert c["refinery_breaches"] == 0
    assert c["queen_orbit_deviation_max_m"] == 0.0


def test_orders_round_trip():
    s = Session()
    s.set_waypoint(2, (800.0, 800.0, 900.0), 40.0)
    s.set_waypoint(0, (0.0, 0.0, 0.0), 40.0)  # the Queen takes no orders
    s.step(1)
    kinds = [(e[1], e[2]) for e in s.drain_events()]
    assert ("ORDER_ACCEPTED", 2) in kinds
    assert ("ORDER_REJECTED", 0) in kinds


def test_step_releases_the_gil():
    """While Rust steps, another Python thread must keep running."""
    s = Session()
    s.random_raid(64)
    progress = 0
    stop = threading.Event()

    def spin():
        nonlocal progress
        while not stop.is_set():
            progress += 1

    t = threading.Thread(target=spin)
    t.start()
    time.sleep(0.05)
    before = progress
    s.step(120 * 600)  # ~10 simulated minutes in one call
    during = progress - before
    stop.set()
    t.join()
    assert during > 1000, f"other thread starved: {during} iterations"


def test_bad_config_raises():
    with pytest.raises(ValueError, match="orbit"):
        Session('{"queen": {"orbit_radius_m": 50.0}}')
    with pytest.raises(ValueError):
        Session('{"queen": {"radius": 3000.0}}')


def test_terrain_install():
    s = Session()
    h = np.full((11, 11), 250.0, dtype=np.float32)
    s.set_terrain(h, 1500.0, -7500.0, -7500.0)
    assert s.ground_height(0.0, 0.0) == pytest.approx(250.0)
    assert s.positions()[QUEEN, 2] == pytest.approx(250.0 + 1500.0)
    s.step(1)
    with pytest.raises(ValueError, match="before the first tick"):
        s.set_terrain(h, 1500.0, -7500.0, -7500.0)
