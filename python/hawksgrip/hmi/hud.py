"""HUD text builders (pure functions; HTML for Qt rich-text labels).

Claims policy (ADR-0008): engine compute time and physical kill-chain time
are shown separately, no "x times faster than a human" figure is shown, and
the zero-perturbation panel shows the orbit deviation next to the counters
that can actually move.
"""

from __future__ import annotations

from html import escape

from hawksgrip.hmi import theme


def _row(label: str, value: str, color: str = theme.INK) -> str:
    return f"<tr><td style='color:{theme.INK_2}; padding-right:14px'>{escape(label)}</td><td style='color:{color}'><b>{escape(value)}</b></td></tr>"


def _table(title: str, rows: list[str]) -> str:
    return f"<div style='color:{theme.INK_3}; letter-spacing:1px'>{escape(title)}</div><table cellspacing='0'>{''.join(rows)}</table>"


def status_html(t_s: float, sector: str, band: str, visibility_m: float, mode: str, time_scale: float, rtf: float, paused: bool, camera: str) -> str:
    storm = visibility_m < 3_000.0
    band_label = {"VIS": "VIS", "SWIR": "SWIR", "LWIR": "LWIR · THERMAL"}.get(band, band)
    rows = [
        _row("T+", f"{t_s:8.1f} s" + ("  PAUSED" if paused else "")),
        _row("SECTOR", sector),
        _row("BAND", band_label, theme.KINDAH if band != "VIS" else theme.INK),
        _row("VISIBILITY", f"{visibility_m / 1e3:.1f} km" + ("  SHAMAL" if storm else ""), theme.KINDAH if storm else theme.INK),
        _row("VOICE", mode.upper()),
        _row("TIME", f"x{time_scale:g}  (engine {rtf:,.0f}x real time)"),
        _row("CAMERA", camera),
    ]
    return _table("SQ-7 HAWKSGRIP · SHOWROOM", rows)


def counters_html(counters: dict, sensor: dict, tracks: list[dict]) -> str:
    hostile = sum(t["state"] == "hostile" for t in tracks)
    locked = sum(bool(t["locked"]) for t in tracks)
    rows = [
        _row("REFINERY BREACHES", str(counters["refinery_breaches"]), theme.RED if counters["refinery_breaches"] else theme.INK),
        _row("QUEEN ORBIT DEVIATION", f"{counters['queen_orbit_deviation_max_m']:.2f} m"),
        _row("THREATS REACHING QUEEN", str(counters["threats_reaching_queen"])),
        _row("AEGIS INTERVENTIONS", "n/a (Phases 0-3)", theme.INK_3),
    ]
    sense = [
        _row("TRACKS HOSTILE / LOCKED", f"{hostile} / {locked}", theme.ORANGE if hostile else theme.INK),
        _row("DETECTIONS", f"{sensor['detections']:,}"),
        _row("BIRDS IGNORED", str(sensor["birds_ignored"])),
        _row("FALSE ALARMS CLEARED", f"{sensor['false_alarms_cleared']} / {sensor['false_alarms']}"),
        _row("BAND SWITCHES", str(sensor["band_switches"])),
    ]
    note = f"<div style='color:{theme.INK_3}; margin-top:4px'>interception not modelled in Phases 0-3:<br>breaches are expected</div>"
    return _table("ZERO-PERTURBATION", rows) + "<br>" + _table("KINDAH'S EYES", sense) + note


def latest_chain(timelines: list[dict]) -> dict | None:
    """The most recently locked drone timeline, or the latest detected one."""
    drones = [t for t in timelines if t["class"] == "drone" and t["detected"] is not None]
    if not drones:
        return None
    locked = [t for t in drones if t["locked"] is not None]
    pool = locked or drones
    return max(pool, key=lambda t: (t["locked"] or 0, t["detected"]))


def kill_chain_html(timelines: list[dict], hz: int, compute_us: float) -> str:
    t = latest_chain(timelines)
    rows = []
    if t is None:
        rows.append(_row("AWAITING", "first detection"))
    else:

        def dt(a, b):
            if t[a] is None or t[b] is None:
                return "…"
            return f"{(t[b] - t[a]) / hz:6.2f} s"

        rows += [
            _row("OBJECT", f"OBJ {t['slot']:03d}"),
            _row("DETECT → CONFIRM", dt("detected", "confirmed")),
            _row("CONFIRM → CLASSIFY", dt("confirmed", "classified")),
            _row("CLASSIFY → LOCK", dt("classified", "locked")),
            _row("DETECT → LOCK", dt("detected", "locked"), theme.ORANGE),
        ]
    rows.append(_row("ENGINE COMPUTE / TICK", f"{compute_us:6.1f} µs (measured)"))
    note = f"<div style='color:{theme.INK_3}; margin-top:4px'>physical time (sim seconds) and compute time are<br>different quantities; frames are 33 ms apart</div>"
    return _table("KILL-CHAIN STOPWATCH (SENSING)", rows) + note


def eyes_summary(task) -> str:
    sweep = int((task == 0).sum())
    confirm = int((task == 1).sum())
    track = int((task == 2).sum())
    return f"EYES  SWEEP {sweep:2d} · CONFIRM {confirm:2d} · TRACK {track:2d}"


HOTKEYS = [
    ("T", "theatrical / clinical voice"),
    ("S", "Shamal sandstorm on / off"),
    ("R", "red team: raid of 4 drones"),
    ("Shift+click", "launch a drone from that direction"),
    ("B", "release 6 birds"),
    ("K", "keep-out volumes"),
    ("G", "GPU coverage layer (150 m AGL)"),
    ("C", "camera: overview / follow Kindah / top-down"),
    ("Space", "pause"),
    ("+ / -", "time scale"),
    ("H", "hide this panel"),
]


def hotkeys_html() -> str:
    rows = [_row(k, v) for k, v in HOTKEYS]
    legend = (
        f"<div style='color:{theme.INK_3}; margin-top:6px'>"
        f"<span style='color:{theme.KINDAH}'>●</span> Kindah &nbsp; <span style='color:{theme.SEVEN}'>●</span> the Seven &nbsp; "
        f"<span style='color:{theme.DRONE}'>●</span> drone (truth) &nbsp; <span style='color:{theme.BIRD}'>●</span> bird (truth)<br>"
        f"<span style='color:{theme.CONE_SWEEP}'>▭</span> sweeping eye &nbsp; <span style='color:{theme.CONE_CONFIRM}'>▭</span> confirming &nbsp; "
        f"<span style='color:{theme.CONE_TRACK}'>▭</span> tracking &nbsp; ○ Kindah's track (belief)</div>"
    )
    return _table("CONTROLS", rows) + legend
