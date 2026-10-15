"""
SQ-7 Hawksgrip - Main Entry Point

A simulation of an AI-powered counter-drone swarm.
Run: python main.py
"""
import pygame
import sys
import random
import numpy as np

import config
from entities.mothership import Mothership
from entities.interceptor import Interceptor
from entities.target_drone import TargetDrone
from systems.physics_engine import PhysicsEngine
from systems.audio_manager import AudioManager
from systems.renderer import Renderer


def main():
    # --- Initialization ---
    pygame.init()
    screen = pygame.display.set_mode((config.SCREEN_WIDTH, config.SCREEN_HEIGHT))
    pygame.display.set_caption("SQ-7 Hawksgrip: Counter-Drone Swarm Simulation")
    clock = pygame.time.Clock()

    # --- Systems ---
    physics = PhysicsEngine()
    audio = AudioManager()
    renderer = Renderer(screen)

    # --- Game State ---
    mothership = Mothership(config.SCREEN_WIDTH // 2, config.SCREEN_HEIGHT // 2)
    interceptors = []
    targets = []
    
    # Initial Swarm Deployment
    for i in range(8):
        angle = (i / 8) * 2 * 3.14159
        dist = 60
        ix = mothership.position[0] + np.cos(angle) * dist
        iy = mothership.position[1] + np.sin(angle) * dist
        interceptors.append(Interceptor(ix, iy))

    running = True
    game_over = False
    game_over_reason = ""
    spawn_timer = 0
    
    print("SQ-7 Hawksgrip Initialized.")
    print("Mission: Protect the Mothership from incoming drone swarms.")

    while running:
        dt = clock.tick(config.FPS) / 1000.0  # Delta time in seconds
        
        # --- Event Handling ---
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                running = False
            elif event.type == pygame.KEYDOWN:
                if event.key == pygame.K_ESCAPE:
                    running = False
                elif event.key == pygame.K_SPACE and not game_over:
                    # Manual spawn wave
                    spawn_wave(interceptors, targets, mothership)

        if game_over:
            renderer.clear()
            renderer.draw_game_over(game_over_reason)
            pygame.display.flip()
            continue

        # --- Logic Updates ---
        
        # 1. Spawn Enemies periodically
        spawn_timer += 1
        if spawn_timer > 180:  # Every ~3 seconds
            spawn_wave(interceptors, targets, mothership)
            spawn_timer = 0

        # 2. Mothership Command: Auction Allocation
        # Run allocation every 30 frames to save CPU
        if pygame.time.get_ticks() % 30 == 0:
            assignments = mothership.run_auction_allocation(
                interceptors, 
                targets, 
                config.GAUSSIAN_NOISE_STD_DEV
            )
            # Apply assignments
            for inter, target in assignments.items():
                if inter.target is None: # Only assign if free
                    inter.assign_target(target)

        # 3. Update Entities
        all_drones = [mothership] + interceptors + targets
        physics.update_all(all_drones, dt)

        # 4. Collision Detection
        destroyed = physics.check_collisions(interceptors, targets)
        
        # 5. Audio Triggers (Scream)
        for inter in interceptors:
            if inter.scream_triggered and inter.active:
                # Play scream once per trigger event
                # We need a flag to ensure we don't play it every frame while true
                if not hasattr(inter, '_scream_played'):
                    audio.play_scream()
                    inter._scream_played = True
        
        # Reset scream flags for dead/finished interceptors
        for inter in interceptors:
            if not inter.active:
                if hasattr(inter, '_scream_played'):
                    delattr(inter, '_scream_played')
                inter.scream_triggered = False

        # 6. Cleanup Dead Entities
        interceptors = [i for i in interceptors if i.active]
        targets = [t for t in targets if t.active]

        # 7. Win/Loss Conditions
        if not mothership.active:
            game_over = True
            game_over_reason = "MOTHERSHIP DESTROYED"
        elif len(interceptors) == 0 and len(targets) > 5:
            # Overwhelmed
            game_over = True
            game_over_reason = "SWARM OVERWHELMED"

        # --- Rendering ---
        renderer.clear()
        
        # Draw Mothership
        renderer.draw_mothership(mothership)
        
        # Draw Drones
        for t in targets:
            renderer.draw_target(t)
        for i in interceptors:
            renderer.draw_interceptor(i)
            
        # Draw HUD
        renderer.draw_debug_info(int(clock.get_fps()), len(interceptors), len(targets))
        
        pygame.display.flip()

    # --- Shutdown ---
    audio.cleanup()
    pygame.quit()
    sys.exit()


def spawn_wave(interceptors: list, targets: list, mothership: Mothership):
    """Spawns a random wave of target drones from screen edges."""
    count = random.randint(2, 5)
    for _ in range(count):
        # Pick a random edge
        side = random.choice(['top', 'bottom', 'left', 'right'])
        if side == 'top':
            x = random.randint(0, config.SCREEN_WIDTH)
            y = -20
        elif side == 'bottom':
            x = random.randint(0, config.SCREEN_WIDTH)
            y = config.SCREEN_HEIGHT + 20
        elif side == 'left':
            x = -20
            y = random.randint(0, config.SCREEN_HEIGHT)
        else: # right
            x = config.SCREEN_WIDTH + 20
            y = random.randint(0, config.SCREEN_HEIGHT)
            
        t = TargetDrone(x, y)
        
        # Set initial velocity towards center/mothership
        dir_vec = mothership.position - t.position
        if np.linalg.norm(dir_vec) > 0:
            t.velocity = (dir_vec / np.linalg.norm(dir_vec)) * 2.0
            t.heading = np.arctan2(dir_vec[1], dir_vec[0])
            
        targets.append(t)
        
        # Replenish interceptors occasionally if low
        if len(interceptors) < 3:
            ix = mothership.position[0] + random.uniform(-50, 50)
            iy = mothership.position[1] + random.uniform(-50, 50)
            interceptors.append(Interceptor(ix, iy))


if __name__ == "__main__":
    main()
