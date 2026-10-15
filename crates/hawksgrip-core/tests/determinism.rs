//! Determinism is the foundation of every thesis claim: same inputs, same
//! world, bit for bit.

use hawksgrip_core::sim::batch::run_batch;
use hawksgrip_core::{Input, Replay, SimConfig, Vec3, World};

const TICKS: u64 = 120 * 120; // two simulated minutes

fn raid_world(seed: u64, parallel: bool) -> World {
    let mut w = World::new(SimConfig::default(), seed).unwrap();
    w.set_parallel(parallel);
    w.push_input(Input::RandomRaid { count: 12 });
    w.step(600);
    w.push_input(Input::SetWaypoint {
        entity: 3,
        target: Vec3::new(500.0, -800.0, 900.0),
        speed: 40.0,
    });
    w.push_input(Input::SpawnThreat {
        pos: Vec3::new(-7000.0, 0.0, 120.0),
        target: Vec3::new(0.0, 0.0, 20.0),
        speed: 45.0,
    });
    w.step(TICKS - 600);
    w
}

#[test]
fn same_seed_is_bit_identical() {
    let a = raid_world(2026, false);
    let b = raid_world(2026, false);
    assert_eq!(a.state_hash(), b.state_hash());
    assert_eq!(a.checkpoints(), b.checkpoints());
    assert_eq!(a.counters(), b.counters());
}

#[test]
fn different_seed_differs() {
    assert_ne!(
        raid_world(1, false).state_hash(),
        raid_world(2, false).state_hash()
    );
}

#[test]
fn rayon_parallel_equals_sequential() {
    let a = raid_world(77, false);
    let b = raid_world(77, true);
    assert_eq!(a.checkpoints(), b.checkpoints());
}

#[test]
fn replay_reproduces_every_checkpoint() {
    let w = raid_world(9, false);
    let r = Replay::record(&w);
    assert_eq!(r.checkpoints.len() as u64, TICKS / 120);
    let back = Replay::from_json(&r.to_json()).unwrap();
    let rep = back.verify(None).unwrap();
    assert!(rep.ok, "{rep:?}");
    assert_eq!(rep.checkpoints_checked, r.checkpoints.len());
}

#[test]
fn tampered_replay_reports_divergence_tick() {
    let w = raid_world(9, false);
    let mut r = Replay::record(&w);
    // Nudge one input's speed: the run must diverge at or after tick 600.
    for (_, i) in r.inputs.iter_mut() {
        if let Input::SpawnThreat { speed, .. } = i {
            *speed -= 0.001;
        }
    }
    let rep = r.verify(None).unwrap();
    assert!(!rep.ok);
    let t = rep.first_divergence_tick.unwrap();
    assert!((600..=720).contains(&t), "diverged at {t}");
}

#[test]
fn batch_is_ordered_and_reproducible_across_thread_counts() {
    let cfg = SimConfig::default();
    let seeds: Vec<u64> = (100..124).collect();
    let one = run_batch(&cfg, &seeds, 1200, 8, Some(1)).unwrap();
    let many = run_batch(&cfg, &seeds, 1200, 8, Some(12)).unwrap();
    assert_eq!(one, many);
    assert_eq!(one.iter().map(|r| r.seed).collect::<Vec<_>>(), seeds);
}

/// Golden fingerprint of a fixed run. It must match on every OS and toolchain
/// (Windows MSVC and Linux CI alike). If a deliberate behaviour change moves
/// it, update the constant in the same commit and say why in the message.
/// History:
/// - Phase 1: 0x02CB_1F3F_57C3_C200
/// - Phase 2: threats gained a 40 m terrain-following floor (released inside
///   the 1.5 km dive radius), which changes their flat-ground descent
///   (0x765F_2CBC_1EEE_0E1B); then the look-ahead was staggered to 30 Hz
///   (Red-Team P2-13) and its cache added to the state hash
///   (0xE783_7723_6F56_C5FC).
/// - Phase 3: the 24-eye sensor state (eye pointing, tracks, Sensor RNG) and
///   object classes joined the state hash. Kinematics unchanged.
const GOLDEN_RAID_HASH: u64 = 0xC1DD_EB95_0230_29B1;

#[test]
fn golden_fingerprint_is_stable() {
    let w = raid_world(2026, false);
    let h = w.state_hash();
    assert_eq!(h, GOLDEN_RAID_HASH, "got 0x{h:016X}");
}
