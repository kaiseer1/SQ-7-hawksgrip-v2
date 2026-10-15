"""
Mathematical helper functions for vector operations, angle clamping, and noise generation.
"""
import numpy as np
import math


def normalize_vector(vector: np.ndarray) -> np.ndarray:
    """
    Returns a unit vector in the direction of the input vector.
    Returns zero vector if magnitude is 0.
    """
    norm = np.linalg.norm(vector)
    if norm == 0:
        return np.zeros_like(vector)
    return vector / norm


def clamp_angle(angle: float, max_change: float) -> float:
    """
    Clamps an angle change to a maximum turn rate.
    Ensures the shortest path around the circle (-PI to PI).
    
    Args:
        angle: The desired angle change (radians).
        max_change: Maximum allowed angle change per step (radians).
    
    Returns:
        The clamped angle change.
    """
    # Normalize angle to -PI to PI
    while angle > math.pi:
        angle -= 2 * math.pi
    while angle < -math.pi:
        angle += 2 * math.pi
    
    return np.clip(angle, -max_change, max_change)


def add_gaussian_noise(position: np.ndarray, std_dev: float) -> np.ndarray:
    """
    Adds Gaussian noise to a position vector to simulate sensor uncertainty (Fog of War).
    
    Args:
        position: [x, y] position array.
        std_dev: Standard deviation of the noise.
    
    Returns:
        Noisy position array.
    """
    noise = np.random.normal(0, std_dev, size=position.shape)
    return position + noise


def predict_future_position(
    current_pos: np.ndarray, 
    velocity: np.ndarray, 
    time_horizon: float
) -> np.ndarray:
    """
    Simple linear prediction of future position.
    
    Args:
        current_pos: Current [x, y] position.
        velocity: Current velocity vector.
        time_horizon: Time steps into the future to predict.
    
    Returns:
        Predicted [x, y] position.
    """
    return current_pos + velocity * time_horizon
