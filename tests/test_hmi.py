"""Showroom HMI: geometry, HUD text, persona binding, and a live smoke test."""

import numpy as np
import pytest

from hawksgrip import PersonaEngine, Session
from hawksgrip.config import config_json
from hawksgrip.hmi import hud, scene


def session(sector="eastern_sabkha", seed=3):
    return Session(config_json({"terrain": {"sector": sector}}), seed)


# ---- scene geometry ---------------------------------------------------------------


def test_frustum_geometry_matches_the_engine():
    """The cones drawn on screen are the frustums the engine detects with."""
    s = session()
    s.step(120 * 5)
    q = s.positions()[0]
    eyes = s.eyes()["dirs"]
    info = s.sensor_info()
    f, r, u = scene.eye_basis(eyes)
    rng = np.random.default_rng(0)
    pts = q + rng.normal(size=(20_000, 3)) * 2_000.0
    d = pts - q
    d /= np.linalg.norm(d, axis=1, keepdims=True)
    fz = d @ f.T
    py_in = ((fz > 0) & (np.abs(d @ r.T) <= info["tan_h"] * fz) & (np.abs(d @ u.T) <= info["tan_v"] * fz)).any(axis=1)
    rust_in = (s.coverage_cpu(pts) & 1) > 0
    assert py_in.sum() > 200
    assert (py_in == rust_in).mean() > 0.9995


def test_frustum_segments_shape_and_apex():
    q = np.array([1.0, 2.0, 3.0])
    dirs = np.array([[1.0, 0, 0], [0, 0, -1.0]])
    seg = scene.frustum_segments(q, dirs, 0.1, 0.08, 100.0)
    assert seg.shape == (2 * 16, 3)
    # Even vertices of the first 4 pairs are the apex; the far plane is 100 m out.
    assert np.allclose(seg[0:8:2], q)
    assert np.allclose(seg[1, 0], 101.0)


def test_terrain_colors_and_thermal_grayscale():
    s = session("red_sea_corridor")
    info = s.terrain_info()
    x, y, z = scene.terrain_grid(s.terrain_heights(), info["cell_m"], info["origin"], 10)
    assert z.shape == (len(x), len(y))
    c = scene.terrain_colors(z, info["cell_m"] * 10, thermal=False)
    t = scene.terrain_colors(z, info["cell_m"] * 10, thermal=True)
    assert c.shape == z.shape + (4,) and c.dtype == np.float32
    land = z > 1.0
    assert np.allclose(t[land][:, 0], t[land][:, 1]) and np.allclose(t[land][:, 1], t[land][:, 2])
    assert not np.allclose(c[land][:, 0], c[land][:, 2])


def test_trails_never_bridge_a_respawn():
    tr = scene.Trails(3, 5)
    a = np.array([True, True, False])
    for k in range(4):
        tr.push(np.full((3, 3), float(k)), a)
    seg, age = tr.segments(np.array([True, False, False]))
    assert len(seg) == 2 * 3 and len(age) == len(seg)
    tr.push(np.full((3, 3), 9.0), np.array([False, True, False]))  # slot 0 dies
    tr.push(np.full((3, 3), 10.0), np.array([True, True, False]))  # and respawns
    seg, _ = tr.segments(np.array([True, False, False]))
    assert len(seg) == 0


def test_ray_plane_and_rings():
    p = scene.ray_plane(np.array([0, 0, 10.0]), np.array([1.0, 0, -1.0]), 0.0)
    assert np.allclose(p, [10, 0, 0])
    assert scene.ray_plane(np.array([0, 0, 10.0]), np.array([1.0, 0, 1.0]), 0.0) is None
    assert scene.ring_segments(np.zeros((3, 3)), 5.0, n=8).shape == (3 * 8 * 2, 3)


# ---- HUD ------------------------------------------------------------------------------

TL = {"slot": 9, "class": "drone", "spawn": 0, "first_opportunity": 10, "detected": 12, "confirmed": 24, "classified": 48, "classified_as": "drone", "locked": 60, "end": None, "end_reason": None}


def test_kill_chain_stopwatch():
    html = hud.kill_chain_html([TL, dict(TL, slot=4, locked=None, detected=100)], 120, 15.3)
    assert "OBJ 009" in html  # the locked one is preferred
    assert "0.10 s" in html and "0.20 s" in html and "0.40 s" in html  # 12->24, 24->48, 12->60
    assert "15.3 µs (measured)" in html
    assert "faster than" not in html  # claims policy
    assert "AWAITING" in hud.kill_chain_html([], 120, 0.0)


def test_counters_panel_is_honest():
    c = {"refinery_breaches": 3, "queen_orbit_deviation_max_m": 0.0, "threats_reaching_queen": 0}
    sc = {"detections": 10, "birds_ignored": 1, "false_alarms_cleared": 2, "false_alarms": 2, "band_switches": 1}
    html = hud.counters_html(c, sc, [{"state": "hostile", "locked": True}])
    assert ">3<" in html and "0.00 m" in html
    assert "interception not modelled" in html and "n/a (Phases 0-3)" in html


def test_eyes_summary():
    assert hud.eyes_summary(np.array([0] * 20 + [1] + [2] * 3)) == "EYES  SWEEP 20 · CONFIRM  1 · TRACK  3"


# ---- persona binding ------------------------------------------------------------------


def test_persona_voices_and_toggles():
    s = session("highland_wadi", 4)
    p = PersonaEngine(4, 120, "clinical")
    s.random_raid(3)
    lines = []
    for _ in range(120 * 30 // 10):
        s.step(10)
        lines += p.voice(s, s.drain_events())
    assert any(l[1] == "KINDAH" and l[3].startswith("DETECT |") for l in lines)
    assert any(l[1].startswith("NODE-") for l in lines)
    assert p.toggle() == "theatrical" and p.mode == "theatrical"
    with pytest.raises(ValueError):
        PersonaEngine(0, 120, "shouting")


def test_persona_cannot_change_the_simulation():
    def run(with_persona):
        s = session("highland_wadi", 8)
        p = PersonaEngine(8)
        s.random_raid(6)
        s.spawn_birds(6)
        for k in range(120 * 40 // 10):
            s.step(10)
            ev = s.drain_events()
            if with_persona:
                p.voice(s, ev)
                if k % 7 == 0:
                    p.toggle()
        return s.state_hash()

    assert run(True) == run(False)


# ---- live showroom smoke test ------------------------------------------------------------

pytest.importorskip("PySide6")
pytest.importorskip("pyqtgraph")


def test_showroom_runs_and_obeys_hotkeys(tmp_path):
    from PySide6 import QtCore, QtGui

    from hawksgrip.hmi import showroom

    out = tmp_path / "frame.png"
    app, w, args = showroom.build(
        ["--sector", "eastern_sabkha", "--birds", "4", "--at", "0.5:raid:3", "--frames", "240",
         "--screenshot", str(out), "--width", "960", "--height", "540", "--terrain-step", "10"]
    )
    showroom.run_scripted(app, w, args)
    img = QtGui.QImage(str(out))
    assert img.width() == 960 and img.height() == 540
    # Not a blank frame: sample pixel variance.
    px = np.array([QtGui.QColor(img.pixel(x, y)).lightness() for x in range(0, 960, 37) for y in range(0, 540, 29)])
    assert px.std() > 10
    s = w.s
    assert s.counters()["threats_spawned"] == 3

    def key(k):
        w.keyPressEvent(QtGui.QKeyEvent(QtCore.QEvent.Type.KeyPress, k, QtCore.Qt.KeyboardModifier.NoModifier))

    K = QtCore.Qt.Key
    key(K.Key_T)
    assert w.persona.mode == "clinical"
    key(K.Key_S)
    key(K.Key_R)
    for _ in range(4):
        w.frame(fixed_ticks=2)
        app.processEvents()
    assert s.sensor_info()["band"] == "LWIR" and w.thermal and w.storm
    assert s.counters()["threats_spawned"] == 7
    key(K.Key_K)
    assert all(it.visible() for it in w.keepout_items)
    key(K.Key_G)
    w.frame(fixed_ticks=2)
    assert w.coverage_on
    key(K.Key_C)
    assert showroom.CAMERAS[w.camera_i] == "follow Kindah"
    key(K.Key_Space)
    t = s.tick
    w.frame(fixed_ticks=2)
    assert s.tick == t  # paused
    w.launch_from(np.array([0.0, 5_000.0, 0.0]))  # the Shift+click path
    s.step(1)
    assert s.counters()["threats_spawned"] == 8
    log = w.log.toPlainText()
    assert "VOICE -> CLINICAL" in log and "RED TEAM" in log
    w.close()
