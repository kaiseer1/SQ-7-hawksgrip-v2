//! Deterministic random numbers: xoshiro256++ seeded through SplitMix64.
//!
//! Both are public-domain algorithms by Blackman & Vigna (2018). They are
//! implemented here (about 40 lines) rather than imported so the exact
//! sequence can never change under us when a dependency is upgraded.
//!
//! Every subsystem draws from its own *named stream*. Adding a sensor-noise
//! draw in Phase 3 therefore cannot shift the threat-spawn sequence of a
//! scenario recorded in Phase 1.

/// One SplitMix64 step. Used for seeding and stream derivation.
pub fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Independent random streams, one per subsystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum Stream {
    Scenario = 1,
    Sensor = 2,
    Auction = 3,
    Environment = 4,
    Persona = 5,
    Sweep = 6,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn from_seed(seed: u64) -> Self {
        let mut sm = seed;
        let s = [
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
            splitmix64(&mut sm),
        ];
        Self { s }
    }

    /// Generator for one named stream of a run seed.
    pub fn stream(seed: u64, stream: Stream) -> Self {
        let mut a = seed;
        let mut b = (stream as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
        Self::from_seed(splitmix64(&mut a) ^ splitmix64(&mut b))
    }

    /// Raw generator state (for state hashing).
    pub fn state(&self) -> [u64; 4] {
        self.s
    }

    pub fn next_u64(&mut self) -> u64 {
        let s = &mut self.s;
        let result = s[0].wrapping_add(s[3]).rotate_left(23).wrapping_add(s[0]);
        let t = s[1] << 17;
        s[2] ^= s[0];
        s[3] ^= s[1];
        s[1] ^= s[2];
        s[0] ^= s[3];
        s[2] ^= t;
        s[3] = s[3].rotate_left(45);
        result
    }

    /// Uniform in [0, 1) with 53 bits of precision.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in [lo, hi).
    pub fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_reference_vector() {
        // Published reference output for seed 0.
        let mut s = 0u64;
        assert_eq!(splitmix64(&mut s), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64(&mut s), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    fn same_seed_same_sequence() {
        let mut a = Rng::from_seed(42);
        let mut b = Rng::from_seed(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn streams_are_independent() {
        let mut a = Rng::stream(7, Stream::Scenario);
        let mut b = Rng::stream(7, Stream::Sensor);
        let same = (0..1000).filter(|_| a.next_u64() == b.next_u64()).count();
        assert_eq!(same, 0);
    }

    #[test]
    fn uniform_is_roughly_uniform() {
        let mut r = Rng::from_seed(1);
        let n = 200_000;
        let mut bins = [0u32; 10];
        let mut sum = 0.0;
        for _ in 0..n {
            let x = r.next_f64();
            assert!((0.0..1.0).contains(&x));
            sum += x;
            bins[(x * 10.0) as usize] += 1;
        }
        assert!((sum / n as f64 - 0.5).abs() < 0.005);
        for b in bins {
            assert!((b as f64 / n as f64 - 0.1).abs() < 0.005);
        }
    }
}
