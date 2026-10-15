"""
Phase 3 Test - Verify mothership detects incoming threat.
Run from project root: python test_phase3.py
"""

import config
from environment import World
from agents import Threat, Mothership


def test_mothership_detection():
    print("=== Phase 3 Test: Mothership Detection ===\n")
    
    # Create world and mothership
    world = World()
    mothership = Mothership()
    world.mothership = mothership
    
    print(f"Mothership at: {mothership.position}")
    print(f"Detection radius: {mothership.detection_radius} m")
    print()
    
    # Spawn threat from East
    threat = Threat.spawn_from_direction(0)
    world.register_threat(threat)
    
    print(f"Threat spawned at: ({threat.position[0]:.1f}, {threat.position[1]:.1f})")
    print(f"Initial distance: {mothership.distance_to_threat(threat):.1f} m")
    print(f"In range: {mothership.is_threat_in_range(threat)}")
    print()
    
    # Calculate time to enter detection radius
    dist_to_detection = mothership.distance_to_threat(threat) - mothership.detection_radius
    time_to_detection = dist_to_detection / threat.speed
    print(f"Distance to detection boundary: {dist_to_detection:.1f} m")
    print(f"Estimated time to detection: {time_to_detection:.1f} s")
    print()
    
    # Simulate until detection
    print("Simulating...\n")
    detected = False
    step = 0
    max_steps = int(600 / config.TIME_STEP)  # Max 10 minutes
    
    while step < max_steps:
        # Update threat position
        threat.update(config.TIME_STEP)
        world.time += config.TIME_STEP
        step += 1
        
        # Check for new detections
        newly_detected = mothership.detect_threats([threat])
        
        if newly_detected and not detected:
            detected = True
            dist = mothership.distance_to_threat(threat)
            print(f"  t={world.time:6.1f}s | DETECTION! Threat entered range")
            print(f"             Distance: {dist:.1f} m")
            print(f"             Threat pos: ({threat.position[0]:.1f}, {threat.position[1]:.1f})")
            print()
        
        # Print progress every 5 seconds before detection
        if not detected and abs(world.time % 5.0) < config.TIME_STEP:
            dist = mothership.distance_to_threat(threat)
            in_range = "YES" if mothership.is_threat_in_range(threat) else "NO"
            print(f"  t={world.time:6.1f}s | dist={dist:.1f}m | in_range={in_range}")
        
        # Stop shortly after detection for demo
        if detected and world.time > time_to_detection + 2.0:
            break
    
    # Verify detection was reported only once
    print("Testing duplicate detection prevention...")
    newly_detected_again = mothership.detect_threats([threat])
    if len(newly_detected_again) == 0:
        print("  PASS: Same threat not reported twice")
    else:
        print("  FAIL: Threat reported multiple times")
    
    print("\n=== Phase 3 Test Complete ===")


if __name__ == "__main__":
    test_mothership_detection()
