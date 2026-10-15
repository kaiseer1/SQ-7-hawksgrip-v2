"""
Target Drone Class

Implements Evasive AI: jinking and dropping altitude when an interceptor gets too close.
"""
import numpy as np
import math
import random
from entities.base_drone import BaseDrone


class TargetDrone(BaseDrone):
    def __init__(self, x: float, y: float):
        # Targets are agile but slower than interceptors
        super().__init__(x, y, max_speed=6.0, max_accel=8.0, max_turn=0.25)
        self.evasion_threshold = 150.0  # Distance to start evading
        self.jink_timer = 0
        self.move_pattern = "straight"  # straight, jink_left, jink_right, drop

    def update(self, dt: float):
        if not self.active:
            return
        
        # Dummy world state for standalone testing if needed, 
        # but normally passed from main loop
        world_state = {"interceptors": [], "bounds": (0, 0, 1920, 1080)}
        desired_accel = self.get_action(world_state)
        self.apply_physics(desired_accel, dt)

    def get_action(self, world_state: dict) -> np.ndarray:
        """
        Determines acceleration based on evasion logic.
        """
        interceptors = world_state.get("interceptors", [])
        
        # Find closest threat
        closest_dist = float('inf')
        closest_interceptor_pos = None
        
        for inter in interceptors:
            if not inter.active:
                continue
            dist = self.get_distance_to(inter.position)
            if dist < closest_dist:
                closest_dist = dist
                closest_interceptor_pos = inter.position

        # Evasion Logic
        if closest_interceptor_pos is not None and closest_dist < self.evasion_threshold:
            # Vector from interceptor to me (run away vector)
            run_vector = self.position - closest_interceptor_pos
            
            # Add some randomness (jink)
            if self.jink_timer <= 0:
                self.jink_timer = random.randint(20, 40)  # Frames until next jink decision
                choice = random.choice(["left", "right", "drop"])
                
                if choice == "left":
                    perp = np.array([-run_vector[1], run_vector[0]])
                elif choice == "right":
                    perp = np.array([run_vector[1], -run_vector[0]])
                else:
                    perp = np.array([0.0, 5.0]) # Drop down bias
                
                run_vector += perp * 0.5

            self.jink_timer -= 1
            
            # Normalize and scale
            if np.linalg.norm(run_vector) > 0:
                run_vector = run_vector / np.linalg.norm(run_vector)
            
            return run_vector * self.max_acceleration

        # Default behavior: Move forward or patrol (simple sine wave for now)
        # In a full sim, this would be a path towards a target objective
        forward = np.array([math.cos(self.heading), math.sin(self.heading)])
        if np.linalg.norm(forward) == 0:
            forward = np.array([1.0, 0.0])
            
        return forward * (self.max_acceleration * 0.5)
