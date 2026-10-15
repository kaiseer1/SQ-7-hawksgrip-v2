"""GPU side (PyTorch/CUDA). Torch is imported lazily, so the core simulation
and headless batches run without it."""

from hawksgrip.gpu.device import VramBudget, VramBudgetExceeded, cuda_report

__all__ = ["VramBudget", "VramBudgetExceeded", "cuda_report"]
