import numpy as np
import pytest

from hawksgrip import Session
from hawksgrip.config import config_json


def sabkha(seed=1, **over):
    cfg = {"terrain": {"sector": "eastern_sabkha"}} | over
    return Session(config_json(cfg), seed)


def test_sensor_info_and_band_physics():
    s = sabkha()
    info = s.sensor_info()
    assert info["band"] == "VIS" and info["n_eyes"] == 24
    r = info["detection_range_m"]
    # VIS: 2 m target, 2 px at 0.109 mrad -> ~9.2 km geometric limit.
    assert 8_000 < r["VIS"] < 9_200
    assert r["SWIR"] < r["VIS"]


def test_sandstorm_key_switches_to_thermal():
    s = sabkha()
    s.set_visibility(600.0)
    s.step(1)
    kinds = [e[1] for e in s.drain_events()]
    assert "VISIBILITY_CHANGED" in kinds and "BAND_SWITCH" in kinds
    assert s.sensor_info()["band"] == "LWIR"
    s.set_visibility(5.0)  # outside 10..100000 m: rejected
    s.step(1)
    assert [e[1] for e in s.drain_events()] == ["ORDER_REJECTED"]


def test_eyes_tracks_and_timelines_round_trip():
    s = sabkha(seed=3)
    s.spawn_threat((-6500.0, 1000.0, 150.0), (0.0, 0.0, 30.0), 40.0)
    s.spawn_birds(5)
    s.step(120 * 45)
    e = s.eyes()
    assert e["dirs"].shape == (24, 3)
    np.testing.assert_allclose(np.linalg.norm(e["dirs"], axis=1), 1.0, atol=1e-9)
    assert set(np.unique(e["task"])) <= {0, 1, 2}
    tl = [t for t in s.timelines() if t["class"] == "drone"]
    assert len(tl) == 1 and tl[0]["detected"] is not None
    assert all(set(t) >= {"id", "slot", "state", "est_p"} for t in s.tracks())
    c = s.sensor_counters()
    assert c["detections"] > 0


def test_cpu_coverage_flags():
    s = sabkha()
    q = s.positions()[0]
    d = s.eyes()["dirs"]
    # A point straight along eye 0's boresight at 2 km: in view and detectable.
    vox = np.array([q + 2000.0 * d[0], q + 30_000.0 * d[0], q - 2000.0 * d[0]])
    flags = s.coverage_cpu(vox)
    assert flags[0] == 3  # in view + detectable
    assert flags[1] & 1 and not flags[1] & 2  # in view, too far
    s.set_coverage_voxels(vox)
    np.testing.assert_array_equal(s.coverage_resident(), flags)


torch = pytest.importorskip("torch")
needs_cuda = pytest.mark.skipif(not torch.cuda.is_available(), reason="no CUDA device")


def airspace(n_xy=120, n_z=6):
    xs = np.linspace(-7400, 7400, n_xy)
    zs = np.linspace(30, 1200, n_z)
    g = np.stack(np.meshgrid(xs, xs, zs, indexing="ij"), axis=-1).reshape(-1, 3)
    return g


@needs_cuda
@pytest.mark.parametrize("vis", [20_000.0, 700.0])
def test_gpu_coverage_agrees_with_rust_reference(vis):
    from hawksgrip.gpu.coverage import CoverageGPU

    s = sabkha(seed=4)
    if vis < 20_000:
        s.set_visibility(vis)
    s.random_raid(6)
    s.step(120 * 20)
    vox = airspace()
    cpu = s.coverage_cpu(vox)
    cov = CoverageGPU(s, vox)
    gpu = cov.frame().cpu().numpy()
    agree = (cpu == gpu).mean()
    assert agree > 0.999, f"agreement {agree:.5f}"
    assert (cpu & 1).sum() > 0 and (cpu & 2).sum() > 0
    cov.release()


@needs_cuda
def test_gpu_revisit_accumulator():
    from hawksgrip.gpu.coverage import CoverageGPU

    s = sabkha(seed=5)
    vox = airspace(60, 3)
    cov = CoverageGPU(s, vox)
    for k in range(150):  # 5 s of camera frames
        s.step(4)
        cov.accumulate(cov.frame(), s.time)
    gap, never = cov.revisit(s.time)
    # Only voxels within detection range can ever count as seen.
    detectable = torch.as_tensor(s.coverage_cpu(vox) & 2 > 0, device=gap.device)
    seen_frac = 1.0 - never[detectable].float().mean().item()
    assert seen_frac > 0.97, f"{seen_frac:.3f}"
    assert torch.isfinite(gap[~never]).all()
    cov.release()
