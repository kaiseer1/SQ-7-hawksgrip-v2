//! Kindah's 24-eye compound optical array (Phase 3): sweep, detection,
//! tracking, classification, convergence and allocation.
//!
//! Pipeline per camera frame (30 Hz; the gimbals slew every 120 Hz tick):
//!
//! 1. **Detect.** For every airborne object: which settled eyes have it in
//!    their frustum, the single-eye probability of detection `pd` (optics.rs),
//!    and the authoritative line-of-sight (terrain + refinery). With n eyes on
//!    it, the frame detects it with probability 1 - (1 - pd)^n, from one draw
//!    of the Sensor RNG stream.
//! 2. **Confirm** (M-of-N). A fresh detection opens a tentative track, and the
//!    detecting eye stops sweeping to stare at it. M hits in N frames confirm it.
//! 3. **Classify** (temporal-signature filter). After K hits the track is
//!    labelled drone or bird. The probability of being right grows with
//!    pixels on target: p = 1 - 0.5 exp(-px / px0). This is the optical
//!    analogue of a radar micro-Doppler classifier: rotor flicker and
//!    wing-beat show up as temporal signatures, and resolve better with more pixels.
//!    Birds are ignored, but get one re-examination if they come near the refinery.
//! 4. **Converge ("SHE LOOKS AT YOU").** A lone hostile triggers a lock burst:
//!    *every* eye snaps to it for `lock_burst_s`. Afterwards `sentinel_eyes`
//!    return to sweeping, so she never loses situational awareness. With
//!    several hostiles at once there is no total burst (it would starve the
//!    others); step 5 shares the eyes instead.
//! 5. **Allocate.** The remaining eyes are shared among hostile tracks by
//!    priority (time-to-refinery). If there are more tracks than eyes allow,
//!    tracks share eyes by time-division (TDM). Every sweeping eye re-tiles
//!    the whole search region, so coverage is never abandoned, only slowed.
//!
//! Detections use truth association (the simulation knows which object was
//! seen). Clutter association is out of scope and recorded as such.

pub mod coverage;
pub mod eyes;
pub mod optics;
pub mod sweep;

use crate::config::{SensorConfig, SimConfig};
use crate::kinematics::KState;
use crate::sim::hash::Fnv64;
use crate::sim::rng::{Rng, Stream};
use crate::telemetry::EventKind;
use crate::vec3::Vec3;
use eyes::{Eye, Task};
use optics::{Band, Optics};
use serde::Serialize;
use sweep::Sector;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum TargetClass {
    Drone = 0,
    Bird = 1,
}

impl TargetClass {
    pub fn name(self) -> &'static str {
        match self {
            TargetClass::Drone => "drone",
            TargetClass::Bird => "bird",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum TrackState {
    Tentative = 0,
    /// Confirmed, being classified.
    Confirmed = 1,
    Hostile = 2,
    /// Classified as wildlife.
    Ignored = 3,
    Lost = 4,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Track {
    pub id: u32,
    pub slot: usize,
    pub state: TrackState,
    /// Hit history of the last N frames (bit 0 = latest), for M-of-N.
    #[serde(skip)]
    window: u32,
    #[serde(skip)]
    frames: u32,
    #[serde(skip)]
    class_hits: u32,
    #[serde(skip)]
    class_px_sum: f64,
    pub class_est: Option<TargetClass>,
    /// Re-examinations done, and the pixels the last decision was based on.
    #[serde(skip)]
    reexams: u32,
    #[serde(skip)]
    decided_px: f64,
    /// Estimated position and velocity (alpha-beta filter on noisy fixes).
    pub est_p: Vec3,
    pub est_v: Vec3,
    pub last_hit_tick: u64,
    #[serde(skip)]
    has_estimate: bool,
    /// Constant-velocity Kalman covariance shared by the three axes
    /// (isotropic measurement noise): [[p11, p12], [p12, p22]].
    #[serde(skip)]
    cov: [f64; 3],
    pub burst_until: u64,
    pub locked: bool,
    pub eyes: u32,
    /// Second look by the converged eyes done, and the evidence gathered for it.
    pub verified: bool,
    pub evidence: f64,
    /// Pixels on target at the latest hit.
    pub last_px: f64,
}

/// Per-object timeline for time-to-detection metrics (ticks).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Timeline {
    pub slot: usize,
    pub class: TargetClass,
    pub spawn: u64,
    /// First frame Kindah *could* have detected it had an eye been pointing
    /// there: line-of-sight and single-frame pd >= 5 % (or the detection
    /// itself, if luck came first).
    pub first_opportunity: Option<u64>,
    pub detected: Option<u64>,
    pub confirmed: Option<u64>,
    pub classified: Option<u64>,
    pub classified_as: Option<TargetClass>,
    pub locked: Option<u64>,
    pub end: Option<u64>,
    pub end_reason: Option<&'static str>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct SensorCounters {
    pub detections: u64,
    pub tracks_confirmed: u64,
    pub tentative_dropped: u64,
    pub declared_hostile: u64,
    pub birds_ignored: u64,
    /// Birds declared hostile.
    pub false_alarms: u64,
    /// Drones declared birds (dangerous) at some decision.
    pub missed_drones: u64,
    /// ...of which later corrected by a re-examination.
    pub missed_drones_recovered: u64,
    /// False alarms withdrawn after the converged eyes took a second look.
    pub false_alarms_cleared: u64,
    pub reexaminations: u64,
    pub locks: u64,
    pub tracks_lost: u64,
    pub band_switches: u64,
}

/// What the sensors need from the world each tick.
pub struct SensorCtx<'a> {
    pub tick: u64,
    pub queen: KState,
    pub kin: &'a [KState],
    /// Class of each slot if it is an airborne object the eyes can see.
    pub class: &'a [Option<TargetClass>],
    pub refinery_center: Vec3,
    pub los: &'a dyn Fn(Vec3, Vec3) -> bool,
}

pub type SensorEvent = (EventKind, usize, Vec3);

#[derive(Clone, Debug)]
pub struct Sensors {
    cfg: SensorConfig,
    optics: Optics,
    band: Band,
    eyes: Vec<Eye>,
    tiling: Vec<Sector>,
    cells: Vec<Vec<sweep::Cell>>,
    sweepers: Vec<usize>,
    tracks: Vec<Track>,
    next_id: u32,
    active_timelines: Vec<Option<Timeline>>,
    finished: Vec<Timeline>,
    rng: Rng,
    counters: SensorCounters,
    tan_h: f64,
    tan_v: f64,
    frame_ticks: u64,
    dt: f64,
    el_min: f64,
    el_max: f64,
}

const SETTLE_RAD: f64 = 0.5 * std::f64::consts::PI / 180.0;
const REFERENCE_AGL_M: f64 = 100.0;
/// A detection "opportunity" exists once a pointed eye would have at least
/// this single-frame chance (and line-of-sight).
const OPPORTUNITY_PD: f64 = 0.05;
/// Track filter limits: extrapolate at most this long without a fix, restart
/// the filter after a longer gap, and never believe a speed above this.
const MAX_EXTRAPOLATION_S: f64 = 3.0;
const MAX_TRACK_SPEED: f64 = 120.0;
/// Kalman tuning: initial speed uncertainty and white-acceleration process noise.
const INITIAL_SPEED_SIGMA: f64 = 30.0;
const PROCESS_ACCEL_SIGMA: f64 = 6.0;

fn kf_predict(t: &mut Track, dt: f64) {
    let q = PROCESS_ACCEL_SIGMA * PROCESS_ACCEL_SIGMA;
    let [p11, p12, p22] = t.cov;
    t.cov = [
        p11 + 2.0 * dt * p12 + dt * dt * p22 + q * dt * dt * dt * dt / 4.0,
        p12 + dt * p22 + q * dt * dt * dt / 2.0,
        p22 + q * dt * dt,
    ];
    t.est_p += t.est_v * dt;
}

fn kf_update(t: &mut Track, z: Vec3, r: f64) {
    let [p11, p12, p22] = t.cov;
    let s = p11 + r;
    let (k1, k2) = (p11 / s, p12 / s);
    let innov = z - t.est_p;
    t.est_p += innov * k1;
    t.est_v = (t.est_v + innov * k2).clamp_norm(MAX_TRACK_SPEED);
    t.cov = [(1.0 - k1) * p11, (1.0 - k1) * p12, p22 - k2 * p12];
}
/// Sweep cells are at most this fraction of the frustum, so footprints overlap.
const CELL_OVERLAP: f64 = 0.85;
/// Aim jitter inside a cell, in cell units (stays within the overlap slack).
const CELL_JITTER: f64 = 0.08;

impl Sensors {
    pub fn new(sim: &SimConfig, seed: u64, queen: Vec3, ground_ref_z: f64, n_slots: usize) -> Self {
        let cfg = sim.sensors.clone();
        let optics = Optics {
            bands: [cfg.visible, cfg.swir, cfg.lwir],
            detect_px: cfg.detect_px,
            pd_max: cfg.pd_max,
            visibility_m: sim.environment.visibility_m,
            dust_scale_height_m: cfg.dust_scale_height_m,
            ground_ref_z,
        };
        let mut rng = Rng::stream(seed, Stream::Sensor);
        let (el_min, el_max) = (
            cfg.search_el_min_deg.to_radians(),
            cfg.search_el_max_deg.to_radians(),
        );
        let tiling = sweep::tile(cfg.n_eyes, el_min, el_max);
        let (fh, fv) = (cfg.fov_h_deg.to_radians(), cfg.fov_v_deg.to_radians());
        let cells: Vec<Vec<sweep::Cell>> = tiling
            .iter()
            .map(|s| sweep::cells(s, fh, fv, CELL_OVERLAP))
            .collect();
        let eyes = (0..cfg.n_eyes)
            .map(|i| {
                let shift = [rng.next_f64(), rng.next_f64()];
                let c = &cells[i];
                let d = c[sweep::visit(0, c.len(), shift)].direction(0.0, 0.0);
                Eye {
                    dir: d,
                    aim: d,
                    task: Task::Sweep,
                    settled: true,
                    sector: i,
                    sweep_k: 0,
                    shift,
                    dwell_left: cfg.dwell_frames,
                }
            })
            .collect();
        let band = if cfg.auto_band {
            optics.best_band(queen, REFERENCE_AGL_M, cfg.drone_size_m)
        } else {
            Band::Visible
        };
        Sensors {
            tan_h: crate::math::tan(cfg.fov_h_deg.to_radians() / 2.0),
            tan_v: crate::math::tan(cfg.fov_v_deg.to_radians() / 2.0),
            frame_ticks: (sim.tick_hz / cfg.frame_hz) as u64,
            dt: 1.0 / sim.tick_hz as f64,
            sweepers: (0..cfg.n_eyes).collect(),
            tiling,
            cells,
            eyes,
            band,
            optics,
            tracks: Vec::new(),
            next_id: 1,
            active_timelines: vec![None; n_slots],
            finished: Vec::new(),
            rng,
            counters: SensorCounters::default(),
            el_min,
            el_max,
            cfg,
        }
    }

    // ---- lifecycle hooks from the world ------------------------------------

    pub fn on_spawn(&mut self, slot: usize, class: TargetClass, tick: u64) {
        self.active_timelines[slot] = Some(Timeline {
            slot,
            class,
            spawn: tick,
            first_opportunity: None,
            detected: None,
            confirmed: None,
            classified: None,
            classified_as: None,
            locked: None,
            end: None,
            end_reason: None,
        });
    }

    /// The object in `slot` left the sky (breach, out of bounds, ...).
    pub fn on_end(
        &mut self,
        slot: usize,
        tick: u64,
        reason: &'static str,
        events: &mut Vec<SensorEvent>,
    ) {
        if let Some(mut t) = self.active_timelines[slot].take() {
            t.end = Some(tick);
            t.end_reason = Some(reason);
            self.finished.push(t);
        }
        if let Some(k) = self.tracks.iter().position(|t| t.slot == slot) {
            let tr = self.tracks.remove(k);
            self.release_eyes(tr.id);
            events.push((EventKind::TrackDropped, slot, tr.est_p));
        }
    }

    /// Change ground-level visibility (sandstorm on/off) and re-pick the band.
    pub fn set_visibility(
        &mut self,
        visibility_m: f64,
        queen: Vec3,
        events: &mut Vec<SensorEvent>,
    ) {
        self.optics.visibility_m = visibility_m;
        if self.cfg.auto_band {
            let b = self
                .optics
                .best_band(queen, REFERENCE_AGL_M, self.cfg.drone_size_m);
            if b != self.band {
                self.band = b;
                self.counters.band_switches += 1;
                events.push((
                    EventKind::BandSwitch,
                    usize::MAX,
                    Vec3::new(b as u8 as f64, visibility_m, 0.0),
                ));
            }
        }
    }

    fn release_eyes(&mut self, track: u32) {
        for e in &mut self.eyes {
            if e.task.track() == Some(track) {
                e.task = Task::Sweep;
            }
        }
    }

    // ---- per tick -----------------------------------------------------------

    pub fn tick(&mut self, ctx: &SensorCtx) -> Vec<SensorEvent> {
        let step = self.cfg.slew_rate_deg_s.to_radians() * self.dt;
        for e in &mut self.eyes {
            e.dir = eyes::slew(e.dir, e.aim, step);
            e.settled = eyes::angle(e.dir, e.aim) < SETTLE_RAD;
        }
        if ctx.tick % self.frame_ticks == 0 {
            self.frame(ctx)
        } else {
            Vec::new()
        }
    }

    fn size_of(&self, c: TargetClass) -> f64 {
        match c {
            TargetClass::Drone => self.cfg.drone_size_m,
            TargetClass::Bird => self.cfg.bird_size_m,
        }
    }

    fn frame(&mut self, ctx: &SensorCtx) -> Vec<SensorEvent> {
        let mut ev = Vec::new();
        let q = ctx.queen.p;
        let tick = ctx.tick;

        // 1. Detection. Pre-compute each settled eye's frame once.
        let looking: Vec<(usize, Vec3)> = self
            .eyes
            .iter()
            .enumerate()
            .filter(|(_, e)| e.settled)
            .map(|(i, e)| (i, e.dir))
            .collect();
        let mut hits: Vec<(usize, Vec<usize>, f64)> = Vec::new();
        for (slot, class) in ctx.class.iter().enumerate() {
            let Some(class) = *class else { continue };
            let p = ctx.kin[slot].p;
            let d = p - q;
            let range = d.norm();
            if range < 1.0 {
                continue;
            }
            let u = d / range;
            let on: Vec<usize> = looking
                .iter()
                .filter(|(_, f)| eyes::in_frustum(*f, self.tan_h, self.tan_v, u))
                .map(|(i, _)| *i)
                .collect();
            let need_opp = self.active_timelines[slot]
                .as_ref()
                .is_some_and(|t| t.first_opportunity.is_none());
            if on.is_empty() && !need_opp {
                continue;
            }
            let size = self.size_of(class);
            let pd = self.optics.pd(self.band, q, p, size);
            if pd < 1e-6 || (on.is_empty() && pd < OPPORTUNITY_PD) {
                continue;
            }
            if !(ctx.los)(q, p) {
                continue;
            }
            if need_opp && pd >= OPPORTUNITY_PD {
                if let Some(t) = self.active_timelines[slot].as_mut() {
                    t.first_opportunity = Some(tick);
                }
            }
            if !on.is_empty() {
                // 1 - (1 - pd)^n by explicit multiplication (no platform pow).
                let miss: f64 = on.iter().fold(1.0, |m, _| m * (1.0 - pd));
                let p_frame = 1.0 - miss;
                if self.rng.next_f64() < p_frame {
                    hits.push((slot, on, self.optics.pixels(self.band, size, range)));
                }
            }
        }

        // 2-3. Tracker: open, confirm, classify, lose.
        let mut hit_slots: Vec<(usize, usize, f64)> = Vec::new(); // (slot, eyes, px)
        for (slot, on, px) in hits {
            self.counters.detections += 1;
            if !self.tracks.iter().any(|t| t.slot == slot) {
                let id = self.next_id;
                self.next_id += 1;
                self.tracks.push(Track {
                    id,
                    slot,
                    state: TrackState::Tentative,
                    window: 0,
                    frames: 0,
                    class_hits: 0,
                    class_px_sum: 0.0,
                    class_est: None,
                    reexams: 0,
                    decided_px: 0.0,
                    est_p: ctx.kin[slot].p,
                    est_v: Vec3::ZERO,
                    last_hit_tick: tick,
                    has_estimate: false,
                    cov: [0.0; 3],
                    burst_until: 0,
                    locked: false,
                    eyes: 0,
                    verified: false,
                    evidence: 0.0,
                    last_px: 0.0,
                });
                // The confirmation stare is assigned in `allocate`, after last
                // frame's track tasks are cleared (Red-Team P3-10).
                if let Some(t) = self.active_timelines[slot].as_mut() {
                    t.detected.get_or_insert(tick);
                    t.first_opportunity.get_or_insert(tick);
                }
                ev.push((EventKind::Detected, slot, ctx.kin[slot].p));
            }
            hit_slots.push((slot, on.len(), px));
        }
        self.update_tracks(ctx, &hit_slots, &mut ev);

        // 4-5. Allocation and aiming.
        self.allocate(ctx);
        self.aim(ctx);
        self.check_locks(ctx, &mut ev);
        self.verify(ctx, &mut ev);
        ev
    }

    /// A noisy 3-D fix of `p` and its 1-sigma error (the larger of the range
    /// and cross-range errors: a conservative isotropic bound for the filter).
    fn noisy_fix(&mut self, q: Vec3, p: Vec3, n_eyes: usize) -> (Vec3, f64) {
        let d = p - q;
        let range = d.norm();
        let u = d / range;
        let (r, up) = eyes::basis(u);
        let sig_a = self.cfg.angle_noise_mrad * 1e-3 / (n_eyes.max(1) as f64).sqrt();
        // A laser rangefinder is available once enough eyes hold the target.
        let sig_r = if n_eyes >= self.cfg.lock_eyes {
            self.cfg.lrf_noise_m
        } else {
            self.cfg.passive_range_frac * range
        };
        let g = |rng: &mut Rng| {
            // Box-Muller from the Sensor stream (two uniforms per normal).
            let u1 = rng.next_f64().max(1e-300);
            let u2 = rng.next_f64();
            (-2.0 * crate::math::ln(u1)).sqrt() * crate::math::sin_cos(std::f64::consts::TAU * u2).1
        };
        let (ea, eb, er) = (g(&mut self.rng), g(&mut self.rng), g(&mut self.rng));
        let z = p + r * (ea * sig_a * range) + up * (eb * sig_a * range) + u * (er * sig_r);
        (z, sig_r.max(sig_a * range))
    }

    fn update_tracks(
        &mut self,
        ctx: &SensorCtx,
        hits: &[(usize, usize, f64)],
        ev: &mut Vec<SensorEvent>,
    ) {
        let q = ctx.queen.p;
        let tick = ctx.tick;
        let frame_dt = self.frame_ticks as f64 * self.dt;
        let lost_ticks = (self.cfg.lost_after_s / self.dt).round() as u64;
        let pending_lost_ticks = (self.cfg.pending_lost_after_s / self.dt).round() as u64;
        let (m, n) = (self.cfg.confirm_m, self.cfg.confirm_n);
        let mut remove = Vec::new();
        for k in 0..self.tracks.len() {
            let slot = self.tracks[k].slot;
            let hit = hits.iter().find(|h| h.0 == slot).copied();
            // Constant-velocity Kalman filter, per axis (Red-Team P3-7). It
            // predicts every frame and corrects with each fix weighted by that
            // fix's noise, so sparse sweep fixes and 70 m passive-range noise
            // are both handled. Extrapolation stops after MAX_EXTRAPOLATION_S.
            {
                let t = &mut self.tracks[k];
                if t.has_estimate
                    && tick.saturating_sub(t.last_hit_tick) as f64 * self.dt <= MAX_EXTRAPOLATION_S
                {
                    kf_predict(t, frame_dt);
                }
            }
            if let Some((_, n_eyes, px)) = hit {
                let (z, sigma) = self.noisy_fix(q, ctx.kin[slot].p, n_eyes);
                let t = &mut self.tracks[k];
                t.last_px = px;
                let gap_s = tick.saturating_sub(t.last_hit_tick) as f64 * self.dt;
                // After a gap longer than the extrapolation window both the
                // estimate and its covariance are frozen and stale: restart.
                if !t.has_estimate || gap_s > MAX_EXTRAPOLATION_S {
                    t.est_p = z;
                    t.est_v = Vec3::ZERO;
                    t.cov = [
                        sigma * sigma,
                        0.0,
                        INITIAL_SPEED_SIGMA * INITIAL_SPEED_SIGMA,
                    ];
                    t.has_estimate = true;
                } else {
                    kf_update(t, z, sigma * sigma);
                }
                t.last_hit_tick = tick;
            }
            let t = &mut self.tracks[k];
            match t.state {
                TrackState::Tentative => {
                    t.window = ((t.window << 1) | hit.is_some() as u32) & ((1 << n) - 1);
                    t.frames += 1;
                    if t.window.count_ones() >= m {
                        t.state = TrackState::Confirmed;
                        self.counters.tracks_confirmed += 1;
                        if let Some(tl) = self.active_timelines[slot].as_mut() {
                            tl.confirmed.get_or_insert(tick);
                        }
                        ev.push((EventKind::TrackConfirmed, slot, t.est_p));
                    } else if t.frames >= n {
                        remove.push(k);
                    }
                }
                TrackState::Confirmed => {
                    // Only frames with enough pixels carry a usable temporal
                    // signature (Red-Team P3-3): far, tiny targets wait.
                    if let Some((_, _, px)) = hit {
                        if px >= self.cfg.classify_min_px {
                            t.class_hits += 1;
                            t.class_px_sum += px;
                        }
                    }
                    if t.class_hits >= self.cfg.classify_hits {
                        let px = t.class_px_sum / t.class_hits as f64;
                        let p_right = 1.0 - 0.5 * crate::math::exp(-px / self.cfg.classify_px0);
                        let truth = self.active_timelines[slot]
                            .as_ref()
                            .map(|tl| tl.class)
                            .unwrap_or(TargetClass::Drone);
                        // Asymmetric costs: missing a drone risks the refinery,
                        // a false alarm costs a few eyes for a moment (and the
                        // converged second look clears it). So the decision is
                        // biased toward "drone" (Red-Team P3-8):
                        //   P(drone -> bird) = e (1 - bias),  P(bird -> drone) = min(0.5, e (1 + bias)).
                        let e = 1.0 - p_right;
                        let bias = self.cfg.classify_drone_bias;
                        let err = match truth {
                            TargetClass::Drone => e * (1.0 - bias),
                            TargetClass::Bird => (e * (1.0 + bias)).min(0.5),
                        };
                        let right = self.rng.next_f64() >= err;
                        let est = match (truth, right) {
                            (c, true) => c,
                            (TargetClass::Drone, false) => TargetClass::Bird,
                            (TargetClass::Bird, false) => TargetClass::Drone,
                        };
                        let t = &mut self.tracks[k];
                        if t.class_est == Some(TargetClass::Bird)
                            && est == TargetClass::Drone
                            && truth == TargetClass::Drone
                        {
                            self.counters.missed_drones_recovered += 1;
                        }
                        t.class_est = Some(est);
                        t.decided_px = px;
                        if let Some(tl) = self.active_timelines[slot].as_mut() {
                            tl.classified = Some(tick);
                            tl.classified_as = Some(est);
                        }
                        let code = Vec3::new(est as u8 as f64, truth as u8 as f64, px);
                        ev.push((EventKind::TrackClassified, slot, code));
                        match est {
                            TargetClass::Drone => {
                                t.state = TrackState::Hostile;
                                // Every hostile verdict earns its own second look,
                                // including re-examined former "birds".
                                t.verified = false;
                                t.evidence = 0.0;
                                t.burst_until =
                                    tick + (self.cfg.lock_burst_s / self.dt).round() as u64;
                                self.counters.declared_hostile += 1;
                                if truth == TargetClass::Bird {
                                    self.counters.false_alarms += 1;
                                }
                                ev.push((EventKind::Convergence, slot, t.est_p));
                            }
                            TargetClass::Bird => {
                                t.state = TrackState::Ignored;
                                self.counters.birds_ignored += 1;
                                if truth == TargetClass::Drone {
                                    self.counters.missed_drones += 1;
                                }
                            }
                        }
                        let id = t.id;
                        self.release_confirm(id);
                    } else if tick.saturating_sub(t.last_hit_tick) > pending_lost_ticks {
                        // Unclassified tracks are refreshed only by sweep passes,
                        // so they get a longer memory than tracks held by eyes.
                        t.state = TrackState::Lost;
                        self.counters.tracks_lost += 1;
                        let id = t.id;
                        ev.push((EventKind::TrackLost, slot, self.tracks[k].est_p));
                        self.release_eyes(id);
                    }
                }
                TrackState::Hostile => {
                    if tick.saturating_sub(t.last_hit_tick) > lost_ticks {
                        t.state = TrackState::Lost;
                        t.locked = false;
                        self.counters.tracks_lost += 1;
                        let id = t.id;
                        ev.push((EventKind::TrackLost, slot, self.tracks[k].est_p));
                        self.release_eyes(id);
                    }
                }
                TrackState::Lost => {
                    if hit.is_some() {
                        // A reacquired track keeps its verdict: re-rolling the
                        // classifier on every reacquisition would hand each bird
                        // a fresh chance to become a false alarm.
                        t.state = match t.class_est {
                            Some(TargetClass::Drone) => TrackState::Hostile,
                            Some(TargetClass::Bird) => TrackState::Ignored,
                            None => TrackState::Confirmed,
                        };
                        ev.push((EventKind::TrackReacquired, slot, t.est_p));
                    }
                }
                TrackState::Ignored => {
                    // Safety net (Red-Team P3-5): "wildlife" gets another look
                    // whenever it is seen with 1.5x the pixels its verdict was
                    // based on (it came closer), and once more if it nears the
                    // refinery. At most `max_reexams` looks per track.
                    let near = (t.est_p - ctx.refinery_center).horizontal().norm()
                        < self.cfg.reexamine_radius_m;
                    let sharper = hit.is_some() && t.last_px >= 1.5 * t.decided_px;
                    if (near || sharper) && t.reexams < self.cfg.max_reexams {
                        t.reexams += 1;
                        t.state = TrackState::Confirmed;
                        t.class_hits = 0;
                        t.class_px_sum = 0.0;
                        self.counters.reexaminations += 1;
                        ev.push((EventKind::TrackReexamined, slot, t.est_p));
                    }
                }
            }
        }
        for k in remove.into_iter().rev() {
            let t = self.tracks.remove(k);
            self.counters.tentative_dropped += 1;
            self.release_eyes(t.id);
        }
    }

    fn release_confirm(&mut self, track: u32) {
        for e in &mut self.eyes {
            if e.task == (Task::Confirm { track }) {
                e.task = Task::Sweep;
            }
        }
    }

    /// Decide every eye's task for the coming frame (see module docs).
    fn allocate(&mut self, ctx: &SensorCtx) {
        let tick = ctx.tick;
        let q = ctx.queen.p;
        // A confirmation stare is kept while the track is tentative, or while
        // it is confirmed and close enough to classify (enough pixels).
        // Otherwise the eye goes back to sweeping (Red-Team P3-4).
        let min_px = self.cfg.classify_min_px;
        let useful = |t: &Track| match t.state {
            TrackState::Tentative => true,
            TrackState::Confirmed => t.last_px >= min_px,
            _ => false,
        };
        for i in 0..self.eyes.len() {
            if let Task::Confirm { track } = self.eyes[i].task {
                if !self.tracks.iter().any(|t| t.id == track && useful(t)) {
                    self.eyes[i].task = Task::Sweep;
                }
            }
        }
        let mut hostile: Vec<(u32, f64, u64, u64)> = self
            .tracks
            .iter()
            .filter(|t| t.state == TrackState::Hostile)
            .map(|t| {
                // Priority: estimated time to reach the refinery (smaller first).
                let dist = (t.est_p - ctx.refinery_center).horizontal().norm();
                let speed = t.est_v.horizontal().norm().max(5.0);
                (t.id, dist / speed, t.burst_until, t.id as u64)
            })
            .collect();
        hostile.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().then(a.0.cmp(&b.0)));

        // Clear previous Track tasks; they are reassigned below (keeping
        // continuity by preferring eyes already on the same track).
        let prev: Vec<Option<u32>> = self
            .eyes
            .iter()
            .map(|e| {
                if let Task::Track { track } = e.task {
                    Some(track)
                } else {
                    None
                }
            })
            .collect();
        for e in &mut self.eyes {
            if matches!(e.task, Task::Track { .. }) {
                e.task = Task::Sweep;
            }
        }
        // Lock burst ("SHE LOOKS AT YOU"): a lone hostile takes every eye. With
        // several hostiles, total convergence would starve the others, so the
        // sentinel/TDM allocation below applies instead (Red-Team P3-6).
        if let [(id, _, burst_until, _)] = hostile[..]
            && burst_until > tick
        {
            for e in &mut self.eyes {
                e.task = Task::Track { track: id };
            }
            self.retile();
            return;
        }

        // Classification stares come before extra eyes on known hostiles: an
        // unclassified track may be the next drone (Red-Team P3-10: with stale
        // Track tasks counted as busy, new tracks waited 23 s median to be
        // classified). Capped at `max_confirm_eyes`, never below the sentinels.
        let needing: Vec<u32> = self
            .tracks
            .iter()
            .filter(|t| {
                t.state == TrackState::Tentative
                    || (t.state == TrackState::Confirmed && t.last_px >= min_px)
            })
            .map(|t| t.id)
            .filter(|id| {
                !self
                    .eyes
                    .iter()
                    .any(|e| e.task == Task::Confirm { track: *id })
            })
            .collect();
        for id in needing {
            let confirming = self
                .eyes
                .iter()
                .filter(|e| matches!(e.task, Task::Confirm { .. }))
                .count();
            if self.sweeping() <= self.cfg.sentinel_eyes || confirming >= self.cfg.max_confirm_eyes
            {
                break;
            }
            let dir = self.track_dir(q, id);
            if let Some(i) = self.nearest_free(dir, |e| e.task == Task::Sweep) {
                self.eyes[i].task = Task::Confirm { track: id };
            }
        }
        if hostile.is_empty() {
            self.retile();
            return;
        }

        let pool: Vec<usize> = (0..self.eyes.len())
            .filter(|&i| self.eyes[i].task == Task::Sweep)
            .collect();
        let available = pool.len().saturating_sub(self.cfg.sentinel_eyes);
        let n_h = hostile.len();
        let min = self.cfg.min_eyes_per_track.max(1);
        let mut quota: Vec<(u32, usize)> = Vec::new();
        if n_h * min <= available {
            let base = available / n_h;
            let extra = available % n_h;
            for (j, h) in hostile.iter().enumerate() {
                quota.push((h.0, base + usize::from(j < extra)));
            }
        } else {
            // Time-division: rotate which tracks hold eyes each TDM slot.
            let m = (available / min).max(if available > 0 { 1 } else { 0 });
            let slot_ticks = ((self.cfg.tdm_slot_s / self.dt).round() as u64).max(1);
            let rot = (tick / slot_ticks) as usize;
            for j in 0..m {
                let h = hostile[(rot * m + j) % n_h];
                quota.push((
                    h.0,
                    if j == 0 {
                        available - min * (m - 1)
                    } else {
                        min
                    },
                ));
            }
        }
        for (id, want) in quota {
            let dir = self.track_dir(q, id);
            let mut got = 0;
            for (e, p) in self.eyes.iter_mut().zip(&prev) {
                if got < want && *p == Some(id) && e.task == Task::Sweep {
                    e.task = Task::Track { track: id };
                    got += 1;
                }
            }
            while got < want {
                let free = |e: &Eye| e.task == Task::Sweep;
                match self.nearest_free(dir, free) {
                    Some(i) => {
                        self.eyes[i].task = Task::Track { track: id };
                        got += 1;
                    }
                    None => break,
                }
            }
        }
        self.retile();
    }

    fn sweeping(&self) -> usize {
        self.eyes.iter().filter(|e| e.task == Task::Sweep).count()
    }

    fn track_dir(&self, q: Vec3, id: u32) -> Vec3 {
        self.tracks
            .iter()
            .find(|t| t.id == id)
            .map(|t| (t.est_p - q).normalized_or_zero())
            .unwrap_or(Vec3::new(1.0, 0.0, 0.0))
    }

    fn nearest_free(&self, dir: Vec3, free: impl Fn(&Eye) -> bool) -> Option<usize> {
        let mut best: Option<(usize, f64)> = None;
        for (i, e) in self.eyes.iter().enumerate() {
            if !free(e) {
                continue;
            }
            let a = eyes::angle(e.dir, dir);
            if best.is_none_or(|(_, b)| a < b) {
                best = Some((i, a));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Re-tile the search region across the eyes currently sweeping.
    fn retile(&mut self) {
        let sweepers: Vec<usize> = (0..self.eyes.len())
            .filter(|&i| self.eyes[i].task == Task::Sweep)
            .collect();
        if sweepers == self.sweepers {
            return;
        }
        self.tiling = if sweepers.is_empty() {
            Vec::new()
        } else {
            sweep::tile(sweepers.len(), self.el_min, self.el_max)
        };
        let (fh, fv) = (
            self.cfg.fov_h_deg.to_radians(),
            self.cfg.fov_v_deg.to_radians(),
        );
        self.cells = self
            .tiling
            .iter()
            .map(|s| sweep::cells(s, fh, fv, CELL_OVERLAP))
            .collect();
        for (j, &i) in sweepers.iter().enumerate() {
            let e = &mut self.eyes[i];
            e.sector = j;
            e.dwell_left = 0; // move to a point in the new sector next frame
        }
        self.sweepers = sweepers;
    }

    fn aim(&mut self, ctx: &SensorCtx) {
        let q = ctx.queen.p;
        let dwell = self.cfg.dwell_frames;
        for i in 0..self.eyes.len() {
            match self.eyes[i].task {
                Task::Sweep => {
                    let e = &mut self.eyes[i];
                    if e.settled && e.dwell_left > 0 {
                        e.dwell_left -= 1;
                    }
                    if e.dwell_left == 0 {
                        e.sweep_k += 1;
                        let cells = &self.cells[e.sector];
                        let cell = cells[sweep::visit(e.sweep_k, cells.len(), e.shift)];
                        let [u, v] = sweep::r2(e.sweep_k, e.shift);
                        e.aim = cell.direction(
                            CELL_JITTER * (2.0 * u - 1.0),
                            CELL_JITTER * (2.0 * v - 1.0),
                        );
                        e.dwell_left = dwell;
                    }
                }
                Task::Confirm { track } | Task::Track { track } => {
                    let d = self.track_dir(q, track);
                    let e = &mut self.eyes[i];
                    e.aim = d;
                    e.dwell_left = 0;
                }
            }
        }
    }

    /// "Twenty-four times I look upon you": the eyes held on a hostile pile up
    /// evidence, sum over frames of (settled eyes on target x pixels on
    /// target). Only when it reaches `verify_evidence` is the verdict
    /// re-decided, with error 0.5 exp(-evidence / px0) (~1e-6 at the default).
    /// One decisive look, rather than repeated weak ones: re-rolling weak
    /// checks would eventually stand down a real drone (Red-Team P3-9).
    fn verify(&mut self, ctx: &SensorCtx, ev: &mut Vec<SensorEvent>) {
        let q = ctx.queen.p;
        for k in 0..self.tracks.len() {
            let t = &self.tracks[k];
            if t.state != TrackState::Hostile || t.verified || t.eyes == 0 {
                continue;
            }
            let slot = t.slot;
            let Some(truth) = self.active_timelines[slot].as_ref().map(|tl| tl.class) else {
                continue;
            };
            let size = self.size_of(truth);
            let range = (ctx.kin[slot].p - q).norm();
            let px = self.optics.pixels(self.band, size, range);
            let gained = t.eyes as f64 * px;
            let t = &mut self.tracks[k];
            t.evidence += gained;
            if t.evidence < self.cfg.verify_evidence {
                continue;
            }
            let err = 0.5 * crate::math::exp(-t.evidence / self.cfg.classify_px0);
            let says_bird = match truth {
                TargetClass::Bird => self.rng.next_f64() >= err,
                TargetClass::Drone => self.rng.next_f64() < err,
            };
            let t = &mut self.tracks[k];
            t.verified = true;
            if says_bird {
                t.state = TrackState::Ignored;
                t.class_est = Some(TargetClass::Bird);
                t.locked = false;
                // A verdict backed by many eyes: re-examine only if seen with
                // twice the pixels it was verified at.
                t.decided_px = 2.0 * px;
                let id = t.id;
                if truth == TargetClass::Bird {
                    self.counters.false_alarms_cleared += 1;
                } else {
                    self.counters.missed_drones += 1;
                }
                if let Some(tl) = self.active_timelines[slot].as_mut() {
                    tl.classified_as = Some(TargetClass::Bird);
                }
                ev.push((
                    EventKind::TrackClassified,
                    slot,
                    Vec3::new(TargetClass::Bird as u8 as f64, truth as u8 as f64, px),
                ));
                self.release_eyes(id);
            }
        }
    }

    fn check_locks(&mut self, ctx: &SensorCtx, ev: &mut Vec<SensorEvent>) {
        let q = ctx.queen.p;
        for k in 0..self.tracks.len() {
            let (id, slot) = (self.tracks[k].id, self.tracks[k].slot);
            let u = (ctx.kin[slot].p - q).normalized_or_zero();
            let n = self
                .eyes
                .iter()
                .filter(|e| {
                    e.task == Task::Track { track: id }
                        && e.settled
                        && eyes::in_frustum(e.dir, self.tan_h, self.tan_v, u)
                })
                .count() as u32;
            let t = &mut self.tracks[k];
            t.eyes = n;
            if t.state == TrackState::Hostile && !t.locked && n as usize >= self.cfg.lock_eyes {
                t.locked = true;
                self.counters.locks += 1;
                if let Some(tl) = self.active_timelines[slot].as_mut() {
                    tl.locked.get_or_insert(ctx.tick);
                }
                ev.push((EventKind::EyesLocked, slot, Vec3::new(n as f64, 0.0, 0.0)));
            }
        }
    }

    // ---- read access ---------------------------------------------------------

    pub fn eyes(&self) -> &[Eye] {
        &self.eyes
    }
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }
    pub fn band(&self) -> Band {
        self.band
    }
    pub fn optics(&self) -> &Optics {
        &self.optics
    }
    pub fn counters(&self) -> &SensorCounters {
        &self.counters
    }
    pub fn frustum_tans(&self) -> (f64, f64) {
        (self.tan_h, self.tan_v)
    }
    /// Finished timelines plus the ones still in flight.
    pub fn timelines(&self) -> Vec<Timeline> {
        let mut v = self.finished.clone();
        v.extend(self.active_timelines.iter().flatten().cloned());
        v
    }

    pub fn hash_into(&self, h: &mut Fnv64) {
        h.write_u64(self.band as u64);
        for e in &self.eyes {
            for v in [e.dir, e.aim] {
                h.write_f64(v.x);
                h.write_f64(v.y);
                h.write_f64(v.z);
            }
            h.write_u64(e.task.code());
            h.write_u64(e.sweep_k);
            h.write_u64(e.dwell_left as u64);
            h.write_u64(e.sector as u64);
        }
        for t in &self.tracks {
            h.write_u64(t.id as u64);
            h.write_u64(t.slot as u64);
            h.write_u64(t.state as u64);
            h.write_u64(t.window as u64);
            h.write_f64(t.est_p.x);
            h.write_f64(t.est_p.y);
            h.write_f64(t.est_p.z);
        }
        for s in self.rng.state() {
            h.write_u64(s);
        }
    }
}
