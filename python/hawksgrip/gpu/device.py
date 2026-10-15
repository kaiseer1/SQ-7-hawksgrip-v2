"""CUDA device report and the VRAM budget guard.

The RTX 3070 has 8 GB, shared with the desktop compositor and the 3D
showroom. The simulation's own GPU tensors (terrain, frustum ray grids,
Monte Carlo batches) are registered with a ``VramBudget`` that refuses
any allocation pushing past the ceiling. A VRAM spike then fails as a
clear error at the allocation site, not as an out-of-memory crash
somewhere else.
"""

from __future__ import annotations

from dataclasses import dataclass, field

MIB = 1024 * 1024

# Sim-owned VRAM ceiling. The terrain needs ~9 MB (ADR-0004). 1 GiB leaves
# >6 GB for the renderer, the CUDA context (~300-500 MB) and the OS.
DEFAULT_CEILING_BYTES = 1024 * MIB


def cuda_report() -> dict:
    """Describe the CUDA device, or say why there is none."""
    try:
        import torch
    except ImportError:
        return {"available": False, "reason": "torch not installed"}
    if not torch.cuda.is_available():
        return {"available": False, "reason": "CUDA not available", "torch": torch.__version__}
    idx = torch.cuda.current_device()
    props = torch.cuda.get_device_properties(idx)
    free, total = torch.cuda.mem_get_info(idx)
    return {
        "available": True,
        "torch": torch.__version__,
        "cuda": torch.version.cuda,
        "device": props.name,
        "compute_capability": f"{props.major}.{props.minor}",
        "sm_count": props.multi_processor_count,
        "vram_total_mib": total // MIB,
        "vram_free_mib": free // MIB,
    }


class VramBudgetExceeded(MemoryError):
    pass


@dataclass
class VramBudget:
    ceiling_bytes: int = DEFAULT_CEILING_BYTES
    _owned: dict[str, int] = field(default_factory=dict)

    @property
    def used_bytes(self) -> int:
        return sum(self._owned.values())

    def reserve(self, name: str, nbytes: int) -> None:
        """Account for an allocation before making it."""
        if nbytes < 0:
            raise ValueError("nbytes must be >= 0")
        prev = self._owned.get(name, 0)
        if self.used_bytes - prev + nbytes > self.ceiling_bytes:
            raise VramBudgetExceeded(
                f"{name}: {nbytes / MIB:.1f} MiB would exceed the {self.ceiling_bytes / MIB:.0f} MiB sim budget "
                f"({self.used_bytes / MIB:.1f} MiB in use)"
            )
        self._owned[name] = nbytes

    def register(self, name: str, tensor) -> None:
        """Account for an existing tensor (element size x element count)."""
        self.reserve(name, tensor.element_size() * tensor.nelement())

    def release(self, name: str) -> None:
        self._owned.pop(name, None)

    def report(self) -> dict[str, float]:
        return {k: v / MIB for k, v in sorted(self._owned.items())} | {"_total_mib": self.used_bytes / MIB}
