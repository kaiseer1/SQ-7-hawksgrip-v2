import json
from pathlib import Path

import pytest

from hawksgrip import Session
from hawksgrip.config import config_json, default_config, load_scenario

ROOT = Path(__file__).resolve().parents[1]


def test_defaults_come_from_rust():
    d = default_config()
    assert d["tick_hz"] == 120
    assert d["world"]["half_extent_m"] == 7500.0  # 15 x 15 km sector (ADR-0004)
    assert d["world"]["cell_m"] == 10.0


def test_partial_override_merges():
    c = json.loads(config_json({"queen": {"orbit_radius_m": 2800.0}}))
    assert c["queen"]["orbit_radius_m"] == 2800.0
    assert c["queen"]["airspeed_mps"] == 25.0


def test_typo_is_an_error():
    with pytest.raises(ValueError, match="unknown field"):
        config_json({"queen": {"orbit_radus_m": 2800.0}})


@pytest.mark.parametrize("name", ["single_raid.toml", "waves.toml"])
def test_shipped_scenarios_load_and_run(name):
    sc = load_scenario(ROOT / "scenarios" / name)
    s = Session(sc.config_json, sc.seed)
    for r in sc.raids:
        s.random_raid(r.count)
    s.step(120)
    assert s.counters()["threats_spawned"] == sum(r.count for r in sc.raids)


def test_scenario_rejects_unknown_tables(tmp_path):
    p = tmp_path / "bad.toml"
    p.write_text("[scenario]\nname='x'\n[simm]\ntick_hz=120\n")
    with pytest.raises(ValueError, match="unknown top-level"):
        load_scenario(p)
