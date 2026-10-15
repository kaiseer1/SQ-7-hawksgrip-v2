import pytest

from hawksgrip.gpu import VramBudget, VramBudgetExceeded, cuda_report

MIB = 1024 * 1024


def test_budget_accounting():
    b = VramBudget(ceiling_bytes=100 * MIB)
    b.reserve("terrain", 9 * MIB)
    b.reserve("rays", 80 * MIB)
    with pytest.raises(VramBudgetExceeded, match="exceed"):
        b.reserve("montecarlo", 20 * MIB)
    b.reserve("rays", 60 * MIB)  # resizing replaces, not adds
    b.reserve("montecarlo", 20 * MIB)
    assert b.used_bytes == 89 * MIB
    b.release("rays")
    assert b.report()["_total_mib"] == pytest.approx(29.0)


def test_cuda_report_shape():
    r = cuda_report()
    assert "available" in r
    if r["available"]:
        assert r["vram_total_mib"] > 0


def test_terrain_tensor_fits_budget_on_gpu():
    torch = pytest.importorskip("torch")
    if not torch.cuda.is_available():
        pytest.skip("no CUDA device")
    b = VramBudget()
    z = torch.zeros((1501, 1501), dtype=torch.float32, device="cuda")
    b.register("terrain", z)
    assert b.used_bytes < 10 * MIB  # ADR-0004: about 9 MB
