//! External inputs (Red-Team spawns, operator orders).
//!
//! Inputs are never applied mid-tick. They queue up and take effect at the
//! start of the next tick, and they are logged with that tick number. A run
//! is therefore fully described by (config, seed, terrain, input log), which
//! is exactly what a replay file stores.

use crate::vec3::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    /// Spawn one threat at `pos` flying straight at `target`.
    SpawnThreat { pos: Vec3, target: Vec3, speed: f64 },
    /// Spawn `count` threats from the scenario RNG stream (seeded, replayable).
    RandomRaid { count: u32 },
    /// Send one of the Seven (entity 1..=7) to a point and hold there.
    SetWaypoint {
        entity: usize,
        target: Vec3,
        speed: f64,
    },
    /// Return one of the Seven to its station slot under the Queen.
    ReturnToStation { entity: usize },
    /// Release `count` wandering birds (from the scenario RNG stream).
    SpawnBirds { count: u32 },
    /// Change ground-level visibility: the [S] sandstorm key.
    SetVisibility { visibility_m: f64 },
}
