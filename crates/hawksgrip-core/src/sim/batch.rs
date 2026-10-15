//! Headless Monte Carlo batches: many independent worlds, one per seed,
//! spread across CPU threads with rayon.
//!
//! This is where Phase-1 parallelism pays. One world has about 80 bodies, too
//! few to split across threads, but independent runs scale almost linearly.
//! Results come back in seed order, so a batch is as reproducible as one run.

use crate::config::SimConfig;
use crate::sim::input::Input;
use crate::sim::world::{World, build_terrain};
use crate::telemetry::Counters;
use crate::terrain::{Heightfield, SectorInfo};
use rayon::prelude::*;
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BatchResult {
    pub seed: u64,
    pub ticks: u64,
    pub final_hash: u64,
    pub counters: Counters,
}

pub fn run_one(cfg: &SimConfig, seed: u64, ticks: u64, raid: u32) -> Result<BatchResult, String> {
    let (hf, info) = build_terrain(cfg)?;
    run_on(cfg, seed, ticks, raid, hf, info)
}

fn run_on(
    cfg: &SimConfig,
    seed: u64,
    ticks: u64,
    raid: u32,
    hf: Arc<Heightfield>,
    info: Arc<SectorInfo>,
) -> Result<BatchResult, String> {
    let mut w = World::with_terrain(cfg.clone(), seed, hf, Some(info))?;
    if raid > 0 {
        w.push_input(Input::RandomRaid { count: raid });
    }
    w.step(ticks);
    Ok(BatchResult {
        seed,
        ticks,
        final_hash: w.state_hash(),
        counters: w.counters().clone(),
    })
}

/// Run every seed. `threads = None` uses rayon's global pool (all cores).
pub fn run_batch(
    cfg: &SimConfig,
    seeds: &[u64],
    ticks: u64,
    raid: u32,
    threads: Option<usize>,
) -> Result<Vec<BatchResult>, String> {
    cfg.validate()?;
    let (hf, info) = build_terrain(cfg)?; // once, shared by every run
    let job = || {
        seeds
            .par_iter()
            .map(|&s| run_on(cfg, s, ticks, raid, hf.clone(), info.clone()))
            .collect::<Result<Vec<_>, _>>()
    };
    match threads {
        None => job(),
        Some(n) => rayon::ThreadPoolBuilder::new()
            .num_threads(n.max(1))
            .build()
            .map_err(|e| e.to_string())?
            .install(job),
    }
}
