"""
Hawksgrip v0.1 - Visualization Renderer
2D pygame display for simulation.
"""

import config

try:
    import pygame
    PYGAME_AVAILABLE = True
except ImportError:
    PYGAME_AVAILABLE = False
    print("Warning: pygame not installed. Visualization disabled.")


# Colors (RGB)
COLOR_BACKGROUND = (20, 20, 30)
COLOR_GRID = (40, 40, 50)
COLOR_MOTHERSHIP = (100, 150, 255)
COLOR_INTERCEPTOR_IDLE = (100, 255, 100)
COLOR_INTERCEPTOR_PURSUING = (255, 255, 100)
COLOR_INTERCEPTOR_COMPLETE = (150, 150, 150)
COLOR_THREAT = (255, 80, 80)
COLOR_THREAT_INACTIVE = (100, 50, 50)
COLOR_ASSET = (255, 215, 0)
COLOR_BREACH_ZONE = (255, 50, 50, 50)
COLOR_DETECTION_ZONE = (100, 150, 255, 30)
COLOR_PURSUIT_LINE = (255, 255, 100)
COLOR_TEXT = (220, 220, 220)
COLOR_TEXT_HIGHLIGHT = (255, 255, 100)


class Renderer:
    """
    Pygame-based 2D renderer for Hawksgrip simulation.
    """
    
    def __init__(self, width=800, height=800):
        """
        Args:
            width: Window width in pixels
            height: Window height in pixels
        """
        if not PYGAME_AVAILABLE:
            self.enabled = False
            return
        
        self.enabled = True
        self.width = width
        self.height = height
        
        # Calculate scale and offset to center world
        self.scale = config.RENDER_SCALE
        self.offset_x = width // 2
        self.offset_y = height // 2
        
        # Initialize pygame
        pygame.init()
        self.screen = pygame.display.set_mode((width, height))
        pygame.display.set_caption("Hawksgrip v0.1 - Counter-UAS Simulation")
        self.clock = pygame.time.Clock()
        self.font = pygame.font.SysFont("monospace", 14)
        self.font_large = pygame.font.SysFont("monospace", 18, bold=True)
        
        self.running = True
    
    def world_to_screen(self, position):
        """Convert world coordinates to screen coordinates."""
        # World: Y+ is up, Screen: Y+ is down
        sx = int(position[0] * self.scale + self.offset_x)
        sy = int(-position[1] * self.scale + self.offset_y)
        return (sx, sy)
    
    def world_to_screen_radius(self, radius):
        """Convert world radius to screen pixels."""
        return int(radius * self.scale)
    
    def handle_events(self):
        """Process pygame events. Returns False if window closed."""
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                self.running = False
                return False
            elif event.type == pygame.KEYDOWN:
                if event.key == pygame.K_ESCAPE:
                    self.running = False
                    return False
        return True
    
    def render(self, world, extra_info=None):
        """
        Render current world state.
        
        Args:
            world: World object
            extra_info: Optional dict with additional display info
        """
        if not self.enabled:
            return
        
        # Clear screen
        self.screen.fill(COLOR_BACKGROUND)
        
        # Draw grid
        self._draw_grid()
        
        # Draw detection zone (mothership range)
        if world.mothership:
            self._draw_detection_zone(world.mothership)
        
        # Draw breach zone (protected asset)
        self._draw_breach_zone()
        
        # Draw protected asset
        self._draw_asset()
        
        # Draw pursuit lines
        self._draw_pursuit_lines(world.interceptors, world.threats)
        
        # Draw threats
        for threat in world.threats:
            self._draw_threat(threat)
        
        # Draw interceptors
        for interceptor in world.interceptors:
            self._draw_interceptor(interceptor)
        
        # Draw mothership
        if world.mothership:
            self._draw_mothership(world.mothership)
        
        # Draw HUD
        self._draw_hud(world, extra_info)
        
        # Update display
        pygame.display.flip()
        self.clock.tick(config.RENDER_FPS)
    
    def _draw_grid(self):
        """Draw background grid."""
        grid_spacing = 1000  # 1km grid
        grid_px = self.world_to_screen_radius(grid_spacing)
        
        # Vertical lines
        for x in range(-5000, 5001, grid_spacing):
            sx, _ = self.world_to_screen((x, 0))
            pygame.draw.line(self.screen, COLOR_GRID, (sx, 0), (sx, self.height), 1)
        
        # Horizontal lines
        for y in range(-5000, 5001, grid_spacing):
            _, sy = self.world_to_screen((0, y))
            pygame.draw.line(self.screen, COLOR_GRID, (0, sy), (self.width, sy), 1)
    
    def _draw_detection_zone(self, mothership):
        """Draw mothership detection radius."""
        center = self.world_to_screen(mothership.position)
        radius = self.world_to_screen_radius(mothership.detection_radius)
        
        # Create transparent surface
        surf = pygame.Surface((radius * 2, radius * 2), pygame.SRCALPHA)
        pygame.draw.circle(surf, (100, 150, 255, 20), (radius, radius), radius)
        pygame.draw.circle(surf, (100, 150, 255, 60), (radius, radius), radius, 1)
        self.screen.blit(surf, (center[0] - radius, center[1] - radius))
    
    def _draw_breach_zone(self):
        """Draw protected zone breach radius."""
        center = self.world_to_screen(config.ASSET_POSITION)
        radius = self.world_to_screen_radius(config.BREACH_RADIUS)
        
        surf = pygame.Surface((radius * 2, radius * 2), pygame.SRCALPHA)
        pygame.draw.circle(surf, (255, 50, 50, 30), (radius, radius), radius)
        pygame.draw.circle(surf, (255, 50, 50, 100), (radius, radius), radius, 2)
        self.screen.blit(surf, (center[0] - radius, center[1] - radius))
    
    def _draw_asset(self):
        """Draw protected asset marker."""
        center = self.world_to_screen(config.ASSET_POSITION)
        pygame.draw.circle(self.screen, COLOR_ASSET, center, 8)
        pygame.draw.circle(self.screen, (255, 255, 255), center, 8, 2)
    
    def _draw_mothership(self, mothership):
        """Draw mothership."""
        pos = self.world_to_screen(mothership.position)
        pygame.draw.circle(self.screen, COLOR_MOTHERSHIP, pos, 12)
        pygame.draw.circle(self.screen, (255, 255, 255), pos, 12, 2)
        
        # Label
        label = self.font.render("M", True, (255, 255, 255))
        self.screen.blit(label, (pos[0] - 4, pos[1] - 6))
    
    def _draw_interceptor(self, interceptor):
        """Draw an interceptor drone."""
        pos = self.world_to_screen(interceptor.position)
        
        # Color based on state
        if interceptor.is_idle:
            color = COLOR_INTERCEPTOR_IDLE
        elif interceptor.is_pursuing:
            color = COLOR_INTERCEPTOR_PURSUING
        else:
            color = COLOR_INTERCEPTOR_COMPLETE
        
        # Draw drone
        pygame.draw.circle(self.screen, color, pos, 6)
        
        # Fuel indicator (small arc)
        if interceptor.fuel < 1.0:
            fuel_angle = interceptor.fuel * 360
            # Simple fuel bar below drone
            bar_width = 12
            bar_height = 3
            fuel_width = int(bar_width * interceptor.fuel)
            bar_x = pos[0] - bar_width // 2
            bar_y = pos[1] + 10
            pygame.draw.rect(self.screen, (50, 50, 50), (bar_x, bar_y, bar_width, bar_height))
            fuel_color = (100, 255, 100) if interceptor.fuel > 0.3 else (255, 100, 100)
            pygame.draw.rect(self.screen, fuel_color, (bar_x, bar_y, fuel_width, bar_height))
    
    def _draw_threat(self, threat):
        """Draw a threat drone."""
        pos = self.world_to_screen(threat.position)
        
        color = COLOR_THREAT if threat.is_active else COLOR_THREAT_INACTIVE
        
        # Draw as diamond shape
        size = 7
        points = [
            (pos[0], pos[1] - size),
            (pos[0] + size, pos[1]),
            (pos[0], pos[1] + size),
            (pos[0] - size, pos[1])
        ]
        pygame.draw.polygon(self.screen, color, points)
        
        if threat.is_active:
            pygame.draw.polygon(self.screen, (255, 255, 255), points, 1)
    
    def _draw_pursuit_lines(self, interceptors, threats):
        """Draw lines from pursuing interceptors to their targets."""
        for interceptor in interceptors:
            if not interceptor.is_pursuing:
                continue
            
            # Find target threat
            target = None
            for threat in threats:
                if threat.id == interceptor.assigned_threat_id:
                    target = threat
                    break
            
            if target and target.is_active:
                start = self.world_to_screen(interceptor.position)
                end = self.world_to_screen(target.position)
                pygame.draw.line(self.screen, COLOR_PURSUIT_LINE, start, end, 1)
    
    def _draw_hud(self, world, extra_info):
        """Draw heads-up display with stats."""
        y = 10
        line_height = 18
        
        # Time
        time_text = f"Time: {world.time:.1f}s"
        self._draw_text(time_text, (10, y), COLOR_TEXT)
        y += line_height
        
        # Threat count
        active_threats = len([t for t in world.threats if t.is_active])
        total_threats = len(world.threats)
        threat_text = f"Threats: {active_threats}/{total_threats}"
        self._draw_text(threat_text, (10, y), COLOR_THREAT if active_threats > 0 else COLOR_TEXT)
        y += line_height
        
        # Interceptor status
        idle = sum(1 for i in world.interceptors if i.is_idle)
        pursuing = sum(1 for i in world.interceptors if i.is_pursuing)
        intc_text = f"Interceptors: {idle} idle, {pursuing} pursuing"
        self._draw_text(intc_text, (10, y), COLOR_TEXT)
        y += line_height
        
        # Events
        hits = sum(1 for e in world.events if e[0] == "intercept")
        breaches = sum(1 for e in world.events if e[0] == "breach")
        events_text = f"Intercepts: {hits}  Breaches: {breaches}"
        color = COLOR_TEXT if breaches == 0 else (255, 100, 100)
        self._draw_text(events_text, (10, y), color)
        y += line_height
        
        # Extra info
        if extra_info:
            y += 5
            for key, value in extra_info.items():
                info_text = f"{key}: {value}"
                self._draw_text(info_text, (10, y), COLOR_TEXT)
                y += line_height
        
        # Instructions at bottom
        inst_text = "ESC to quit"
        self._draw_text(inst_text, (10, self.height - 25), (100, 100, 100))
    
    def _draw_text(self, text, position, color):
        """Render text at position."""
        surface = self.font.render(text, True, color)
        self.screen.blit(surface, position)
    
    def show_result(self, summary, duration=3.0):
        """
        Display end-of-episode result screen.
        
        Args:
            summary: Scoring summary dict
            duration: Seconds to display
        """
        if not self.enabled:
            return
        
        # Semi-transparent overlay
        overlay = pygame.Surface((self.width, self.height), pygame.SRCALPHA)
        overlay.fill((0, 0, 0, 180))
        self.screen.blit(overlay, (0, 0))
        
        # Title
        title = "EPISODE COMPLETE"
        title_surf = self.font_large.render(title, True, COLOR_TEXT_HIGHLIGHT)
        title_rect = title_surf.get_rect(center=(self.width // 2, 100))
        self.screen.blit(title_surf, title_rect)
        
        # Results
        y = 160
        results = [
            f"Intercepts: {summary['hits']}",
            f"Breaches: {summary['breaches']}",
            f"Time: {summary['time']:.1f}s",
            f"Interceptors used: {summary['interceptors_used']}",
            "",
            f"Total Reward: {summary['total_reward']:+.1f}"
        ]
        
        for line in results:
            color = COLOR_TEXT_HIGHLIGHT if "Reward" in line else COLOR_TEXT
            surf = self.font.render(line, True, color)
            rect = surf.get_rect(center=(self.width // 2, y))
            self.screen.blit(surf, rect)
            y += 25
        
        pygame.display.flip()
        
        # Wait
        start_time = pygame.time.get_ticks()
        while pygame.time.get_ticks() - start_time < duration * 1000:
            if not self.handle_events():
                break
            self.clock.tick(30)
    
    def close(self):
        """Clean up pygame resources."""
        if self.enabled:
            pygame.quit()
