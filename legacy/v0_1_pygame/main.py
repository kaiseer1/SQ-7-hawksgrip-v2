"""
Hawksgrip v0.1 - Main Simulation Runner
Entry point for counter-UAS simulation.

Usage:
    python main.py              # Run with visualization
    python main.py --headless   # Run without visualization
    python main.py --threats 3  # Spawn multiple threats
"""

import sys
import argparse
import random

import config
from environment import World
from agents import Mothership, Threat, Interceptor, BaseAgent
from systems import formation, auction, intercept, scoring
from visualization import Renderer, PYGAME_AVAILABLE


def parse_args():
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(description="Hawksgrip v0.1 Counter-UAS Simulation")
    parser.add_argument("--headless", action="store_true", help="Run without visualization")
    parser.add_argument("--threats", type=int, default=1, help="Number of threats to spawn")
    parser.add_argument("--seed", type=int, default=None, help="Random seed for reproducibility")
    parser.add_argument("--max-time", type=float, default=600.0, help="Maximum simulation time (seconds)")
    return parser.parse_args()


def setup_simulation(num_threats=1, seed=None):
    """
    Initialize simulation components.
    
    Args:
        num_threats: Number of hostile drones to spawn
        seed: Random seed (optional)
    
    Returns:
        tuple: (world, mothership, interceptors, threats)
    """
    if seed is not None:
        random.seed(seed)
    
    # Reset agent IDs
    BaseAgent.reset_id_counter()
    
    # Create world
    world = World()
    
    # Create mothership
    mothership = Mothership()
    world.mothership = mothership
    
    # Create interceptors in butterfly formation
    interceptors = formation.create_interceptors_in_formation(
        mothership.position,
        count=config.INTERCEPTOR_COUNT
    )
    world.interceptors = interceptors
    
    # Spawn threats from random directions
    threats = []
    for i in range(num_threats):
        # Spread threats across different angles
        if num_threats == 1:
            angle = random.uniform(0, 360)
        else:
            # Distribute evenly with some randomness
            base_angle = (360 / num_threats) * i
            angle = base_angle + random.uniform(-30, 30)
        
        threat = Threat.spawn_from_direction(angle)
        threats.append(threat)
        world.register_threat(threat)
    
    # Store threats in world
    world.threats = threats
    
    return world, mothership, interceptors, threats


def run_simulation(world, mothership, interceptors, threats, 
                   renderer=None, max_time=600.0, verbose=True):
    """
    Run the simulation loop.
    
    Args:
        world: World object
        mothership: Mothership object
        interceptors: List of Interceptor objects
        threats: List of Threat objects
        renderer: Renderer object (optional)
        max_time: Maximum simulation time
        verbose: Print status messages
    
    Returns:
        dict: Episode summary from scoring system
    """
    if verbose:
        print("\n" + "=" * 50)
        print("SIMULATION START")
        print("=" * 50)
        print(f"Threats: {len(threats)}")
        print(f"Interceptors: {len(interceptors)}")
        print(f"Max time: {max_time}s")
        print()
    
    # Track pending threats (detected but not yet auctioned)
    pending_auction = []
    
    # Main loop
    while world.time < max_time:
        # Handle renderer events (quit, etc.)
        if renderer and renderer.enabled:
            if not renderer.handle_events():
                if verbose:
                    print("Simulation aborted by user")
                break
        
        # Step 1: Mothership detection
        newly_detected = mothership.detect_threats(threats)
        for threat in newly_detected:
            pending_auction.append(threat)
            if verbose:
                print(f"[{world.time:6.1f}s] DETECTED: {threat.id} at ({threat.position[0]:.0f}, {threat.position[1]:.0f})")
        
        # Step 2: Run auctions for newly detected threats
        for threat in pending_auction:
            result = auction.run_auction(interceptors, threat, world)
            
            if result["success"]:
                if verbose:
                    print(f"[{world.time:6.1f}s] AUCTION: {result['winner'].id} assigned to {threat.id} (bid: {result['bid_score']:.3f})")
            else:
                if verbose:
                    print(f"[{world.time:6.1f}s] AUCTION: No interceptor available for {threat.id}")
        
        pending_auction.clear()
        
        # Step 3: Update intercept pursuits
        intercept.update_all_interceptors(interceptors, threats)
        
        # Step 4: Step world (moves agents, checks intercepts/breaches)
        world.step()
        
        # Step 5: Check for new events
        # (Events are logged by world.step())
        for event in world.events:
            # Only print new events (check if event time matches current time)
            if abs(event[1] - world.time) < config.TIME_STEP:
                if event[0] == "intercept":
                    if verbose:
                        print(f"[{world.time:6.1f}s] INTERCEPT: {event[2]} caught {event[3]}")
                elif event[0] == "breach":
                    if verbose:
                        print(f"[{world.time:6.1f}s] BREACH: {event[2]} reached protected zone!")
        
        # Step 6: Render
        if renderer and renderer.enabled:
            renderer.render(world)
        
        # Step 7: Check episode completion
        if world.is_episode_complete():
            if verbose:
                print(f"\n[{world.time:6.1f}s] All threats resolved")
            break
    
    # Calculate final score
    total_reward, summary = scoring.calculate_reward_from_world(world)
    
    return summary


def main():
    """Main entry point."""
    args = parse_args()
    
    # Setup
    world, mothership, interceptors, threats = setup_simulation(
        num_threats=args.threats,
        seed=args.seed
    )
    
    # Create renderer
    renderer = None
    if not args.headless and PYGAME_AVAILABLE:
        renderer = Renderer(width=800, height=800)
    elif not args.headless and not PYGAME_AVAILABLE:
        print("Warning: pygame not available, running headless")
    
    # Run simulation
    try:
        summary = run_simulation(
            world=world,
            mothership=mothership,
            interceptors=interceptors,
            threats=threats,
            renderer=renderer,
            max_time=args.max_time,
            verbose=True
        )
        
        # Show results
        print("\n")
        print(scoring.format_score_report(summary))
        print()
        print(scoring.evaluate_performance(summary))
        
        # Show result screen in renderer
        if renderer and renderer.enabled:
            renderer.show_result(summary, duration=5.0)
    
    finally:
        # Cleanup
        if renderer:
            renderer.close()
    
    return summary


if __name__ == "__main__":
    main()
