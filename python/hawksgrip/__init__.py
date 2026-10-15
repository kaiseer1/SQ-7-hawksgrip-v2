"""SQ-7 Hawksgrip: a closed-environment Critical Infrastructure Protection
simulation. The deterministic engine is Rust (``hawksgrip._core``); Python
drives it, runs GPU sensing, and hosts the HMI and analysis.
"""

from hawksgrip._core import (
    FIRST_SEVEN,
    FIRST_THREAT,
    QUEEN,
    SEVEN,
    PersonaEngine,
    Session,
    default_config_json,
    run_batch,
    validate_config,
    verify_replay,
)
from hawksgrip._core import __version__ as core_version

__version__ = "0.2.0"

ROLE_QUEEN = 0
ROLE_SEVEN = 1
ROLE_THREAT = 2

__all__ = [
    "FIRST_SEVEN",
    "FIRST_THREAT",
    "QUEEN",
    "ROLE_QUEEN",
    "ROLE_SEVEN",
    "ROLE_THREAT",
    "SEVEN",
    "PersonaEngine",
    "Session",
    "core_version",
    "default_config_json",
    "run_batch",
    "validate_config",
    "verify_replay",
]
