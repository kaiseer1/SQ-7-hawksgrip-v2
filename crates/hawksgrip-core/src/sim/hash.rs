//! FNV-1a 64-bit hashing over exact bit patterns, used for state fingerprints.
//! Two runs are "identical" only if every f64 matches to the last bit.

#[derive(Clone, Copy, Debug)]
pub struct Fnv64(u64);

impl Default for Fnv64 {
    fn default() -> Self {
        Self::new()
    }
}

impl Fnv64 {
    const OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01B3;

    pub fn new() -> Self {
        Self(Self::OFFSET)
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    pub fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }

    pub fn write_f64(&mut self, v: f64) {
        self.write_u64(v.to_bits());
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_reference_vectors() {
        let mut h = Fnv64::new();
        h.write_bytes(b"");
        assert_eq!(h.finish(), 0xCBF2_9CE4_8422_2325);
        let mut h = Fnv64::new();
        h.write_bytes(b"a");
        assert_eq!(h.finish(), 0xAF63_DC4C_8601_EC8C);
        let mut h = Fnv64::new();
        h.write_bytes(b"foobar");
        assert_eq!(h.finish(), 0x8594_4171_F739_67E8);
    }

    #[test]
    fn distinguishes_negative_zero() {
        let mut a = Fnv64::new();
        a.write_f64(0.0);
        let mut b = Fnv64::new();
        b.write_f64(-0.0);
        assert_ne!(a.finish(), b.finish());
    }
}
