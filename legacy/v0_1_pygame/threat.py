"""
Hawksgrip v0.1 - Threat Drone
Hostile UAV that moves toward the protected asset.
"""

import math
import config
from agents.base_agent import BaseAgent
from utils import math_utils


class Threat(BaseAgent):
    """
    Hostile drone moving toward protected zone.
    
    Behavior:
    - Spawns outside detection radius
    - Moves in straight line toward ASSET_POSITION
    - Speed defined by THREAT_SPEED
    - Becomes inactive when intercepted or breaches
    """
    
    def __init__(self, position, target=None):
        """
        Args:
            position: Spawn (x, y) position
            target: Target position, defaults to ASSET_POSITION
        """
        super().__init__(position, velocity=(0.0, 0.0), agent_type="threat")
        
        self.target = target if target else config.ASSET_POSITION
        self.speed = config.THREAT_SPEED
        
        # Calculate velocity toward target
        self._set_velocity_toward_target()
    
    def _set_velocity_toward_target(self):
        """Set velocity vector pointing at target with constant speed."""
        direction = math_utils.subtract(self.target, self.position)
        direction = math_utils.normalize(direction)
        self.velocity = math_utils.scale(direction, self.speed)
    
    @staticmethod
    def spawn_from_direction(angle_degrees):
        """
        Factory method: spawn threat from a specific direction.
        
        Args:
            angle_degrees: Direction from asset (0=East, 90=North, etc.)
        
        Returns:
            Threat instance positioned outside detection radius
        """
        # Spawn distance: detection radius + margin
        spawn_dist = config.MOTHERSHIP_DETECTION_RADIUS + config.THREAT_SPAWN_MARGIN
        
        angle_rad = math.radians(angle_degrees)
        
        # Position relative to asset
        asset = config.ASSET_POSITION
        x = asset[0] + spawn_dist * math.cos(angle_rad)
        y = asset[1] + spawn_dist * math.sin(angle_rad)
        
        return Threat(position=(x, y))
    
    @staticmethod
    def spawn_random():
        """
        Factory method: spawn threat from random direction.
        
        Returns:
            Threat instance from random angle
        """
        import random
        angle = random.uniform(0, 360)
        return Threat.spawn_from_direction(angle)
    
    def distance_to_target(self):
        """Return current distance to target."""
        return math_utils.distance(self.position, self.target)
    
    def time_to_target(self):
        """Estimate seconds until reaching target (straight line)."""
        dist = self.distance_to_target()
        if self.speed < 1e-9:
            return float('inf')
        return dist / self.speed
