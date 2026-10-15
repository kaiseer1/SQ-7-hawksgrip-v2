//! Transcendental functions for the simulation state path.
//!
//! `f64::sin`/`cos` call the platform C library, and Windows (MSVC CRT),
//! Linux (glibc) and macOS can disagree in the last bit. `sqrt` and basic
//! arithmetic are correctly rounded by IEEE-754 everywhere; trigonometry is
//! not. Anything that feeds simulation state therefore uses the pure-Rust
//! `libm` port (same code on every OS), so a replay recorded on one machine
//! verifies on another (ADR-0007). The same holds for exp, ln, pow and tan.
//! Test code and display code may use std.

#[inline]
pub fn sin_cos(x: f64) -> (f64, f64) {
    (libm::sin(x), libm::cos(x))
}

#[inline]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}

#[inline]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}

#[inline]
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}

#[inline]
pub fn tan(x: f64) -> f64 {
    libm::tan(x)
}

#[cfg(test)]
mod tests {
    #[test]
    fn agrees_with_std_to_an_ulp() {
        for i in 0..10_000 {
            let x = (i as f64 - 5000.0) * 0.0137;
            let (s, c) = super::sin_cos(x);
            assert!((s - x.sin()).abs() <= 2.0 * f64::EPSILON);
            assert!((c - x.cos()).abs() <= 2.0 * f64::EPSILON);
        }
    }
}
