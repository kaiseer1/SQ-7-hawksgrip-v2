//! Simulation configuration. The Rust `Default` impls are the single source of
//! truth; Python scenario files are partial overrides on top of them.
//!
//! Every numeric default is listed in `docs/claims.md` with its source or the
//! tag SWEPT (a placeholder that the evaluation varies, not a claim).

use crate::kinematics::Limits;
use crate::sensing::optics::BandParams;
use crate::terrain::Sector;
use crate::vec3::Vec3;
use serde::{Deserialize, Serialize};

/// The Seven. Fixed by design, not configurable.
pub const SEVEN: usize = 7;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SimConfig {
    /// Fixed simulation tick rate (ADR-0001).
    pub tick_hz: u32,
    /// Record a state-hash checkpoint every N ticks for replay verification.
    pub checkpoint_every: u64,
    pub world: WorldConfig,
    pub queen: QueenConfig,
    pub seven: SevenConfig,
    pub threat: ThreatConfig,
    pub telemetry: TelemetryConfig,
    pub terrain: TerrainConfig,
    pub refinery: RefineryConfig,
    pub environment: EnvironmentConfig,
    pub sensors: SensorConfig,
    pub birds: BirdConfig,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            tick_hz: 120,
            checkpoint_every: 120,
            world: WorldConfig::default(),
            queen: QueenConfig::default(),
            seven: SevenConfig::default(),
            threat: ThreatConfig::default(),
            telemetry: TelemetryConfig::default(),
            terrain: TerrainConfig::default(),
            refinery: RefineryConfig::default(),
            environment: EnvironmentConfig::default(),
            sensors: SensorConfig::default(),
            birds: BirdConfig::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorldConfig {
    /// Sector is a square of side 2 * half_extent centred on the origin (ADR-0004).
    pub half_extent_m: f64,
    /// Terrain grid spacing.
    pub cell_m: f64,
    /// Centre of the synthetic refinery complex (z ignored; ground is sampled).
    pub refinery_center: Vec3,
    /// The refinery footprint is a square of side 2 * half_size.
    pub refinery_half_size_m: f64,
    /// A threat inside the footprint below this height above ground is a breach.
    pub refinery_breach_ceiling_m: f64,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            half_extent_m: 7_500.0,
            cell_m: 10.0,
            refinery_center: Vec3::ZERO,
            refinery_half_size_m: 1_000.0,
            refinery_breach_ceiling_m: 150.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct QueenConfig {
    pub orbit_radius_m: f64,
    /// Orbit height above the ground at the orbit centre.
    pub orbit_alt_agl_m: f64,
    pub airspeed_mps: f64,
    pub phase0_rad: f64,
    pub clockwise: bool,
    pub limits: Limits,
}

impl Default for QueenConfig {
    fn default() -> Self {
        Self {
            orbit_radius_m: 3_000.0,
            orbit_alt_agl_m: 1_500.0,
            airspeed_mps: 25.0,
            phase0_rad: 0.0,
            clockwise: false,
            limits: Limits {
                v_max: 35.0,
                a_max: 3.0,
                j_max: 5.0,
                omega_max: 0.25,
                v_turn_eps: 1.0,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SevenConfig {
    pub limits: Limits,
    /// Radius of the station ring around the Queen.
    pub station_radius_m: f64,
    /// How far below the Queen the station ring sits.
    pub station_below_m: f64,
    /// PD gains for station keeping (zeta = kd / (2 sqrt(kp))).
    pub station_kp: f64,
    pub station_kd: f64,
    /// Velocity-loop gain for waypoint flight.
    pub waypoint_kv: f64,
    pub waypoint_arrive_m: f64,
}

impl Default for SevenConfig {
    fn default() -> Self {
        Self {
            limits: Limits {
                v_max: 50.0,
                a_max: 25.0,
                j_max: 150.0,
                omega_max: 2.5,
                v_turn_eps: 1.0,
            },
            station_radius_m: 150.0,
            station_below_m: 60.0,
            station_kp: 0.8,
            station_kd: 1.6,
            waypoint_kv: 2.0,
            waypoint_arrive_m: 5.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThreatConfig {
    /// Maximum simultaneous threats (fixed slots, so exported arrays never move).
    pub capacity: usize,
    pub limits: Limits,
    pub kv: f64,
    /// Random-raid spawn parameters.
    pub spawn_radius_m: f64,
    pub spawn_alt_agl_min_m: f64,
    pub spawn_alt_agl_max_m: f64,
    pub speed_min_mps: f64,
    pub speed_max_mps: f64,
    /// Height above ground of the aim point inside the refinery.
    pub aim_alt_agl_m: f64,
    /// Terrain following: minimum height above ground en route.
    pub terrain_follow_agl_m: f64,
    /// How far ahead (seconds of flight) terrain is scanned for climbs.
    pub terrain_lookahead_s: f64,
    /// Inside this horizontal distance of the aim point the floor is released
    /// so the threat can dive onto the target.
    pub dive_radius_m: f64,
}

impl Default for ThreatConfig {
    fn default() -> Self {
        Self {
            capacity: 64,
            limits: Limits {
                v_max: 50.0,
                a_max: 8.0,
                j_max: 30.0,
                omega_max: 0.6,
                v_turn_eps: 1.0,
            },
            kv: 1.5,
            spawn_radius_m: 7_000.0,
            spawn_alt_agl_min_m: 50.0,
            spawn_alt_agl_max_m: 400.0,
            speed_min_mps: 25.0,
            speed_max_mps: 45.0,
            aim_alt_agl_m: 20.0,
            terrain_follow_agl_m: 40.0,
            terrain_lookahead_s: 8.0,
            dive_radius_m: 1_500.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryConfig {
    /// Ring-buffer size. Oldest events are dropped (and counted) on overflow.
    pub event_capacity: usize,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            event_capacity: 4_096,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TerrainConfig {
    /// Which Saudi sector archetype to generate (ADR-0009).
    pub sector: Sector,
    /// Terrain seed, independent of the run seed, so many raids can be flown
    /// over one map and maps can be held out for evaluation.
    pub seed: u64,
    /// Minimum clearance between Kindah/the Seven's station ring and the
    /// highest ground under their orbit.
    pub queen_min_clearance_m: f64,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            sector: Sector::Flat,
            seed: 1,
            queen_min_clearance_m: 300.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RefineryConfig {
    /// Multiplies every structure's keep-out margin (sensitivity sweeps).
    pub keepout_margin_scale: f64,
}

impl Default for RefineryConfig {
    fn default() -> Self {
        Self {
            keepout_margin_scale: 1.0,
        }
    }
}

/// Saudi climate parameters. Consumed by Phase 3 (optics: visibility) and
/// Phase 4 (thermal/battery: ambient temperature).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EnvironmentConfig {
    pub ambient_temp_c: f64,
    /// Meteorological visibility; Shamal dust storms drop it below 1 km.
    pub visibility_m: f64,
    /// Direction the wind blows FROM (degrees clockwise from north).
    pub wind_from_deg: f64,
    pub wind_speed_mps: f64,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            ambient_temp_c: 40.0,
            visibility_m: 20_000.0,
            wind_from_deg: 330.0,
            wind_speed_mps: 6.0,
        }
    }
}

/// Kindah's compound optical array (Phase 3). Every number is SWEPT unless
/// noted in docs/claims.md.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SensorConfig {
    pub n_eyes: usize,
    pub fov_h_deg: f64,
    pub fov_v_deg: f64,
    /// Camera frame rate; must divide tick_hz.
    pub frame_hz: u32,
    pub slew_rate_deg_s: f64,
    /// Frames each sweep point is stared at once the eye has settled.
    pub dwell_frames: u32,
    pub search_el_min_deg: f64,
    pub search_el_max_deg: f64,
    pub visible: BandParams,
    pub swir: BandParams,
    pub lwir: BandParams,
    /// Johnson detection criterion: pixels across the target.
    pub detect_px: f64,
    pub pd_max: f64,
    pub dust_scale_height_m: f64,
    /// Switch bands automatically when visibility changes.
    pub auto_band: bool,
    pub drone_size_m: f64,
    pub bird_size_m: f64,
    /// M-of-N confirmation.
    pub confirm_m: u32,
    pub confirm_n: u32,
    /// Hits needed before classification, and the pixel scale of its accuracy.
    pub classify_hits: u32,
    pub classify_px0: f64,
    /// Frames with fewer pixels on target carry no usable signature.
    pub classify_min_px: f64,
    /// Bias of the drone/bird decision toward "drone", in [0, 1).
    pub classify_drone_bias: f64,
    /// Evidence (sum of eyes x pixels over frames) the post-lock second look
    /// gathers before re-deciding drone vs bird.
    pub verify_evidence: f64,
    /// Ignored (bird) tracks within this distance of the refinery are re-examined.
    pub reexamine_radius_m: f64,
    /// Maximum re-examinations of one "wildlife" track.
    pub max_reexams: u32,
    /// "SHE LOOKS AT YOU": every eye converges on a new hostile for this long.
    pub lock_burst_s: f64,
    /// Eyes that always keep sweeping once the burst is over.
    pub sentinel_eyes: usize,
    pub min_eyes_per_track: usize,
    /// Most eyes that may stare to confirm/classify new tracks at once.
    pub max_confirm_eyes: usize,
    /// Settled eyes on target needed to call a track locked (and to range it).
    pub lock_eyes: usize,
    pub tdm_slot_s: f64,
    /// Hostile tracks (held by eyes) are lost after this long without a hit.
    pub lost_after_s: f64,
    /// Unclassified tracks are refreshed only by sweep passes: longer memory.
    pub pending_lost_after_s: f64,
    /// Per-eye angular measurement noise (1 sigma).
    pub angle_noise_mrad: f64,
    /// Laser rangefinder noise once locked; passive range noise as a fraction before.
    pub lrf_noise_m: f64,
    pub passive_range_frac: f64,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            n_eyes: 24,
            fov_h_deg: 12.0,
            fov_v_deg: 9.0,
            frame_hz: 30,
            slew_rate_deg_s: 600.0,
            dwell_frames: 2,
            // Straight down included: -85 left a 5 degree nadir cone (~120 m
            // disc under her) never searched (Red-Team P3-11).
            search_el_min_deg: -90.0,
            search_el_max_deg: 5.0,
            // 12 deg over 1920 px.
            visible: BandParams {
                ifov_mrad: 0.109,
                contrast: 0.30,
                contrast_threshold: 0.02,
                extinction_scale: 1.0,
            },
            // 12 deg over 640 px.
            swir: BandParams {
                ifov_mrad: 0.327,
                contrast: 0.25,
                contrast_threshold: 0.02,
                extinction_scale: 0.6,
            },
            // 12 deg over 640 px; electric drones run cool, so thermal contrast is modest.
            lwir: BandParams {
                ifov_mrad: 0.327,
                contrast: 0.15,
                contrast_threshold: 0.01,
                extinction_scale: 0.35,
            },
            detect_px: 2.0,
            pd_max: 0.9,
            dust_scale_height_m: 800.0,
            auto_band: true,
            drone_size_m: 2.0,
            bird_size_m: 0.6,
            confirm_m: 2,
            confirm_n: 3,
            classify_hits: 6,
            classify_px0: 3.0,
            classify_min_px: 2.0,
            classify_drone_bias: 0.8,
            verify_evidence: 40.0,
            reexamine_radius_m: 2_000.0,
            max_reexams: 3,
            lock_burst_s: 0.5,
            sentinel_eyes: 4,
            min_eyes_per_track: 2,
            max_confirm_eyes: 6,
            lock_eyes: 2,
            tdm_slot_s: 0.25,
            lost_after_s: 2.0,
            pending_lost_after_s: 8.0,
            angle_noise_mrad: 0.2,
            lrf_noise_m: 1.0,
            passive_range_frac: 0.1,
        }
    }
}

/// Native wildlife: slow, wandering, low. The eyes must learn to ignore them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BirdConfig {
    pub limits: Limits,
    pub speed_min_mps: f64,
    pub speed_max_mps: f64,
    pub alt_agl_min_m: f64,
    pub alt_agl_max_m: f64,
    /// Birds wander between random points within this radius of the centre.
    pub wander_radius_m: f64,
}

impl Default for BirdConfig {
    fn default() -> Self {
        Self {
            limits: Limits {
                v_max: 18.0,
                a_max: 6.0,
                j_max: 40.0,
                omega_max: 2.0,
                v_turn_eps: 1.0,
            },
            speed_min_mps: 8.0,
            speed_max_mps: 15.0,
            alt_agl_min_m: 30.0,
            alt_agl_max_m: 300.0,
            wander_radius_m: 6_500.0,
        }
    }
}

impl SimConfig {
    pub fn from_json(s: &str) -> Result<Self, String> {
        let cfg: SimConfig = serde_json::from_str(s).map_err(|e| format!("config: {e}"))?;
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("config serialises")
    }

    /// Reject configurations that are physically or numerically impossible.
    pub fn validate(&self) -> Result<(), String> {
        let err = |m: String| Err(m);
        if !(30..=1000).contains(&self.tick_hz) {
            return err(format!("tick_hz {} outside 30..=1000", self.tick_hz));
        }
        if self.checkpoint_every == 0 {
            return err("checkpoint_every must be >= 1".into());
        }
        let w = &self.world;
        for (name, v) in [
            ("world.half_extent_m", w.half_extent_m),
            ("world.cell_m", w.cell_m),
            ("world.refinery_half_size_m", w.refinery_half_size_m),
            (
                "world.refinery_breach_ceiling_m",
                w.refinery_breach_ceiling_m,
            ),
        ] {
            if !(v.is_finite() && v > 0.0) {
                return err(format!("{name} must be positive and finite"));
            }
        }
        self.queen.limits.validate("queen.limits")?;
        self.seven.limits.validate("seven.limits")?;
        self.threat.limits.validate("threat.limits")?;

        // The Queen's orbit must be flyable within her own limits, so an
        // unbroken orbit is a property of the configuration, not a hope.
        let q = &self.queen;
        if !(q.orbit_radius_m > 0.0 && q.airspeed_mps > 0.0 && q.orbit_alt_agl_m > 0.0) {
            return err("queen orbit radius, airspeed and altitude must be positive".into());
        }
        let turn_rate = q.airspeed_mps / q.orbit_radius_m;
        let centripetal = q.airspeed_mps * q.airspeed_mps / q.orbit_radius_m;
        if q.airspeed_mps > q.limits.v_max {
            return err(format!(
                "queen airspeed {} > v_max {}",
                q.airspeed_mps, q.limits.v_max
            ));
        }
        if turn_rate > q.limits.omega_max {
            return err(format!(
                "queen orbit needs {turn_rate:.4} rad/s > omega_max"
            ));
        }
        if centripetal > q.limits.a_max {
            return err(format!("queen orbit needs {centripetal:.3} m/s^2 > a_max"));
        }
        let c = w.refinery_center;
        let reach = c.x.abs().max(c.y.abs()) + q.orbit_radius_m + self.seven.station_radius_m;
        if reach >= w.half_extent_m {
            return err("queen orbit (plus station ring) leaves the sector".into());
        }

        let t = &self.threat;
        if t.capacity == 0 || t.capacity > 1024 {
            return err("threat.capacity must be in 1..=1024".into());
        }
        if !(t.spawn_radius_m > 0.0 && t.spawn_radius_m < w.half_extent_m) {
            return err("threat.spawn_radius_m must be inside the sector".into());
        }
        if !(t.speed_min_mps > 0.0
            && t.speed_min_mps <= t.speed_max_mps
            && t.speed_max_mps <= t.limits.v_max)
        {
            return err("threat speeds must satisfy 0 < min <= max <= v_max".into());
        }
        if !(t.spawn_alt_agl_min_m > 0.0 && t.spawn_alt_agl_min_m <= t.spawn_alt_agl_max_m) {
            return err("threat spawn altitudes must satisfy 0 < min <= max".into());
        }
        if !(t.terrain_follow_agl_m > 0.0 && t.terrain_lookahead_s > 0.0 && t.dive_radius_m >= 0.0)
        {
            return err("threat terrain-following parameters must be positive".into());
        }
        if self.telemetry.event_capacity == 0 {
            return err("telemetry.event_capacity must be >= 1".into());
        }
        if !(0.0..).contains(&self.terrain.queen_min_clearance_m) {
            return err("terrain.queen_min_clearance_m must be >= 0".into());
        }
        if !(0.0..=10.0).contains(&self.refinery.keepout_margin_scale) {
            return err("refinery.keepout_margin_scale must be in 0..=10".into());
        }
        let e = &self.environment;
        if !(-20.0..=60.0).contains(&e.ambient_temp_c) {
            return err("environment.ambient_temp_c must be in -20..=60 C".into());
        }
        if !(10.0..=100_000.0).contains(&e.visibility_m) {
            return err("environment.visibility_m must be in 10..=100000".into());
        }
        if !(0.0..=40.0).contains(&e.wind_speed_mps) || !e.wind_from_deg.is_finite() {
            return err("environment wind must be finite with speed in 0..=40 m/s".into());
        }
        let s = &self.sensors;
        if !(1..=64).contains(&s.n_eyes) {
            return err("sensors.n_eyes must be in 1..=64".into());
        }
        if s.frame_hz == 0 || self.tick_hz % s.frame_hz != 0 {
            return err(format!(
                "sensors.frame_hz {} must divide tick_hz {}",
                s.frame_hz, self.tick_hz
            ));
        }
        for (name, v) in [("fov_h_deg", s.fov_h_deg), ("fov_v_deg", s.fov_v_deg)] {
            if !(v > 0.0 && v < 90.0) {
                return err(format!("sensors.{name} must be in (0, 90)"));
            }
        }
        for (name, v) in [
            ("slew_rate_deg_s", s.slew_rate_deg_s),
            ("detect_px", s.detect_px),
            ("dust_scale_height_m", s.dust_scale_height_m),
            ("drone_size_m", s.drone_size_m),
            ("bird_size_m", s.bird_size_m),
            ("classify_px0", s.classify_px0),
            ("classify_min_px", s.classify_min_px),
            ("verify_evidence", s.verify_evidence),
            ("tdm_slot_s", s.tdm_slot_s),
            ("lost_after_s", s.lost_after_s),
            ("pending_lost_after_s", s.pending_lost_after_s),
            ("lrf_noise_m", s.lrf_noise_m),
        ] {
            if !(v.is_finite() && v > 0.0) {
                return err(format!("sensors.{name} must be positive and finite"));
            }
        }
        if !(s.pd_max > 0.0 && s.pd_max <= 1.0) {
            return err("sensors.pd_max must be in (0, 1]".into());
        }
        if !(-90.0..=90.0).contains(&s.search_el_min_deg)
            || !(-90.0..=90.0).contains(&s.search_el_max_deg)
            || s.search_el_min_deg >= s.search_el_max_deg
        {
            return err("sensors search elevations must satisfy -90 <= min < max <= 90".into());
        }
        if !(1 <= s.confirm_m && s.confirm_m <= s.confirm_n && s.confirm_n <= 31) {
            return err("sensors confirmation must satisfy 1 <= M <= N <= 31".into());
        }
        if s.classify_hits == 0 || s.dwell_frames == 0 {
            return err("sensors.classify_hits and dwell_frames must be >= 1".into());
        }
        if s.sentinel_eyes >= s.n_eyes {
            return err("sensors.sentinel_eyes must leave at least one eye for tracking".into());
        }
        if !(0.0..1.0).contains(&s.classify_drone_bias) {
            return err("sensors.classify_drone_bias must be in [0, 1)".into());
        }
        if !(0.0..=10.0).contains(&s.lock_burst_s) || !(0.0..1.0).contains(&s.passive_range_frac) {
            return err("sensors.lock_burst_s in 0..=10 and passive_range_frac in [0, 1)".into());
        }
        for (name, b) in [
            ("visible", &s.visible),
            ("swir", &s.swir),
            ("lwir", &s.lwir),
        ] {
            if !(b.ifov_mrad > 0.0
                && b.contrast > 0.0
                && b.contrast_threshold > 0.0
                && b.extinction_scale > 0.0)
            {
                return err(format!("sensors.{name} band parameters must be positive"));
            }
        }
        let b = &self.birds;
        b.limits.validate("birds.limits")?;
        if !(b.speed_min_mps > 0.0
            && b.speed_min_mps <= b.speed_max_mps
            && b.speed_max_mps <= b.limits.v_max)
        {
            return err("bird speeds must satisfy 0 < min <= max <= v_max".into());
        }
        if !(b.alt_agl_min_m > 0.0 && b.alt_agl_min_m <= b.alt_agl_max_m) {
            return err("bird altitudes must satisfy 0 < min <= max".into());
        }
        if !(b.wander_radius_m > 0.0 && b.wander_radius_m < self.world.half_extent_m) {
            return err("birds.wander_radius_m must be inside the sector".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        SimConfig::default().validate().unwrap();
    }

    #[test]
    fn json_round_trip() {
        let c = SimConfig::default();
        assert_eq!(SimConfig::from_json(&c.to_json()).unwrap(), c);
    }

    #[test]
    fn partial_override_and_typo_rejection() {
        let c = SimConfig::from_json(r#"{"queen": {"orbit_radius_m": 2500.0}}"#).unwrap();
        assert_eq!(c.queen.orbit_radius_m, 2500.0);
        assert_eq!(c.queen.airspeed_mps, 25.0);
        assert!(SimConfig::from_json(r#"{"queen": {"orbit_radius": 2500.0}}"#).is_err());
    }

    #[test]
    fn sector_names_parse() {
        let c =
            SimConfig::from_json(r#"{"terrain": {"sector": "highland_wadi", "seed": 9}}"#).unwrap();
        assert_eq!(c.terrain.sector, Sector::HighlandWadi);
        assert!(SimConfig::from_json(r#"{"terrain": {"sector": "mars"}}"#).is_err());
        assert!(SimConfig::from_json(r#"{"environment": {"ambient_temp_c": 75.0}}"#).is_err());
    }

    #[test]
    fn infeasible_orbit_rejected() {
        // 25 m/s on a 100 m radius needs 6.25 m/s^2 > a_max 3.
        let e = SimConfig::from_json(r#"{"queen": {"orbit_radius_m": 100.0}}"#).unwrap_err();
        assert!(e.contains("queen orbit"), "{e}");
    }
}
