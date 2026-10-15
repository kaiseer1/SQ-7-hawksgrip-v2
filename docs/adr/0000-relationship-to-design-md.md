# ADR-0000: Relationship to the pre-v2 `docs/design.md` concept

**Status:** Accepted · **Date:** 2026-10-14

## Context

The repository folder already held `docs/design.md` and `docs/study-guide.md`
from an earlier concept iteration. The v2 specification agreed on 2026-10-14
(Rust + Python, Kindah and her Seven, auction engine, all-electric,
three Saudi sectors) differs from that concept in several places. Both
documents are kept **unmodified**; this record says which ideas carry over.

## Decision

**Carried over (good ideas the v2 spec benefits from):**

| From design.md | Where it lands in v2 |
|---|---|
| Command from **tracks, not truth** (noisy measurements, filtered per track) | Phase 3 optics produce tracks; the auction (Phase 5) only sees tracks |
| **Persona invariance:** switching theatrical/clinical never changes behaviour, and a test proves it | Phase 7: test asserts identical state hashes in both modes |
| **Claims register:** every number has a public source or is marked SWEPT | Created as [`docs/claims.md`](../claims.md) in Phase 1 |
| Experiments with multiple seeds, bootstrap 95 % CIs, held-out terrain | Phase 8 evaluation protocol |
| "Why she flies": airborne node vs ground mast on the same terrain | Phase 8 ablation |
| Honest sensor limits (dust, glare, poor passive ranging) | Phase 3 (Shamal attenuation, SWIR switch) |
| Ground nest as recovery fallback | Phase 4: kept as the fallback when Royal Trophallaxis is unavailable |

**Superseded by the v2 specification:**

| design.md | v2 |
|---|---|
| PPO-learned Queen brain | Deterministic Sovereign Master–Worker Auction (a learned layer may be added later as an ablation, never in the authoritative loop) |
| Net + parachute capture, "Final Stoop" ram | Geometric **capture event** (interceptor within capture radius); reusable vs sacrificial is an economics label |
| Named real-world threat types | Parametric threat classes (speed, altitude profile, manoeuvre limits), all SWEPT |
| Copernicus GLO-30 tiles of real places | Three procedural Saudi sector generators; optional public-DEM loader with no real facility coordinates |
| Radar + compound eye | 24-camera compound optical array (radar out of scope for v2) |

## Consequences

- One coherent v2 design; useful earlier ideas are preserved with traceability.
- `docs/study-guide.md` remains valid background reading (its chapters on
  simulation, seeds, assignment and evaluation apply to v2 unchanged).
