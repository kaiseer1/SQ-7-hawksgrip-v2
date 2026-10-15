"""Headless runner (Phase 1). The 3D showroom replaces the console in Phase 7.

    python -m hawksgrip.app --scenario scenarios/single_raid.toml
    python -m hawksgrip.app --scenario scenarios/single_raid.toml --realtime

It prints NATO-style clinical telemetry once per simulated second, saves a
replay to runs/, and re-verifies it bit for bit.
"""

from __future__ import annotations

import argparse
import time
from pathlib import Path

import numpy as np

from hawksgrip import FIRST_THREAT, Session, verify_replay
from hawksgrip.bridge import FrameBuffers, LockstepDriver
from hawksgrip.config import load_scenario

RENDER_HZ = 60


def _fmt_event(ev, hz: int) -> str:
    tick, kind, ent, (x, y, z) = ev
    who = f"#{ent:03d}" if ent is not None else "----"
    return f"  [EVT t={tick / hz:8.2f}s] {kind:<22} {who}  E{x:+9.1f} N{y:+9.1f} U{z:+7.1f}"


def run(scenario_path: str, realtime: bool, out_dir: Path) -> int:
    sc = load_scenario(scenario_path)
    s = Session(sc.config_json, sc.seed)
    frame = FrameBuffers(s)
    hz = s.tick_hz
    total_ticks = round(sc.duration_s * hz)
    raids = list(sc.raids)
    birds = list(sc.birds)
    ti = s.terrain_info()
    print(f"SQ-7 HAWKSGRIP | scenario {sc.name} | seed {sc.seed} | {hz} Hz | {sc.duration_s:.0f} s")
    print(f"TERRAIN {ti['label']} | relief {ti['min_m']:.0f}..{ti['max_m']:.0f} m | refinery pad {ti['pad_z']:.0f} m")

    driver = LockstepDriver(s) if realtime else None
    last_report = -1
    wall0 = time.perf_counter_ns()
    prev = wall0
    while s.tick < total_ticks:
        while raids and raids[0].at_s * hz <= s.tick:
            s.random_raid(raids.pop(0).count)
        while birds and birds[0].at_s * hz <= s.tick:
            s.spawn_birds(birds.pop(0).count)
        if driver:
            now = time.perf_counter_ns()
            driver.advance(now - prev)
            prev = now
            time.sleep(1 / RENDER_HZ)
        else:
            # As fast as possible, but stop at each raid and report boundary.
            pending = [r.at_s for r in raids[:1] + birds[:1]]
            next_raid = round(min(pending) * hz) if pending else total_ticks
            s.step(max(1, min(hz - s.tick % hz, next_raid - s.tick, total_ticks - s.tick)))

        for ev in s.drain_events():
            print(_fmt_event(ev, hz))
        sec = s.tick // hz
        if sec != last_report:
            last_report = sec
            frame.refresh()
            c = s.counters()
            tracks = int(np.count_nonzero(frame.active[FIRST_THREAT:]))
            tr = s.tracks()
            hostile = sum(t["state"] == "hostile" for t in tr)
            locked = sum(t["locked"] for t in tr)
            print(
                f"T+{s.time:7.1f}s | AIRBORNE {tracks:3d} | HOSTILE {hostile:2d} LOCKED {locked:2d} "
                f"[{s.sensor_info()['band']}] | REFINERY BREACHES {c['refinery_breaches']:3d} | "
                f"QUEEN ORBIT DEV {c['queen_orbit_deviation_max_m']:.2f} m | AEGIS {c['aegis_interventions']} | "
                f"GROUND IMPACTS {c['ground_impacts']}"
            )

    wall = (time.perf_counter_ns() - wall0) / 1e9
    print(f"done: {s.time:.1f} sim-s in {wall:.3f} wall-s ({s.time / max(wall, 1e-9):.0f}x real time)")
    if driver and driver.ticks_dropped:
        print(f"note: {driver.ticks_dropped} ticks dropped by frame overruns")

    out_dir.mkdir(parents=True, exist_ok=True)
    replay_path = out_dir / f"{sc.name}-seed{sc.seed}.replay.json"
    replay_path.write_text(s.export_replay(), encoding="utf-8")
    rep = verify_replay(replay_path.read_text(encoding="utf-8"))
    print(f"replay: {replay_path} | verified={rep['ok']} ({rep['checkpoints_checked']} checkpoints)")
    return 0 if rep["ok"] else 1


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--scenario", default="scenarios/single_raid.toml")
    p.add_argument("--realtime", action="store_true", help="pace at wall-clock speed (60 fps frames, 120 Hz ticks)")
    p.add_argument("--out", default="runs", type=Path)
    a = p.parse_args()
    return run(a.scenario, a.realtime, a.out)


if __name__ == "__main__":
    raise SystemExit(main())
