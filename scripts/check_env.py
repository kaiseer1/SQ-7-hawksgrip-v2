"""Phase 0 environment check: is this machine ready to build and run Hawksgrip?

    .venv/Scripts/python scripts/check_env.py
"""

from __future__ import annotations

import ctypes
import os
import platform
import shutil
import subprocess
import sys


def total_ram_gib() -> float | None:
    if sys.platform != "win32":
        try:
            return os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_PHYS_PAGES") / 2**30
        except (ValueError, OSError):
            return None

    class MemStatus(ctypes.Structure):
        _fields_ = [
            ("dwLength", ctypes.c_ulong),
            ("dwMemoryLoad", ctypes.c_ulong),
            ("ullTotalPhys", ctypes.c_ulonglong),
            ("ullAvailPhys", ctypes.c_ulonglong),
            ("ullTotalPageFile", ctypes.c_ulonglong),
            ("ullAvailPageFile", ctypes.c_ulonglong),
            ("ullTotalVirtual", ctypes.c_ulonglong),
            ("ullAvailVirtual", ctypes.c_ulonglong),
            ("ullAvailExtendedVirtual", ctypes.c_ulonglong),
        ]

    m = MemStatus()
    m.dwLength = ctypes.sizeof(MemStatus)
    ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(m))
    return m.ullTotalPhys / 2**30


def tool_version(cmd: list[str]) -> str:
    exe = shutil.which(cmd[0]) or shutil.which(cmd[0], path=os.path.expanduser("~/.cargo/bin"))
    if not exe:
        return "MISSING"
    try:
        return subprocess.run([exe, *cmd[1:]], capture_output=True, text=True, timeout=30).stdout.strip()
    except (OSError, subprocess.SubprocessError) as e:
        return f"error: {e}"


def main() -> int:
    ok = True
    print(f"python      {sys.version.split()[0]} ({platform.platform()})")
    print(f"cpu         {platform.processor()} | logical threads: {os.cpu_count()}")
    ram = total_ram_gib()
    print(f"ram         {ram:.1f} GiB" if ram else "ram         unknown")
    print(f"rustc       {tool_version(['rustc', '--version'])}")
    print(f"cargo       {tool_version(['cargo', '--version'])}")

    try:
        import numpy

        print(f"numpy       {numpy.__version__}")
    except ImportError:
        print("numpy       MISSING")
        ok = False

    try:
        import hawksgrip

        print(f"hawksgrip   {hawksgrip.__version__} (core {hawksgrip.core_version})")
    except ImportError as e:
        print(f"hawksgrip   NOT BUILT ({e}); run: maturin develop --release")
        ok = False

    from hawksgrip.gpu import cuda_report

    r = cuda_report()
    if r["available"]:
        print(
            f"cuda        {r['device']} | CC {r['compute_capability']} | {r['sm_count']} SMs | "
            f"VRAM {r['vram_free_mib']}/{r['vram_total_mib']} MiB free | torch {r['torch']} (CUDA {r['cuda']})"
        )
    else:
        print(f"cuda        unavailable: {r['reason']}")
        ok = False

    print("READY" if ok else "NOT READY")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
