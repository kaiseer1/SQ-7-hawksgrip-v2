"""
Hawksgrip v0.1 - Intercept Behavior
Proportional navigation pursuit logic for interceptors.

Simplified pursuit: steer interceptor directly toward threat position.
Future versions may implement predictive intercept points.
"""

from utils import math_utils


def calculate_pursuit_velocity(interceptor, threat):
    """
    Calculate velocity vector for interceptor to pursue threat.
    
    Simple pursuit: point directly at current threat position.
    
    Args:
        interceptor: Interceptor object
        threat: Threat object
    
    Returns:
        (vx, vy) velocity tuple at interceptor's max speed
    """
    if not threat.is_active:
        return (0.0, 0.0)
    
    # Direction from interceptor to threat
    direction = math_utils.subtract(threat.position, interceptor.position)
    direction = math_utils.normalize(direction)
    
    # Scale to max speed
    velocity = math_utils.scale(direction, interceptor.max_speed)
    
    return velocity


def calculate_lead_pursuit_velocity(interceptor, threat, lead_factor=0.5):
    """
    Calculate velocity with lead prediction.
    
    Aims ahead of threat based on its velocity.
    
    Args:
        interceptor: Interceptor object
        threat: Threat object
        lead_factor: How far ahead to aim (0=pure pursuit, 1=full lead)
    
    Returns:
        (vx, vy) velocity tuple
    """
    if not threat.is_active:
        return (0.0, 0.0)
    
    # Estimate time to intercept (rough)
    dist = math_utils.distance(interceptor.position, threat.position)
    closing_speed = interceptor.max_speed + math_utils.magnitude(threat.velocity)
    
    if closing_speed < 1e-9:
        return calculate_pursuit_velocity(interceptor, threat)
    
    time_to_intercept = dist / closing_speed
    
    # Predict threat position
    predicted_pos = (
        threat.position[0] + threat.velocity[0] * time_to_intercept * lead_factor,
        threat.position[1] + threat.velocity[1] * time_to_intercept * lead_factor
    )
    
    # Direction to predicted position
    direction = math_utils.subtract(predicted_pos, interceptor.position)
    direction = math_utils.normalize(direction)
    
    velocity = math_utils.scale(direction, interceptor.max_speed)
    
    return velocity


def update_interceptor_pursuit(interceptor, threats):
    """
    Update a single interceptor's velocity based on its assigned threat.
    
    Args:
        interceptor: Interceptor object (must be in PURSUING state)
        threats: List of all Threat objects (to find assigned target)
    
    Returns:
        bool: True if pursuit updated, False if target not found/inactive
    """
    if not interceptor.is_pursuing:
        return False
    
    if interceptor.assigned_threat_id is None:
        return False
    
    # Find assigned threat
    target = None
    for threat in threats:
        if threat.id == interceptor.assigned_threat_id:
            target = threat
            break
    
    if target is None or not target.is_active:
        # Target lost or destroyed
        interceptor.velocity = (0.0, 0.0)
        return False
    
    # Calculate and set pursuit velocity
    velocity = calculate_lead_pursuit_velocity(interceptor, target)
    interceptor.set_velocity(velocity)
    
    return True


def update_all_interceptors(interceptors, threats):
    """
    Update velocities for all pursuing interceptors.
    
    Args:
        interceptors: List of Interceptor objects
        threats: List of Threat objects
    
    Returns:
        int: Number of interceptors actively pursuing
    """
    active_pursuits = 0
    
    for interceptor in interceptors:
        if interceptor.is_pursuing:
            if update_interceptor_pursuit(interceptor, threats):
                active_pursuits += 1
    
    return active_pursuits


def get_intercept_status(interceptor, threat, kill_radius=None):
    """
    Get status of an intercept engagement.
    
    Args:
        interceptor: Interceptor object
        threat: Threat object
        kill_radius: Distance for successful intercept (default from config)
    
    Returns:
        dict with:
            - distance: Current distance
            - closing: True if distance decreasing
            - in_range: True if within kill radius
            - time_estimate: Estimated seconds to intercept
    """
    import config
    
    if kill_radius is None:
        kill_radius = config.KILL_RADIUS
    
    dist = math_utils.distance(interceptor.position, threat.position)
    
    # Calculate closing rate
    relative_vel = math_utils.subtract(interceptor.velocity, threat.velocity)
    to_threat = math_utils.subtract(threat.position, interceptor.position)
    to_threat_norm = math_utils.normalize(to_threat)
    
    closing_rate = math_utils.dot(relative_vel, to_threat_norm)
    
    # Time estimate
    if closing_rate > 0:
        time_estimate = dist / closing_rate
    else:
        time_estimate = float('inf')
    
    return {
        "distance": dist,
        "closing": closing_rate > 0,
        "closing_rate": closing_rate,
        "in_range": dist <= kill_radius,
        "time_estimate": time_estimate
    }


def find_threat_by_id(threats, threat_id):
    """
    Find a threat by its ID.
    
    Args:
        threats: List of Threat objects
        threat_id: ID string to find
    
    Returns:
        Threat object or None
    """
    for threat in threats:
        if threat.id == threat_id:
            return threat
    return None
