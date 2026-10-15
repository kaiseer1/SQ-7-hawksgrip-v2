"""Showroom colours. Categorical hues are the dark-mode steps of the reference
palette (dataviz skill, references/palette.md); text uses ink tokens, never
series colours."""

from __future__ import annotations

BG = "#0b0d10"
BG_STORM = "#211a12"
BG_THERMAL = "#050505"
PANEL_RGBA = "rgba(14, 16, 20, 205)"
INK = "#ffffff"
INK_2 = "#c3c2b7"
INK_3 = "#8a8980"

# Categorical (dark-mode steps), fixed roles.
BLUE = "#3987e5"
ORANGE = "#d95926"
AQUA = "#199e70"
YELLOW = "#c98500"
MAGENTA = "#d55181"
RED = "#e66767"

KINDAH = "#f0b429"  # royal gold, brightened for a dark scene
SEVEN = "#1baf7a"
DRONE = RED
BIRD = INK_2
ORBIT = "#8a6d1f"
CONE_SWEEP = BLUE
CONE_CONFIRM = YELLOW
CONE_TRACK = ORANGE
CONE_FLASH = "#ffffff"
TRACK_TENTATIVE = BLUE
TRACK_CONFIRMED = YELLOW
TRACK_HOSTILE = ORANGE
TRACK_LOST = INK_3
KEEPOUT = RED
DUST = "#c8a46a"
SEA = "#0e2738"

TERRAIN_RAMP = ["#1d1a16", "#3b3226", "#6b5639", "#9c7c52", "#c8a46a", "#e9d4a4"]
THERMAL_RAMP = ["#0d0d0d", "#2e2e2e", "#5c5c5c", "#8f8f8f", "#c4c4c4"]

SPEAKER = {"KINDAH": KINDAH, "SYSTEM": INK_3}  # nodes use SEVEN


def rgba(hex_color: str, alpha: float = 1.0) -> tuple[float, float, float, float]:
    h = hex_color.lstrip("#")
    return (int(h[0:2], 16) / 255, int(h[2:4], 16) / 255, int(h[4:6], 16) / 255, alpha)
