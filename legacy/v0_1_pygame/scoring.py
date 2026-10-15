"""
Hawksgrip v0.1 - Scoring System
Reward/punishment calculation for episode evaluation.

Reward structure:
- R_HIT = +100 (each successful intercept)
- R_BREACH = -500 (each threat reaching protected zone)
- R_TIME = -0.3 (per second of episode duration)
- R_EXTRA_INTERCEPTOR = -20 (each interceptor beyond first used)
- R_COLLISION = -300 (per collision, stub for v0.1)
- R_PERFECT_DEFENSE = +100 (bonus for zero breaches)
"""

import config


def calculate_reward(events, episode_time, interceptors_used, total_threats=1):
    """
    Calculate total reward for an episode.
    
    Args:
        events: List of event tuples from world.events
        episode_time: Total episode duration in seconds
        interceptors_used: Number of unique interceptors that engaged
        total_threats: Total number of threats in episode
    
    Returns:
        tuple: (total_reward, summary_dict)
    """
    # Count events
    hits = sum(1 for e in events if e[0] == "intercept")
    breaches = sum(1 for e in events if e[0] == "breach")
    collisions = sum(1 for e in events if e[0] == "collision")
    
    # Calculate reward components
    r_hit = hits * config.R_HIT
    r_breach = breaches * config.R_BREACH
    r_time = episode_time * config.R_TIME
    
    # Extra interceptor penalty (first one is free)
    extra_interceptors = max(0, interceptors_used - 1)
    r_extra = extra_interceptors * config.R_EXTRA_INTERCEPTOR
    
    r_collision = collisions * config.R_COLLISION
    
    # Perfect defense bonus (all threats intercepted, zero breaches)
    if breaches == 0 and hits > 0:
        r_perfect = config.R_PERFECT_DEFENSE
    else:
        r_perfect = 0
    
    # Total
    total_reward = r_hit + r_breach + r_time + r_extra + r_collision + r_perfect
    
    summary = {
        "hits": hits,
        "breaches": breaches,
        "collisions": collisions,
        "interceptors_used": interceptors_used,
        "total_threats": total_threats,
        "time": episode_time,
        "r_hit": r_hit,
        "r_breach": r_breach,
        "r_time": r_time,
        "r_extra_interceptor": r_extra,
        "r_collision": r_collision,
        "r_perfect_defense": r_perfect,
        "total_reward": total_reward
    }
    
    return (total_reward, summary)


def calculate_reward_from_world(world):
    """
    Convenience function to calculate reward directly from world state.
    
    Args:
        world: World object with events and time
    
    Returns:
        tuple: (total_reward, summary_dict)
    """
    total_threats = len(world.threats)
    return calculate_reward(
        events=world.events,
        episode_time=world.time,
        interceptors_used=world.get_interceptors_used_count(),
        total_threats=total_threats
    )


def format_score_report(summary):
    """
    Format scoring summary for display.
    
    Args:
        summary: Dict from calculate_reward()
    
    Returns:
        str: Formatted multi-line report
    """
    lines = []
    lines.append("=" * 50)
    lines.append("EPISODE SCORE REPORT")
    lines.append("=" * 50)
    lines.append("")
    lines.append("Outcomes:")
    lines.append(f"  Threats:            {summary.get('total_threats', '?')}")
    lines.append(f"  Intercepts:         {summary['hits']}")
    lines.append(f"  Breaches:           {summary['breaches']}")
    lines.append(f"  Collisions:         {summary['collisions']}")
    lines.append(f"  Interceptors used:  {summary['interceptors_used']}")
    lines.append(f"  Episode time:       {summary['time']:.1f}s")
    lines.append("")
    lines.append("Reward Breakdown:")
    lines.append(f"  Intercepts:         {summary['r_hit']:+.0f}  ({summary['hits']} × {config.R_HIT})")
    lines.append(f"  Breaches:           {summary['r_breach']:+.0f}  ({summary['breaches']} × {config.R_BREACH})")
    lines.append(f"  Time penalty:       {summary['r_time']:+.1f}  ({summary['time']:.1f}s × {config.R_TIME})")
    
    extra = max(0, summary['interceptors_used'] - 1)
    lines.append(f"  Extra interceptors: {summary['r_extra_interceptor']:+.0f}  ({extra} × {config.R_EXTRA_INTERCEPTOR})")
    lines.append(f"  Collisions:         {summary['r_collision']:+.0f}  ({summary['collisions']} × {config.R_COLLISION})")
    lines.append(f"  Perfect defense:    {summary['r_perfect_defense']:+.0f}")
    lines.append("")
    lines.append("-" * 50)
    lines.append(f"  TOTAL REWARD:       {summary['total_reward']:+.1f}")
    lines.append("=" * 50)
    
    return "\n".join(lines)


def evaluate_performance(summary):
    """
    Provide qualitative assessment of episode performance.
    
    Args:
        summary: Dict from calculate_reward()
    
    Returns:
        str: Performance grade and comment
    """
    total = summary["total_reward"]
    hits = summary["hits"]
    breaches = summary["breaches"]
    total_threats = summary.get("total_threats", 1)
    
    # Calculate intercept rate
    if total_threats > 0:
        intercept_rate = hits / total_threats
    else:
        intercept_rate = 0
    
    # Determine grade
    if breaches > 0:
        if intercept_rate >= 0.5:
            grade = "C"
            comment = "Partial defense - some threats breached"
        else:
            grade = "F"
            comment = "Mission failed - protected zone breached"
    elif hits == 0:
        grade = "D"
        comment = "No threats engaged"
    elif total >= 150:
        grade = "A+"
        comment = "Outstanding performance"
    elif total >= 100:
        grade = "A"
        comment = "Excellent performance"
    elif total >= 50:
        grade = "B"
        comment = "Good performance"
    elif total >= 0:
        grade = "C"
        comment = "Acceptable performance"
    else:
        grade = "D"
        comment = "Poor efficiency"
    
    return f"Grade: {grade} - {comment}"
