//! Phase 3 integration: Kindah's 24-eye compound optical array.

use hawksgrip_core::sensing::eyes::{self, Task};
use hawksgrip_core::sensing::optics::Band;
use hawksgrip_core::sensing::{TargetClass, TrackState};
use hawksgrip_core::sim::world::QUEEN;
use hawksgrip_core::telemetry::EventKind;
use hawksgrip_core::terrain::Sector;
use hawksgrip_core::{Input, Replay, SimConfig, Vec3, World};

fn world(sector: Sector, seed: u64) -> World {
    let mut c = SimConfig::default();
    c.terrain.sector = sector;
    World::new(c, seed).unwrap()
}

#[test]
fn kill_chain_timeline_is_ordered_for_a_single_intruder() {
    let mut w = world(Sector::EasternSabkha, 3);
    w.push_input(Input::SpawnThreat {
        pos: Vec3::new(-6_500.0, 1_000.0, 150.0),
        target: Vec3::new(0.0, 0.0, 30.0),
        speed: 40.0,
    });
    let mut kinds = Vec::new();
    for _ in 0..(120 * 90) {
        w.step(1);
        kinds.extend(w.drain_events().into_iter().map(|e| e.kind));
        if kinds.contains(&EventKind::EyesLocked) {
            break;
        }
    }
    let tl = w.sensors().timelines();
    let t = tl.iter().find(|t| t.class == TargetClass::Drone).unwrap();
    let chain = [
        t.first_opportunity,
        t.detected,
        t.confirmed,
        t.classified,
        t.locked,
    ];
    assert!(chain.iter().all(|x| x.is_some()), "{t:?}");
    let ticks: Vec<u64> = chain.iter().map(|x| x.unwrap()).collect();
    assert!(ticks.windows(2).all(|w| w[0] <= w[1]), "{ticks:?}");
    assert_eq!(t.classified_as, Some(TargetClass::Drone));
    for k in [
        EventKind::Detected,
        EventKind::TrackConfirmed,
        EventKind::TrackClassified,
        EventKind::Convergence,
    ] {
        assert!(kinds.contains(&k), "missing {k:?}");
    }
}

#[test]
fn she_looks_at_you_then_sentinels_return() {
    let mut w = world(Sector::EasternSabkha, 5);
    w.push_input(Input::SpawnThreat {
        pos: Vec3::new(5_000.0, -3_000.0, 200.0),
        target: Vec3::new(0.0, 0.0, 30.0),
        speed: 35.0,
    });
    let burst_ticks = (w.config().sensors.lock_burst_s * 120.0) as u64;
    let mut converged_at = None;
    for _ in 0..(120 * 120) {
        w.step(1);
        if w.drain_events()
            .iter()
            .any(|e| e.kind == EventKind::Convergence)
        {
            converged_at = Some(w.clock().tick());
            break;
        }
    }
    let t0 = converged_at.expect("never converged");
    // Within the burst: every eye is assigned to the hostile.
    w.step(8);
    let id = w
        .sensors()
        .tracks()
        .iter()
        .find(|t| t.state == TrackState::Hostile)
        .unwrap()
        .id;
    assert!(
        w.sensors()
            .eyes()
            .iter()
            .all(|e| e.task == Task::Track { track: id })
    );
    // After the burst: sentinels sweep again, the rest stay on the target.
    w.step(burst_ticks + 8);
    assert!(w.clock().tick() > t0 + burst_ticks);
    let sweeping = w
        .sensors()
        .eyes()
        .iter()
        .filter(|e| e.task == Task::Sweep)
        .count();
    assert_eq!(sweeping, w.config().sensors.sentinel_eyes);
    let tr = w.sensors().tracks().iter().find(|t| t.id == id).unwrap();
    assert!(tr.locked && tr.eyes >= 2, "{tr:?}");
}

#[test]
fn saturation_raid_keeps_sentinels_and_shares_eyes_fairly() {
    let mut w = world(Sector::EasternSabkha, 7);
    w.push_input(Input::RandomRaid { count: 40 });
    let n_eyes = w.config().sensors.n_eyes;
    let sentinels = w.config().sensors.sentinel_eyes;
    let mut eyes_seen: std::collections::BTreeMap<u32, u32> = Default::default();
    let mut hostile_ids: std::collections::BTreeSet<u32> = Default::default();
    let mut min_sweepers_outside_burst = usize::MAX;
    for _ in 0..(120 * 150) {
        w.step(1);
        let tick = w.clock().tick();
        let s = w.sensors();
        // Eye tasks are set at camera frames (every 4 ticks) and persist until
        // the next frame, so a burst's assignment can outlive burst_until by a frame.
        let in_burst = s.tracks().iter().any(|t| t.burst_until + 4 >= tick);
        if !in_burst {
            let sw = s.eyes().iter().filter(|e| e.task == Task::Sweep).count();
            min_sweepers_outside_burst = min_sweepers_outside_burst.min(sw);
        }
        for e in s.eyes() {
            if let Task::Track { track } = e.task {
                *eyes_seen.entry(track).or_default() += 1;
            }
        }
        for t in s.tracks() {
            if t.state == TrackState::Hostile {
                hostile_ids.insert(t.id);
            }
        }
    }
    assert_eq!(w.sensors().eyes().len(), n_eyes);
    assert!(
        min_sweepers_outside_burst >= sentinels,
        "sentinels abandoned: {min_sweepers_outside_burst}"
    );
    // Every track that was ever declared hostile received eyes at some point (TDM fairness).
    let c = w.sensors().counters().clone();
    assert!(
        hostile_ids.len() >= 30,
        "only {} hostile tracks: {c:?}",
        hostile_ids.len()
    );
    let starved: Vec<_> = hostile_ids
        .iter()
        .filter(|id| !eyes_seen.contains_key(id))
        .collect();
    assert!(
        starved.is_empty(),
        "hostile tracks that never got an eye: {starved:?}"
    );
}

#[test]
fn sandstorm_switches_band_and_shortens_detection_range() {
    let detection_ranges = |vis: Option<f64>| {
        let mut w = world(Sector::EasternSabkha, 11);
        let mut band = w.sensors().band();
        if let Some(v) = vis {
            w.push_input(Input::SetVisibility { visibility_m: v });
            w.step(1);
            band = w.sensors().band();
        }
        w.push_input(Input::RandomRaid { count: 24 });
        let mut ranges = Vec::new();
        for _ in 0..(120 * 200) {
            w.step(1);
            let q = w.kin()[QUEEN].p;
            for e in w.drain_events() {
                if e.kind == EventKind::Detected {
                    ranges.push((Vec3::from(e.data) - q).norm());
                }
            }
        }
        (
            band,
            ranges.iter().sum::<f64>() / ranges.len().max(1) as f64,
            ranges.len(),
        )
    };
    let (b_clear, r_clear, n_clear) = detection_ranges(None);
    let (b_storm, r_storm, n_storm) = detection_ranges(Some(600.0));
    assert_eq!(b_clear, Band::Visible);
    assert_eq!(b_storm, Band::Lwir);
    assert!(
        n_clear >= 20 && n_storm >= 10,
        "{n_clear} / {n_storm} detections"
    );
    assert!(
        r_storm < 0.6 * r_clear,
        "storm {r_storm:.0} m vs clear {r_clear:.0} m"
    );
}

#[test]
fn wildlife_is_mostly_ignored_and_never_breaches() {
    let mut w = world(Sector::EasternSabkha, 13);
    w.push_input(Input::SpawnBirds { count: 30 });
    w.step(120 * 600);
    let c = w.sensors().counters().clone();
    eprintln!("{c:?}");
    assert_eq!(w.counters().refinery_breaches, 0);
    assert_eq!(w.counters().threats_spawned, 0);
    // Birds are only classifiable within ~1.8 km (3 px at 0.6 m); over ten
    // minutes of wandering, many come that close.
    assert!(c.birds_ignored >= 10, "{c:?}");
    // After the converged eyes' second look, at most a small fraction of
    // birds remain declared hostile.
    let standing = c.false_alarms - c.false_alarms_cleared;
    assert!(standing * 10 <= c.birds_ignored + c.false_alarms, "{c:?}");
}

#[test]
fn sensing_replays_bit_for_bit_with_storm_and_birds() {
    let mut w = world(Sector::HighlandWadi, 21);
    w.push_input(Input::RandomRaid { count: 12 });
    w.push_input(Input::SpawnBirds { count: 10 });
    w.step(120 * 20);
    w.push_input(Input::SetVisibility {
        visibility_m: 700.0,
    });
    w.step(120 * 40);
    w.push_input(Input::SetVisibility {
        visibility_m: 15_000.0,
    });
    w.step(120 * 20);
    let rep = Replay::record(&w).verify(None).unwrap();
    assert!(rep.ok, "{rep:?}");
}

#[test]
fn sweep_revisits_every_direction_quickly() {
    // No targets: all 24 eyes sweep. Sample a grid of directions over the
    // search region and record the longest gap between looks over 20 s.
    let mut w = world(Sector::Flat, 1);
    let s = &w.config().sensors;
    let (th, tv) = (
        s.fov_h_deg.to_radians() / 2.0,
        s.fov_v_deg.to_radians() / 2.0,
    );
    let (lo, hi) = (
        s.search_el_min_deg.to_radians(),
        s.search_el_max_deg.to_radians(),
    );
    let (tan_h, tan_v) = (th.tan(), tv.tan());
    let mut dirs = Vec::new();
    for i in 0..72 {
        for j in 0..18 {
            let az = std::f64::consts::TAU * (i as f64 + 0.5) / 72.0;
            let el = lo + (hi - lo) * (j as f64 + 0.5) / 18.0;
            dirs.push(Vec3::new(
                el.cos() * az.cos(),
                el.cos() * az.sin(),
                el.sin(),
            ));
        }
    }
    let mut last = vec![0u64; dirs.len()];
    let mut worst = 0u64;
    let frame = 4;
    for _ in 0..(120 * 20 / frame) {
        w.step(frame);
        let t = w.clock().tick();
        for (k, d) in dirs.iter().enumerate() {
            let seen = w
                .sensors()
                .eyes()
                .iter()
                .any(|e| e.settled && eyes::in_frustum(e.dir, tan_h, tan_v, *d));
            if seen {
                worst = worst.max(t - last[k]);
                last[k] = t;
            }
        }
    }
    let t_end = w.clock().tick();
    worst = worst.max(last.iter().map(|&l| t_end - l).max().unwrap());
    let worst_s = worst as f64 / 120.0;
    eprintln!("worst revisit over 1296 directions: {worst_s:.2} s");
    assert!(
        worst_s < 6.0,
        "blind spot: a direction went {worst_s:.2} s unobserved"
    );
}

#[test]
fn track_estimates_stay_physical_through_long_gaps() {
    // Sparse sweep fixes on wandering birds must never blow the filter up.
    let mut w = world(Sector::HighlandWadi, 31);
    w.push_input(Input::SpawnBirds { count: 30 });
    w.push_input(Input::RandomRaid { count: 10 });
    for _ in 0..(120 * 300 / 60) {
        w.step(60);
        let q = w.kin()[QUEEN].p;
        let now = w.clock().tick();
        for t in w.sensors().tracks() {
            let truth = w.kin()[t.slot].p;
            assert!(t.est_v.norm() <= 120.0 + 1e-9, "{t:?}");
            // Recently fixed tracks: error within ~4 sigma of passive ranging
            // (10 % of range) plus margin. Stale (lost/ignored) tracks may drift.
            if now - t.last_hit_tick <= 3 * 120 {
                let bound = 0.4 * (truth - q).norm() + 100.0;
                let err = (t.est_p - truth).norm();
                assert!(err < bound, "error {err:.0} m > {bound:.0} m: {t:?}");
            }
        }
    }
}
