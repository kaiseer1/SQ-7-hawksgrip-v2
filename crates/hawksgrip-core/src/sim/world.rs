//! The deterministic world: fixed slots, structure-of-arrays state, one tick
//! at a time.
//!
//! Slot layout (never changes during a run, so exported arrays never move):
//!   0           Kindah (the Queen)
//!   1..=7       the Seven
//!   8..8+cap    threat slots (inactive until spawned)

use crate::config::{SEVEN, SimConfig};
use crate::kinematics::guidance::{FollowEnv, TerrainFollow};
use crate::kinematics::{KState, Limits, RoyalOrbit, guidance, point_mass};
use crate::refinery::Refinery;
use crate::sensing::{SensorCtx, SensorEvent, Sensors, TargetClass};
use crate::sim::clock::Clock;
use crate::sim::hash::Fnv64;
use crate::sim::input::Input;
use crate::sim::rng::{Rng, Stream};
use crate::telemetry::{Counters, Event, EventKind, EventRing};
use crate::terrain::generator::{Frame, generate};
use crate::terrain::{Heightfield, SectorInfo, los};
use crate::vec3::Vec3;
use rayon::prelude::*;
use std::sync::Arc;

pub const QUEEN: usize = 0;
pub const FIRST_SEVEN: usize = 1;
pub const FIRST_THREAT: usize = 1 + SEVEN;
const NO_ENTITY: u32 = u32::MAX;

/// One body's slice of the per-tick update: (index, ((kinematics, look-ahead
/// cache), (behaviour, (limits, role)))).
type BodyMut<'a> = (
    usize,
    (
        (&'a mut KState, &'a mut f64),
        (&'a Behaviour, (&'a Limits, &'a Role)),
    ),
);
/// Terrain look-ahead refresh period in ticks (120 Hz / 4 = 30 Hz).
const LOOKAHEAD_EVERY: u64 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Role {
    Queen = 0,
    Seven = 1,
    Threat = 2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Behaviour {
    Inactive,
    /// Closed-form royal orbit (Queen only).
    Orbit,
    /// Station keeping in slot `slot` of the ring under the Queen.
    Station {
        slot: usize,
    },
    /// Fly to `target`; with `arrive`, decelerate and hold there.
    Waypoint {
        target: Vec3,
        speed: f64,
        arrive: bool,
    },
    /// Holding at a reached waypoint.
    Hold {
        at: Vec3,
    },
}

impl Behaviour {
    fn code(&self) -> u64 {
        match self {
            Behaviour::Inactive => 0,
            Behaviour::Orbit => 1,
            Behaviour::Station { .. } => 2,
            Behaviour::Waypoint { .. } => 3,
            Behaviour::Hold { .. } => 4,
        }
    }
}

#[derive(Clone, Debug)]
pub struct World {
    cfg: SimConfig,
    seed: u64,
    clock: Clock,
    orbit: RoyalOrbit,
    terrain: Arc<Heightfield>,
    /// None for externally supplied (DEM) terrain.
    sector_info: Option<Arc<SectorInfo>>,
    refinery: Arc<Refinery>,
    role: Vec<Role>,
    behaviour: Vec<Behaviour>,
    limits: Vec<Limits>,
    kin: Vec<KState>,
    /// Cached terrain look-ahead climb rate per body (threats only).
    lookahead_vz: Vec<f64>,
    events: EventRing,
    counters: Counters,
    pending: Vec<Input>,
    input_log: Vec<(u64, Input)>,
    checkpoints: Vec<(u64, u64)>,
    rng_scenario: Rng,
    parallel: bool,
    /// Class of each airborne object the eyes can see (None = empty slot,
    /// the Queen or the Seven).
    obj_class: Vec<Option<TargetClass>>,
    sensors: Sensors,
}

/// Generate the configured sector once. Batches share the result across
/// worlds (it is immutable), so a 1000-run Monte Carlo builds terrain once.
pub fn build_terrain(cfg: &SimConfig) -> Result<(Arc<Heightfield>, Arc<SectorInfo>), String> {
    let w = &cfg.world;
    let g = generate(
        cfg.terrain.sector,
        cfg.terrain.seed,
        Frame {
            half_extent: w.half_extent_m,
            cell: w.cell_m,
            center: [w.refinery_center.x, w.refinery_center.y],
            refinery_half: w.refinery_half_size_m,
        },
    )?;
    Ok((Arc::new(g.heightfield), Arc::new(g.info)))
}

impl World {
    pub fn new(cfg: SimConfig, seed: u64) -> Result<Self, String> {
        cfg.validate()?;
        let (terrain, info) = build_terrain(&cfg)?;
        Self::with_terrain(cfg, seed, terrain, Some(info))
    }

    /// Build a world on pre-generated terrain (shared by batches).
    pub fn with_terrain(
        cfg: SimConfig,
        seed: u64,
        terrain: Arc<Heightfield>,
        sector_info: Option<Arc<SectorInfo>>,
    ) -> Result<Self, String> {
        cfg.validate()?;
        let n = FIRST_THREAT + cfg.threat.capacity;
        let mut role = vec![Role::Threat; n];
        let mut limits = vec![cfg.threat.limits; n];
        role[QUEEN] = Role::Queen;
        limits[QUEEN] = cfg.queen.limits;
        for i in FIRST_SEVEN..FIRST_THREAT {
            role[i] = Role::Seven;
            limits[i] = cfg.seven.limits;
        }
        let mut w = Self {
            orbit: RoyalOrbit::new(Vec3::ZERO, 1.0, 1.0, 0.0, false),
            refinery: Arc::new(Refinery::synthetic(Vec3::ZERO, 1.0, 0.0, 1.0)),
            terrain,
            sector_info,
            role,
            behaviour: vec![Behaviour::Inactive; n],
            limits,
            kin: vec![KState::default(); n],
            lookahead_vz: vec![f64::NEG_INFINITY; n],
            events: EventRing::new(cfg.telemetry.event_capacity),
            counters: Counters::default(),
            pending: Vec::new(),
            input_log: Vec::new(),
            checkpoints: Vec::new(),
            rng_scenario: Rng::stream(seed, Stream::Scenario),
            parallel: false,
            obj_class: vec![None; n],
            sensors: Sensors::new(&cfg, seed, Vec3::ZERO, 0.0, n),
            clock: Clock::new(cfg.tick_hz),
            seed,
            cfg,
        };
        w.place_on_terrain()?;
        Ok(w)
    }

    /// Everything that depends on the ground: the refinery on its pad,
    /// Kindah's orbit altitude (checked for clearance), the Seven on station.
    fn place_on_terrain(&mut self) -> Result<(), String> {
        let c = self.cfg.world.refinery_center;
        let pad_z = self.terrain.height(c.x, c.y);
        self.refinery = Arc::new(Refinery::synthetic(
            Vec3::new(c.x, c.y, pad_z),
            self.cfg.world.refinery_half_size_m,
            pad_z,
            self.cfg.refinery.keepout_margin_scale,
        ));
        let q = &self.cfg.queen;
        let s = &self.cfg.seven;
        let alt = pad_z + q.orbit_alt_agl_m;
        // The orbit and the Seven's ring sweep an annulus R +- ring radius;
        // the highest ground under it must stay clear by the configured margin.
        let highest = [
            q.orbit_radius_m - s.station_radius_m,
            q.orbit_radius_m,
            q.orbit_radius_m + s.station_radius_m,
        ]
        .iter()
        .map(|&r| self.terrain.max_on_circle(c.x, c.y, r, 10.0))
        .fold(f64::NEG_INFINITY, f64::max);
        let clearance = alt - s.station_below_m - highest;
        if clearance < self.cfg.terrain.queen_min_clearance_m - s.station_below_m {
            return Err(format!(
                "orbit terrain clearance too small: ground reaches {highest:.0} m under an orbit at {alt:.0} m \
                 (raise queen.orbit_alt_agl_m or lower terrain.queen_min_clearance_m)"
            ));
        }
        self.place_royal_household();
        // The eyes are rebuilt for the final terrain (dust is referenced to the pad).
        self.sensors = Sensors::new(
            &self.cfg,
            self.seed,
            self.kin[QUEEN].p,
            pad_z,
            self.kin.len(),
        );
        Ok(())
    }

    /// Put Kindah on her orbit and the Seven exactly on their station slots,
    /// moving with them, so tick 0 starts in steady state.
    fn place_royal_household(&mut self) {
        let q = &self.cfg.queen;
        let c = self.cfg.world.refinery_center;
        let alt = self.terrain.height(c.x, c.y) + q.orbit_alt_agl_m;
        self.orbit = RoyalOrbit::new(
            Vec3::new(c.x, c.y, alt),
            q.orbit_radius_m,
            q.airspeed_mps,
            q.phase0_rad,
            q.clockwise,
        );
        let qs = self.orbit.state_at(0.0);
        self.kin[QUEEN] = qs;
        self.behaviour[QUEEN] = Behaviour::Orbit;
        let s = &self.cfg.seven;
        for k in 0..SEVEN {
            let slot = guidance::station_slot(&qs, k, SEVEN, s.station_radius_m, s.station_below_m);
            self.kin[FIRST_SEVEN + k] = slot;
            self.behaviour[FIRST_SEVEN + k] = Behaviour::Station { slot: k };
        }
    }

    /// Replace the terrain. Only allowed before the first tick, so a run
    /// never changes ground under its own feet.
    pub fn set_terrain(&mut self, hf: Heightfield) -> Result<(), String> {
        if self.clock.tick() != 0 {
            return Err("terrain can only be set before the first tick".into());
        }
        let previous = (self.terrain.clone(), self.sector_info.clone());
        self.terrain = Arc::new(hf);
        self.sector_info = None;
        if let Err(e) = self.place_on_terrain() {
            (self.terrain, self.sector_info) = previous;
            self.place_on_terrain()?;
            return Err(e);
        }
        Ok(())
    }

    /// Authoritative line-of-sight: terrain (Lipschitz march, ADR-0010) and
    /// refinery structures.
    pub fn line_of_sight(&self, a: Vec3, b: Vec3) -> bool {
        los::visible(&self.terrain, a, b, los::DEFAULT_EPS_M) && !self.refinery.occludes(a, b)
    }

    pub fn set_parallel(&mut self, on: bool) {
        self.parallel = on;
    }

    pub fn push_input(&mut self, input: Input) {
        self.pending.push(input);
    }

    pub fn step(&mut self, n: u64) {
        for _ in 0..n {
            self.tick();
        }
    }

    fn emit(&mut self, kind: EventKind, entity: usize, data: Vec3) {
        let entity = if entity == usize::MAX {
            NO_ENTITY
        } else {
            entity as u32
        };
        self.events.push(Event {
            tick: self.clock.tick(),
            kind,
            entity,
            data: data.to_array(),
        });
    }

    fn tick(&mut self) {
        let tick = self.clock.tick();

        // 1. Inputs take effect at the start of the tick they are logged at.
        let pending = std::mem::take(&mut self.pending);
        for input in pending {
            self.apply_input(&input);
            self.input_log.push((tick, input));
        }

        // 2. Kindah: closed-form orbit. Nothing may perturb her.
        let dt = self.clock.dt();
        let q = self.orbit.state_at(self.clock.time_at(tick + 1));
        self.kin[QUEEN] = q;
        let dev = (self.kin[QUEEN].p - q.p).norm();
        if dev > self.counters.queen_orbit_deviation_max_m {
            self.counters.queen_orbit_deviation_max_m = dev;
        }

        // 3. Everyone else: guidance -> limits -> integrate. Each body depends
        //    only on its own state and the Queen's, so the result is identical
        //    whether this loop runs sequentially or on rayon's thread pool.
        let s = &self.cfg.seven;
        let (kp, kd, kv_seven, ring_r, ring_below) = (
            s.station_kp,
            s.station_kd,
            s.waypoint_kv,
            s.station_radius_m,
            s.station_below_m,
        );
        let kv_threat = self.cfg.threat.kv;
        let t = &self.cfg.threat;
        let follow = TerrainFollow {
            agl: t.terrain_follow_agl_m,
            lookahead_s: t.terrain_lookahead_s,
            dive_radius: t.dive_radius_m,
            kz: 1.0,
        };
        let env = FollowEnv {
            hf: &self.terrain,
            tf: follow,
        };
        // Look-ahead refresh slots are staggered by body index: each body
        // refreshes on one tick in four (30 Hz), a quarter of bodies per tick.
        let phase = tick % LOOKAHEAD_EVERY;
        let update = |(i, ((k, look), (b, (lim, role)))): BodyMut<'_>| {
            let kv = if *role == Role::Threat {
                kv_threat
            } else {
                kv_seven
            };
            let a_cmd = match *b {
                Behaviour::Inactive | Behaviour::Orbit => return,
                Behaviour::Station { slot } => {
                    let r = guidance::station_slot(&q, slot, SEVEN, ring_r, ring_below);
                    guidance::track(k, &r, kp, kd)
                }
                Behaviour::Waypoint {
                    target,
                    speed,
                    arrive,
                } => {
                    if *role == Role::Threat {
                        let refresh = i as u64 % LOOKAHEAD_EVERY == phase;
                        guidance::waypoint_terrain_following(
                            k, target, speed, kv, &env, look, refresh,
                        )
                    } else {
                        guidance::waypoint(k, target, speed, kv, arrive, 0.5 * lim.a_max)
                    }
                }
                Behaviour::Hold { at } => {
                    guidance::waypoint(k, at, lim.v_max, kv, true, 0.5 * lim.a_max)
                }
            };
            *k = point_mass::step(*k, a_cmd, lim, dt);
        };
        let kin = &mut self.kin[FIRST_SEVEN..];
        let look = &mut self.lookahead_vz[FIRST_SEVEN..];
        let beh = &self.behaviour[FIRST_SEVEN..];
        let lim = &self.limits[FIRST_SEVEN..];
        let role = &self.role[FIRST_SEVEN..];
        if self.parallel {
            kin.par_iter_mut()
                .zip(look.par_iter_mut())
                .zip(beh.par_iter().zip(lim.par_iter().zip(role.par_iter())))
                .enumerate()
                .for_each(update);
        } else {
            kin.iter_mut()
                .zip(look.iter_mut())
                .zip(beh.iter().zip(lim.iter().zip(role.iter())))
                .enumerate()
                .for_each(update);
        }

        // 4. Rules and events, sequential in slot order (deterministic order).
        self.apply_rules();

        // 5. Kindah's eyes: slew every tick, detect/track/allocate every frame.
        let (terrain, refinery) = (&self.terrain, &self.refinery);
        let los = |a: Vec3, b: Vec3| {
            los::visible(terrain, a, b, los::DEFAULT_EPS_M) && !refinery.occludes(a, b)
        };
        let ctx = SensorCtx {
            tick,
            queen: self.kin[QUEEN],
            kin: &self.kin,
            class: &self.obj_class,
            refinery_center: self.cfg.world.refinery_center,
            los: &los,
        };
        let sensor_events = self.sensors.tick(&ctx);
        self.emit_all(sensor_events);

        self.clock.advance();
        let now = self.clock.tick();
        if now % self.cfg.checkpoint_every == 0 {
            self.checkpoints.push((now, self.state_hash()));
        }
    }

    fn emit_all(&mut self, events: Vec<SensorEvent>) {
        for (kind, entity, data) in events {
            self.emit(kind, entity, data);
        }
    }

    /// Take a body out of the sky and close its sensor timeline.
    fn deactivate(&mut self, i: usize, kind: EventKind, reason: &'static str) {
        let p = self.kin[i].p;
        self.behaviour[i] = Behaviour::Inactive;
        self.obj_class[i] = None;
        self.emit(kind, i, p);
        let mut ev = Vec::new();
        self.sensors.on_end(i, self.clock.tick(), reason, &mut ev);
        self.emit_all(ev);
    }

    fn apply_rules(&mut self) {
        let w = self.cfg.world.clone();
        let arrive_m = self.cfg.seven.waypoint_arrive_m;
        for i in FIRST_SEVEN..self.kin.len() {
            let b = self.behaviour[i];
            if b == Behaviour::Inactive {
                continue;
            }
            let p = self.kin[i].p;
            let ground = self.terrain.height(p.x, p.y);
            if p.z < ground {
                self.counters.ground_impacts += 1;
                self.deactivate(i, EventKind::GroundImpact, "ground_impact");
                continue;
            }
            match self.role[i] {
                Role::Threat if self.obj_class[i] == Some(TargetClass::Bird) => {
                    // Birds wander: a new random waypoint whenever one is reached.
                    if let Behaviour::Waypoint { target, speed, .. } = b {
                        if (target - p).horizontal().norm() < 80.0 {
                            let next = self.bird_waypoint();
                            self.behaviour[i] = Behaviour::Waypoint {
                                target: next,
                                speed,
                                arrive: false,
                            };
                        }
                    }
                }
                Role::Threat => {
                    let c = w.refinery_center;
                    let inside = (p.x - c.x).abs() <= w.refinery_half_size_m
                        && (p.y - c.y).abs() <= w.refinery_half_size_m
                        && p.z - ground <= w.refinery_breach_ceiling_m;
                    if inside {
                        self.counters.refinery_breaches += 1;
                        self.deactivate(i, EventKind::RefineryBreach, "refinery_breach");
                    } else if p.x.abs() > w.half_extent_m || p.y.abs() > w.half_extent_m {
                        self.counters.threats_out_of_bounds += 1;
                        self.deactivate(i, EventKind::ThreatOutOfBounds, "out_of_bounds");
                    }
                }
                Role::Seven => {
                    if let Behaviour::Waypoint {
                        target,
                        arrive: true,
                        ..
                    } = b
                    {
                        if (target - p).norm() <= arrive_m {
                            self.behaviour[i] = Behaviour::Hold { at: target };
                            self.emit(EventKind::WaypointReached, i, p);
                        }
                    }
                }
                Role::Queen => {}
            }
        }
    }

    /// A random point inside the wander radius, at bird height (scenario stream).
    fn bird_waypoint(&mut self) -> Vec3 {
        let b = self.cfg.birds.clone();
        let c = self.cfg.world.refinery_center;
        let r = &mut self.rng_scenario;
        let az = r.uniform(0.0, std::f64::consts::TAU);
        let rad = b.wander_radius_m * r.next_f64().sqrt(); // uniform over the disc
        let alt = r.uniform(b.alt_agl_min_m, b.alt_agl_max_m);
        let (s, co) = crate::math::sin_cos(az);
        let (x, y) = (c.x + rad * co, c.y + rad * s);
        Vec3::new(x, y, self.terrain.height(x, y) + alt)
    }

    fn apply_input(&mut self, input: &Input) {
        match *input {
            Input::SpawnThreat { pos, target, speed } => self.spawn_threat(pos, target, speed),
            Input::RandomRaid { count } => {
                let t = self.cfg.threat.clone();
                let c = self.cfg.world.refinery_center;
                let hs = 0.8 * self.cfg.world.refinery_half_size_m;
                for _ in 0..count {
                    // Draw every number before spawning, so a rejected spawn
                    // does not shift the rest of the sequence.
                    let r = &mut self.rng_scenario;
                    let az = r.uniform(0.0, std::f64::consts::TAU);
                    let alt = r.uniform(t.spawn_alt_agl_min_m, t.spawn_alt_agl_max_m);
                    let speed = r.uniform(t.speed_min_mps, t.speed_max_mps);
                    let ax = c.x + r.uniform(-hs, hs);
                    let ay = c.y + r.uniform(-hs, hs);
                    let (s_az, c_az) = crate::math::sin_cos(az);
                    let (sx, sy) = (c.x + t.spawn_radius_m * c_az, c.y + t.spawn_radius_m * s_az);
                    let pos = Vec3::new(sx, sy, self.terrain.height(sx, sy) + alt);
                    let target = Vec3::new(ax, ay, self.terrain.height(ax, ay) + t.aim_alt_agl_m);
                    self.spawn_threat(pos, target, speed);
                }
            }
            Input::SpawnBirds { count } => {
                for _ in 0..count {
                    let pos = self.bird_waypoint();
                    let first = self.bird_waypoint();
                    let b = &self.cfg.birds;
                    let (lo, hi) = (b.speed_min_mps, b.speed_max_mps);
                    let speed = self.rng_scenario.uniform(lo, hi);
                    self.spawn_object(pos, first, speed, TargetClass::Bird);
                }
                self.emit(
                    EventKind::BirdsSpawned,
                    usize::MAX,
                    Vec3::new(count as f64, 0.0, 0.0),
                );
            }
            Input::SetVisibility { visibility_m } => {
                if (10.0..=100_000.0).contains(&visibility_m) {
                    let mut ev = Vec::new();
                    let q = self.kin[QUEEN].p;
                    self.sensors.set_visibility(visibility_m, q, &mut ev);
                    self.emit(
                        EventKind::VisibilityChanged,
                        usize::MAX,
                        Vec3::new(visibility_m, 0.0, 0.0),
                    );
                    self.emit_all(ev);
                } else {
                    self.emit(
                        EventKind::OrderRejected,
                        usize::MAX,
                        Vec3::new(visibility_m, 0.0, 0.0),
                    );
                }
            }
            Input::SetWaypoint {
                entity,
                target,
                speed,
            } => {
                let ok = (FIRST_SEVEN..FIRST_THREAT).contains(&entity)
                    && self.behaviour[entity] != Behaviour::Inactive
                    && target.is_finite()
                    && speed.is_finite()
                    && speed > 0.0;
                if ok {
                    self.behaviour[entity] = Behaviour::Waypoint {
                        target,
                        speed,
                        arrive: true,
                    };
                    self.emit(EventKind::OrderAccepted, entity, target);
                } else {
                    let e = if entity < self.kin.len() {
                        entity
                    } else {
                        usize::MAX
                    };
                    self.emit(EventKind::OrderRejected, e, target);
                }
            }
            Input::ReturnToStation { entity } => {
                if (FIRST_SEVEN..FIRST_THREAT).contains(&entity)
                    && self.behaviour[entity] != Behaviour::Inactive
                {
                    self.behaviour[entity] = Behaviour::Station {
                        slot: entity - FIRST_SEVEN,
                    };
                    self.emit(EventKind::OrderAccepted, entity, Vec3::ZERO);
                } else {
                    let e = if entity < self.kin.len() {
                        entity
                    } else {
                        usize::MAX
                    };
                    self.emit(EventKind::OrderRejected, e, Vec3::ZERO);
                }
            }
        }
    }

    fn spawn_threat(&mut self, pos: Vec3, target: Vec3, speed: f64) {
        self.spawn_object(pos, target, speed, TargetClass::Drone);
    }

    fn spawn_object(&mut self, pos: Vec3, target: Vec3, speed: f64, class: TargetClass) {
        let lim = match class {
            TargetClass::Drone => self.cfg.threat.limits,
            TargetClass::Bird => self.cfg.birds.limits,
        };
        if !(pos.is_finite() && target.is_finite() && speed.is_finite() && speed > 0.0) {
            self.emit(EventKind::OrderRejected, usize::MAX, pos);
            return;
        }
        let Some(slot) =
            (FIRST_THREAT..self.kin.len()).find(|&i| self.behaviour[i] == Behaviour::Inactive)
        else {
            self.counters.spawn_rejected += 1;
            self.emit(EventKind::SpawnRejected, usize::MAX, pos);
            return;
        };
        let speed = speed.min(lim.v_max);
        let v = (target - pos).normalized_or_zero() * speed;
        self.kin[slot] = KState {
            p: pos,
            v,
            a: Vec3::ZERO,
        };
        self.lookahead_vz[slot] = f64::NEG_INFINITY; // forces a refresh next tick
        self.limits[slot] = lim;
        self.obj_class[slot] = Some(class);
        self.sensors.on_spawn(slot, class, self.clock.tick());
        self.behaviour[slot] = Behaviour::Waypoint {
            target,
            speed,
            arrive: false,
        };
        if class == TargetClass::Drone {
            self.counters.threats_spawned += 1;
            self.emit(EventKind::ThreatSpawned, slot, pos);
        }
    }

    /// Exact fingerprint of the dynamic state (bit patterns, not values).
    /// Covers every per-body field that influences future ticks, including
    /// the cached terrain look-ahead.
    pub fn state_hash(&self) -> u64 {
        let mut h = Fnv64::new();
        h.write_u64(self.clock.tick());
        for ((k, b), look) in self.kin.iter().zip(&self.behaviour).zip(&self.lookahead_vz) {
            h.write_u64(b.code());
            if *b == Behaviour::Inactive {
                continue;
            }
            for v in [k.p, k.v, k.a] {
                h.write_f64(v.x);
                h.write_f64(v.y);
                h.write_f64(v.z);
            }
            h.write_f64(*look);
        }
        for c in &self.obj_class {
            h.write_u64(c.map_or(9, |c| c as u64));
        }
        self.sensors.hash_into(&mut h);
        h.finish()
    }

    // ---- read access -------------------------------------------------------

    pub fn config(&self) -> &SimConfig {
        &self.cfg
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn clock(&self) -> &Clock {
        &self.clock
    }
    pub fn terrain(&self) -> &Heightfield {
        &self.terrain
    }
    pub fn sector_info(&self) -> Option<&SectorInfo> {
        self.sector_info.as_deref()
    }
    pub fn refinery(&self) -> &Refinery {
        &self.refinery
    }
    pub fn sensors(&self) -> &Sensors {
        &self.sensors
    }
    pub fn object_class(&self, i: usize) -> Option<TargetClass> {
        self.obj_class[i]
    }
    pub fn orbit(&self) -> &RoyalOrbit {
        &self.orbit
    }
    pub fn n_entities(&self) -> usize {
        self.kin.len()
    }
    pub fn kin(&self) -> &[KState] {
        &self.kin
    }
    pub fn roles(&self) -> &[Role] {
        &self.role
    }
    pub fn behaviours(&self) -> &[Behaviour] {
        &self.behaviour
    }
    pub fn is_active(&self, i: usize) -> bool {
        self.behaviour[i] != Behaviour::Inactive
    }
    pub fn counters(&self) -> &Counters {
        &self.counters
    }
    pub fn events_dropped(&self) -> u64 {
        self.events.dropped()
    }
    pub fn drain_events(&mut self) -> Vec<Event> {
        self.events.drain()
    }
    pub fn input_log(&self) -> &[(u64, Input)] {
        &self.input_log
    }
    pub fn checkpoints(&self) -> &[(u64, u64)] {
        &self.checkpoints
    }

    /// Copy positions and velocities into caller-owned flat buffers of
    /// length 3 * n_entities (row-major x, y, z). No allocation (ADR-0006).
    pub fn snapshot_into(&self, pos: &mut [f64], vel: &mut [f64]) {
        assert_eq!(pos.len(), 3 * self.kin.len());
        assert_eq!(vel.len(), 3 * self.kin.len());
        for (i, k) in self.kin.iter().enumerate() {
            pos[3 * i..3 * i + 3].copy_from_slice(&k.p.to_array());
            vel[3 * i..3 * i + 3].copy_from_slice(&k.v.to_array());
        }
    }
}
