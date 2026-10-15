"""
Hawksgrip v0.1 - World Environment
Manages simulation space, time, agents, and event logging.
"""

import config
from utils import math_utils


class World:
    def __init__(self):
        self.width = config.WORLD_WIDTH
        self.height = config.WORLD_HEIGHT
        self.dt = config.TIME_STEP
        self.time = 0.0
        self.mothership = None
        self.interceptors = []
        self.threats = []
        self.events = []
        self._active_threat_ids = set()
        self._interceptors_used = set()

    def reset(self):
        self.time = 0.0
        self.events = []
        self._active_threat_ids = set()
        self._interceptors_used = set()

    def register_threat(self, threat):
        self.threats.append(threat)
        self._active_threat_ids.add(threat.id)

    def register_interceptor_engagement(self, interceptor_id):
        self._interceptors_used.add(interceptor_id)

    def get_interceptors_used_count(self):
        return len(self._interceptors_used)

    def step(self):
        self.time += self.dt
        if self.mothership:
            self.mothership.update(self.dt)
        for interceptor in self.interceptors:
            interceptor.update(self.dt)
        for threat in self.threats:
            if threat.is_active:
                threat.update(self.dt)
        self._check_intercepts()
        self._check_breaches()

    def _check_intercepts(self):
        for interceptor in self.interceptors:
            if not interceptor.is_pursuing:
                continue
            for threat in self.threats:
                if not threat.is_active:
                    continue
                if threat.id not in self._active_threat_ids:
                    continue
                if threat.id != interceptor.assigned_threat_id:
                    continue
                dist = math_utils.distance(interceptor.position, threat.position)
                if dist <= config.KILL_RADIUS:
                    self.events.append(("intercept", self.time, interceptor.id, threat.id))
                    threat.is_active = False
                    self._active_threat_ids.discard(threat.id)
                    interceptor.on_intercept_complete()
                    break

    def _check_breaches(self):
        asset_pos = config.ASSET_POSITION
        for threat in self.threats:
            if not threat.is_active:
                continue
            if threat.id not in self._active_threat_ids:
                continue
            dist = math_utils.distance(threat.position, asset_pos)
            if dist <= config.BREACH_RADIUS:
                self.events.append(("breach", self.time, threat.id))
                threat.is_active = False
                self._active_threat_ids.discard(threat.id)

    def is_episode_complete(self):
        return len(self._active_threat_ids) == 0

    def get_active_threats(self):
        return [t for t in self.threats if t.id in self._active_threat_ids]

    def is_in_bounds(self, position):
        x, y = position
        half_w = self.width / 2
        half_h = self.height / 2
        return -half_w <= x <= half_w and -half_h <= y <= half_h

    def get_episode_summary(self):
        hits = sum(1 for e in self.events if e[0] == "intercept")
        breaches = sum(1 for e in self.events if e[0] == "breach")
        collisions = sum(1 for e in self.events if e[0] == "collision")
        return {
            "hits": hits,
            "breaches": breaches,
            "collisions": collisions,
            "interceptors_used": self.get_interceptors_used_count(),
            "time": self.time,
            "events": self.events.copy()
        }