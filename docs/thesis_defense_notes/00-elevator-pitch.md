# Elevator pitch (30 seconds)

`[AGENT: THESIS DEFENSE COACH & EXPLAINER]`

> Cheap drones cost thousands to build and millions to shoot down, and the
> 2019 Abqaiq attack showed that this cost gap is a strategic vulnerability.
> **SQ-7 Hawksgrip** is a simulation framework for a different answer: one
> airborne command node with a 24-eye optical array commands seven reusable,
> all-electric interceptors through a cryptographically signed auction. It
> runs in real time on a single gaming PC, with a deterministic engine in Rust
> and GPU-accelerated sensing in Python. My thesis measures what matters:
> decision latency, sensor coverage, survival probability with confidence
> bounds, and the economic exchange ratio, where an intercept costs
> electricity, not missiles.

## The 10-second version

*"A Rust-and-GPU simulation that tests whether one flying command node and
seven electric interceptors can defend a refinery more cheaply than missiles,
and proves every number it shows."*

## Three phrases to keep using

1. **"Deterministic and replayable":** every result can be re-run bit for bit.
2. **"Measured, not claimed":** every number on screen comes from a named benchmark or test.
3. **"Simulation of a defensive concept":** synthetic refinery, generic threats, geometric intercepts.

## Words to avoid in front of the committee

- "100 % / 0.00 % / guaranteed": say "0 of 10 000 trials; 95 % upper bound 0.03 %".
- "10 000× faster than a human": say "decision computed in N µs; physical kill chain is dominated by sensor frames and flight time".
- "Zero-copy" as a speed claim: at this scale it isn't one (ADR-0006).
