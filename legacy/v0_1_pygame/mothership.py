"""
Hawksgrip v0.1 - Mothership
Sensor and command node. Detection only - no auction logic here.
"""

import config
from agents.base_agent import BaseAgent
from utils import math_utils


class Mothership(BaseAgent):
    """
    Persistent airborne command platform.
    
    Responsibilities (v0.1):
    - Maintain fixed position above protected asset
    - Detect threats within detection radius
    - Report detected threats (auction handled externally)
    
    NOT responsible for:
    - Running auctions (handled by systems/auction.py)
    - Assigning interceptors (handled by main loop)
    """
    
    def __init__(self, position=None):
        """
        Args:
            position: (x, y) position, defaults to MOTHERSHIP_POSITION
        """
        if position is None:
            position = config.MOTHERSHIP_POSITION
        
        super().__init__(position, velocity=(0.0, 0.0), agent_type="mothership")
        
        self.detection_radius = config.MOTHERSHIP_DETECTION_RADIUS
        
        # Set of threat IDs currently detected (in range)
        self._detected_threats = set()
        
        # Set of threat IDs already reported (to avoid duplicate alerts)
        self._reported_threats = set()
    
    def update(self, dt):
        """
        Mothership is stationary in v0.1.
        Override to prevent movement.
        """
        pass
    
    def detect_threats(self, threats):
        """
        Scan list of threats and return newly detected ones.
        
        Args:
            threats: List of Threat objects to scan
        
        Returns:
            List of Threat objects that are:
            - Within detection radius
            - Active (not already intercepted/breached)
            - Not yet reported
        """
        newly_detected = []
        current_detected = set()
        
        for threat in threats:
            if not threat.is_active:
                continue
            
            dist = math_utils.distance(self.position, threat.position)
            
            if dist <= self.detection_radius:
                current_detected.add(threat.id)
                
                # Check if this is a new detection
                if threat.id not in self._reported_threats:
                    newly_detected.append(threat)
                    self._reported_threats.add(threat.id)
        
        self._detected_threats = current_detected
        return newly_detected
    
    def get_detected_threats(self, threats):
        """
        Return all threats currently within detection radius.
        
        Args:
            threats: List of Threat objects to scan
        
        Returns:
            List of active Threat objects within range
        """
        detected = []
        
        for threat in threats:
            if not threat.is_active:
                continue
            
            dist = math_utils.distance(self.position, threat.position)
            
            if dist <= self.detection_radius:
                detected.append(threat)
        
        return detected
    
    def is_threat_in_range(self, threat):
        """Check if a specific threat is within detection radius."""
        if not threat.is_active:
            return False
        dist = math_utils.distance(self.position, threat.position)
        return dist <= self.detection_radius
    
    def distance_to_threat(self, threat):
        """Return distance to a threat."""
        return math_utils.distance(self.position, threat.position)
    
    def reset_detections(self):
        """Clear detection history (call at episode start)."""
        self._detected_threats = set()
        self._reported_threats = set()
