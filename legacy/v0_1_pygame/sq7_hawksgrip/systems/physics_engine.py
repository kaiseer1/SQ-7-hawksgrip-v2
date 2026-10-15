"""
Physics Engine

Handles collision detection, boundary checks, and coordinates updates for all drones.
"""
import numpy as np
import config


class PhysicsEngine:
    def __init__(self):
        self.width = config.SCREEN_WIDTH
        self.height = config.SCREEN_HEIGHT
        self.padding = config.SIMULATION_BOUNDARY_PADDING

    def check_boundaries(self, drone):
        """
        Keeps drones within the simulation boundaries.
        Simple bounce/stop logic at edges.
        """
        x, y = drone.position
        min_x, max_x = self.padding, self.width - self.padding
        min_y, max_y = self.padding, self.height - self.padding

        hit_wall = False
        
        if x < min_x:
            drone.position[0] = min_x
            drone.velocity[0] *= -0.5
            hit_wall = True
        elif x > max_x:
            drone.position[0] = max_x
            drone.velocity[0] *= -0.5
            hit_wall = True
            
        if y < min_y:
            drone.position[1] = min_y
            drone.velocity[1] *= -0.5
            hit_wall = True
        elif y > max_y:
            drone.position[1] = max_y
            drone.velocity[1] *= -0.5
            hit_wall = True
            
        return hit_wall

    def check_collisions(self, interceptors: list, targets: list) -> list:
        """
        Checks for collisions between interceptors and targets.
        Returns a list of destroyed target objects.
        """
        destroyed = []
        
        for inter in interceptors:
            if not inter.active:
                continue
                
            for target in targets:
                if not target.active:
                    continue
                
                dist = inter.get_distance_to(target.position)
                collision_threshold = inter.radius + target.radius
                
                if dist < collision_threshold:
                    # Collision detected
                    inter.active = False
                    target.active = False
                    destroyed.append(target)
                    
        return destroyed

    def update_all(self, drones: list, dt: float):
        """
        Updates physics for a list of drones.
        """
        for drone in drones:
            if drone.active:
                drone.update(dt)
                self.check_boundaries(drone)
