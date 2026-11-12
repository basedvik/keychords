//! Design tokens from design.md: the device palette, root hues around the circle of fifths,
//! and chord-family tints.

use std::sync::LazyLock;

use chords::{PitchClass, Quality};
use rune::Color;

pub const BODY: &str = "#E7E4F1";
pub const INK: &str = "#26223F";
pub const INK_SOFT: &str = "#6E6890";
pub const CAP: &str = "#F8F7FC";
pub const CAP_EDGE: &str = "#C9C4DC";
pub const SCREEN: &str = "#1E1A38";
pub const METER_LOW: &str = "#5FD38D";
pub const METER_HOT: &str = "#F6A04D";

static BODY_COLOR: LazyLock<Color> = LazyLock::new(|| parse(BODY));
static INK_COLOR: LazyLock<Color> = LazyLock::new(|| parse(INK));
static INK_SOFT_COLOR: LazyLock<Color> = LazyLock::new(|| parse(INK_SOFT));
static CAP_COLOR: LazyLock<Color> = LazyLock::new(|| parse(CAP));
static CAP_EDGE_COLOR: LazyLock<Color> = LazyLock::new(|| parse(CAP_EDGE));
static SCREEN_COLOR: LazyLock<Color> = LazyLock::new(|| parse(SCREEN));
static METER_LOW_COLOR: LazyLock<Color> = LazyLock::new(|| parse(METER_LOW));
static METER_HOT_COLOR: LazyLock<Color> = LazyLock::new(|| parse(METER_HOT));

fn parse(h: &str) -> Color {
    rune::color::hex(h).expect("valid colour")
}

pub fn body() -> Color {
    *BODY_COLOR
}
pub fn ink() -> Color {
    *INK_COLOR
}
pub fn ink_soft() -> Color {
    *INK_SOFT_COLOR
}
pub fn cap() -> Color {
    *CAP_COLOR
}
pub fn cap_edge() -> Color {
    *CAP_EDGE_COLOR
}
pub fn screen() -> Color {
    *SCREEN_COLOR
}
pub fn meter_low() -> Color {
    *METER_LOW_COLOR
}
pub fn meter_hot() -> Color {
    *METER_HOT_COLOR
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Triad,
    Seventh,
    Colour,
    Extended,
}

/// Steps around the circle of fifths from C: `(pc * 7) % 12` (C 0, G 1, D 2, … F 11).
pub fn fifths_index(pc: PitchClass) -> u8 {
    (pc * 7) % 12
}

/// The root hue in degrees, placed around the circle of fifths.
pub fn root_hue(pc: PitchClass) -> f32 {
    20.0 + 30.0 * fifths_index(pc % 12) as f32
}

/// `rune::oklch(0.72, 0.16, 20 + 30 * fifths_index(pc))`; `pc` is taken mod 12.
pub fn root_color(pc: PitchClass) -> Color {
    rune::oklch(0.72, 0.16, root_hue(pc))
}

/// A screen ribbon for voice `v` of `voices`: the bass keeps the root's full saturation and
/// the upper voices step toward a lighter tint of the same hue, so every ribbon stays in key.
pub fn voice_color(pc: PitchClass, v: usize, voices: usize) -> Color {
    let t = if voices <= 1 { 0.0 } else { v as f32 / (voices - 1) as f32 };
    rune::oklch(0.72 + 0.20 * t, 0.16 - 0.10 * t, root_hue(pc))
}

/// Triad: maj m dim aug sus2 sus4. Seventh: maj7 m7 dom7 m7♭5 dim7 7sus4.
/// Colour: add9 maj6 m6 6/9. Extended: maj9 m9 dom9 dom13 m11 7♭9 7♯9 maj7♯11.
pub fn family(q: Quality) -> Family {
    match q {
        Quality::Maj | Quality::Min | Quality::Dim | Quality::Aug | Quality::Sus2 | Quality::Sus4 => Family::Triad,
        Quality::Maj7 | Quality::Min7 | Quality::Dom7 | Quality::Min7b5 | Quality::Dim7 | Quality::Dom7Sus4 => Family::Seventh,
        Quality::Add9 | Quality::Maj6 | Quality::Min6 | Quality::SixNine => Family::Colour,
        Quality::Maj9 | Quality::Min9 | Quality::Dom9 | Quality::Dom13 | Quality::Min11 | Quality::Dom7b9 | Quality::Dom7s9 | Quality::Maj7s11 => Family::Extended,
    }
}

/// Triad oklch(0.93, 0.045, 250); Seventh oklch(0.93, 0.05, 305);
/// Colour oklch(0.94, 0.05, 160); Extended oklch(0.92, 0.06, 45).
pub fn family_tint(f: Family) -> Color {
    match f {
        Family::Triad => rune::oklch(0.93, 0.045, 250.0),
        Family::Seventh => rune::oklch(0.93, 0.05, 305.0),
        Family::Colour => rune::oklch(0.94, 0.05, 160.0),
        Family::Extended => rune::oklch(0.92, 0.06, 45.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifths_wrap_around_the_circle() {
        let order: Vec<u8> = [0u8, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10, 5].iter().map(|&pc| fifths_index(pc)).collect();
        assert_eq!(order, (0..12).collect::<Vec<u8>>(), "C G D A E B F# C# G# D# A# F");
    }

    #[test]
    fn root_colors_follow_the_formula() {
        for pc in 0..12u8 {
            let want = rune::oklch(0.72, 0.16, 20.0 + 30.0 * fifths_index(pc) as f32);
            assert_eq!(root_color(pc), want, "pc {pc}");
        }
        assert_eq!(root_color(14), root_color(2), "mod 12");
        assert_ne!(root_color(0), root_color(7), "neighbours differ");
    }

    #[test]
    fn every_quality_has_its_family() {
        use Quality::*;
        let table = [
            (Maj, Family::Triad), (Min, Family::Triad), (Dim, Family::Triad), (Aug, Family::Triad),
            (Sus2, Family::Triad), (Sus4, Family::Triad),
            (Maj7, Family::Seventh), (Min7, Family::Seventh), (Dom7, Family::Seventh),
            (Min7b5, Family::Seventh), (Dim7, Family::Seventh), (Dom7Sus4, Family::Seventh),
            (Add9, Family::Colour), (Maj6, Family::Colour), (Min6, Family::Colour), (SixNine, Family::Colour),
            (Maj9, Family::Extended), (Min9, Family::Extended), (Dom9, Family::Extended), (Dom13, Family::Extended),
            (Min11, Family::Extended), (Dom7b9, Family::Extended), (Dom7s9, Family::Extended), (Maj7s11, Family::Extended),
        ];
        assert_eq!(table.len(), 24);
        for (q, f) in table {
            assert_eq!(family(q), f, "{q:?}");
        }
    }

    #[test]
    fn family_tints_follow_the_spec() {
        assert_eq!(family_tint(Family::Triad), rune::oklch(0.93, 0.045, 250.0));
        assert_eq!(family_tint(Family::Seventh), rune::oklch(0.93, 0.05, 305.0));
        assert_eq!(family_tint(Family::Colour), rune::oklch(0.94, 0.05, 160.0));
        assert_eq!(family_tint(Family::Extended), rune::oklch(0.92, 0.06, 45.0));
    }

    #[test]
    fn palette_hexes_parse() {
        for h in [BODY, INK, INK_SOFT, CAP, CAP_EDGE, SCREEN, METER_LOW, METER_HOT] {
            assert!(rune::color::hex(h).is_some(), "{h}");
        }
    }
}
