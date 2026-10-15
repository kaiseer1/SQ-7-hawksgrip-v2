"""
Phase 2 Test - Verify threat moves toward asset.
Run from project root: python test_phase2.py
"""

import config
from environment import World
from agents import Threat
from utils import math_utils


def test_threat_movement():
    print("=== Phase 2 Test: Threat Movement ===\n")
    
    # Create world
    world = World()
    
    # Spawn threat from East (0 degrees)
    threat = Threat.spawn_from_direction(0)
    world.register_threat(threat)
    world.threats.append(threat)
    
    print(f"Threat spawned at: ({threat.position[0]:.1f}, {threat.position[1]:.1f})")
    print(f"Target: {threat.target}")
    print(f"Speed: {threat.speed} m/s")
    print(f"Initial distance to target: {threat.distance_to_target():.1f} m")
    print(f"Estimated time to target: {threat.time_to_target():.1f} s")
    print()
    
    # Simulate 100 seconds
    sim_time = 100.0
    steps = int(sim_time / config.TIME_STEP)
    
    print(f"Simulating {sim_time} seconds ({steps} steps)...\n")
    
    for i in range(steps):
        threat.update(config.TIME_STEP)
        
        # Print every 10 seconds
        elapsed = (i + 1) * config.TIME_STEP
        if abs(elapsed % 10.0) < config.TIME_STEP:
            dist = threat.distance_to_target()
            print(f"  t={elapsed:5.1f}s | pos=({threat.position[0]:7.1f}, {threat.position[1]:7.1f}) | dist={dist:.1f}m")
    
    print()
    final_dist = threat.distance_to_target()
    print(f"Final position: ({threat.position[0]:.1f}, {threat.position[1]:.1f})")
    print(f"Final distance to target: {final_dist:.1f} m")
    
    # Check if breach would occur
    if final_dist <= config.BREACH_RADIUS:
        print(f"STATUS: Threat within BREACH_RADIUS ({config.BREACH_RADIUS}m) - BREACH!")
    else:
        print(f"STATUS: Threat still {final_dist - config.BREACH_RADIUS:.1f}m outside breach zone")
    
    print("\n=== Phase 2 Test Complete ===")


if __name__ == "__main__":
    test_threat_movement()
