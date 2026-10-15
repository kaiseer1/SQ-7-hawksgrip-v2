"""SQ-7 Hawksgrip 3D showroom (Phases 0-3).

    .venv/Scripts/python -m hawksgrip.hmi.showroom --sector highland_wadi --raid 6 --birds 12

One OpenGL viewport (no per-camera rendering): terrain, the synthetic
refinery, Kindah on her orbit, the Seven, drones and birds (ground truth),
Kindah's tracks (her beliefs), and her 24 eyes as live sightline frustums.
The HUD shows the zero-perturbation counters, the sensing kill-chain
stopwatch and the persona telemetry log. See ``hud.HOTKEYS`` for controls.

Scripted runs for documentation and tests::

    python -m hawksgrip.hmi.showroom --at 1:raid:6 --at 30:storm \\
        --frames 3600 --screenshot results/figures/showroom.png
"""

from __future__ import annotations

import argparse
import sys
import time
from html import escape
from pathlib import Path

import numpy as np
import pyqtgraph as pg
import pyqtgraph.opengl as gl
from PySide6 import QtCore, QtGui, QtWidgets

from hawksgrip import FIRST_SEVEN, FIRST_THREAT, SEVEN, PersonaEngine, Session
from hawksgrip.bridge import LockstepDriver
from hawksgrip.config import config_json
from hawksgrip.hmi import hud, scene, theme

KM = 1e-3  # world metres -> scene kilometres
STORM_VIS_M = 700.0
CLEAR_VIS_M = 20_000.0
TIME_SCALES = [0.5, 1.0, 2.0, 4.0, 8.0, 16.0]
CAMERAS = ["overview", "follow Kindah", "top-down"]
LOG_LINES = 160
TRAIL_LEN = 160  # samples
TRAIL_EVERY = 6  # ticks between trail samples (0.05 s)
CONE_LENGTH_M = 2_200.0
HUD_EVERY_FRAMES = 30  # 2 Hz at 60 fps


class View(gl.GLViewWidget):
    """GL viewport that reports Shift+clicks (plain drags still orbit the camera)."""

    shift_clicked = QtCore.Signal(float, float)

    def mousePressEvent(self, ev):
        if ev.button() == QtCore.Qt.MouseButton.LeftButton and ev.modifiers() & QtCore.Qt.KeyboardModifier.ShiftModifier:
            p = ev.position()
            self.shift_clicked.emit(p.x(), p.y())
            return
        super().mousePressEvent(ev)

    def unproject(self, x: float, y: float):
        """World ray (origin, direction) in scene units through widget pixel (x, y)."""
        w, h = max(self.width(), 1), max(self.height(), 1)
        m = self.projectionMatrix() * self.viewMatrix()
        inv, ok = m.inverted()
        if not ok:
            return None
        nx, ny = 2.0 * x / w - 1.0, 1.0 - 2.0 * y / h
        a = inv.map(QtGui.QVector4D(nx, ny, -1.0, 1.0))
        b = inv.map(QtGui.QVector4D(nx, ny, 1.0, 1.0))
        a = np.array([a.x(), a.y(), a.z()]) / a.w()
        b = np.array([b.x(), b.y(), b.z()]) / b.w()
        d = b - a
        return a, d / np.linalg.norm(d)


def _label(parent, css_extra: str = "") -> QtWidgets.QLabel:
    lab = QtWidgets.QLabel(parent)
    lab.setTextFormat(QtCore.Qt.TextFormat.RichText)
    lab.setStyleSheet(
        f"QLabel {{ background: {theme.PANEL_RGBA}; color: {theme.INK}; border-radius: 6px; padding: 8px 10px;"
        f" font-family: Consolas, 'Courier New', monospace; font-size: 11px; {css_extra} }}"
    )
    lab.setAttribute(QtCore.Qt.WidgetAttribute.WA_TransparentForMouseEvents)
    return lab


class Showroom(QtWidgets.QWidget):
    def __init__(self, session: Session, persona: PersonaEngine, args):
        super().__init__()
        self.s = session
        self.persona = persona
        self.args = args
        self.hz = session.tick_hz
        self.driver = LockstepDriver(session, max_ticks_per_frame=self.hz)
        self.time_scale_i = TIME_SCALES.index(1.0)
        self.paused = False
        self.camera_i = 0
        self.flash_until = -1.0
        self.banner_until = -1.0
        self.compute_us = 0.0
        self.rtf = 0.0
        self.frame_i = 0
        self.thermal = None
        self.storm = None
        self.show_keepout = False
        self.coverage_on = False
        self.coverage = None
        self.n = session.n_entities
        self.classes = session.object_classes()
        self.trails = scene.Trails(self.n, TRAIL_LEN)
        self.tick_last_trail = -TRAIL_EVERY
        self.timelines: list[dict] = []
        self.tracks: list[dict] = []
        self.dust = scene.Dust(2500, 7_500.0, 1_400.0, seed=1)
        self.wall_prev = time.perf_counter_ns()

        self.setWindowTitle("SQ-7 Hawksgrip · Showroom")
        self.resize(args.width, args.height)
        self.view = View(self)
        self.view.setBackgroundColor(theme.BG)
        self.view.shift_clicked.connect(self.on_shift_click)
        lay = QtWidgets.QVBoxLayout(self)
        lay.setContentsMargins(0, 0, 0, 0)
        lay.addWidget(self.view)

        self._build_static()
        self._build_dynamic()
        self._build_hud()
        self.apply_camera()
        self.update_environment(force=True)

        self.timer = QtCore.QTimer(self)
        self.timer.timeout.connect(self.on_timer)

    # ---- construction ----------------------------------------------------------

    def _build_static(self):
        info = self.s.terrain_info()
        self.info = info
        x, y, z = scene.terrain_grid(self.s.terrain_heights(), info["cell_m"], info["origin"], self.args.terrain_step)
        self.terrain_z = z
        self.terrain_cell = info["cell_m"] * self.args.terrain_step
        self.terrain = gl.GLSurfacePlotItem(
            x=x * KM, y=y * KM, z=z * KM, colors=scene.terrain_colors(z, self.terrain_cell, False), smooth=True, computeNormals=False
        )
        self.terrain.setGLOptions("opaque")
        self.view.addItem(self.terrain)

        pad = info["pad_z"]
        foot = np.array([[-1000, -1000], [1000, -1000], [1000, 1000], [-1000, 1000], [-1000, -1000]], float)
        self.view.addItem(
            gl.GLLinePlotItem(pos=np.column_stack([foot, np.full(5, pad + 2)]) * KM, color=theme.rgba(theme.KINDAH, 0.55), width=1.5, antialias=True)
        )
        # Refinery structures (true scale) and their keep-out volumes (toggle K).
        self.keepout_items = []
        for st in self.s.refinery_structures():
            for key, is_keepout in (("solid", False), ("keepout", True)):
                sol = st[key]
                if sol["shape"] == "cylinder":
                    v, f = scene.cylinder_mesh(sol["cx"], sol["cy"], sol["r"], sol["z0"], sol["z1"])
                else:
                    v, f = scene.box_mesh(sol["min"], sol["max"])
                md = gl.MeshData(vertexes=v * KM, faces=f)
                if is_keepout:
                    item = gl.GLMeshItem(meshdata=md, drawFaces=False, drawEdges=True, edgeColor=theme.rgba(theme.KEEPOUT, 0.35), smooth=False)
                    item.setVisible(False)
                    self.keepout_items.append(item)
                else:
                    item = gl.GLMeshItem(meshdata=md, color=theme.rgba("#b9b6ad", 1.0), shader="shaded", smooth=False)
                self.view.addItem(item)
        o = self.s.orbit_info()
        self.view.addItem(
            gl.GLLinePlotItem(pos=scene.circle(o["center"], o["radius_m"]) * KM, color=theme.rgba(theme.ORBIT, 0.7), width=1.2, antialias=True)
        )

    def _build_dynamic(self):
        v = self.view
        self.dust_item = gl.GLScatterPlotItem(pos=np.zeros((1, 3)), size=2.0, color=theme.rgba(theme.DUST, 0.0), pxMode=True)
        self.dust_item.setGLOptions("translucent")
        v.addItem(self.dust_item)
        self.coverage_item = gl.GLScatterPlotItem(pos=np.zeros((1, 3)), size=3.0, color=(0, 0, 0, 0), pxMode=True)
        self.coverage_item.setGLOptions("translucent")
        self.coverage_item.setVisible(False)
        v.addItem(self.coverage_item)
        self.trail_item = gl.GLLinePlotItem(pos=np.zeros((2, 3)), mode="lines", width=1.2, antialias=True)
        self.trail_item.setGLOptions("translucent")
        v.addItem(self.trail_item)
        self.cones = gl.GLLinePlotItem(pos=np.zeros((2, 3)), mode="lines", width=1.0, antialias=True)
        self.cones.setGLOptions("additive")
        v.addItem(self.cones)
        self.rings = gl.GLLinePlotItem(pos=np.zeros((2, 3)), mode="lines", width=2.0, antialias=True)
        self.rings.setGLOptions("translucent")
        v.addItem(self.rings)
        self.objects = gl.GLScatterPlotItem(pos=np.zeros((1, 3)), size=6.0, color=(1, 1, 1, 0), pxMode=True)
        self.objects.setGLOptions("translucent")
        v.addItem(self.objects)
        font = QtGui.QFont("Consolas", 9)
        self.kindah_label = gl.GLTextItem(pos=(0, 0, 0), text="KINDAH", color=QtGui.QColor(theme.KINDAH), font=QtGui.QFont("Consolas", 10, QtGui.QFont.Weight.Bold))
        v.addItem(self.kindah_label)
        self.track_labels = []
        for _ in range(16):
            t = gl.GLTextItem(pos=(0, 0, 0), text="", color=QtGui.QColor(theme.ORANGE), font=font)
            t.setVisible(False)
            v.addItem(t)
            self.track_labels.append(t)

    def _build_hud(self):
        self.hud_status = _label(self)
        self.hud_counters = _label(self)
        self.hud_chain = _label(self)
        self.hud_keys = _label(self)
        self.hud_keys.setText(hud.hotkeys_html())
        self.hud_eyes = _label(self, "font-size: 12px;")
        self.banner = QtWidgets.QLabel(self)
        self.banner.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
        self.banner.setStyleSheet(
            f"QLabel {{ color: {theme.KINDAH}; background: rgba(0,0,0,150); border: 1px solid {theme.KINDAH};"
            " border-radius: 8px; padding: 10px 24px; font-family: Georgia, serif; font-size: 26px; letter-spacing: 6px; }"
        )
        self.banner.setAttribute(QtCore.Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        self.banner.hide()
        self.log = QtWidgets.QTextEdit(self)
        self.log.setReadOnly(True)
        self.log.setFocusPolicy(QtCore.Qt.FocusPolicy.NoFocus)
        self.log.setVerticalScrollBarPolicy(QtCore.Qt.ScrollBarPolicy.ScrollBarAlwaysOff)
        self.log.setStyleSheet(
            f"QTextEdit {{ background: {theme.PANEL_RGBA}; color: {theme.INK_2}; border: none; border-radius: 6px; padding: 6px;"
            " font-family: Consolas, 'Courier New', monospace; font-size: 11px; }"
        )
        self.log_lines: list[str] = []

    def resizeEvent(self, ev):
        super().resizeEvent(ev)
        W, H, m = self.width(), self.height(), 12
        for lab in (self.hud_status, self.hud_counters, self.hud_chain, self.hud_keys, self.hud_eyes):
            lab.adjustSize()
        self.hud_status.move(m, m)
        self.hud_counters.move(W - self.hud_counters.width() - m, m)
        self.hud_chain.move(W - self.hud_chain.width() - m, m + self.hud_counters.height() + 8)
        self.hud_eyes.move(m, m + self.hud_status.height() + 8)
        lw, lh = int(W * 0.50), int(H * 0.27)
        self.log.setGeometry(m, H - lh - m, lw, lh)
        self.hud_keys.move(W - self.hud_keys.width() - m, H - self.hud_keys.height() - m)
        self.banner.adjustSize()
        self.banner.move((W - self.banner.width()) // 2, int(H * 0.12))

    # ---- environment and camera -----------------------------------------------

    def update_environment(self, force: bool = False):
        info = self.s.sensor_info()
        self.band = info["band"]
        self.visibility = info["visibility_m"]
        thermal = self.band != "VIS"
        storm = self.visibility < 3_000.0
        if force or thermal != self.thermal:
            self.thermal = thermal
            c = scene.terrain_colors(self.terrain_z, self.terrain_cell, thermal)
            self.terrain.setData(colors=c)
            # pyqtgraph 0.14: a colours-only setData does not schedule a redraw.
            self.terrain._meshdata.setVertexColors(c.reshape(-1, 4))
            self.terrain.meshDataChanged()
        if force or storm != self.storm or thermal != self.thermal:
            self.storm = storm
            bg = theme.BG_THERMAL if thermal and not storm else (theme.BG_STORM if storm else theme.BG)
            self.view.setBackgroundColor(bg)
        if self.coverage is not None:
            self.coverage.refresh_params()

    def apply_camera(self):
        name = CAMERAS[self.camera_i]
        c = self.s.orbit_info()["center"]
        if name == "overview":
            self.view.setCameraPosition(pos=QtGui.QVector3D(0, 0, c[2] * KM * 0.35), distance=21, elevation=26, azimuth=-62)
        elif name == "top-down":
            self.view.setCameraPosition(pos=QtGui.QVector3D(0, 0, 0), distance=26, elevation=89, azimuth=-90)
        # "follow Kindah" is updated every frame in update_scene.

    # ---- inputs -------------------------------------------------------------------

    def keyPressEvent(self, ev):
        k = ev.key()
        Key = QtCore.Qt.Key
        if k == Key.Key_T:
            mode = self.persona.toggle()
            self.push_log(self.s.tick, "SYSTEM", f"VOICE -> {mode.upper()}")
        elif k == Key.Key_S:
            self.s.set_visibility(CLEAR_VIS_M if self.visibility < 3_000.0 else STORM_VIS_M)
        elif k == Key.Key_R:
            self.s.random_raid(4)
            self.push_log(self.s.tick, "SYSTEM", "RED TEAM | RAID OF 4 LAUNCHED")
        elif k == Key.Key_B:
            self.s.spawn_birds(6)
        elif k == Key.Key_K:
            self.show_keepout = not self.show_keepout
            for it in self.keepout_items:
                it.setVisible(self.show_keepout)
        elif k == Key.Key_G:
            self.toggle_coverage()
        elif k == Key.Key_C:
            self.camera_i = (self.camera_i + 1) % len(CAMERAS)
            self.apply_camera()
        elif k == Key.Key_Space:
            self.paused = not self.paused
        elif k in (Key.Key_Plus, Key.Key_Equal):
            self.time_scale_i = min(self.time_scale_i + 1, len(TIME_SCALES) - 1)
        elif k == Key.Key_Minus:
            self.time_scale_i = max(self.time_scale_i - 1, 0)
        elif k == Key.Key_H:
            self.hud_keys.setVisible(not self.hud_keys.isVisible())
        elif k == Key.Key_Escape:
            self.close()
        else:
            super().keyPressEvent(ev)

    def on_shift_click(self, x: float, y: float):
        ray = self.view.unproject(x, y)
        if ray is None:
            return
        p = scene.ray_plane(ray[0], ray[1], self.info["pad_z"] * KM)
        if p is None:
            return
        self.launch_from(np.array(p) / KM)

    def launch_from(self, ground_point_m: np.ndarray):
        d = ground_point_m[:2]
        n = np.linalg.norm(d)
        d = d / n if n > 100 else np.array([1.0, 0.0])
        sx, sy = 7_000.0 * d
        start = (sx, sy, self.s.ground_height(sx, sy) + 120.0)
        target = (0.0, 0.0, self.info["pad_z"] + 20.0)
        self.s.spawn_threat(start, target, 40.0)
        brg = (np.degrees(np.arctan2(d[0], d[1])) + 360) % 360
        self.push_log(self.s.tick, "SYSTEM", f"RED TEAM | DRONE LAUNCHED FROM BEARING {brg:03.0f}")

    def toggle_coverage(self):
        self.coverage_on = not self.coverage_on
        self.coverage_item.setVisible(self.coverage_on)
        if self.coverage_on and self.coverage is None:
            xs = np.linspace(-7_400, 7_400, 120)
            gx, gy = np.meshgrid(xs, xs)
            gz = np.array([self.s.ground_height(a, b) for a, b in zip(gx.ravel(), gy.ravel())]) + 150.0
            self.cov_vox = np.column_stack([gx.ravel(), gy.ravel(), gz])
            self.cov_last = np.full(len(self.cov_vox), -1e9)
            try:
                import torch

                if not torch.cuda.is_available():
                    raise ImportError
                from hawksgrip.gpu.coverage import CoverageGPU

                self.coverage = CoverageGPU(self.s, self.cov_vox)
                self.coverage_backend = "GPU"
            except ImportError:
                self.s.set_coverage_voxels(self.cov_vox)
                self.coverage = None
                self.coverage_backend = "CPU"
            self.push_log(self.s.tick, "SYSTEM", f"COVERAGE LAYER ON ({self.coverage_backend}, {len(self.cov_vox):,} VOXELS AT 150 M AGL)")

    def run_script(self, t_s: float):
        """Scripted red-team actions (--at T:ACTION[:ARG])."""
        while self.args.script and self.args.script[0][0] <= t_s:
            _, action, arg = self.args.script.pop(0)
            if action == "raid":
                self.s.random_raid(int(arg or 4))
            elif action == "birds":
                self.s.spawn_birds(int(arg or 6))
            elif action == "storm":
                self.s.set_visibility(float(arg or STORM_VIS_M))
            elif action == "clear":
                self.s.set_visibility(CLEAR_VIS_M)
            elif action == "voice":
                self.persona.set_mode(arg or "clinical")
            elif action == "camera":
                self.camera_i = CAMERAS.index(arg.replace("_", " ")) if arg else 0
                self.apply_camera()
            elif action == "coverage":
                self.toggle_coverage()
            elif action == "keepout":
                self.keyPressEvent(QtGui.QKeyEvent(QtCore.QEvent.Type.KeyPress, QtCore.Qt.Key.Key_K, QtCore.Qt.KeyboardModifier.NoModifier))
            elif action == "launch":
                az = np.radians(float(arg or 0.0))
                self.launch_from(np.array([np.sin(az), np.cos(az), 0.0]) * 5_000.0)

    # ---- the frame loop -------------------------------------------------------------

    def on_timer(self):
        now = time.perf_counter_ns()
        wall = now - self.wall_prev
        self.wall_prev = now
        self.frame(wall_dt_ns=wall)

    def frame(self, wall_dt_ns: int | None = None, fixed_ticks: int | None = None):
        self.run_script(self.s.time)
        t0 = time.perf_counter_ns()
        n = 0
        if not self.paused:
            self.driver.time_scale = TIME_SCALES[self.time_scale_i]
            n = self.driver.advance(wall_dt_ns) if fixed_ticks is None else self.s.step(fixed_ticks) and fixed_ticks
        elapsed = time.perf_counter_ns() - t0
        if n:
            us = elapsed / n / 1e3
            self.compute_us = us if self.compute_us == 0 else 0.9 * self.compute_us + 0.1 * us
            self.rtf = 1e6 / (self.compute_us * self.hz)
        events = self.s.drain_events()
        self.on_events(events)
        self.update_scene(dt_s=n / self.hz)
        # Any change to an overlay label makes Qt re-composite the whole window
        # over the GL view (~10-15 ms at 1600 x 900), so the HUD refreshes all
        # panels together at 2 Hz rather than a little every frame (measured:
        # see docs/adr/0012).
        if self.frame_i % HUD_EVERY_FRAMES == 0:
            self.update_hud()
        self.frame_i += 1

    def on_events(self, events):
        for ev in events:
            kind = ev[1]
            if kind in ("THREAT_SPAWNED", "BIRDS_SPAWNED"):
                self.classes = self.s.object_classes()
            if kind == "CONVERGENCE":
                self.flash_until = self.s.time + 0.35
                self.banner_until = self.s.time + 2.5
                self.banner.setText("SHE LOOKS AT YOU" if self.persona.mode == "theatrical" else "ARRAY CONVERGE · ALL EYES")
                self.resizeEvent(None)
            if kind in ("BAND_SWITCH", "VISIBILITY_CHANGED"):
                self.update_environment()
        for tick, speaker, _kind, text in self.persona.voice(self.s, events):
            self.push_log(tick, speaker, text)

    def push_log(self, tick: int, speaker: str, text: str):
        color = theme.SPEAKER.get(speaker, theme.SEVEN)
        line = (
            f"<span style='color:{theme.INK_3}'>{tick / self.hz:8.1f}s</span> "
            f"<span style='color:{color}'><b>{escape(speaker):<8}</b></span> {escape(text)}"
        )
        self.log_lines.append(line)
        # Append incrementally; rebuild only when the buffer is 50 % over size
        # (a full setHtml per line caused 20+ ms frame spikes).
        if len(self.log_lines) > LOG_LINES * 3 // 2:
            self.log_lines = self.log_lines[-LOG_LINES:]
            self.log.setHtml("<br>".join(self.log_lines))
        else:
            self.log.append(line)
        self.log.verticalScrollBar().setValue(self.log.verticalScrollBar().maximum())

    def update_scene(self, dt_s: float):
        pos = self.s.positions()
        active = self.s.active_mask()
        q = pos[0]
        tick = self.s.tick
        # Truth layer: Kindah, the Seven, drones and birds.
        drone = (self.classes == 0) & active
        bird = (self.classes == 1) & active
        sev = slice(FIRST_SEVEN, FIRST_SEVEN + SEVEN)
        cols = np.zeros((self.n, 4), np.float32)
        size = np.zeros(self.n, np.float32)
        cols[0], size[0] = theme.rgba(theme.KINDAH), 16
        cols[sev], size[sev] = theme.rgba(theme.SEVEN), 9
        hot = "#ffffff" if self.thermal else theme.DRONE
        cols[drone], size[drone] = theme.rgba(hot), 12
        cols[bird], size[bird] = theme.rgba("#9a9a9a" if self.thermal else theme.BIRD, 0.9), 6
        show = active.copy()
        self.objects.setData(pos=pos[show] * KM, color=cols[show], size=size[show])
        self.kindah_label.setData(pos=(q + np.array([0, 0, 180.0])) * KM)

        # Trails (truth), sampled every TRAIL_EVERY ticks.
        if tick - self.tick_last_trail >= TRAIL_EVERY:
            self.tick_last_trail = tick
            self.trails.push(pos, drone | bird)
        seg, age = self.trails.segments(drone | bird)
        if len(seg):
            base = np.array(theme.rgba("#ffffff" if self.thermal else theme.DRONE))
            tc = np.tile(base, (len(seg), 1)).astype(np.float32)
            tc[:, 3] = 0.08 + 0.55 * age
            self.trail_item.setData(pos=seg * KM, color=tc)
        else:
            self.trail_item.setData(pos=np.zeros((2, 3)), color=(0, 0, 0, 0))

        # Kindah's 24 eyes.
        eyes = self.s.eyes()
        info = self.s.sensor_info()
        segs = scene.frustum_segments(q, eyes["dirs"], info["tan_h"], info["tan_v"], CONE_LENGTH_M)
        flash = self.s.time < self.flash_until
        self.cones.setData(pos=segs * KM, color=scene.cone_colors(eyes["task"], flash, self.thermal))
        self.eye_task = eyes["task"]

        # Kindah's beliefs: track rings and labels.
        self.tracks = self.s.tracks()
        shown = [t for t in self.tracks if t["state"] in ("tentative", "confirmed", "hostile", "lost")]
        if shown:
            centers = np.array([t["est_p"] for t in shown])
            ring = scene.ring_segments(centers, 140.0)
            colmap = {"tentative": theme.TRACK_TENTATIVE, "confirmed": theme.TRACK_CONFIRMED, "hostile": theme.TRACK_HOSTILE, "lost": theme.TRACK_LOST}
            rc = np.repeat(np.array([theme.rgba(colmap[t["state"]], 0.95) for t in shown], np.float32), len(ring) // len(shown), axis=0)
            self.rings.setData(pos=ring * KM, color=rc)
        else:
            self.rings.setData(pos=np.zeros((2, 3)), color=(0, 0, 0, 0))
        hostile = [t for t in shown if t["state"] in ("hostile", "lost")][: len(self.track_labels)]
        for lab, t in zip(self.track_labels, hostile + [None] * (len(self.track_labels) - len(hostile))):
            if t is None:
                lab.setVisible(False)
                continue
            text = f"OBJ {t['slot']:03d} " + ("LOCK" if t["locked"] else t["state"].upper()) + (f" · {t['eyes']} EYES" if t["eyes"] else "")
            lab.setData(pos=(np.array(t["est_p"]) + np.array([0, 0, 200.0])) * KM, text=text)
            lab.setVisible(True)

        # Shamal dust.
        if self.storm:
            env = self.args
            p = self.dust.advance(max(dt_s, 1 / 60), env.wind_from_deg, 15.0)
            self.dust_item.setData(pos=(p + np.array([0, 0, self.info["pad_z"]])) * KM, color=theme.rgba(theme.DUST, 0.30), size=2.2)
            self.dust_item.setVisible(True)
        else:
            self.dust_item.setVisible(False)

        # GPU coverage layer.
        if self.coverage_on:
            if self.coverage is not None:
                flags = self.coverage.frame().cpu().numpy()
            else:
                flags = self.s.coverage_resident()
            seen = flags == 3
            self.cov_last[seen] = self.s.time
            age = np.clip(self.s.time - self.cov_last, 0, 4.0) / 4.0
            c = scene.ramp(1.0 - age, ["#0b1f38", theme.BLUE, "#cde2fb"]).astype(np.float32)
            c[:, 3] = np.where(flags & 2, 0.25 + 0.55 * (1 - age), 0.0)
            self.coverage_item.setData(pos=self.cov_vox * KM, color=c, size=3.0)

        if CAMERAS[self.camera_i] == "follow Kindah":
            v = self.s.velocities()[0]
            heading = np.degrees(np.arctan2(v[1], v[0]))
            self.view.setCameraPosition(pos=QtGui.QVector3D(*(q * KM)), distance=4.5, elevation=18, azimuth=heading + 180)

        if self.s.time > self.banner_until:
            self.banner.hide()
        elif not self.banner.isVisible():
            self.banner.show()

    def update_hud(self, part: int | None = None):
        """Refresh HUD panels: all of them, or one (0 status, 1 counters, 2 chain, 3 eyes)."""
        parts = range(4) if part is None else [part]
        for p in parts:
            if p == 0:
                self.hud_status.setText(
                    hud.status_html(
                        self.s.time, self.info["label"], self.band, self.visibility, self.persona.mode,
                        TIME_SCALES[self.time_scale_i], self.rtf, self.paused, CAMERAS[self.camera_i],
                    )
                )
            elif p == 1:
                self.hud_counters.setText(hud.counters_html(self.s.counters(), self.s.sensor_counters(), self.tracks))
            elif p == 2:
                self.timelines = self.s.timelines()
                self.hud_chain.setText(hud.kill_chain_html(self.timelines, self.hz, self.compute_us))
            else:
                self.hud_eyes.setText(hud.eyes_summary(getattr(self, "eye_task", np.zeros(24))))
        sizes = tuple(lab.sizeHint().toTuple() for lab in (self.hud_status, self.hud_counters, self.hud_chain, self.hud_eyes))
        if sizes != getattr(self, "_hud_sizes", None):
            self._hud_sizes = sizes
            self.resizeEvent(None)


def parse_at(spec: str):
    parts = spec.split(":")
    if len(parts) < 2:
        raise argparse.ArgumentTypeError(f"--at expects T:ACTION[:ARG], got {spec!r}")
    return (float(parts[0]), parts[1], parts[2] if len(parts) > 2 else None)


def build(argv=None):
    """Parse arguments and create (app, window, args) without running."""
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sector", default="highland_wadi", choices=["flat", "eastern_sabkha", "red_sea_corridor", "highland_wadi"])
    ap.add_argument("--seed", type=int, default=7)
    ap.add_argument("--raid", type=int, default=0, help="drones launched at t = 0")
    ap.add_argument("--birds", type=int, default=12)
    ap.add_argument("--visibility", type=float, default=CLEAR_VIS_M)
    ap.add_argument("--mode", default="theatrical", choices=["theatrical", "clinical"])
    ap.add_argument("--at", dest="script", action="append", type=parse_at, default=[], help="scripted action T:ACTION[:ARG] (raid, birds, storm, clear, voice, camera, coverage, keepout, launch)")
    ap.add_argument("--terrain-step", type=int, default=5, help="terrain mesh subsampling (5 = 50 m)")
    ap.add_argument("--width", type=int, default=1600)
    ap.add_argument("--height", type=int, default=900)
    ap.add_argument("--frames", type=int, default=0, help="run this many fixed-step frames, then exit")
    ap.add_argument("--ticks-per-frame", type=int, default=2)
    ap.add_argument("--screenshot", type=Path, help="save the final frame (with --frames)")
    args = ap.parse_args(argv)
    args.script.sort(key=lambda a: a[0])
    args.wind_from_deg = 330.0

    cfg = config_json({"terrain": {"sector": args.sector}, "environment": {"visibility_m": args.visibility}})
    s = Session(cfg, args.seed)
    if args.raid:
        s.random_raid(args.raid)
    if args.birds:
        s.spawn_birds(args.birds)
    persona = PersonaEngine(args.seed, s.tick_hz, args.mode)

    # Shared GL contexts keep pyqtgraph's cached shader programs valid if a
    # window is ever recreated in the same process.
    QtCore.QCoreApplication.setAttribute(QtCore.Qt.ApplicationAttribute.AA_ShareOpenGLContexts)
    app = pg.mkQApp("SQ-7 Hawksgrip")
    w = Showroom(s, persona, args)
    w.show()
    return app, w, args


def run_scripted(app, w, args) -> None:
    """Fixed-step frames (deterministic pacing) and an optional screenshot."""
    for _ in range(args.frames):
        w.frame(fixed_ticks=args.ticks_per_frame)
        app.processEvents()
    w.update_hud()
    app.processEvents()
    if args.screenshot:
        args.screenshot.parent.mkdir(parents=True, exist_ok=True)
        w.grab().save(str(args.screenshot))
        print(f"saved {args.screenshot}")


def main(argv=None) -> int:
    app, w, args = build(argv)
    if args.frames:
        run_scripted(app, w, args)
        w.close()
        return 0
    w.timer.start(16)
    return app.exec()


if __name__ == "__main__":
    sys.exit(main())
