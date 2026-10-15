"""
Hawksgrip v0.1 - Interceptor Drone
Expendable interceptor with fuel, payload, and engagement states.
"""

import config
from agents.base_agent import BaseAgent
from utils import math_utils


class Interceptor(BaseAgent):
    """
    Expendable interceptor drone.
    
    States:
    - IDLE: Holding formation position
    - PURSUING: Chasing assigned threat
    - COMPLETE: Finished engagement (successful or out of fuel)
    
    Attributes for auction bidding:
    - fuel: Normalized 0-1
    - payload_type: String (all "net" for v0.1)
    - utilization: 0.0 if idle, 1.0 if pursuing
    """
    
    # States
    STATE_IDLE = "idle"
    STATE_PURSUING = "pursuing"
    STATE_COMPLETE = "complete"
    
    def __init__(self, position, formation_offset=(0.0, 0.0)):
        """
        Args:
            position: Initial (x, y) position
            formation_offset: Offset from mothership for formation keeping
        """
        super().__init__(position, velocity=(0.0, 0.0), agent_type="interceptor")
        
        # Physical attributes
        self.max_speed = config.INTERCEPTOR_MAX_SPEED
        self.fuel = config.INTERCEPTOR_FUEL_CAPACITY
        self.fuel_burn_rate = config.INTERCEPTOR_FUEL_BURN_RATE
        
        # Payload (all same for v0.1)
        self.payload_type = "net"
        self.payload_match_score = 1.0
        
        # State
        self.state = self.STATE_IDLE
        self.formation_offset = formation_offset
        
        # Target tracking
        self.assigned_threat_id = None
        self._target_position = None
    
    @property
    def is_idle(self):
        return self.state == self.STATE_IDLE
    
    @property
    def is_pursuing(self):
        return self.state == self.STATE_PURSUING
    
    @property
    def is_complete(self):
        return self.state == self.STATE_COMPLETE
    
    @property
    def utilization(self):
        """Return utilization for bid calculation. 0=idle, 1=busy."""
        if self.state == self.STATE_IDLE:
            return 0.0
        return 1.0
    
    def update(self, dt):
        """
        Update interceptor state for one time step.
        
        - IDLE: Stay at formation position (no movement in v0.1)
        - PURSUING: Move toward target, burn fuel
        - COMPLETE: No movement
        """
        if not self.is_active:
            return
        
        if self.state == self.STATE_IDLE:
            # Hold position (formation keeping handled externally)
            self.velocity = (0.0, 0.0)
        
        elif self.state == self.STATE_PURSUING:
            # Burn fuel
            self.fuel -= self.fuel_burn_rate * dt
            
            # Check fuel exhaustion
            if self.fuel <= 0:
                self.fuel = 0
                self.state = self.STATE_COMPLETE
                self.velocity = (0.0, 0.0)
                return
            
            # Movement handled by intercept system
            # Just apply velocity here
            new_x = self.position[0] + self.velocity[0] * dt
            new_y = self.position[1] + self.velocity[1] * dt
            self.position = (new_x, new_y)
        
        elif self.state == self.STATE_COMPLETE:
            self.velocity = (0.0, 0.0)
    
    def can_engage(self):
        """Check if interceptor is available for new assignment."""
        return self.state == self.STATE_IDLE and self.fuel > 0.1 and self.is_active
    
    def assign_target(self, threat_id):
        """
        Assign interceptor to pursue a threat.
        
        Args:
            threat_id: ID of threat to pursue
        """
        if not self.can_engage():
            return False
        
        self.assigned_threat_id = threat_id
        self.state = self.STATE_PURSUING
        return True
    
    def on_intercept_complete(self):
        """Called by world when intercept succeeds."""
        self.state = self.STATE_COMPLETE
        self.velocity = (0.0, 0.0)
        self.assigned_threat_id = None
    
    def get_bid_components(self, threat_position, max_range=None):
        """
        Calculate components for auction bid.
        
        Args:
            threat_position: (x, y) of threat
            max_range: Maximum effective range (default: detection radius)
        
        Returns:
            dict with Pint, Frem, Mpay, U components
        """
        if max_range is None:
            max_range = config.MOTHERSHIP_DETECTION_RADIUS
        
        # Distance-based interception probability
        dist = math_utils.distance(self.position, threat_position)
        p_int = 1.0 - (dist / max_range)
        p_int = math_utils.clamp(p_int, 0.0, 1.0)
        
        # Fuel remaining
        f_rem = self.fuel
        
        # Payload match (all 1.0 for v0.1)
        m_pay = self.payload_match_score
        
        # Utilization
        u = self.utilization
        
        return {
            "Pint": p_int,
            "Frem": f_rem,
            "Mpay": m_pay,
            "U": u,
            "distance": dist
        }
    
    def distance_to(self, position):
        """Return distance to a position."""
        return math_utils.distance(self.position, position)
    
    def reset(self, position, formation_offset):
        """Reset interceptor for new episode."""
        self.position = (float(position[0]), float(position[1]))
        self.velocity = (0.0, 0.0)
        self.fuel = config.INTERCEPTOR_FUEL_CAPACITY
        self.state = self.STATE_IDLE
        self.formation_offset = formation_offset
        self.assigned_threat_id = None
        self._target_position = None
        self.is_active = True
