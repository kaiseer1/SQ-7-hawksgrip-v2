"""
Phase 5 Test - Verify auction system.
Run from project root: python test_phase5.py
"""

import config
from environment import World
from agents import Mothership, Threat
from systems import formation, auction


def test_auction():
    print("=== Phase 5 Test: Auction System ===\n")
    
    # Setup world
    world = World()
    mothership = Mothership()
    world.mothership = mothership
    
    # Create interceptors in formation
    interceptors = formation.create_interceptors_in_formation(
        mothership.position,
        count=config.INTERCEPTOR_COUNT
    )
    world.interceptors = interceptors
    
    print(f"Mothership at: {mothership.position}")
    print(f"Interceptors: {len(interceptors)}")
    print()
    
    # Spawn threat from East
    threat = Threat.spawn_from_direction(0)
    world.register_threat(threat)
    
    print(f"Threat spawned at: ({threat.position[0]:.1f}, {threat.position[1]:.1f})")
    print(f"Threat ID: {threat.id}")
    print()
    
    # Simulate until threat is detected
    print("Simulating until detection...\n")
    
    detected = False
    while not detected:
        threat.update(config.TIME_STEP)
        world.time += config.TIME_STEP
        
        newly_detected = mothership.detect_threats([threat])
        if newly_detected:
            detected = True
            print(f"Threat detected at t={world.time:.1f}s")
            print(f"Threat position: ({threat.position[0]:.1f}, {threat.position[1]:.1f})")
            print()
    
    # Run auction
    print("Running auction...\n")
    result = auction.run_auction(interceptors, threat, world)
    
    print(auction.format_auction_result(result))
    print()
    
    # Verify winner state
    if result["success"]:
        winner = result["winner"]
        print(f"Winner state verification:")
        print(f"  ID: {winner.id}")
        print(f"  State: {winner.state}")
        print(f"  Assigned threat: {winner.assigned_threat_id}")
        print(f"  Can engage: {winner.can_engage()}")
        print()
        
        # Verify world tracking
        print(f"World tracking:")
        print(f"  Interceptors used: {world.get_interceptors_used_count()}")
    
    print()
    
    # Test second auction (winner should not bid again)
    print("=" * 50)
    print("Testing second auction (simulating second threat)...\n")
    
    threat2 = Threat.spawn_from_direction(180)  # From West
    world.register_threat(threat2)
    print(f"Second threat at: ({threat2.position[0]:.1f}, {threat2.position[1]:.1f})")
    print()
    
    result2 = auction.run_auction(interceptors, threat2, world)
    print(auction.format_auction_result(result2))
    print()
    
    # Verify different winner
    if result["success"] and result2["success"]:
        if result["winner"].id != result2["winner"].id:
            print("PASS: Different interceptor assigned to second threat")
        else:
            print("FAIL: Same interceptor assigned twice")
    
    print(f"\nInterceptors used total: {world.get_interceptors_used_count()}")
    
    print()
    
    # Test auction with no available interceptors
    print("=" * 50)
    print("Testing auction with all interceptors busy...\n")
    
    # Mark all interceptors as pursuing
    for intc in interceptors:
        if intc.is_idle:
            intc.assign_target("dummy")
    
    threat3 = Threat.spawn_from_direction(90)  # From North
    result3 = auction.run_auction(interceptors, threat3, world)
    print(auction.format_auction_result(result3))
    
    if not result3["success"]:
        print("\nPASS: Auction correctly returns no winner when all busy")
    
    print("\n=== Phase 5 Test Complete ===")


def test_redundant_auction():
    print("\n" + "=" * 60)
    print("=== Testing Redundant Auction ===\n")
    
    # Fresh setup
    from agents.base_agent import BaseAgent
    BaseAgent.reset_id_counter()
    
    world = World()
    mothership = Mothership()
    interceptors = formation.create_interceptors_in_formation(
        mothership.position,
        count=6
    )
    world.interceptors = interceptors
    
    # Create close threat (high Pint for all)
    threat = Threat((2000.0, 0.0))  # Close threat
    world.register_threat(threat)
    
    print(f"Close threat at: {threat.position}")
    print(f"Target cumulative Pint: 0.95\n")
    
    result = auction.run_auction_redundant(interceptors, threat, world, target_probability=0.95)
    
    print(f"Winners assigned: {len(result['winners'])}")
    for w in result["winners"]:
        print(f"  - {w.id}")
    print(f"Cumulative Pint achieved: {result['cumulative_pint']:.3f}")
    print()
    
    if result["cumulative_pint"] >= 0.95:
        print("PASS: Cumulative probability target met")
    else:
        print(f"INFO: Target not met (may need more interceptors)")
    
    print(f"Interceptors used: {world.get_interceptors_used_count()}")


if __name__ == "__main__":
    test_auction()
    test_redundant_auction()
