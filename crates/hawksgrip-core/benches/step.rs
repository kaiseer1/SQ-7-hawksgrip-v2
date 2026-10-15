use criterion::{Criterion, criterion_group, criterion_main};
use hawksgrip_core::sim::batch::run_batch;
use hawksgrip_core::{Input, SimConfig, World};
use std::hint::black_box;

fn loaded_world(parallel: bool) -> World {
    let mut w = World::new(SimConfig::default(), 1).unwrap();
    w.set_parallel(parallel);
    w.push_input(Input::RandomRaid { count: 64 });
    w.step(1);
    w
}

fn bench_tick(c: &mut Criterion) {
    let mut g = c.benchmark_group("tick_64_threats");
    for (name, par) in [("sequential", false), ("rayon", true)] {
        let mut w = loaded_world(par);
        g.bench_function(name, |b| b.iter(|| black_box(&mut w).step(1)));
    }
    g.finish();
}

fn bench_batch(c: &mut Criterion) {
    let cfg = SimConfig::default();
    let seeds: Vec<u64> = (0..48).collect();
    let mut g = c.benchmark_group("batch_48_runs_10s");
    g.sample_size(10);
    for threads in [1usize, 6, 12] {
        g.bench_function(format!("{threads}_threads"), |b| {
            b.iter(|| run_batch(&cfg, &seeds, 1200, 32, Some(threads)).unwrap())
        });
    }
    g.finish();
}

criterion_group!(benches, bench_tick, bench_batch);
criterion_main!(benches);
