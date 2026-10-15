# SQ-7 HAWKSGRIP — قبضة الصقر
### Airborne Counter-UAS Weapon System — design document

> **SQ-7 = Sky Queen and her Seven.** One Queen, seven hawks: a brood.
> *Seven children. One mother. Nothing passes.*

| Designation | What it is |
|---|---|
| **SQ-7 HAWKSGRIP** | The whole system (Arabic: قبضة الصقر, "the Falcon's Grip") |
| **EQ-7 "the Queen"** | Airborne flying-wing carrier and command node (E = electronic/sensing, Q = unmanned) |
| **FQ-7 #1–#7** | Her seven hawks: reusable interceptors (F = fighter, Q = unmanned) |
| **Nest-7** | Ground nest where hawks land, recharge and re-arm |

Her personal name is a single setting (`persona.queen_name`). The user is choosing it from the pre-Islamic Arabian queens who defied empires.

---

## 1. The Queen (EQ-7)

**Body.** An original flying-wing silhouette with three dorsal humps: two engine intakes and the central dome.

**Radar (long range, forward only).** A conformal array along the lower leading edge.
- Two faces, angled with the wing sweep, each covering about ±60°.
- Behind her is a **blind arc** for the radar, so she flies a **racetrack orbit** to sweep the sky.
- Radar is blocked by terrain (line-of-sight). That is exactly why she flies: from altitude she looks down into valleys a ground radar cannot see.

**Her eyes: a compound eye, like a fly's.** The central dome is covered in many independent passive EO/IR facets (24 to start).
- **SCAN:** every facet sweeps its own sector. Together they give near-spherical coverage at shorter range than the radar, *including the radar's blind arc*. Behind her she is short-sighted, never blind.
- **STARE:** several facets converge on one target to lock it. That gives a precise bearing, identification (real threat vs decoy) and a fast track update.
- **Fast:** the eyes update much faster than the radar sweeps, so she sees jinks and course changes early.
- **Passive:** the eyes emit nothing, so a threat cannot tell it is being watched.
- **Choosing how many facets stare at which target is one of her learned decisions.** When she looks at you, she *is* looking at you.
- **Honest limits (tested, not hidden):** dust storms, sun glare and cloud degrade the eyes, and passive eyes measure direction well but range poorly.

**Honeycomb bays.** Docking under her belly is a stretch goal. The ground nest is the default way home.

**If she cannot catch it, she warns early.** For cruise missiles and other threats too fast for the hawks, her job is to detect early and hand the track to ground defenses. We measure **warning time gained**.

## 2. Her brood (FQ-7 hawks)

- **Airframe:** small Global Hawk-style fixed-wing: V-tail, seeker nose ("the hawk's eye"), electric ducted fan.
- **Capture:** a net + parachute pass. The hawk fires a net ahead of the threat on a close pass, the threat falls away under a parachute, and the hawk flies home.
  - Capture probability depends on miss distance, closing speed and aspect. Because no public data covers heavy targets, this curve is **swept** in experiments.
  - A capture too close to the protected site counts as a **partial leak** (debris still falls inside).
- **The Final Stoop:** a last-resort ram that costs the hawk. It is allowed only when the site or the Queen is at stake and no net pass can make it in time, and an action mask enforces that. Its kill probability is never higher than the net's, so it can't be exploited.
- **Recovery:** land at the **Nest-7** ground nest (default) to recharge and re-arm; docking with the Queen is the stretch goal.
- **Battery:** a drag-polar power model drains a real battery (Wh). There are no hand-waved endurance numbers.
- **Her children have personalities.** Each hawk has a name, a capture record, and two physical traits:
  - **Boldness:** how close it presses before firing the net (closer means a better catch, at more collision risk).
  - **Loyalty:** the battery reserve at which it turns home.
- **The creed:** they serve the Queen and defend her domain. Their chants and callouts are AI-generated from that creed and her persona, varied per hawk, in Arabic and English.

## 3. Her brain (hybrid)

Every decision tick she decides the **queen-level** questions. Learned with PPO:
- per-track **priority**, and how many hawks to **commit**;
- how many hawks to keep **hooded** (reserve);
- which hawks **return** (to the nest, or to her);
- how her **eyes** are tasked (which tracks get a stare);
- whether a **Final Stoop** is authorized (masked).

A **leak-aware Hungarian assignment** then does the hawk-to-track matching, so she is never worse than the classical method she builds on.

She commands from **tracks, not truth**: noisy radar and eye measurements filtered per track.

**Personality is causal.** Falconry moves are her real actions: *hood* = reserve, *waiting on* = loiter overhead, *stoop* = dive, *bind* = net pass. Her tone follows her confidence (value estimate) and hesitation (policy entropy).

**Two modes, one brain.**
- **Theatrical:** full character; the brood chants.
- **Operational:** terse military callouts.

Switching mode never changes what she does, and a test proves it.

**Priority (= reward):** *the land is her domain.* Site and Queen are the top tier, then her hawks.

## 4. Enemies and world

- **Terrain:** real elevation (Copernicus GLO-30) from the Asir / Al Baha mountains, with fictional facilities only.
  - Three tiles: two for training, one held out to test generalization.
- **Phase 1 threats:**
  - Shahed-136-type one-way attack drones, with swept altitude profiles (terrain-hugging and ~1 km GNSS cruise);
  - jet-powered Shahed-238-type;
  - decoys;
  - cruise missiles (detect and warn only).
- **Later:** FPV swarms, recon loiterers, hunters of the Queen, a trained adversary, airport and city domains, a scenario editor.

## 5. What the experiments must answer (the paper claims nothing beyond these)

1. **Learned Queen vs classical commanders.** Greedy, leak-aware Hungarian, Hungarian + reserve, CBBA, and an oracle.
   - At least 5 training seeds; interquartile mean with bootstrap 95% CIs; held-out seeds and terrain tile.
2. **Why she flies.** Airborne Queen vs a ground mast radar on the same terrain, with threat altitude swept.
3. **Performance envelope.** Threat speed swept from Shahed-136 to cruise-missile speeds: where the hawks intercept, and where she can only warn (warning time gained).
4. **Sensitivity.** Net capture odds, battery, threat altitude, radar cross-section, eye degradation (dust/glare). Report where conclusions flip.
5. **Persona invariance and faithfulness.**
   - Mode switch gives identical trajectories.
   - Every callout's stated reason matches the brain's actual top decision factors.

Every number the simulation uses is listed in [`claims.md`](claims.md), either with a public source or marked **SWEPT**.
