"""Terrain helpers on the Python side.

The three Saudi sector generators run in Rust (ADR-0009) and are selected
with ``[sim.terrain] sector = "..."`` in a scenario. This package adds:

- ``dem_loader``: bring your own public DEM (GeoTIFF or .npy), resampled
  to the 10 m sector grid and installed with ``Session.set_terrain``.
"""

from hawksgrip._core import SECTORS
from hawksgrip.terrain.dem_loader import install_dem, load_dem, resample_bilinear

__all__ = ["SECTORS", "install_dem", "load_dem", "resample_bilinear"]
