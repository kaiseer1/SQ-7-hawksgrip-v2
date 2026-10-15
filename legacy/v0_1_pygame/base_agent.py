"""
Hawksgrip v0.1 - Base Agent
Abstract base class for all agents in the simulation.
"""


class BaseAgent:
    """
    Base class for mothership, interceptors, and threats.
    
    Attributes:
        id: Unique identifier string
        position: (x, y) tuple in meters
        velocity: (vx, vy) tuple in m/s
        is_active: Whether agent is still in play
    """
    
    _id_counter = 0
    
    def __init__(self, position, velocity=(0.0, 0.0), agent_type="agent"):
        """
        Args:
            position: Initial (x, y) position
            velocity: Initial (vx, vy) velocity
            agent_type: String prefix for ID generation
        """
        BaseAgent._id_counter += 1
        self.id = f"{agent_type}_{BaseAgent._id_counter}"
        
        self.position = (float(position[0]), float(position[1]))
        self.velocity = (float(velocity[0]), float(velocity[1]))
        self.is_active = True
    
    def update(self, dt):
        """
        Update agent state for one time step.
        Default behavior: move by velocity * dt.
        
        Args:
            dt: Time step in seconds
        """
        if not self.is_active:
            return
        
        new_x = self.position[0] + self.velocity[0] * dt
        new_y = self.position[1] + self.velocity[1] * dt
        self.position = (new_x, new_y)
    
    def set_position(self, position):
        """Set position directly."""
        self.position = (float(position[0]), float(position[1]))
    
    def set_velocity(self, velocity):
        """Set velocity directly."""
        self.velocity = (float(velocity[0]), float(velocity[1]))
    
    def stop(self):
        """Set velocity to zero."""
        self.velocity = (0.0, 0.0)
    
    @classmethod
    def reset_id_counter(cls):
        """Reset ID counter (call between episodes if needed)."""
        cls._id_counter = 0
