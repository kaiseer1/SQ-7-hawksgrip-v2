"""Scenario loading.

The Rust ``SimConfig`` defaults are the single source of truth. A scenario
file only overrides what it changes, and Rust validates the merged result
(unknown keys are rejected, so a typo fails loudly instead of being ignored).

Scenario TOML layout::

    [scenario]
    name = "single_raid"
    seed = 2026
    duration_s = 240.0

    [[scenario.raids]]      # repeatable
    at_s = 0.0
    count = 6

    [[scenario.birds]]      # repeatable: wandering wildlife
    at_s = 0.0
    count = 20

    [sim.queen]             # any SimConfig override
    orbit_radius_m = 3000.0
"""

from __future__ import annotations

import json
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

from hawksgrip import _core


@dataclass(frozen=True)
class Raid:
    at_s: float
    count: int


@dataclass(frozen=True)
class Scenario:
    name: str
    seed: int
    duration_s: float
    config_json: str
    raids: tuple[Raid, ...] = field(default_factory=tuple)
    birds: tuple[Raid, ...] = field(default_factory=tuple)


def default_config() -> dict:
    return json.loads(_core.default_config_json())


def config_json(overrides: dict | None = None) -> str:
    """Merge overrides onto the Rust defaults and validate. Raises ValueError."""
    return _core.validate_config(json.dumps(overrides or {}))


def load_scenario(path: str | Path) -> Scenario:
    path = Path(path)
    with path.open("rb") as f:
        doc = tomllib.load(f)
    unknown = set(doc) - {"scenario", "sim"}
    if unknown:
        raise ValueError(f"{path.name}: unknown top-level tables {sorted(unknown)}")
    meta = doc.get("scenario", {})
    raids = tuple(Raid(float(r.get("at_s", 0.0)), int(r["count"])) for r in meta.get("raids", []))
    birds = tuple(Raid(float(r.get("at_s", 0.0)), int(r["count"])) for r in meta.get("birds", []))
    if any(r.at_s < 0 or r.count < 0 for r in raids + birds):
        raise ValueError(f"{path.name}: raids and birds need at_s >= 0 and count >= 0")
    return Scenario(
        name=str(meta.get("name", path.stem)),
        seed=int(meta.get("seed", 0)),
        duration_s=float(meta.get("duration_s", 60.0)),
        config_json=config_json(doc.get("sim")),
        raids=tuple(sorted(raids, key=lambda r: r.at_s)),
        birds=tuple(sorted(birds, key=lambda r: r.at_s)),
    )
