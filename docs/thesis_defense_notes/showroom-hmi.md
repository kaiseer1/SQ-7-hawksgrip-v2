# Viva Defence Cheat-Sheet: the showroom HMI and Kindah's persona

`[AGENT: THESIS DEFENSE COACH & EXPLAINER]`

## 1. Plain-English pitch

The showroom is a live 3D window onto the simulation. You see the terrain,
the refinery, Kindah circling with her 24 eyes drawn as sightline cones,
drones and birds as they really are, and, as rings, where Kindah *thinks*
they are. She narrates what she perceives in a theatrical royal voice or in
clinical military telemetry. Press `[S]` and a Shamal rolls in: her eyes
switch to thermal and the picture turns white-hot.

## 2. Technical core

- One OpenGL viewport (PySide6 + pyqtgraph). The 24 "cameras" are drawn as
  frustum wireframes from the engine's own eye directions, not rendered as
  24 videos, which costs no extra VRAM.
- The drawn frustums are tested against the engine's detection frustums:
  ≥ 99.95 % agreement on 20 000 random points.
- **Persona invariance:** the voice reads events and never writes back. Proof:
  toggling theatrical/clinical every 97 ticks leaves every per-second state
  hash identical (Rust and Python tests).
- **Honest HUD:**
  - compute time per tick (measured µs) is shown apart from sensing time (sim seconds);
  - breaches are shown truthfully, with "interception not modelled in Phases 0–3";
  - there is no "× faster than a human" figure.

## 3. Committee Q&A prep

**Toughest question:** *"A theatrical queen who calls drones 'filth' and
'locusts': isn't that unprofessional in a defence thesis?"*

**Winning answer:**
- "It's a presentation layer that is provably inert: switching it changes
  zero bits of the simulation, and a test proves it.
- The clinical mode carries exactly the same facts in NATO-style telemetry.
- The persona exists because explainability needs an audience. Narrating
  detection, confirmation, classification and lock in plain language makes
  the kill chain legible to a non-specialist.
- And Kindah only says what her sensors know: she never names a target's
  true class, only her estimate."

**Follow-up:** *"Your HUD shows refinery breaches. Doesn't that undermine the system?"*
→ "It shows the truth of this scope. Phases 0–3 model sensing, not
interception, and the HUD says so on screen. A HUD that displayed 'breaches: 0'
by hiding the counter would be exactly the kind of claim my Red-Team
process exists to stop."
