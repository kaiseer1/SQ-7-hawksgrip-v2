"""
Renderer

Handles all pygame drawing operations, including the specific B-2 shape and swarm visualization.
"""
import pygame
import config
from entities.mothership import Mothership
from entities.interceptor import Interceptor
from entities.target_drone import TargetDrone


class Renderer:
    def __init__(self, screen: pygame.Surface):
        self.screen = screen
        self.font = pygame.font.SysFont("consolas", 18)
        self.big_font = pygame.font.SysFont("consolas", 36, bold=True)

    def clear(self):
        self.screen.fill(config.COLOR_BG)

    def draw_mothership(self, mothership: Mothership):
        """Draws the B-2 Stealth Bomber shape."""
        vertices = mothership.get_b2_vertices(
            scale=config.MOTHERSHIP_SCALE,
            center=mothership.position,
            heading=mothership.heading
        )
        
        # Convert numpy arrays to tuples for pygame
        int_vertices = [(int(v[0]), int(v[1])) for v in vertices]
        
        pygame.draw.polygon(self.screen, config.COLOR_MOTHERSHIP, int_vertices)
        
        # Optional: Draw a subtle outline
        pygame.draw.polygon(self.screen, (60, 60, 70), int_vertices, 2)

    def draw_interceptor(self, inter: Interceptor):
        """Draws interceptor as a sleek triangle."""
        # Calculate tip position based on heading
        tip_x = inter.position[0] + 10 * __import__('math').cos(inter.heading)
        tip_y = inter.position[1] + 10 * __import__('math').sin(inter.heading)
        
        # Base points perpendicular to heading
        import math
        base_angle_1 = inter.heading + 2.5 # ~143 degrees
        base_angle_2 = inter.heading - 2.5
        
        bx1 = inter.position[0] + 8 * math.cos(base_angle_1)
        by1 = inter.position[1] + 8 * math.sin(base_angle_1)
        bx2 = inter.position[0] + 8 * math.cos(base_angle_2)
        by2 = inter.position[1] + 8 * math.sin(base_angle_2)
        
        pts = [(tip_x, tip_y), (bx1, by1), (bx2, by2)]
        pts_int = [(int(p[0]), int(p[1])) for p in pts]
        
        color = config.COLOR_INTERCEPTOR
        if inter.scream_triggered:
            # Flash red/white when screaming
            color = (255, 255, 255)
            
        pygame.draw.polygon(self.screen, color, pts_int)

    def draw_target(self, target: TargetDrone):
        """Draws target as a diamond shape."""
        x, y = int(target.position[0]), int(target.position[1])
        size = 8
        
        pts = [
            (x, y - size),       # Top
            (x + size, y),       # Right
            (x, y + size),       # Bottom
            (x - size, y)        # Left
        ]
        pygame.draw.polygon(self.screen, config.COLOR_TARGET, pts)
        pygame.draw.polygon(self.screen, (0, 0, 0), pts, 1) # Outline

    def draw_debug_info(self, fps: int, interceptors_count: int, targets_count: int):
        """Draws HUD information."""
        info_text = f"FPS: {fps} | Swarm: {interceptors_count} | Threats: {targets_count}"
        surf = self.font.render(info_text, True, config.COLOR_TEXT)
        self.screen.blit(surf, (10, 10))
        
        instructions = "ESC to Quit | SPACE to Spawn Wave"
        inst_surf = self.font.render(instructions, True, config.COLOR_DEBUG)
        self.screen.blit(inst_surf, (10, config.SCREEN_HEIGHT - 30))

    def draw_game_over(self, reason: str):
        """Draws Game Over screen."""
        text = self.big_font.render(reason, True, (255, 50, 50))
        rect = text.get_rect(center=(config.SCREEN_WIDTH//2, config.SCREEN_HEIGHT//2))
        self.screen.blit(text, rect)
