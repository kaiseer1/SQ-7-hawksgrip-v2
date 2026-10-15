"""
Phase 6 Test - Verify intercept pursuit behavior.
Run from project root: python test_phase6.py
"""

import config
from environment import World
from agents import Mothership, Threat, BaseAgent
from systems import formation, auction, intercept


def test_intercept():
    print("=== Phase 6 Test: Intercept Behavior ===\n")
    
    # Reset IDs
    BaseAgent.reset_id_counter()
    
    # Setup world
    world = World()
    mothership = Mothership()
    world.mothership = mothership
    
    # Create interceptors
    interceptors = formation.create_interceptors_in_formation(
        mothership.position,
        count=config.INTERCEPTOR_COUNT
    )
    world.interceptors = interceptors
    
    # Spawn threat from East (closer for faster test)
    threat = Threat((4000.0, 0.0))  # Start at 4km
    world.register_threat(threat)
    world.threats.append(threat)
    
    print(f"Threat at: ({threat.position[0]:.1f}, {threat.position[1]:.1f})")
    print(f"Threat speed: {threat.speed} m/s")
    print(f"Threat velocity: ({threat.velocity[0]:.1f}, {threat.velocity[1]:.1f})")
    print()
    
    # Run auction immediately
    print("Running auction...")
    result = auction.run_auction(interceptors, threat, world)
    
    if not result["success"]:
        print("ERROR: Auction failed")
        return
    
    winner = result["winner"]
    print(f"Winner: {winner.id}")
    print(f"Winner position: ({winner.position[0]:.1f}, {winner.position[1]:.1f})")
    print(f"Winner max speed: {winner.max_speed} m/s")
    print()
    
    # Simulate pursuit
    print("Simulating pursuit...\n")
    print(f"{'Time':>6} | {'Intc Pos':<20} | {'Threat Pos':<20} | {'Dist':>8} | {'Closing':>8} | {'ETA':>8}")
    print("-" * 85)
    
    max_time = 300.0  # Max 5 minutes
    print_interval = 5.0
    last_print = 0.0
    
    while world.time < max_time:
        # Update pursuit velocity
        intercept.update_all_interceptors(interceptors, world.threats)
        
        # Step simulation
        world.step()
        
        # Print status periodically
        if world.time - last_print >= print_interval:
            last_print = world.time
            
            status = intercept.get_intercept_status(winner, threat)
            
            intc_pos = f"({winner.position[0]:7.1f}, {winner.position[1]:7.1f})"
            threat_pos = f"({threat.position[0]:7.1f}, {threat.position[1]:7.1f})"
            closing_str = "YES" if status["closing"] else "NO"
            eta_str = f"{status['time_estimate']:.1f}s" if status['time_estimate'] < 1000 else "N/A"
            
            print(f"{world.time:6.1f} | {intc_pos:<20} | {threat_pos:<20} | {status['distance']:8.1f} | {closing_str:>8} | {eta_str:>8}")
        
        # Check for intercept
        if world.events:
            last_event = world.events[-1]
            if last_event[0] == "intercept":
                print()
                print(f"INTERCEPT SUCCESS at t={last_event[1]:.1f}s")
                print(f"  Interceptor: {last_event[2]}")
                print(f"  Threat: {last_event[3]}")
                break
            elif last_event[0] == "breach":
                print()
                print(f"BREACH at t={last_event[1]:.1f}s")
                print(f"  Threat {last_event[2]} reached protected zone")
                break
        
        # Check if episode complete
        if world.is_episode_complete():
            break
    
    print()
    
    # Final status
    print("Final state:")
    print(f"  Simulation time: {world.time:.1f}s")
    print(f"  Winner state: {winner.state}")
    print(f"  Winner fuel: {winner.fuel:.3f}")
    print(f"  Threat active: {threat.is_active}")
    print(f"  Events: {world.events}")
    
    print("\n=== Phase 6 Test Complete ===")


def test_pursuit_geometry():
    """Test pursuit from different angles."""
    print("\n" + "=" * 60)
    print("=== Testing Pursuit Geometry ===\n")
    
    BaseAgent.reset_id_counter()
    
    angles = [0, 45, 90, 135, 180]
    
    for angle in angles:
        # Fresh setup
        world = World()
        mothership = Mothership()
        interceptors = formation.create_interceptors_in_formation(mothership.position, 6)
        world.interceptors = interceptors
        
        # Spawn threat from angle at 3km
        threat = Threat.spawn_from_direction(angle)
        # Move threat closer for faster test
        threat.position = (
            config.ASSET_POSITION[0] + 3000 * __import__('math').cos(__import__('math').radians(angle)),
            config.ASSET_POSITION[1] + 3000 * __import__('math').sin(__import__('math').radians(angle))
        )
        threat._set_velocity_toward_target()
        
        world.register_threat(threat)
        world.threats.append(threat)
        
        # Auction
        result = auction.run_auction(interceptors, threat, world)
        if not result["success"]:
            print(f"Angle {angle}°: Auction failed")
            continue
        
        winner = result["winner"]
        
        # Simulate until resolution
        while world.time < 200 and not world.is_episode_complete():
            intercept.update_all_interceptors(interceptors, world.threats)
            world.step()
        
        # Result
        outcome = "INTERCEPT" if any(e[0] == "intercept" for e in world.events) else "BREACH"
        print(f"Angle {angle:3}°: {outcome} at t={world.time:5.1f}s by {winner.id}")
        
        BaseAgent.reset_id_counter()
    
    print()


if __name__ == "__main__":
    test_intercept()
    test_pursuit_geometry()
