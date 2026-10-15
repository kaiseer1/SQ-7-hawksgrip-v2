"""
Interceptor Drone Class

Implements Predictive Lead Pursuit guidance and triggers the terminal "Scream" audio.
"""
import numpy as np
import math
from entities.base_drone import BaseDrone
from utils.math_helpers import normalize_vector, predict_future_position


class Interceptor(BaseDrone):
    def __init__(self, x: float, y: float):
        # Interceptors are fast and have high acceleration
        super().__init__(x, y, max_speed=12.0, max_accel=15.0, max_turn=0.15)
        self.target = None  # Reference to TargetDrone object
        self.scream_triggered = False
        self.radius = 8.0

    def assign_target(self, target):
        """Assigns a target drone to intercept."""
        self.target = target
        self.scream_triggered = False

    def clear_target(self):
        """Clears current target."""
        self.target = None
        self.scream_triggered = False

    def update(self, dt: float):
        if not self.active:
            return
        
        world_state = {"interceptors": []} # Context for evasion checks if needed
        desired_accel = self.get_action(world_state)
        self.apply_physics(desired_accel, dt)

    def get_action(self, world_state: dict) -> np.ndarray:
        """
        Calculates acceleration using Predictive Lead Pursuit.
        Calculates where the target WILL be, not where it IS.
        """
        if self.target is None or not self.target.active:
            # No target: hover or return to base logic (simple stop for now)
            return np.array([0.0, 0.0])

        # 1. Estimate Time to Intercept (TTI)
        dist = self.get_distance_to(self.target.position)
        rel_speed = np.linalg.norm(self.velocity - self.target.velocity)
        
        # Avoid division by zero; if closing speed is low, assume a fixed horizon
        if rel_speed < 0.1:
            tti = dist / max(np.linalg.norm(self.velocity), 1.0)
        else:
            tti = dist / rel_speed
        
        # Clamp TTI to prevent looking too far ahead
        tti = min(max(tti, 0.1), 2.0)

        # 2. Predict Future Position
        predicted_pos = predict_future_position(
            self.target.position, 
            self.target.velocity, 
            tti
        )

        # 3. Calculate Vector to Predicted Position
        to_target = predicted_pos - self.position
        
        # 4. Terminal Scream Logic
        current_dist = np.linalg.norm(to_target)
        if current_dist <= 15.0 and not self.scream_triggered:
            self.scream_triggered = True
            # Note: Audio triggering is handled by the main loop or audio manager
            # checking this flag.

        # 5. Return desired acceleration direction
        if np.linalg.norm(to_target) > 0:
            return normalize_vector(to_target) * self.max_acceleration
        return np.array([0.0, 0.0])
