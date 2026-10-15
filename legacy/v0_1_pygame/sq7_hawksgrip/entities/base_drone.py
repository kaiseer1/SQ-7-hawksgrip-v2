"""
Base Drone Class

Abstract base class defining common physics properties and kinematics for all flying objects.
Enforces max_turn_rate and max_acceleration constraints to prevent teleportation.
"""
import numpy as np
import math
from abc import ABC, abstractmethod

from utils.math_helpers import normalize_vector, clamp_angle


class BaseDrone(ABC):
    def __init__(self, x: float, y: float, max_speed: float, max_accel: float, max_turn: float):
        self.position = np.array([x, y], dtype=float)
        self.velocity = np.array([0.0, 0.0], dtype=float)
        self.acceleration = np.array([0.0, 0.0], dtype=float)
        
        self.max_speed = max_speed
        self.max_acceleration = max_accel
        self.max_turn_rate = max_turn
        
        self.heading = 0.0  # Radians, 0 points right (East)
        self.active = True
        self.radius = 10.0  # Collision radius

    @abstractmethod
    def update(self, dt: float):
        """Update physics state. Must be implemented by subclasses."""
        pass

    @abstractmethod
    def get_action(self, world_state: dict) -> np.ndarray:
        """Calculate desired acceleration vector based on AI logic."""
        pass

    def apply_physics(self, desired_accel: np.ndarray, dt: float):
        """
        Applies physics constraints to the desired acceleration and updates position/velocity.
        """
        # 1. Limit Acceleration Magnitude
        accel_mag = np.linalg.norm(desired_accel)
        if accel_mag > self.max_acceleration:
            desired_accel = normalize_vector(desired_accel) * self.max_acceleration

        # 2. Calculate Turn Rate Constraint
        # Determine desired heading from acceleration vector
        if accel_mag > 0.001:
            desired_heading = math.atan2(desired_accel[1], desired_accel[0])
            
            # Calculate angle difference
            angle_diff = desired_heading - self.heading
            # Normalize to -PI to PI
            while angle_diff > math.pi:
                angle_diff -= 2 * math.pi
            while angle_diff < -math.pi:
                angle_diff += 2 * math.pi
            
            # Clamp turn rate
            clamped_turn = clamp_angle(angle_diff, self.max_turn_rate)
            self.heading += clamped_turn
            
            # Re-align acceleration vector to the new constrained heading
            # This ensures the drone turns realistically rather than snapping
            current_speed = np.linalg.norm(self.velocity)
            # We project the acceleration along the new heading but respect max accel
            forward_dir = np.array([math.cos(self.heading), math.sin(self.heading)])
            
            # Simple model: Accelerate in direction of heading
            # If we are turning, we can't accelerate fully sideways immediately
            desired_accel = forward_dir * min(accel_mag, self.max_acceleration)

        # 3. Update Velocity
        self.velocity += desired_accel * dt
        
        # 4. Limit Speed
        speed = np.linalg.norm(self.velocity)
        if speed > self.max_speed:
            self.velocity = normalize_vector(self.velocity) * self.max_speed

        # 5. Update Position
        self.position += self.velocity * dt

    def get_distance_to(self, other_pos: np.ndarray) -> float:
        return np.linalg.norm(self.position - other_pos)
