# ADR-0005: Repository layout and toolchain

**Status:** Accepted · **Implemented:** Phase 0

## Decision

```
crates/hawksgrip-core   pure Rust, no PyO3: all simulation logic, cargo test/bench alone
crates/hawksgrip-py     thin PyO3 cdylib → Python module hawksgrip._core
python/hawksgrip        Python package (maturin "mixed" layout, python-source = "python")
legacy/v0_1_pygame      frozen v0.1 baseline, unchanged code
docs/adr, docs/redteam, docs/thesis_defense_notes, docs/claims.md
```

- **Core without Python:** every algorithm is testable without an interpreter,
  and the binding layer is small enough to show on one slide.
- **abi3 wheel (Python ≥ 3.10):** one build works across interpreter versions.
- **Pinned majors:** pyo3 0.29 and numpy 0.29 (they must match; PyO3 renamed
  `allow_threads` → `detach` in 0.26, so API churn is real).
- **Toolchain:** `rust-toolchain.toml` uses the stable channel; developed and
  verified on **rustc 1.99.0** with MSVC Build Tools 2022. An exact-version pin
  was tried but rustup could not download a second toolchain over this
  network; the golden-hash test (ADR-0007) detects any toolchain change that
  would alter results.
- **Build parallelism:** the existing `.cargo/config.toml` (`jobs = 6`) is kept,
  so the machine stays usable during builds.

## Legacy baseline

The local folder was not a git clone of
`github.com/kaiseer1/SQ-7-hawksgrip`, and its `main` branch has no commits. The
v0.1 tree (`origin/main`, commit 1c68cd7) was extracted with `git archive`
into `legacy/v0_1_pygame/`. **The code is preserved, but its commit history is
not part of the local branch.** Rebasing local `main` onto `origin/main` was
blocked as a destructive local operation. Joining histories
(`git merge --allow-unrelated-histories origin/main`) is left to the author.
