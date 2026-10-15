//! Replay files: (config, seed, terrain fingerprint, input log) plus state-hash
//! checkpoints. Re-running the inputs must reproduce every checkpoint bit for
//! bit; the first mismatching checkpoint pinpoints where two runs diverged.

use crate::config::SimConfig;
use crate::sim::input::Input;
use crate::sim::world::World;
use crate::terrain::Heightfield;
use serde::{Deserialize, Serialize};

pub const REPLAY_FORMAT: u32 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub format: u32,
    pub core_version: String,
    pub seed: u64,
    pub terrain_fingerprint: u64,
    pub config: SimConfig,
    pub ticks: u64,
    pub inputs: Vec<(u64, Input)>,
    pub checkpoints: Vec<(u64, u64)>,
    pub final_hash: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VerifyReport {
    pub ok: bool,
    pub checkpoints_checked: usize,
    /// Tick of the first checkpoint whose hash differs.
    pub first_divergence_tick: Option<u64>,
    pub expected_final: u64,
    pub got_final: u64,
}

impl Replay {
    pub fn record(w: &World) -> Self {
        Replay {
            format: REPLAY_FORMAT,
            core_version: crate::VERSION.to_string(),
            seed: w.seed(),
            terrain_fingerprint: w.terrain().fingerprint(),
            config: w.config().clone(),
            ticks: w.clock().tick(),
            inputs: w.input_log().to_vec(),
            checkpoints: w.checkpoints().to_vec(),
            final_hash: w.state_hash(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("replay serialises")
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        let r: Replay = serde_json::from_str(s).map_err(|e| format!("replay: {e}"))?;
        if r.format != REPLAY_FORMAT {
            return Err(format!(
                "replay format {} unsupported (expected {REPLAY_FORMAT})",
                r.format
            ));
        }
        Ok(r)
    }

    /// Re-run the replay and compare every checkpoint.
    pub fn verify(&self, terrain: Option<Heightfield>) -> Result<VerifyReport, String> {
        let mut w = World::new(self.config.clone(), self.seed)?;
        if let Some(hf) = terrain {
            w.set_terrain(hf)?;
        }
        if w.terrain().fingerprint() != self.terrain_fingerprint {
            return Err("replay was recorded on different terrain".into());
        }
        let mut inputs = self.inputs.iter().peekable();
        for tick in 0..self.ticks {
            while let Some((t, input)) = inputs.peek() {
                if *t != tick {
                    break;
                }
                w.push_input(input.clone());
                inputs.next();
            }
            w.step(1);
        }
        let got = w.checkpoints();
        let mut first = None;
        for (exp, g) in self.checkpoints.iter().zip(got) {
            if exp != g {
                first = Some(exp.0);
                break;
            }
        }
        if first.is_none() && got.len() != self.checkpoints.len() {
            first =
                Some(got.len().min(self.checkpoints.len()) as u64 * self.config.checkpoint_every);
        }
        let got_final = w.state_hash();
        Ok(VerifyReport {
            ok: first.is_none() && got_final == self.final_hash,
            checkpoints_checked: self.checkpoints.len().min(got.len()),
            first_divergence_tick: first,
            expected_final: self.final_hash,
            got_final,
        })
    }
}
