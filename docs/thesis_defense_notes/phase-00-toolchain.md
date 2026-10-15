# Viva Defence Cheat-Sheet: Phase 0 (toolchain and hybrid bridge)

`[AGENT: THESIS DEFENSE COACH & EXPLAINER]`

## 1. Plain-English pitch (for a general or an executive)

The simulator has two halves. A fast, strict Rust "engine room" calculates
what every aircraft does 120 times a second, and a flexible Python "bridge"
draws the screen, uses the graphics card and runs the experiments. Phase 0
built the connection between them and measured that crossing it costs about
**30 billionths of a second**, which is effectively free.

## 2. Technical core

- **Architecture:** Cargo workspace with two crates.
  - `hawksgrip-core`: pure Rust, all simulation logic, tested without Python.
  - `hawksgrip-py`: a ~340-line PyO3 layer that only converts arguments.
- **Build:** `maturin` produces one *abi3* wheel (`hawksgrip._core`) usable
  from Python 3.10+. Python code lives in `python/hawksgrip/` (maturin "mixed" layout).
- **Why Rust + Python, not one language:**
  - Rust gives deterministic, memory-safe, native-speed loops and fearless
    multithreading (`rayon`).
  - Python gives PyTorch/CUDA, plotting and fast HMI iteration.
  - PyO3 lets the Rust loop **release the GIL**, so both run concurrently.
- **Measured costs:**

  | | |
  |---|---|
  | Empty Python→Rust call | 31–33 ns |
  | One full tick, 64 threats | ≈ 4.1 µs |
  | Bridge share of a tick | **< 1 %** |

## 3. Committee Q&A prep

**Toughest question:** *"Why not write it all in Python with NumPy, or all in C++? Isn't two languages extra complexity for a thesis?"*

**Winning answer:**
- "I have a measured baseline for pure Python. My own v0.1 is in
  `legacy/v0_1_pygame`, and Phase 8 compares it directly.
- The hot loop has branchy per-entity logic: limits, rules, events. NumPy
  vectorises that poorly, and a Python loop costs microseconds per entity per tick.
- C++ was excluded by design. Rust gives the same native speed with memory
  safety enforced by the compiler, which matters when the loop releases the
  GIL and threads share data.
- The bridge adds 33 ns per call, under 1 % of a 4 µs tick. So the second
  language costs nothing measurable, and it buys me PyTorch for the GPU work."

**Follow-up they may try:** *"How do you know the GIL is really released?"*
→ "It's a test. A Python thread keeps counting while Rust simulates ten
minutes in one call; it completes over a thousand iterations during that call
(`tests/test_bridge.py::test_step_releases_the_gil`)."
