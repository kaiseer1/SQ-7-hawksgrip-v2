//! Permanent HUD counters ("Zero Perturbation" panel and run statistics).

use serde::Serialize;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Counters {
    pub threats_spawned: u64,
    pub spawn_rejected: u64,
    pub refinery_breaches: u64,
    pub threats_out_of_bounds: u64,
    pub ground_impacts: u64,
    /// Royal Aegis bodyguard overrides (Phase 5). Shown next to the orbit
    /// deviation, which is zero by construction (see Claims Audit).
    pub aegis_interventions: u64,
    pub threats_reaching_queen: u64,
    /// Largest distance between Kindah and her nominal orbit, ever.
    /// Zero by construction today; measured anyway so that any future
    /// code path that perturbs her is caught.
    pub queen_orbit_deviation_max_m: f64,
}
