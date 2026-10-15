//! # hawksgrip-core
//!
//! Deterministic simulation engine for SQ-7 Hawksgrip, a closed-environment
//! Critical Infrastructure Protection simulation. Pure Rust with no Python
//! dependency, so everything here is unit-testable and benchmarkable with
//! `cargo test` and `cargo bench` alone. Python access goes through the thin
//! `hawksgrip-py` crate.
//!
//! Phase map (see docs/ROADMAP.md):
//! - Phase 1: `sim` (clock, RNG streams, world, inputs, replay, batch),
//!   `kinematics` (3-DOF point mass, royal orbit, guidance), `terrain`
//!   (heightfield sampler), `telemetry` (event ring, counters).
//! - Phase 2: `terrain` (three Saudi sector generators, Lipschitz LoS),
//!   `refinery` (synthetic complex, keep-out volumes), climate config.

pub mod config;
pub mod kinematics;
pub mod math;
pub mod refinery;
pub mod sensing;
pub mod sim;
pub mod telemetry;
pub mod terrain;
pub mod vec3;

pub use config::{SEVEN, SimConfig};
pub use sim::{Input, Replay, World};
pub use vec3::Vec3;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
