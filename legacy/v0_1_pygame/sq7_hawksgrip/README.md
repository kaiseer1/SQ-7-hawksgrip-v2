# SQ-7 Hawksgrip (V2.0)
### AI-Powered Counter-Drone Swarm Simulation

## Overview
**SQ-7 Hawksgrip** is a high-fidelity simulation addressing the asymmetric warfare problem: firing a $2M Patriot missile at a $20K drone is economically unsustainable. 

This project simulates a **"Mothership"** (B-2 Stealth Bomber) that deploys a swarm of cheap, reusable AI interceptor drones to destroy incoming threats using advanced guidance algorithms and psychological audio tactics.

## Core Mechanics

### 1. The Mothership (Command Node)
- **Visuals**: Rendered as a sleek, dark-gray B-2 Stealth Bomber flying wing shape.
- **Logic**: Runs an **Auction-Based Task Allocation** algorithm. It perceives targets with Gaussian Noise (Fog of War), broadcasts locations, and assigns targets to the nearest available interceptor based on "bids" (distance).

### 2. The Swarm (Interceptors)
- **Guidance**: Uses **Predictive Lead Pursuit**. Instead of chasing where the target *is*, they calculate where the target *will be* based on velocity vectors and time-to-intercept.
- **Physics**: Constrained by `max_turn_rate` and `max_acceleration`. They cannot teleport; they must bank and accelerate realistically.
- **The "Scream"**: When an interceptor closes within **15 meters** of its target, it triggers a terminal audio event—a high-pitched synthetic screech generated procedurally via `numpy` and `pygame`. This adds tactical personality and auditory feedback.

### 3. The Targets (Hostile Drones)
- **Evasive AI**: Targets are not static. If an interceptor gets too close, they perform **jinking maneuvers** (random lateral shifts) or drop altitude to break the lock.
- **Swarm Behavior**: They spawn in waves from screen edges and attempt to breach the Mothership's perimeter.

## Tech Stack
- **Python 3.15+**
- **Pygame**: Rendering engine and audio mixer.
- **NumPy**: Vector mathematics, physics calculations, and procedural audio generation.
- **OOP Architecture**: Strictly modular design separating entities, systems, and utilities.

## Installation

1. **Ensure Python 3.15+ is installed.**
2. **Install dependencies:**
   ```bash
   pip install -r requirements.txt
   ```

## How to Run
```bash
python main.py
```

## Controls
- **SPACE**: Manually spawn a new wave of enemy drones.
- **ESC**: Quit the simulation.

## Project Structure
```text
sq7_hawksgrip/
├── main.py                  # Entry point and game loop
├── config.py                # Constants (physics limits, colors, audio freqs)
├── requirements.txt         # Dependencies
├── utils/
│   └── math_helpers.py      # Vector normalization, noise, prediction logic
├── entities/
│   ├── base_drone.py        # Abstract physics base class
│   ├── mothership.py        # B-2 Visuals + Auction Algorithm
│   ├── interceptor.py       # Lead Pursuit + Scream Logic
│   └── target_drone.py      # Evasive AI (Jinking)
└── systems/
    ├── physics_engine.py    # Collision & Boundary checks
    ├── audio_manager.py     # Procedural scream generation
    └── renderer.py          # Pygame drawing routines
```

## Key Algorithms Implemented

### Auction-Based Task Allocation
1. Mothership senses targets with added Gaussian noise (`sensor_noise_std`).
2. For each target, available interceptors submit a "bid" equal to their distance.
3. The lowest bidder wins the target.
4. Assignments are updated periodically to handle dynamic changes.

### Predictive Lead Pursuit
$$ P_{future} = P_{target} + (V_{target} \times TTI) $$
Where $TTI$ (Time To Intercept) is estimated by $Distance / RelativeSpeed$.

### Procedural Audio Scream
A sine wave chirp sweeping from 2000Hz to 6000Hz over 1.5 seconds, normalized and converted to 16-bit stereo PCM for immediate playback without external asset files.

## License
MIT License. Built for educational and simulation purposes.
