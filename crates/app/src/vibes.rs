//! Vibes: one click sets sound, glide, strum, reverb and octave together.

use synth::Patch;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vibe {
    pub name: &'static str,
    /// First-word name for the 68 pt key cap.
    pub short: &'static str,
    pub patch: Patch,
    pub glide_ms: f32,
    pub strum_ms: f32,
    /// 0..=1.
    pub reverb: f32,
    pub octave: i8,
}

/// In this order (the header pills select them):
/// | name | patch | glide | strum | reverb | octave |
/// | Neo-soul keys | EPiano | 160 | 25 | 0.35 | 0 |
/// | Night drive | GlidePad | 320 | 0 | 0.55 | 0 |
/// | Lo-fi pluck | Pluck | 90 | 40 | 0.30 | 0 |
/// | Glass bells | Bells | 200 | 60 | 0.50 | 1 |
/// | Choir cloud | Choir | 420 | 0 | 0.70 | 0 |
/// | String swell | StringSection | 380 | 15 | 0.45 | 0 |
pub fn vibes() -> [Vibe; 6] {
    [
        Vibe { name: "Neo-soul keys", short: "Neo-soul", patch: Patch::EPiano, glide_ms: 160.0, strum_ms: 25.0, reverb: 0.35, octave: 0 },
        Vibe { name: "Night drive", short: "Night", patch: Patch::GlidePad, glide_ms: 320.0, strum_ms: 0.0, reverb: 0.55, octave: 0 },
        Vibe { name: "Lo-fi pluck", short: "Lo-fi", patch: Patch::Pluck, glide_ms: 90.0, strum_ms: 40.0, reverb: 0.30, octave: 0 },
        Vibe { name: "Glass bells", short: "Bells", patch: Patch::Bells, glide_ms: 200.0, strum_ms: 60.0, reverb: 0.50, octave: 1 },
        Vibe { name: "Choir cloud", short: "Choir", patch: Patch::Choir, glide_ms: 420.0, strum_ms: 0.0, reverb: 0.70, octave: 0 },
        Vibe { name: "String swell", short: "Strings", patch: Patch::StringSection, glide_ms: 380.0, strum_ms: 15.0, reverb: 0.45, octave: 0 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn six_vibes_in_order() {
        let names: Vec<_> = vibes().iter().map(|v| v.name).collect();
        assert_eq!(names, ["Neo-soul keys", "Night drive", "Lo-fi pluck", "Glass bells", "Choir cloud", "String swell"]);
        let short: Vec<_> = vibes().iter().map(|v| v.short).collect();
        assert_eq!(short, ["Neo-soul", "Night", "Lo-fi", "Bells", "Choir", "Strings"]);
        let patches: Vec<_> = vibes().iter().map(|v| v.patch).collect();
        assert_eq!(patches, [Patch::EPiano, Patch::GlidePad, Patch::Pluck, Patch::Bells, Patch::Choir, Patch::StringSection]);
    }

    #[test]
    fn values_match_the_table() {
        let rows: Vec<_> = vibes().iter().map(|v| (v.glide_ms, v.strum_ms, v.reverb, v.octave)).collect();
        assert_eq!(rows, [(160.0, 25.0, 0.35, 0), (320.0, 0.0, 0.55, 0), (90.0, 40.0, 0.30, 0),
            (200.0, 60.0, 0.50, 1), (420.0, 0.0, 0.70, 0), (380.0, 15.0, 0.45, 0)]);
    }

    #[test]
    fn values_are_in_range() {
        for v in vibes() {
            assert!((0.0..=800.0).contains(&v.glide_ms) && (0.0..=160.0).contains(&v.strum_ms));
            assert!((0.0..=1.0).contains(&v.reverb) && (-2..=2).contains(&v.octave));
        }
    }
}
