//! Fixed-timestep simulation clock.
//!
//! Time is derived from an integer tick counter (`t = tick / hz`) instead of
//! being accumulated as `t += dt`, so it never drifts: after one simulated hour
//! at 120 Hz the clock reads exactly 3600.0 s.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    tick: u64,
    hz: u32,
}

impl Clock {
    pub fn new(hz: u32) -> Self {
        assert!(hz > 0, "tick rate must be positive");
        Self { tick: 0, hz }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn hz(&self) -> u32 {
        self.hz
    }

    pub fn dt(&self) -> f64 {
        1.0 / self.hz as f64
    }

    /// Simulated time at the current tick, in seconds.
    pub fn time(&self) -> f64 {
        self.time_at(self.tick)
    }

    pub fn time_at(&self, tick: u64) -> f64 {
        tick as f64 / self.hz as f64
    }

    pub fn advance(&mut self) {
        self.tick += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_drift_after_one_hour() {
        let mut c = Clock::new(120);
        for _ in 0..(120 * 3600) {
            c.advance();
        }
        assert_eq!(c.time(), 3600.0);

        // The naive accumulator this design avoids does drift.
        let mut t = 0.0f64;
        for _ in 0..(120 * 3600) {
            t += 1.0 / 120.0;
        }
        assert_ne!(t, 3600.0);
    }
}
