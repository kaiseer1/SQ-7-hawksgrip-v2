"""
Hawksgrip v0.1 - Formation Logic
Butterfly formation positioning for interceptor swarm.
"""

from utils import math_utils


# Butterfly formation offsets (meters from mothership)
# Layout:
#
#         [0]     [1]        <- Lead pair (front)
#
#     [2]             [3]    <- Mid-flank pair
#
#         [4]     [5]        <- Rear pair
#

BUTTERFLY_OFFSETS = [
    # Lead pair (front)
    (-150.0, 300.0),    # Index 0: Front-left
    (150.0, 300.0),     # Index 1: Front-right
    
    # Mid-flank pair (sides)
    (-400.0, 0.0),      # Index 2: Left flank
    (400.0, 0.0),       # Index 3: Right flank
    
    # Rear pair (back)
    (-150.0, -300.0),   # Index 4: Rear-left
    (150.0, -300.0),    # Index 5: Rear-right
]


def get_formation_offset(index):
    """
    Get formation offset for interceptor at given index.
    
    Args:
        index: Interceptor index (0-5)
    
    Returns:
        (dx, dy) offset from mothership position
    """
    if 0 <= index < len(BUTTERFLY_OFFSETS):
        return BUTTERFLY_OFFSETS[index]
    return (0.0, 0.0)


def get_formation_position(mothership_position, index):
    """
    Calculate absolute position for interceptor in formation.
    
    Args:
        mothership_position: (x, y) of mothership
        index: Interceptor index (0-5)
    
    Returns:
        (x, y) absolute position
    """
    offset = get_formation_offset(index)
    return math_utils.add(mothership_position, offset)


def get_all_formation_positions(mothership_position, count=6):
    """
    Get all formation positions.
    
    Args:
        mothership_position: (x, y) of mothership
        count: Number of interceptors (default 6)
    
    Returns:
        List of (x, y) positions
    """
    positions = []
    for i in range(min(count, len(BUTTERFLY_OFFSETS))):
        pos = get_formation_position(mothership_position, i)
        positions.append(pos)
    return positions


def create_interceptors_in_formation(mothership_position, count=6):
    """
    Factory function: Create interceptors positioned in butterfly formation.
    
    Args:
        mothership_position: (x, y) of mothership
        count: Number of interceptors to create (default 6)
    
    Returns:
        List of Interceptor objects
    """
    # Import here to avoid circular dependency
    from agents.interceptor import Interceptor
    
    interceptors = []
    
    for i in range(min(count, len(BUTTERFLY_OFFSETS))):
        offset = get_formation_offset(i)
        position = math_utils.add(mothership_position, offset)
        
        interceptor = Interceptor(position=position, formation_offset=offset)
        interceptors.append(interceptor)
    
    return interceptors


def update_formation_positions(interceptors, mothership_position):
    """
    Update idle interceptors to maintain formation.
    Only moves interceptors that are in IDLE state.
    
    Args:
        interceptors: List of Interceptor objects
        mothership_position: Current (x, y) of mothership
    
    Note: In v0.1, mothership is stationary so this is rarely needed.
          Included for future expansion.
    """
    for i, interceptor in enumerate(interceptors):
        if interceptor.is_idle:
            new_pos = get_formation_position(mothership_position, i)
            interceptor.set_position(new_pos)


def get_formation_role(index):
    """
    Get descriptive role name for formation position.
    
    Args:
        index: Interceptor index (0-5)
    
    Returns:
        String role name
    """
    roles = [
        "Lead-Left",
        "Lead-Right",
        "Flank-Left",
        "Flank-Right",
        "Rear-Left",
        "Rear-Right",
    ]
    if 0 <= index < len(roles):
        return roles[index]
    return "Unknown"
