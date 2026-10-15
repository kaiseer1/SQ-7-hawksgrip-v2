"""
SQ-7 Hawksgrip Configuration Constants

Centralized settings for physics, rendering, audio, and simulation parameters.
"""

# --- Simulation Settings ---
FPS = 60
SCREEN_WIDTH = 1920
SCREEN_HEIGHT = 1080
SIMULATION_BOUNDARY_PADDING = 50  # Pixels from edge where drones can operate

# --- Physics Constants ---
MAX_ACCELERATION_INTERCEPTOR = 15.0  # pixels/frame^2
MAX_ACCELERATION_TARGET = 8.0        # pixels/frame^2
MAX_SPEED_INTERCEPTOR = 12.0         # pixels/frame
MAX_SPEED_TARGET = 6.0               # pixels/frame
MAX_TURN_RATE_INTERCEPTOR = 0.15     # radians/frame
MAX_TURN_RATE_TARGET = 0.25          # radians/frame (more agile but slower)

# --- Sensor & Noise Settings ---
SENSOR_RANGE = 600                   # Pixels
GAUSSIAN_NOISE_STD_DEV = 5.0         # Standard deviation for position noise (Fog of War)

# --- Combat Mechanics ---
COLLISION_RADIUS_INTERCEPTOR = 8
COLLISION_RADIUS_TARGET = 10
TERMINAL_SCREAM_DISTANCE = 15.0      # Distance in pixels to trigger audio scream
AUCTION_BID_BASE_COST = 1.0          # Base cost factor for task allocation

# --- Audio Settings ---
AUDIO_SAMPLE_RATE = 44100
SCREAM_DURATION_SEC = 1.5
SCREAM_START_FREQ = 2000             # Hz
SCREAM_END_FREQ = 6000               # Hz
SCREAM_VOLUME = 0.8

# --- Colors (R, G, B) ---
COLOR_BG = (10, 15, 30)              # Dark night sky
COLOR_MOTHERSHIP = (40, 40, 45)      # Dark gray stealth bomber
COLOR_INTERCEPTOR = (0, 255, 150)    # Bright cyan/green friendlies
COLOR_TARGET = (255, 80, 80)         # Aggressive red hostiles
COLOR_TEXT = (220, 220, 220)
COLOR_DEBUG = (100, 100, 100)

# --- Mothership Visuals (B-2 Shape Scale) ---
MOTHERSHIP_SCALE = 3.0
