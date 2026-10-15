"""
Hawksgrip v0.1 - Math Utilities
Basic 2D vector operations. No dependencies except math.
"""

import math


def distance(p1, p2):
    """Euclidean distance between two points (x, y)."""
    dx = p2[0] - p1[0]
    dy = p2[1] - p1[1]
    return math.sqrt(dx * dx + dy * dy)


def normalize(v):
    """
    Return unit vector in same direction.
    Returns (0, 0) if input is zero vector.
    """
    mag = math.sqrt(v[0] * v[0] + v[1] * v[1])
    if mag < 1e-9:
        return (0.0, 0.0)
    return (v[0] / mag, v[1] / mag)


def magnitude(v):
    """Length of vector."""
    return math.sqrt(v[0] * v[0] + v[1] * v[1])


def subtract(p1, p2):
    """Vector from p2 to p1: p1 - p2."""
    return (p1[0] - p2[0], p1[1] - p2[1])


def add(p1, p2):
    """Vector addition: p1 + p2."""
    return (p1[0] + p2[0], p1[1] + p2[1])


def scale(v, s):
    """Multiply vector by scalar."""
    return (v[0] * s, v[1] * s)


def dot(v1, v2):
    """Dot product of two vectors."""
    return v1[0] * v2[0] + v1[1] * v2[1]


def angle_between(v1, v2):
    """
    Angle in radians between two vectors.
    Returns 0 if either vector is zero.
    """
    m1 = magnitude(v1)
    m2 = magnitude(v2)
    if m1 < 1e-9 or m2 < 1e-9:
        return 0.0
    cos_angle = dot(v1, v2) / (m1 * m2)
    # Clamp to handle floating point errors
    cos_angle = max(-1.0, min(1.0, cos_angle))
    return math.acos(cos_angle)


def lerp(a, b, t):
    """Linear interpolation between a and b. t in [0, 1]."""
    return a + (b - a) * t


def lerp_point(p1, p2, t):
    """Linear interpolation between two points."""
    return (lerp(p1[0], p2[0], t), lerp(p1[1], p2[1], t))


def clamp(value, min_val, max_val):
    """Clamp value to range [min_val, max_val]."""
    return max(min_val, min(max_val, value))
