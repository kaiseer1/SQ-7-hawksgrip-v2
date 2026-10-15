import json

import numpy as np
import pytest

from hawksgrip import Session
from hawksgrip.config import config_json
from hawksgrip.terrain import SECTORS, install_dem, load_dem, resample_bilinear

SECTOR_NAMES = ["eastern_sabkha", "red_sea_corridor", "highland_wadi"]


@pytest.fixture(scope="module")
def sessions():
    return {s: Session(config_json({"terrain": {"sector": s}}), 0) for s in SECTOR_NAMES}


def test_sector_list():
    assert SECTORS == ["flat", *SECTOR_NAMES]


@pytest.mark.parametrize("name", SECTOR_NAMES)
def test_terrain_export(sessions, name):
    s = sessions[name]
    h = s.terrain_heights()
    assert h.shape == (1501, 1501) and h.dtype == np.float32
    assert h.nbytes < 10 * 1024 * 1024  # ADR-0004
    info = s.terrain_info()
    assert info["sector"] == name
    assert info["nx"] == info["ny"] == 1501
    # Exported grid and Rust sampler agree at grid points.
    for iy, ix in [(0, 0), (750, 750), (1500, 1500), (300, 1200)]:
        x, y = -7500 + 10 * ix, -7500 + 10 * iy
        assert s.ground_height(x, y) == pytest.approx(float(h[iy, ix]), abs=1e-4)


def test_sector_character(sessions):
    sab, red, high = (sessions[n].terrain_info() for n in SECTOR_NAMES)
    assert sab["max_m"] < 40 and sab["coastline"]
    assert red["max_m"] > 1400 and red["coastline"] and len(red["wadis"]) == 4
    assert high["pad_z"] == pytest.approx(1350.0, abs=1.0) and len(high["wadis"]) == 5


def test_los_scalar_and_batch_agree(sessions):
    s = sessions["highland_wadi"]
    rng = np.random.default_rng(0)
    a = np.column_stack([rng.uniform(-3000, 3000, 500), rng.uniform(-3000, 3000, 500), np.full(500, 2900.0)])
    b = np.column_stack([rng.uniform(-7000, 7000, 500), rng.uniform(-7000, 7000, 500), np.zeros(500)])
    b[:, 2] = [s.ground_height(x, y) + 30 for x, y in b[:, :2]]
    batch = s.los_batch(a, b)
    scalar = np.array([s.line_of_sight(tuple(p), tuple(q)) for p, q in zip(a, b)])
    np.testing.assert_array_equal(batch, scalar)
    assert 0 < batch.sum() < len(batch)  # some hidden, some visible
    with pytest.raises(ValueError):
        s.los_batch(a, b[:10])


def test_refinery_queries(sessions):
    s = sessions["eastern_sabkha"]
    st = s.refinery_structures()
    kinds = {x["kind"] for x in st}
    assert kinds == {"storage_tank", "distillation_column", "process_unit", "flare_stack", "control_room"}
    pad = s.terrain_info()["pad_z"]
    tank = next(x for x in st if x["kind"] == "storage_tank")["solid"]
    above_tank = (tank["cx"], tank["cy"], tank["z1"] + 10.0)
    assert s.in_keepout(above_tank) and s.keepout_distance(above_tank) < 0
    assert not s.in_keepout((0.0, 0.0, pad + 600.0))
    assert not s.path_clear_of_keepout((-990.0, 0.0, pad + 30.0), (-300.0, 0.0, pad + 30.0))
    assert s.path_clear_of_keepout((-990.0, 0.0, pad + 300.0), (-300.0, 0.0, pad + 300.0))


def test_margin_scale_sweeps_keepout():
    p = (-850.0, -720.0 + 35.0 + 90.0, 10.0)
    narrow = Session(config_json({"terrain": {"sector": "eastern_sabkha"}}), 0)
    wide = Session(config_json({"terrain": {"sector": "eastern_sabkha"}, "refinery": {"keepout_margin_scale": 2.0}}), 0)
    assert narrow.keepout_distance(p) > 0 > wide.keepout_distance(p)


def test_orbit_clearance_is_enforced():
    with pytest.raises(ValueError, match="clearance"):
        Session(config_json({"terrain": {"sector": "highland_wadi"}, "queen": {"orbit_alt_agl_m": 400.0}}), 0)


def test_replay_regenerates_procedural_terrain(sessions):
    from hawksgrip import verify_replay

    s = Session(config_json({"terrain": {"sector": "red_sea_corridor", "seed": 5}}), 3)
    s.random_raid(8)
    s.step(120 * 30)
    rep = verify_replay(s.export_replay())
    assert rep["ok"]


# ---- DEM loader -------------------------------------------------------------


def test_resample_bilinear_reproduces_planes():
    yy, xx = np.mgrid[0:11, 0:21]
    plane = 3.0 * xx + 2.0 * yy
    out = resample_bilinear(plane, 41)
    gx = np.linspace(0, 20, 41)[None, :]
    gy = np.linspace(0, 10, 41)[:, None]
    np.testing.assert_allclose(out, 3.0 * gx + 2.0 * gy, atol=1e-9)


@pytest.mark.parametrize("ext", [".npy", ".tif"])
def test_dem_round_trip_and_install(tmp_path, ext):
    # A synthetic north-up raster: a 400 m hill in the north-east corner.
    n = 301
    yy, xx = np.mgrid[0:n, 0:n]
    raw = 100.0 + 400.0 * np.exp(-(((xx - 250) / 30.0) ** 2 + ((yy - 50) / 30.0) ** 2))  # row 50 = near north
    raw[0, 0] = -9999.0
    path = tmp_path / f"dem{ext}"
    if ext == ".npy":
        np.save(path, raw)
    else:
        import tifffile

        tifffile.imwrite(path, raw.astype(np.float32))
    h = load_dem(path, nodata=-9999.0)
    assert h.shape == (1501, 1501) and h.dtype == np.float32
    s = Session(config_json({"queen": {"orbit_alt_agl_m": 1500.0}}), 0)
    install_dem(s, h)
    assert s.terrain_info()["sector"] == "custom"
    # Hill ends up in the north-east (x > 0, y > 0) after the north-up flip.
    assert s.ground_height(5000.0, 5000.0) > s.ground_height(5000.0, -5000.0) + 200


def test_dem_rejects_nonsense(tmp_path):
    p = tmp_path / "bad.npy"
    np.save(p, np.full((10, 10), 20_000.0))
    with pytest.raises(ValueError, match="plausible"):
        load_dem(p)
    with pytest.raises(ValueError, match="unsupported"):
        load_dem(tmp_path / "x.png")


# ---- GPU terrain ------------------------------------------------------------

torch = pytest.importorskip("torch")
needs_cuda = pytest.mark.skipif(not torch.cuda.is_available(), reason="no CUDA device")


@needs_cuda
def test_gpu_terrain_fits_budget_and_matches_cpu_sampler(sessions):
    from hawksgrip.gpu.terrain import TerrainTensor

    s = sessions["highland_wadi"]
    tt = TerrainTensor(s)
    assert tt.nbytes < 10 * 1024 * 1024
    rng = np.random.default_rng(1)
    pts = rng.uniform(-7400, 7400, size=(2000, 2))
    g = tt.sample(torch.tensor(pts[:, 0], device="cuda"), torch.tensor(pts[:, 1], device="cuda")).cpu().numpy()
    c = np.array([s.ground_height(x, y) for x, y in pts])
    np.testing.assert_allclose(g, c, atol=2e-3)  # float32 on GPU vs float64 on CPU


@needs_cuda
@pytest.mark.parametrize("name", SECTOR_NAMES)
def test_gpu_dense_los_agrees_with_authoritative_cpu(sessions, name):
    from hawksgrip.gpu.terrain import TerrainTensor

    s = sessions[name]
    tt = TerrainTensor(s)
    rng = np.random.default_rng(2)
    n = 4000
    # Queen-like origins on the orbit, targets 20-200 m AGL outside the refinery.
    th = rng.uniform(0, 2 * np.pi, n)
    oc = s.orbit_info()["center"]
    a = np.column_stack([3000 * np.cos(th), 3000 * np.sin(th), np.full(n, oc[2])])
    r = rng.uniform(1800, 7000, n)
    phi = rng.uniform(0, 2 * np.pi, n)
    b = np.column_stack([r * np.cos(phi), r * np.sin(phi), np.zeros(n)])
    b[:, 2] = [s.ground_height(x, y) for x, y in b[:, :2]] + rng.uniform(20, 200, n)
    cpu = s.los_batch(a, b)
    gpu = tt.los(a, b).cpu().numpy()
    # The CPU is conservative (blocked => clearance < 1 m somewhere); the GPU
    # samples every 5 m. GPU "blocked" must imply CPU "blocked".
    assert not np.any(~gpu & cpu), "GPU blocked a ray the authoritative CPU sees"
    agreement = (gpu == cpu).mean()
    assert agreement > 0.99, f"{name}: agreement {agreement:.4f}"
    # The GPU port of the Lipschitz march is the same algorithm in float32:
    # it must agree with the CPU except at the edge of the conservative band.
    lip = tt.los_lipschitz(a, b).cpu().numpy()
    assert tt.unresolved == 0
    assert (lip == cpu).mean() > 0.999, f"{name}: Lipschitz agreement {(lip == cpu).mean():.5f}"
