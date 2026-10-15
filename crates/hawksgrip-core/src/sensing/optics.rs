//! Electro-optical detection physics: three bands, dust extinction, and
//! single-frame probability of detection.
//!
//! **Atmosphere.** Koschmieder's law relates meteorological visibility V to
//! the visible-band extinction coefficient: sigma = 3.912 / V (the 2 %
//! contrast threshold). Shamal dust is concentrated near the ground, so
//! extinction decays with height above the refinery pad with scale height H:
//! sigma(z) = k_band * (3.912 / V) * exp(-h / H). Along a straight path the
//! optical depth has a closed form:
//!
//!   tau = k sigma0 L H (e^{-ha/H} - e^{-hb/H}) / (hb - ha)    (ha != hb)
//!   tau = k sigma0 L e^{-h/H}                                  (level path)
//!
//! Longer wavelengths scatter less on dust, so SWIR and LWIR use k < 1
//! (SWEPT; docs/claims.md).
//!
//! **Detection.** Two independent requirements, both from standard EO practice:
//! - *resolution*: pixels on target n = size / (R * IFOV) must reach `detect_px`
//!   (Johnson's detection criterion, about 1 cycle = 2 pixels);
//! - *contrast*: apparent contrast C0 * e^{-tau} must exceed the sensor's
//!   threshold C_th.
//!
//! Each becomes a smooth logistic in log-margin, and their product times
//! pd_max is the per-eye, per-frame probability of detection.

use crate::math::{exp, ln};
use crate::vec3::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Band {
    Visible = 0,
    Swir = 1,
    Lwir = 2,
}

impl Band {
    pub const ALL: [Band; 3] = [Band::Visible, Band::Swir, Band::Lwir];

    pub fn name(self) -> &'static str {
        match self {
            Band::Visible => "VIS",
            Band::Swir => "SWIR",
            Band::Lwir => "LWIR",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BandParams {
    /// Instantaneous field of view of one pixel (milliradians).
    pub ifov_mrad: f64,
    /// Inherent target/background contrast.
    pub contrast: f64,
    /// Minimum apparent contrast the sensor can detect.
    pub contrast_threshold: f64,
    /// Extinction relative to the visible band (dust scatters less at longer wavelengths).
    pub extinction_scale: f64,
}

/// Optics-wide parameters (copied from config so this module has no config dependency).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Optics {
    pub bands: [BandParams; 3],
    pub detect_px: f64,
    pub pd_max: f64,
    pub visibility_m: f64,
    pub dust_scale_height_m: f64,
    /// Altitude the dust profile is referenced to (refinery pad).
    pub ground_ref_z: f64,
}

/// Steepness of the logistic in log-margin: margin 0.8 -> 14 %, 1.25 -> 86 %.
const SHARPNESS: f64 = 8.0;

fn logistic(x: f64) -> f64 {
    1.0 / (1.0 + exp(-x))
}

impl Optics {
    pub fn band(&self, b: Band) -> &BandParams {
        &self.bands[b as usize]
    }

    /// Ground-level extinction coefficient for a band (1/m).
    pub fn sigma_ground(&self, b: Band) -> f64 {
        self.band(b).extinction_scale * 3.912 / self.visibility_m
    }

    /// Optical depth of the straight path a -> b (closed form, see module docs).
    pub fn optical_depth(&self, b: Band, pa: Vec3, pb: Vec3) -> f64 {
        let l = (pb - pa).norm();
        let hsc = self.dust_scale_height_m;
        let (ha, hb) = (pa.z - self.ground_ref_z, pb.z - self.ground_ref_z);
        let s0 = self.sigma_ground(b);
        let dh = hb - ha;
        if dh.abs() < 1e-6 {
            s0 * l * exp(-ha / hsc)
        } else {
            s0 * l * hsc * (exp(-ha / hsc) - exp(-hb / hsc)) / dh
        }
    }

    /// Pixels across a target of `size` metres at `range` metres.
    pub fn pixels(&self, b: Band, size: f64, range: f64) -> f64 {
        size / (range.max(1e-3) * self.band(b).ifov_mrad * 1e-3)
    }

    /// Single-eye, single-frame probability of detection.
    pub fn pd(&self, b: Band, observer: Vec3, target: Vec3, size: f64) -> f64 {
        let range = (target - observer).norm();
        let bp = self.band(b);
        let px_margin = self.pixels(b, size, range) / self.detect_px;
        let c_app = bp.contrast * exp(-self.optical_depth(b, observer, target));
        let c_margin = c_app / bp.contrast_threshold;
        if px_margin <= 0.0 || c_margin <= 0.0 {
            return 0.0;
        }
        self.pd_max * logistic(SHARPNESS * ln(px_margin)) * logistic(SHARPNESS * ln(c_margin))
    }

    /// Range at which pd falls to half of pd_max for a target `agl` metres
    /// above the reference ground, seen from `observer`. Bisection on slant
    /// range; used to pick the best band and for the HUD.
    pub fn detection_range(&self, b: Band, observer: Vec3, agl: f64, size: f64) -> f64 {
        let dz = observer.z - (self.ground_ref_z + agl);
        let at = |r: f64| {
            let h = (r * r - dz * dz).max(0.0).sqrt();
            Vec3::new(observer.x + h, observer.y, self.ground_ref_z + agl)
        };
        let half = 0.5 * self.pd_max;
        let (mut lo, mut hi) = (dz.abs().max(1.0), 60_000.0);
        if self.pd(b, observer, at(lo), size) < half {
            return 0.0;
        }
        for _ in 0..60 {
            let mid = 0.5 * (lo + hi);
            if self.pd(b, observer, at(mid), size) >= half {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo
    }

    /// The band with the longest detection range for the reference geometry.
    /// Ties go to the earlier band (VIS before SWIR before LWIR).
    pub fn best_band(&self, observer: Vec3, agl: f64, size: f64) -> Band {
        let mut best = (Band::Visible, f64::NEG_INFINITY);
        for b in Band::ALL {
            let r = self.detection_range(b, observer, agl, size);
            if r > best.1 + 1e-6 {
                best = (b, r);
            }
        }
        best.0
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn optics(visibility_m: f64) -> Optics {
        Optics {
            bands: [
                BandParams {
                    ifov_mrad: 0.109,
                    contrast: 0.30,
                    contrast_threshold: 0.02,
                    extinction_scale: 1.0,
                },
                BandParams {
                    ifov_mrad: 0.327,
                    contrast: 0.25,
                    contrast_threshold: 0.02,
                    extinction_scale: 0.6,
                },
                BandParams {
                    ifov_mrad: 0.327,
                    contrast: 0.15,
                    contrast_threshold: 0.01,
                    extinction_scale: 0.35,
                },
            ],
            detect_px: 2.0,
            pd_max: 0.9,
            visibility_m,
            dust_scale_height_m: 800.0,
            ground_ref_z: 0.0,
        }
    }

    #[test]
    fn koschmieder_two_percent_at_visibility() {
        // A level path at ground level, length V, transmits exp(-3.912) = 2 %.
        let o = optics(5_000.0);
        let tau = o.optical_depth(Band::Visible, Vec3::ZERO, Vec3::new(5_000.0, 0.0, 0.0));
        assert!(((-tau).exp() - 0.02).abs() < 1e-3);
    }

    #[test]
    fn closed_form_matches_numerical_integral() {
        let o = optics(1_000.0);
        let (a, b) = (
            Vec3::new(0.0, 0.0, 2_000.0),
            Vec3::new(4_000.0, 1_000.0, 30.0),
        );
        let n = 200_000;
        let l = (b - a).norm();
        let num: f64 = (0..n)
            .map(|k| {
                let p = a + (b - a) * ((k as f64 + 0.5) / n as f64);
                o.sigma_ground(Band::Swir) * (-(p.z) / 800.0).exp() * l / n as f64
            })
            .sum();
        let closed = o.optical_depth(Band::Swir, a, b);
        assert!((num - closed).abs() / closed < 1e-6, "{num} vs {closed}");
        // Symmetric in direction.
        assert!((o.optical_depth(Band::Swir, b, a) - closed).abs() < 1e-9);
    }

    #[test]
    fn pd_falls_with_range_and_dust() {
        let o = optics(20_000.0);
        let obs = Vec3::new(0.0, 0.0, 1_500.0);
        let mut prev = 1.0;
        for r in [1_000.0, 3_000.0, 6_000.0, 9_000.0, 12_000.0] {
            let p = o.pd(Band::Visible, obs, Vec3::new(r, 0.0, 100.0), 2.0);
            assert!(p <= prev + 1e-12);
            prev = p;
        }
        let dusty = optics(800.0);
        let t = Vec3::new(3_000.0, 0.0, 100.0);
        assert!(dusty.pd(Band::Visible, obs, t, 2.0) < o.pd(Band::Visible, obs, t, 2.0));
    }

    #[test]
    fn band_switches_to_thermal_in_a_shamal() {
        let obs = Vec3::new(0.0, 0.0, 1_500.0);
        let clear = optics(20_000.0);
        let storm = optics(600.0);
        assert_eq!(clear.best_band(obs, 100.0, 2.0), Band::Visible);
        assert_eq!(storm.best_band(obs, 100.0, 2.0), Band::Lwir);
        let r_vis_clear = clear.detection_range(Band::Visible, obs, 100.0, 2.0);
        // Geometric limit: 2 m / (2 px * 0.109 mrad) = 9.17 km slant range.
        assert!(
            r_vis_clear > 8_000.0 && r_vis_clear < 9_200.0,
            "{r_vis_clear}"
        );
        let r_lwir_storm = storm.detection_range(Band::Lwir, obs, 100.0, 2.0);
        let r_vis_storm = storm.detection_range(Band::Visible, obs, 100.0, 2.0);
        assert!(
            r_lwir_storm > r_vis_storm,
            "{r_lwir_storm} vs {r_vis_storm}"
        );
    }
}
