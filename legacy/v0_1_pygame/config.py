"""
Hawksgrip v0.1 - Configuration Constants
All tunables in one place. No logic here.
"""

# === World ===
WORLD_WIDTH = 10000      # meters
WORLD_HEIGHT = 10000     # meters
TIME_STEP = 0.1          # seconds per simulation tick

# === Protected Asset ===
ASSET_POSITION = (0, 0)  # center of protected zone
BREACH_RADIUS = 500      # meters - threat wins if inside this
KILL_RADIUS = 50         # meters - intercept success distance

# === Mothership ===
MOTHERSHIP_POSITION = (0, 0)      # same as asset for v0.1
MOTHERSHIP_DETECTION_RADIUS = 8000  # meters

# === Interceptors ===
INTERCEPTOR_COUNT = 6
INTERCEPTOR_MAX_SPEED = 33.3      # m/s (~120 km/h)
INTERCEPTOR_FUEL_CAPACITY = 1.0   # normalized 0-1
INTERCEPTOR_FUEL_BURN_RATE = 0.002  # per second while pursuing

# === Threat ===
THREAT_SPEED = 20.0      # m/s (~72 km/h)
THREAT_SPAWN_MARGIN = 500  # spawn this far outside detection radius

# === Scoring / Rewards ===
R_HIT = 100              # per successful intercept
R_BREACH = -500          # per threat reaching protected zone
R_TIME = -0.3            # per second of episode duration
R_EXTRA_INTERCEPTOR = -20  # per extra interceptor beyond first
R_COLLISION = -300       # per collision (stub for v0.1)
R_PERFECT_DEFENSE = 100  # bonus for zero breaches

# === Visualization ===
RENDER_SCALE = 0.04      # pixels per meter
RENDER_FPS = 30