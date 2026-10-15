# Viva Defence Cheat-Sheet: Phase 3 (the 24-eye compound optical array)

`[AGENT: THESIS DEFENSE COACH & EXPLAINER]` with `[AGENT: CUDA & OPTICS]`

## 1. Plain-English pitch

Kindah carries 24 small cameras on fast gimbals. Normally they sweep the
sky below her in a pattern that never repeats but never misses a spot for
more than about three seconds. When one spots something, it stares to
confirm it, tells drones from birds, and, if it's a lone drone, all 24 snap
onto it at once. Then she looks again with all those eyes to make sure it
really isn't a bird. In clear weather she locks onto a drone a median of
about 10 seconds after it appears. In a dust storm she switches to thermal,
and the numbers show honestly that this is where she is weakest.

## 2. Mathematical core

**(a) Detection physics.** Pixels on target n = size / (R · IFOV); contrast
after dust C₀·e^(−τ). Koschmieder: σ = 3.912 / V. Dust is thickest near the
ground with scale height H, so along a slant path

```
τ = k_band · σ₀ · L · H (e^(−h_a/H) − e^(−h_b/H)) / (h_b − h_a)
```

Pd = Pd_max · S(8 ln(n/2)) · S(8 ln(C/C_th)), where S is the logistic
function. The VIS → LWIR switch is not a rule I wrote. It falls out of
comparing the three bands' detection ranges.

**(b) Sweep with a guarantee.** Each eye tiles its sector with cells no larger
than its footprint (azimuth width scaled by 1/cos(el)). In cycle c it visits
cell (i·stride + offset_c) mod n, with stride coprime to n: every cell once
per cycle, in a new order each cycle. **Revisit ≤ 2 cycles**, measured worst
2.87 s.

**(c) Combining eyes.** n eyes on target: P = 1 − (1 − pd)ⁿ.

**(d) Asymmetric classification + decisive second look.** First verdict:
error e = 0.5·e^(−px/3), split as e(1 − b) for drone → bird and e(1 + b)
for bird → drone, with b = 0.8. The converged eyes then gather evidence
E = Σ eyes × px and re-decide once, at E = 40: error 0.5·e^(−40/3) ≈ 10⁻⁶.

**(e) Tracking.** Constant-velocity Kalman filter. Process noise 6 m/s²;
measurement noise from the eyes (0.2 mrad/√n) and range (10 % passive, 1 m
with a laser rangefinder once locked).

## 3. Key results

| | Clear | Shamal (800 m) |
|---|---|---|
| Median spawn → lock | 9–13 s | 79–89 s |
| Drones never locked before breach | **0 / 384** | **184 / 384** |
| Bird false alarms (all cleared by the second look) | 85 / 85 cleared | 4 / 4 cleared |
| Worst sweep revisit | 2.87 s | 2.87 s |
| GPU coverage speed-up (≥ 400 k voxels), agreement | 2.5–2.7×, 100 % | |

## 4. Committee Q&A prep

**Toughest question:** *"You claim 24 eyes give total situational awareness,
but your own numbers say half the drones get through in a sandstorm. Isn't
the concept broken?"*

**Winning answer:**
- "The concept has a measured operating envelope, and I found its edge
  instead of hiding it.
- In clear weather all 384 drones in my study were locked before reaching the
  refinery, with a median of about 10 seconds.
- In an 800-metre Shamal, physics takes over. Visible light can't see down
  through the dust at all, and thermal is resolution-limited to about 3 km.
  With her standard 3 km orbit, half the drones are never locked.
- I tested the obvious mitigation: tighten the orbit when visibility drops.
  At 800 m radius, unlocked drones fall from 50 % to 2 %, though the Seven then
  have about 35 seconds from lock to breach.
- That trade-off between detection coverage and engagement time is a result of
  the thesis, not a flaw in it. It also tells you exactly what a real system
  would need: a storm orbit, ground-based thermal pickets, or larger optics."

**Follow-up:** *"Why not point all 24 eyes at every threat? Isn't that the 'SHE LOOKS AT YOU' feature?"*
- "For a lone intruder, all 24 snap on for half a second.
- I measured what happens if every new threat does that in a 40-drone raid:
  the bursts chain together and every other track starves.
- So with several threats she keeps 4 sentinel eyes sweeping and shares the
  rest by time-to-refinery, with time-division when over-subscribed. That's
  the 'never lose situational awareness' requirement from my specification,
  enforced by a test."

**Follow-up:** *"Optical 'micro-Doppler'? Doppler is a radar concept."*
- "Correct, and I don't call it radar. The optical analogue is temporal
  signature: rotor flicker and wing-beat frequency in the pixel time series.
- I model it as a classifier whose accuracy depends on pixels on target, with
  parameters marked SWEPT in my claims register.
- It's deliberately biased toward 'drone' because the costs are asymmetric.
  A decisive multi-eye second look then clears the false alarms: 89 of 89 in my study."

**Follow-up:** *"Where did the GPU actually help?"*
- "Dense coverage volumes: 400 000 to 1.6 million voxels tested against 24
  frustums plus the dust physics, every frame.
- That's 2.5–2.7× faster than 12 Rust threads, with identical results.
- Terrain line-of-sight stayed on the CPU, where it measured faster. Each
  GPU claim in my thesis is sized to its measurement."
