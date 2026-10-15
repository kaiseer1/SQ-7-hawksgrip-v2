use hawksgrip_core::sim::world::{FIRST_SEVEN, FIRST_THREAT, QUEEN};
use hawksgrip_core::telemetry::EventKind;
use hawksgrip_core::{Input, SEVEN, SimConfig, Vec3, World};

fn world() -> World {
    World::new(SimConfig::default(), 0).unwrap()
}

#[test]
fn queen_holds_orbit_exactly() {
    let mut w = world();
    w.push_input(Input::RandomRaid { count: 20 });
    w.step(120 * 300);
    let c = w.config().clone();
    let q = w.kin()[QUEEN];
    let r = q.p.horizontal().norm();
    assert!((r - c.queen.orbit_radius_m).abs() < 1e-6);
    assert!((q.v.norm() - c.queen.airspeed_mps).abs() < 1e-9);
    assert_eq!(q.p.z, c.queen.orbit_alt_agl_m);
    assert_eq!(w.counters().queen_orbit_deviation_max_m, 0.0);
}

#[test]
fn seven_keep_station_under_the_queen() {
    let mut w = world();
    w.step(120 * 120);
    let c = w.config().seven.clone();
    let q = w.kin()[QUEEN];
    for k in 0..SEVEN {
        let slot = hawksgrip_core::kinematics::guidance::station_slot(
            &q,
            k,
            SEVEN,
            c.station_radius_m,
            c.station_below_m,
        );
        let err = (w.kin()[FIRST_SEVEN + k].p - slot.p).norm();
        assert!(err < 0.5, "node {} station error {err} m", k + 1);
    }
}

#[test]
fn undefended_raid_breaches_the_refinery() {
    // Phase 1 has no interception yet: every threat must reach the refinery.
    // This proves the breach counter is wired correctly.
    let mut w = world();
    w.push_input(Input::RandomRaid { count: 10 });
    w.step(120 * 400);
    let c = w.counters();
    assert_eq!(c.threats_spawned, 10);
    assert_eq!(c.refinery_breaches, 10);
    let ev = w.drain_events();
    assert_eq!(
        ev.iter()
            .filter(|e| e.kind == EventKind::RefineryBreach)
            .count(),
        10
    );
    assert!((FIRST_THREAT..w.n_entities()).all(|i| !w.is_active(i)));
}

#[test]
fn spawns_beyond_capacity_are_rejected_and_counted() {
    let mut w = world();
    let cap = w.config().threat.capacity as u32;
    w.push_input(Input::RandomRaid { count: cap + 5 });
    w.step(1);
    assert_eq!(w.counters().threats_spawned, cap as u64);
    assert_eq!(w.counters().spawn_rejected, 5);
}

#[test]
fn waypoint_order_is_flown_and_reported() {
    let mut w = world();
    let target = Vec3::new(1000.0, 1000.0, 800.0);
    w.push_input(Input::SetWaypoint {
        entity: 2,
        target,
        speed: 45.0,
    });
    w.step(120 * 180);
    assert!((w.kin()[2].p - target).norm() < 5.0);
    assert!(w.kin()[2].v.norm() < 1.0);
    let ev = w.drain_events();
    assert!(
        ev.iter()
            .any(|e| e.kind == EventKind::OrderAccepted && e.entity == 2)
    );
    assert!(
        ev.iter()
            .any(|e| e.kind == EventKind::WaypointReached && e.entity == 2)
    );
}

#[test]
fn invalid_orders_are_rejected_not_applied() {
    let mut w = world();
    w.push_input(Input::SetWaypoint {
        entity: QUEEN,
        target: Vec3::ZERO,
        speed: 10.0,
    });
    w.push_input(Input::SetWaypoint {
        entity: 99_999,
        target: Vec3::ZERO,
        speed: 10.0,
    });
    w.push_input(Input::SetWaypoint {
        entity: 1,
        target: Vec3::new(f64::NAN, 0.0, 0.0),
        speed: 10.0,
    });
    w.step(1);
    let ev = w.drain_events();
    assert_eq!(
        ev.iter()
            .filter(|e| e.kind == EventKind::OrderRejected)
            .count(),
        3
    );
}

#[test]
fn ground_impact_is_detected() {
    let mut w = world();
    w.push_input(Input::SpawnThreat {
        pos: Vec3::new(-5000.0, 0.0, 60.0),
        target: Vec3::new(-3000.0, 0.0, -500.0),
        speed: 40.0,
    });
    w.step(120 * 30);
    assert_eq!(w.counters().ground_impacts, 1);
}

#[test]
fn terrain_only_settable_before_first_tick() {
    let mut w = world();
    let hf =
        hawksgrip_core::terrain::Heightfield::new(2, 2, 15_000.0, -7500.0, -7500.0, vec![100.0; 4])
            .unwrap();
    w.set_terrain(hf.clone()).unwrap();
    // Orbit altitude follows the ground under the refinery.
    assert_eq!(w.kin()[QUEEN].p.z, 100.0 + w.config().queen.orbit_alt_agl_m);
    w.step(1);
    assert!(w.set_terrain(hf).is_err());
}
