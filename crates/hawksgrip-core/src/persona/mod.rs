//! `RoyalPersonaEngine`: Kindah's voice and the Seven's devotion telemetry.
//!
//! **Design rule (persona invariance, ADR-0000).** The persona only *reads*
//! events the world has already emitted; it owns its own RNG stream and never
//! writes back. Switching theatrical/clinical (the `[T]` key) therefore cannot
//! change what the simulation does. `tests/persona.rs` proves it with state
//! hashes.
//!
//! - **Theatrical:** Kindah, an unyielding pre-Islamic Arabian monarch and bee
//!   queen, speaks with regal disdain (electric lightning above "peasant"
//!   combustion). The Seven chant their devotion beside their real metrics.
//! - **Clinical:** terse, NATO-style telemetry with the same facts.
//!
//! Kindah only speaks about what her sensors know: a classification line uses
//! the *estimated* class, never the hidden truth.

mod lines;

use crate::sim::rng::{Rng, Stream};
use crate::telemetry::{Event, EventKind};
use crate::vec3::Vec3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Theatrical,
    Clinical,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Mode::Theatrical => "theatrical",
            Mode::Clinical => "clinical",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub tick: u64,
    /// "KINDAH", "NODE-03" or "SYSTEM".
    pub speaker: String,
    pub kind: &'static str,
    pub text: String,
}

/// What the persona may know when it speaks.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    pub queen: Vec3,
    pub band: &'static str,
    pub visibility_m: f64,
}

#[derive(Clone, Debug)]
pub struct RoyalPersonaEngine {
    mode: Mode,
    rng: Rng,
    hz: u32,
    last_pick: [usize; lines::POOLS],
    next_idle: u64,
    next_heartbeat: u64,
    heartbeat_node: u8,
}

const IDLE_EVERY_S: u64 = 25;
const HEARTBEAT_EVERY_S: u64 = 4;

impl RoyalPersonaEngine {
    pub fn new(seed: u64, hz: u32, mode: Mode) -> Self {
        Self {
            mode,
            rng: Rng::stream(seed, Stream::Persona),
            hz,
            last_pick: [usize::MAX; lines::POOLS],
            next_idle: 5 * hz as u64,
            next_heartbeat: 2 * hz as u64,
            heartbeat_node: 0,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn set_mode(&mut self, m: Mode) {
        self.mode = m;
    }

    pub fn toggle(&mut self) -> Mode {
        self.mode = match self.mode {
            Mode::Theatrical => Mode::Clinical,
            Mode::Clinical => Mode::Theatrical,
        };
        self.mode
    }

    /// Pick a line from a pool, never the same one twice in a row.
    fn pick(&mut self, pool: usize) -> &'static str {
        let options = lines::THEATRICAL[pool];
        let n = options.len();
        let mut i = (self.rng.next_u64() % n as u64) as usize;
        if n > 1 && i == self.last_pick[pool] {
            i = (i + 1 + (self.rng.next_u64() % (n as u64 - 1)) as usize) % n;
        }
        self.last_pick[pool] = i;
        options[i]
    }

    fn line(&self, tick: u64, speaker: &str, kind: &'static str, text: String) -> Line {
        Line {
            tick,
            speaker: speaker.to_string(),
            kind,
            text,
        }
    }

    /// Voice one world event, or None if the persona has nothing to say.
    pub fn on_event(&mut self, e: &Event, ctx: &Ctx) -> Option<Line> {
        let p = Vec3::from(e.data);
        let f = Facts::new(e, ctx, p);
        use EventKind as K;
        let (pool, clinical, speaker) = match e.kind {
            K::Detected => (
                lines::DETECTED,
                format!(
                    "DETECT | {} | BRG {:03} | RNG {} KM | ALT {} M | {}",
                    f.obj, f.brg, f.rng_km, f.alt, ctx.band
                ),
                "KINDAH",
            ),
            K::TrackConfirmed => (
                lines::CONFIRMED,
                format!(
                    "TRACK CONFIRMED | {} | BRG {:03} | RNG {} KM",
                    f.obj, f.brg, f.rng_km
                ),
                "KINDAH",
            ),
            K::TrackClassified => {
                // data = (estimated class, true class, pixels); only the estimate is spoken.
                let drone = e.data[0] == 0.0;
                let pool = if drone {
                    lines::CLASS_DRONE
                } else {
                    lines::CLASS_BIRD
                };
                let c = format!(
                    "CLASSIFIED {} | {} | {:.1} PX",
                    if drone { "UAS HOSTILE" } else { "BIOLOGICAL" },
                    f.obj,
                    e.data[2]
                );
                (pool, c, "KINDAH")
            }
            K::Convergence => (
                lines::CONVERGENCE,
                format!(
                    "ARRAY CONVERGE | ALL EYES | {} | BRG {:03} | RNG {} KM",
                    f.obj, f.brg, f.rng_km
                ),
                "KINDAH",
            ),
            K::EyesLocked => (
                lines::LOCKED,
                format!("LOCK | {} | {} EYES SETTLED", f.obj, e.data[0] as u32),
                "KINDAH",
            ),
            K::TrackLost => (
                lines::LOST,
                format!("TRACK LOST | {} | LAST BRG {:03}", f.obj, f.brg),
                "KINDAH",
            ),
            K::TrackReacquired => (
                lines::REACQUIRED,
                format!(
                    "TRACK REACQUIRED | {} | BRG {:03} | RNG {} KM",
                    f.obj, f.brg, f.rng_km
                ),
                "KINDAH",
            ),
            K::TrackReexamined => (
                lines::REEXAMINED,
                format!("RE-EXAMINE | {} | RNG {} KM", f.obj, f.rng_km),
                "KINDAH",
            ),
            K::BandSwitch => {
                let lwir = e.data[0] as u8 == 2;
                let pool = if lwir || e.data[0] as u8 == 1 {
                    lines::BAND_DARK
                } else {
                    lines::BAND_CLEAR
                };
                (
                    pool,
                    format!(
                        "BAND SWITCH -> {} | VIS {:.1} KM",
                        ctx.band,
                        e.data[1] / 1e3
                    ),
                    "KINDAH",
                )
            }
            K::VisibilityChanged => {
                let storm = e.data[0] < 3_000.0;
                let pool = if storm {
                    lines::STORM
                } else {
                    lines::CLEAR_SKY
                };
                (
                    pool,
                    format!("VISIBILITY {:.1} KM", e.data[0] / 1e3),
                    "KINDAH",
                )
            }
            K::RefineryBreach => (
                lines::BREACH,
                format!(
                    "REFINERY BREACH | {} | INTERCEPTION NOT MODELLED (PHASES 0-3)",
                    f.obj
                ),
                "KINDAH",
            ),
            K::BirdsSpawned => {
                return Some(self.line(
                    e.tick,
                    "SYSTEM",
                    e.kind.name(),
                    format!("RED TEAM | {} BIRDS RELEASED", e.data[0] as u32),
                ));
            }
            K::GroundImpact => {
                return Some(self.line(
                    e.tick,
                    "SYSTEM",
                    e.kind.name(),
                    format!("GROUND IMPACT | {}", f.obj),
                ));
            }
            K::ThreatOutOfBounds => {
                return Some(self.line(
                    e.tick,
                    "SYSTEM",
                    e.kind.name(),
                    format!("LEFT SECTOR | {}", f.obj),
                ));
            }
            // Spawns are truth Kindah does not know; drops/orders are bookkeeping.
            _ => return None,
        };
        let text = match self.mode {
            Mode::Clinical => clinical,
            Mode::Theatrical => f.fill(self.pick(pool)),
        };
        Some(self.line(e.tick, speaker, e.kind.name(), text))
    }

    /// Time-driven lines: Kindah's idle musings and the Seven's heartbeats.
    /// `station_err_m[k]` is node k's distance from its station slot (a real metric).
    pub fn tick(
        &mut self,
        tick: u64,
        ctx: &Ctx,
        eyes_sweeping: usize,
        station_err_m: &[f64; 7],
    ) -> Vec<Line> {
        let mut out = Vec::new();
        if tick >= self.next_idle {
            self.next_idle = tick + IDLE_EVERY_S * self.hz as u64;
            let text = match self.mode {
                Mode::Clinical => format!(
                    "STATUS | EYES SWEEPING {eyes_sweeping}/24 | BAND {} | VIS {:.1} KM",
                    ctx.band,
                    ctx.visibility_m / 1e3
                ),
                Mode::Theatrical => self.pick(lines::IDLE).to_string(),
            };
            out.push(self.line(
                tick,
                if self.mode == Mode::Clinical {
                    "SYSTEM"
                } else {
                    "KINDAH"
                },
                "IDLE",
                text,
            ));
        }
        if tick >= self.next_heartbeat {
            self.next_heartbeat = tick + HEARTBEAT_EVERY_S * self.hz as u64;
            let k = self.heartbeat_node as usize;
            self.heartbeat_node = (self.heartbeat_node + 1) % 7;
            let err = station_err_m[k];
            let status = if err < 5.0 { "ON STATION" } else { "TASKED" };
            let node = format!("NODE-{:02}", k + 1);
            let metrics = format!("[{node} | HB OK | STN ERR {err:.1} M | {status}]");
            let text = match self.mode {
                Mode::Clinical => metrics,
                Mode::Theatrical => format!("{metrics}: \"{}\"", self.pick(lines::CHANT)),
            };
            out.push(self.line(tick, &node, "HEARTBEAT", text));
        }
        out
    }
}

/// Facts derived from an event, for both voices.
struct Facts {
    obj: String,
    brg: u32,
    rng_km: String,
    alt: i64,
    eyes: u32,
    band: &'static str,
    vis_km: String,
}

impl Facts {
    fn new(e: &Event, ctx: &Ctx, p: Vec3) -> Self {
        let d = p - ctx.queen;
        // Compass bearing from Kindah (clockwise from north), display only.
        let brg = (libm::atan2(d.x, d.y).to_degrees() + 360.0) % 360.0;
        Facts {
            obj: if e.entity == u32::MAX {
                "OBJ ---".into()
            } else {
                format!("OBJ {:03}", e.entity)
            },
            brg: brg.round() as u32 % 360,
            rng_km: format!("{:.1}", d.norm() / 1e3),
            alt: p.z.round() as i64,
            eyes: e.data[0] as u32,
            band: ctx.band,
            vis_km: format!("{:.1}", ctx.visibility_m / 1e3),
        }
    }

    fn fill(&self, template: &str) -> String {
        template
            .replace("{brg}", &format!("{:03}", self.brg))
            .replace("{rng_km}", &self.rng_km)
            .replace("{eyes}", &self.eyes.to_string())
            .replace("{band}", self.band)
            .replace("{vis_km}", &self.vis_km)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(kind: EventKind, data: [f64; 3]) -> Event {
        Event {
            tick: 100,
            kind,
            entity: 12,
            data,
        }
    }

    fn ctx() -> Ctx {
        Ctx {
            queen: Vec3::new(0.0, 0.0, 1500.0),
            band: "VIS",
            visibility_m: 20_000.0,
        }
    }

    #[test]
    fn clinical_lines_carry_the_facts() {
        let mut p = RoyalPersonaEngine::new(1, 120, Mode::Clinical);
        // Due east of Kindah: 4 km out, 1.2 km below her (slant 4.18 km), 300 m up.
        let l = p
            .on_event(&ev(EventKind::Detected, [4_000.0, 0.0, 300.0]), &ctx())
            .unwrap();
        assert!(
            l.text
                .starts_with("DETECT | OBJ 012 | BRG 090 | RNG 4.2 KM | ALT 300 M"),
            "{}",
            l.text
        );
        assert_eq!(l.speaker, "KINDAH");
    }

    #[test]
    fn classification_speaks_the_estimate_not_the_truth() {
        let mut p = RoyalPersonaEngine::new(1, 120, Mode::Clinical);
        // Estimated bird (1), truly a drone (0): she must say BIOLOGICAL.
        let l = p
            .on_event(&ev(EventKind::TrackClassified, [1.0, 0.0, 2.4]), &ctx())
            .unwrap();
        assert!(
            l.text.contains("BIOLOGICAL") && !l.text.contains("HOSTILE"),
            "{}",
            l.text
        );
    }

    #[test]
    fn theatrical_never_repeats_a_line_back_to_back_and_fills_placeholders() {
        let mut p = RoyalPersonaEngine::new(3, 120, Mode::Theatrical);
        let mut last = String::new();
        for _ in 0..200 {
            let l = p
                .on_event(&ev(EventKind::Detected, [3_000.0, 1_000.0, 200.0]), &ctx())
                .unwrap();
            assert_ne!(l.text, last);
            assert!(!l.text.contains('{'), "unfilled placeholder: {}", l.text);
            last = l.text;
        }
    }

    #[test]
    fn toggle_switches_voice_and_spawns_stay_secret() {
        let mut p = RoyalPersonaEngine::new(1, 120, Mode::Theatrical);
        assert_eq!(p.toggle(), Mode::Clinical);
        assert_eq!(p.toggle(), Mode::Theatrical);
        // Kindah does not know about a spawn until she sees it.
        assert!(
            p.on_event(&ev(EventKind::ThreatSpawned, [0.0; 3]), &ctx())
                .is_none()
        );
    }

    #[test]
    fn heartbeats_rotate_through_the_seven_with_real_metrics() {
        let mut p = RoyalPersonaEngine::new(1, 120, Mode::Theatrical);
        let errs = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 7.5];
        let mut nodes = Vec::new();
        for t in 0..(120 * 40) {
            for l in p.tick(t, &ctx(), 24, &errs) {
                if l.kind == "HEARTBEAT" {
                    nodes.push(l.speaker.clone());
                    if l.speaker == "NODE-07" {
                        assert!(l.text.contains("STN ERR 7.5 M | TASKED"), "{}", l.text);
                    }
                }
            }
        }
        assert_eq!(
            &nodes[..7],
            &[
                "NODE-01", "NODE-02", "NODE-03", "NODE-04", "NODE-05", "NODE-06", "NODE-07"
            ]
        );
    }

    #[test]
    fn every_pool_is_nonempty_and_line_lengths_fit_the_hud() {
        for pool in lines::THEATRICAL {
            assert!(!pool.is_empty());
            for l in pool.iter() {
                assert!(l.len() <= 160, "too long for the HUD: {l}");
            }
        }
    }
}
