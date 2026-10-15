import json

from hawksgrip import Session, run_batch, verify_replay


def _run(seed: int, parallel: bool = False) -> Session:
    s = Session(seed=seed)
    s.set_parallel(parallel)
    s.random_raid(10)
    s.step(500)
    s.spawn_threat((-6000.0, 2000.0, 150.0), (0.0, 0.0, 20.0), 42.0)
    s.set_waypoint(5, (-1500.0, 0.0, 700.0), 45.0)
    s.step(120 * 60)
    return s


def test_same_seed_same_hash():
    assert _run(11).state_hash() == _run(11).state_hash()


def test_seed_matters():
    assert _run(11).state_hash() != _run(12).state_hash()


def test_parallel_kinematics_bit_identical():
    assert _run(5, False).state_hash() == _run(5, True).state_hash()


def test_replay_verifies_and_detects_tampering():
    s = _run(21)
    replay = s.export_replay()
    rep = verify_replay(replay)
    assert rep["ok"] and rep["got_final"] == s.state_hash()

    doc = json.loads(replay)
    for _, inp in doc["inputs"]:
        if inp["type"] == "spawn_threat":
            inp["speed"] += 1e-9  # one part in ten billion
    bad = verify_replay(json.dumps(doc))
    assert not bad["ok"]
    assert bad["first_divergence_tick"] is not None


def test_batch_matches_single_runs_and_thread_count():
    seeds = list(range(40, 52))
    a = run_batch(seeds, 1200, 6, threads=1)
    b = run_batch(seeds, 1200, 6, threads=12)
    assert a == b
    assert [r[0] for r in a] == seeds
    s = Session(seed=40)
    s.random_raid(6)
    s.step(1200)
    assert a[0][1] == s.state_hash()
