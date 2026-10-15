//! Phase 2 integration: sectors, orbit clearance, terrain following,
//! line-of-sight, refinery keep-out.

use hawksgrip_core::sim::batch::run_batch;
use hawksgrip_core::sim::world::{FIRST_THREAT, QUEEN, build_terrain};
use hawksgrip_core::terrain::{Sector, los};
use hawksgrip_core::{Input, SimConfig, Vec3, World};

const SECTORS: [Sector; 3] = [
    Sector::EasternSabkha,
    Sector::RedSeaCorridor,
    Sector::HighlandWadi,
];

fn cfg(sector: Sector) -> SimConfig {
    let mut c = SimConfig::default();
    c.terrain.sector = sector;
    c
}

/// Golden terrain fingerprints (terrain seed 1). Any platform or compiler
/// difference in the generators changes these.
const GOLDEN_TERRAIN: [(Sector, u64); 3] = [
    (Sector::EasternSabkha, 0x1AFC_2863_0B0F_8688),
    (Sector::RedSeaCorridor, 0x0984_4CFA_F236_4A65),
    (Sector::HighlandWadi, 0x47C3_9D39_3462_61BF),
];

#[test]
fn golden_terrain_fingerprints() {
    let mut report = String::new();
    let mut ok = true;
    for (s, want) in GOLDEN_TERRAIN {
        let (hf, _) = build_terrain(&cfg(s)).unwrap();
        let got = hf.fingerprint();
        report.push_str(&format!("{s:?}: 0x{got:016X}\n"));
        ok &= got == want;
    }
    assert!(ok, "terrain fingerprints changed:\n{report}");
}

#[test]
fn every_sector_builds_a_world_with_orbit_clearance() {
    for s in SECTORS {
        let w = World::new(cfg(s), 0).unwrap();
        let q = w.kin()[QUEEN];
        assert!(q.p.z - w.terrain().height(q.p.x, q.p.y) > 300.0, "{s:?}");
        // Heights are stored as f32, the generator's pad level is f64.
        assert!((w.refinery().center.z - w.sector_info().unwrap().pad_z).abs() < 1e-3);
    }
}

#[test]
fn low_orbit_over_highlands_is_rejected() {
    let mut c = cfg(Sector::HighlandWadi);
    c.queen.orbit_alt_agl_m = 400.0;
    let e = World::new(c, 0).unwrap_err();
    assert!(e.contains("clearance"), "{e}");
}

#[test]
fn raids_fly_over_terrain_without_crashing() {
    // 32 threats per sector from every direction, terrain following on:
    // none may hit the ground, and all must reach the refinery (no defence yet).
    for s in SECTORS {
        let r = run_batch(&cfg(s), &[1, 2, 3], 120 * 420, 32, None).unwrap();
        for b in &r {
            assert_eq!(b.counters.ground_impacts, 0, "{s:?} seed {}", b.seed);
            assert_eq!(b.counters.refinery_breaches, 32, "{s:?} seed {}", b.seed);
        }
    }
}

#[test]
fn threats_hold_their_floor_en_route() {
    let mut w = World::new(cfg(Sector::RedSeaCorridor), 4).unwrap();
    w.push_input(Input::RandomRaid { count: 32 });
    let agl = w.config().threat.terrain_follow_agl_m;
    let dive = w.config().threat.dive_radius_m;
    let mut worst: f64 = f64::INFINITY;
    for _ in 0..(120 * 300) {
        w.step(1);
        for i in FIRST_THREAT..w.n_entities() {
            if !w.is_active(i) {
                continue;
            }
            let p = w.kin()[i].p;
            if p.horizontal().norm() > dive + 1_200.0 {
                worst = worst.min(p.z - w.terrain().height(p.x, p.y));
            }
        }
    }
    // Climbs are acceleration-limited, so allow a bounded dip below the floor.
    assert!(worst > 0.25 * agl, "lowest en-route AGL {worst:.1} m");
}

#[test]
fn ridge_masks_a_wadi_floor_from_the_queen() {
    // In the highlands, a point deep in a wadi far from the basin should be
    // hidden from at least part of Kindah's orbit, while the same point
    // raised 1 km is visible from everywhere on it.
    let w = World::new(cfg(Sector::HighlandWadi), 0).unwrap();
    let wadi = &w.sector_info().unwrap().wadis[0];
    let line = wadi.centreline(50.0);
    let p = line[line.len() / 4];
    let floor = Vec3::new(p[0], p[1], w.terrain().height(p[0], p[1]) + 20.0);
    let o = w.orbit();
    let views = |target: Vec3| {
        (0..72)
            .filter(|k| {
                let th = std::f64::consts::TAU * *k as f64 / 72.0;
                let q = o.center + Vec3::new(o.radius * th.cos(), o.radius * th.sin(), 0.0);
                w.line_of_sight(q, target)
            })
            .count()
    };
    let low = views(floor);
    let high = views(floor + Vec3::new(0.0, 0.0, 1_000.0));
    assert!(low < 72, "wadi floor visible from the whole orbit");
    assert_eq!(high, 72);
}

#[test]
fn los_agrees_with_dense_reference_on_real_sectors() {
    let mut rng = hawksgrip_core::sim::Rng::from_seed(77);
    for s in SECTORS {
        let (hf, _) = build_terrain(&cfg(s)).unwrap();
        let mut disagree = 0;
        for _ in 0..400 {
            let a = Vec3::new(
                rng.uniform(-3000.0, 3000.0),
                rng.uniform(-3000.0, 3000.0),
                0.0,
            );
            let b = Vec3::new(
                rng.uniform(-6000.0, 6000.0),
                rng.uniform(-6000.0, 6000.0),
                0.0,
            );
            let a = a + Vec3::new(0.0, 0.0, hf.height(a.x, a.y) + rng.uniform(20.0, 1500.0));
            let b = b + Vec3::new(0.0, 0.0, hf.height(b.x, b.y) + rng.uniform(10.0, 300.0));
            let fast = los::visible(&hf, a, b, 0.5);
            let (slow, min_c) = los::visible_reference(&hf, a, b, 0.5, 1.0);
            if fast {
                assert!(
                    slow || min_c > 0.0,
                    "{s:?}: march saw through terrain (min clearance {min_c})"
                );
            }
            if fast != slow {
                disagree += 1;
            }
        }
        assert!(disagree <= 4, "{s:?}: {disagree}/400 disagreements");
    }
}

#[test]
fn keepout_and_structure_occlusion_in_world() {
    let w = World::new(cfg(Sector::EasternSabkha), 0).unwrap();
    let r = w.refinery();
    let pad = r.center.z;
    assert!(r.in_keepout(Vec3::new(-850.0, -720.0, pad + 40.0)));
    assert!(!r.in_keepout(Vec3::new(0.0, 0.0, pad + 500.0)));
    // A sightline skimming 40 m over the pad through the columns is blocked.
    assert!(!w.line_of_sight(
        Vec3::new(150.0, -600.0, pad + 40.0),
        Vec3::new(150.0, 600.0, pad + 40.0)
    ));
}
