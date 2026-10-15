"""
Mothership Class

Acts as the high-altitude command node. 
Visualized as a sleek, dark-gray B-2 Stealth Bomber shape.
Runs Auction-Based Task Allocation to assign targets to the swarm.
"""
import numpy as np
import math
from entities.base_drone import BaseDrone


class Mothership(BaseDrone):
    def __init__(self, x: float, y: float):
        # Mothership is large, slow, and acts as a command station (mostly stationary or slow patrol)
        super().__init__(x, y, max_speed=2.0, max_accel=1.0, max_turn=0.05)
        self.radius = 40.0
        self.assigned_targets = {}  # Map: Interceptor ID -> Target ID

    def update(self, dt: float):
        if not self.active:
            return
        
        # Mothership doesn't chase; it patrols or holds position
        # Simple patrol logic: move slowly forward
        world_state = {}
        desired_accel = self.get_action(world_state)
        self.apply_physics(desired_accel, dt)

    def get_action(self, world_state: dict) -> np.ndarray:
        """
        Mothership AI: Maintain altitude and slow patrol.
        """
        # Just keep moving forward slowly
        forward = np.array([math.cos(self.heading), math.sin(self.heading)])
        if np.linalg.norm(forward) == 0:
            forward = np.array([1.0, 0.0])
        return forward * 0.5

    def run_auction_allocation(self, interceptors: list, targets: list, sensor_noise_std: float):
        """
        Auction-Based Task Allocation Algorithm.
        
        1. Mothership 'sees' targets with Gaussian noise (Fog of War).
        2. Broadcasts target locations to swarm.
        3. Interceptors 'bid' based on distance/fuel.
        4. Assigns target to lowest bidder.
        
        Returns: Dictionary mapping Interceptor -> Target
        """
        import random
        
        # 1. Simulate Sensor Noise (Fog of War)
        noisy_targets = []
        for t in targets:
            if not t.active:
                continue
            # Add noise to position perception
            noise = np.random.normal(0, sensor_noise_std, 2)
            perceived_pos = t.position + noise
            noisy_targets.append({
                'id': id(t),
                'real_obj': t,
                'perceived_pos': perceived_pos
            })

        # 2. Clear old assignments if targets are gone
        # (Simplified: we just recalculate all every few frames in this sim)
        self.assigned_targets = {}

        # 3. Bidding Process
        # For each target, find the best interceptor
        available_interceptors = [i for i in interceptors if i.active and i.target is None]
        
        for t_data in noisy_targets:
            best_bid = float('inf')
            best_interceptor = None
            
            for inter in available_interceptors:
                if inter in self.assigned_targets.values(): # Already assigned in this round?
                    continue
                
                # Bid = Distance to perceived target (lower is better)
                dist = np.linalg.norm(inter.position - t_data['perceived_pos'])
                
                # Add a small random factor to simulate communication latency/uncertainty
                bid = dist + random.uniform(-5, 5)
                
                if bid < best_bid:
                    best_bid = bid
                    best_interceptor = inter
            
            if best_interceptor:
                self.assigned_targets[best_interceptor] = t_data['real_obj']
                # Mark as unavailable for next target iteration
                available_interceptors.remove(best_interceptor)

        return self.assigned_targets

    def get_b2_vertices(self, scale: float, center: np.ndarray, heading: float):
        """
        Generates the polygon vertices for a B-2 Stealth Bomber shape.
        """
        # Basic B-2 silhouette coordinates (normalized)
        # Flying wing shape: wide, swept back
        base_shape = np.array([
            [0.0, -0.5],   # Nose
            [0.6, -0.4],   # Right Wing Tip (forward)
            [0.8, 0.2],    # Right Wing Root (back)
            [0.3, 0.5],    # Tail Center (indent)
            [-0.8, 0.2],   # Left Wing Root (back)
            [-0.6, -0.4],  # Left Wing Tip (forward)
        ])
        
        # Rotation Matrix
        rot_matrix = np.array([
            [math.cos(heading), -math.sin(heading)],
            [math.sin(heading), math.cos(heading)]
        ])
        
        transformed_points = []
        for pt in base_shape:
            # Scale
            scaled = pt * scale
            # Rotate
            rotated = np.dot(scaled, rot_matrix.T)
            # Translate to center
            final = rotated + center
            transformed_points.append(final)
            
        return transformed_points
