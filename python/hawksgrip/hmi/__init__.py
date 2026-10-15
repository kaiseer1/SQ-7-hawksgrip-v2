"""SQ-7 Hawksgrip showroom HMI (Phase 0-3 scope).

- ``theme``: colours (dark-mode reference palette).
- ``scene``: pure geometry (terrain colours, frustum wireframes, trails,
  picking rays). numpy only, unit-tested without a window.
- ``hud``: pure HUD text builders, unit-tested.
- ``showroom``: the Qt/OpenGL application (``python -m hawksgrip.hmi.showroom``).

The showroom renders ground truth (solid markers) next to Kindah's own
beliefs (track rings and labels). The difference between the two is the
point of the demonstration.
"""
