"""
Hawksgrip v0.1 - Auction System
Distributed auction-based engagement protocol.

Bid formula: B = Pint × Frem × Mpay × (1 - U)

Where:
- Pint: Interception probability (distance-based)
- Frem: Fuel remaining (0-1)
- Mpay: Payload match score (1.0 for v0.1)
- U: Utilization (0 if idle, 1 if busy)
"""

import config
from utils import math_utils


def calculate_bid(interceptor, threat_position, max_range=None):
    """
    Calculate bid score for a single interceptor.
    
    Args:
        interceptor: Interceptor object
        threat_position: (x, y) of threat
        max_range: Maximum effective range (default: detection radius)
    
    Returns:
        float: Bid score (0.0 if cannot engage)
    """
    if not interceptor.can_engage():
        return 0.0
    
    comps = interceptor.get_bid_components(threat_position, max_range)
    
    # B = Pint × Frem × Mpay × (1 - U)
    bid = comps["Pint"] * comps["Frem"] * comps["Mpay"] * (1.0 - comps["U"])
    
    return bid


def collect_bids(interceptors, threat_position, max_range=None):
    """
    Collect bids from all interceptors for a threat.
    
    Args:
        interceptors: List of Interceptor objects
        threat_position: (x, y) of threat
        max_range: Maximum effective range
    
    Returns:
        List of (interceptor, bid_score, components) tuples, sorted by bid descending
    """
    bids = []
    
    for interceptor in interceptors:
        if not interceptor.can_engage():
            continue
        
        comps = interceptor.get_bid_components(threat_position, max_range)
        bid_score = comps["Pint"] * comps["Frem"] * comps["Mpay"] * (1.0 - comps["U"])
        
        bids.append((interceptor, bid_score, comps))
    
    # Sort by bid score, highest first
    bids.sort(key=lambda x: x[1], reverse=True)
    
    return bids


def select_winner(interceptors, threat_position, max_range=None):
    """
    Select the best interceptor for a threat.
    
    Args:
        interceptors: List of Interceptor objects
        threat_position: (x, y) of threat
        max_range: Maximum effective range
    
    Returns:
        tuple: (winning_interceptor, bid_score, components) or (None, 0, None) if none available
    """
    bids = collect_bids(interceptors, threat_position, max_range)
    
    if not bids:
        return (None, 0.0, None)
    
    # Winner is highest bid
    winner, score, comps = bids[0]
    return (winner, score, comps)


def run_auction(interceptors, threat, world=None):
    """
    Run complete auction for a detected threat.
    
    This is the main entry point called by the simulation loop.
    
    Args:
        interceptors: List of Interceptor objects
        threat: Threat object
        world: World object (for registering interceptor engagement)
    
    Returns:
        dict: Auction result containing:
            - success: bool
            - winner: Interceptor or None
            - bid_score: float
            - threat_id: str
            - all_bids: list of (interceptor_id, score) tuples
    """
    threat_position = threat.position
    
    # Collect all bids
    bids = collect_bids(interceptors, threat_position)
    
    # Build result
    result = {
        "success": False,
        "winner": None,
        "bid_score": 0.0,
        "threat_id": threat.id,
        "all_bids": [(b[0].id, b[1]) for b in bids]
    }
    
    if not bids:
        return result
    
    # Select winner
    winner, score, comps = bids[0]
    
    # Assign target to winner
    if winner.assign_target(threat.id):
        result["success"] = True
        result["winner"] = winner
        result["bid_score"] = score
        
        # Register engagement with world
        if world is not None:
            world.register_interceptor_engagement(winner.id)
    
    return result


def run_auction_redundant(interceptors, threat, world=None, target_probability=0.95):
    """
    Run auction with redundant assignment for high-priority threats.
    Assigns multiple interceptors until cumulative Pint >= target_probability.
    
    Args:
        interceptors: List of Interceptor objects
        threat: Threat object
        world: World object
        target_probability: Cumulative success probability target (default 0.95)
    
    Returns:
        dict: Auction result with list of winners
    """
    threat_position = threat.position
    
    # Collect all bids
    bids = collect_bids(interceptors, threat_position)
    
    result = {
        "success": False,
        "winners": [],
        "cumulative_pint": 0.0,
        "threat_id": threat.id,
        "all_bids": [(b[0].id, b[1]) for b in bids]
    }
    
    if not bids:
        return result
    
    # Assign interceptors until cumulative Pint meets target
    cumulative = 0.0
    
    for interceptor, score, comps in bids:
        if cumulative >= target_probability:
            break
        
        if interceptor.assign_target(threat.id):
            result["winners"].append(interceptor)
            
            # Cumulative probability: P(at least one success) = 1 - Π(1 - Pint_i)
            cumulative = 1.0 - (1.0 - cumulative) * (1.0 - comps["Pint"])
            
            if world is not None:
                world.register_interceptor_engagement(interceptor.id)
    
    result["success"] = len(result["winners"]) > 0
    result["cumulative_pint"] = cumulative
    
    return result


def format_auction_result(result):
    """
    Format auction result for logging/display.
    
    Args:
        result: Dict from run_auction()
    
    Returns:
        str: Formatted string
    """
    lines = []
    lines.append(f"Auction for {result['threat_id']}:")
    
    if result["success"]:
        if "winners" in result:
            # Redundant auction
            winner_ids = [w.id for w in result["winners"]]
            lines.append(f"  Winners: {', '.join(winner_ids)}")
            lines.append(f"  Cumulative Pint: {result['cumulative_pint']:.3f}")
        else:
            # Single winner
            lines.append(f"  Winner: {result['winner'].id}")
            lines.append(f"  Bid score: {result['bid_score']:.3f}")
    else:
        lines.append("  No interceptors available")
    
    if result["all_bids"]:
        lines.append("  All bids:")
        for intc_id, score in result["all_bids"]:
            lines.append(f"    {intc_id}: {score:.3f}")
    
    return "\n".join(lines)
