# Phase 0 Red-Team attack log: toolchain and skeleton

`[AGENT: ADVERSARIAL BREAKER (RED-TEAM)]`: *"A thesis that cannot be rebuilt from a clean
clone is an anecdote. Prove the bridge exists and costs what you say."*

| # | Attack | Evidence | Fix / outcome | Status |
|---|---|---|---|---|
| P0-1 | "Your local repo is not your GitHub repo." | Local `main` had **no commits** and no remote; the v0.1 code lived only on GitHub | Added `origin`; extracted `origin/main` (1c68cd7) into `legacy/v0_1_pygame/` with `git archive`. Rewriting local `main` onto GitHub history was blocked as destructive, so **code preserved, history not joined** | Open: author decides on `git merge --allow-unrelated-histories` |
| P0-2 | "No Rust compiler on the machine." | `cargo` / `rustc` absent | Installed the official rustup (stable, minimal profile + clippy, rustfmt); rustc **1.99.0**. MSVC Build Tools 2022 already present | Fixed |
| P0-3 | "Pinning an exact toolchain makes the build fragile." | Pinning `1.99.0` failed: rustup tried to re-download over a resetting connection | Stable channel + documented verified version; the golden-hash test (Phase 1) detects any toolchain change that alters results | Fixed |
| P0-4 | "PyO3 API churn will break you." | PyO3 renamed `allow_threads` → `detach` in 0.26; pyo3 and numpy crates must match | Pinned pyo3 0.29 + numpy 0.29 in one place (`[workspace.dependencies]`) | Fixed |
| P0-5 | "Will the wheel only work on this exact Python?" | | `abi3-py310` build: one wheel for Python 3.10+ | Fixed |
| P0-6 | "8 GB VRAM is not 8 GB for you." | `check_env.py`: **7 113 / 8 191 MiB free** at idle; the desktop uses ~1 GB | `VramBudget` guard with a 1 GiB sim ceiling; allocations beyond it fail at the call site (tested) | Fixed |
| P0-7 | "Bridge overhead will eat your 120 Hz budget." | `noop` call **31–33 ns**; `step(1)` with 64 threats ≈ 4.1 µs | Overhead < 1 % of a tick. Not a bottleneck | Measured |
| P0-8 | "Debug symbols and build junk will end up in git." | `maturin develop` drops `_core.pyd` and `_core.pdb` in the package dir | `.gitignore` covers `*.pyd` (via `*.py[cod]`) and now `*.pdb` | Fixed |

## Verdict

Phase 0 exit criterion met: `import hawksgrip; Session().step(1)` works, and
the bridge cost is measured and negligible. One item needs the author's decision (P0-1).
