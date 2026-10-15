//! Persona invariance (ADR-0000): Kindah's voice never changes what she does.

use hawksgrip_core::persona::{Ctx, Mode, RoyalPersonaEngine};
use hawksgrip_core::sim::world::QUEEN;
use hawksgrip_core::terrain::Sector;
use hawksgrip_core::{Input, SimConfig, World};

fn run(toggle_every: Option<u64>) -> (u64, Vec<(u64, u64)>, usize) {
    let mut c = SimConfig::default();
    c.terrain.sector = Sector::HighlandWadi;
    let mut w = World::new(c, 9).unwrap();
    let mut persona = RoyalPersonaEngine::new(9, 120, Mode::Theatrical);
    w.push_input(Input::RandomRaid { count: 12 });
    w.push_input(Input::SpawnBirds { count: 8 });
    let mut spoken = 0;
    for t in 0..(120 * 120) {
        w.step(1);
        if let Some(k) = toggle_every {
            if t % k == 0 {
                persona.toggle();
            }
        }
        if t == 120 * 40 {
            w.push_input(Input::SetVisibility {
                visibility_m: 700.0,
            });
        }
        let ctx = Ctx {
            queen: w.kin()[QUEEN].p,
            band: w.sensors().band().name(),
            visibility_m: w.sensors().optics().visibility_m,
        };
        for e in w.drain_events() {
            spoken += persona.on_event(&e, &ctx).is_some() as usize;
        }
        spoken += persona.tick(t, &ctx, 24, &[0.0; 7]).len();
    }
    (w.state_hash(), w.checkpoints().to_vec(), spoken)
}

#[test]
fn switching_voice_never_changes_the_simulation() {
    let (h_theatrical, cp_a, spoken_a) = run(None);
    let (h_toggling, cp_b, spoken_b) = run(Some(97));
    assert_eq!(h_theatrical, h_toggling);
    assert_eq!(cp_a, cp_b, "every per-second checkpoint must match");
    assert!(
        spoken_a > 50 && spoken_b > 50,
        "the persona did speak: {spoken_a} / {spoken_b}"
    );
}
