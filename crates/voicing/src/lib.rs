//! Voicing and voice leading (architecture.md §6).
//!
//! A voicing is a bass note plus 2..=5 upper voices. The bass is the root placed in a fixed
//! register window. Upper voices realise the quality's `upper_template` pitch classes, one
//! note per pitch class, and move to the next chord by least total motion.

use chords::{Chord, Midi, PitchClass, Quality};

/// Lowest bass note before octave shift: G2.
pub const BASS_LO: Midi = 43;
/// Highest bass note before octave shift: F♯3.
pub const BASS_HI: Midi = 54;
/// Upper-voice band before octave shift: A3..=G5.
pub const BAND_LO: Midi = 57;
pub const BAND_HI: Midi = 79;
/// Largest allowed gap between adjacent upper voices, in semitones.
pub const MAX_GAP: u8 = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voicing {
    pub bass: Midi,
    /// Ascending, distinct.
    pub upper: Vec<Midi>,
}

/// Intervals above the root that the upper voices realise (only `% 12` matters).
/// Triads and sus chords keep their root up top; larger chords leave the root to the bass;
/// dom13 drops the 5th and 11th (G13 = G | A F B E).
pub fn upper_template(q: Quality) -> &'static [u8] {
    use Quality::*;
    match q {
        Maj => &[0, 4, 7],
        Min => &[0, 3, 7],
        Dim => &[0, 3, 6],
        Aug => &[0, 4, 8],
        Maj7 => &[4, 7, 11],
        Min7 => &[3, 7, 10],
        Dom7 => &[4, 7, 10],
        Min7b5 => &[3, 6, 10],
        Sus2 => &[0, 2, 7],
        Sus4 => &[0, 5, 7],
        Add9 => &[4, 7, 14],
        Maj6 => &[4, 7, 9],
        Maj9 => &[4, 7, 11, 14],
        Min9 => &[3, 7, 10, 14],
        Dom9 => &[4, 7, 10, 14],
        Dom13 => &[4, 10, 14, 21],
        Dim7 => &[3, 6, 9],
        Dom7Sus4 => &[5, 7, 10],
        Min6 => &[3, 7, 9],
        SixNine => &[4, 7, 9, 14],
        Min11 => &[3, 7, 10, 14, 17],
        Dom7b9 => &[4, 7, 10, 13],
        Dom7s9 => &[4, 7, 10, 15],
        Maj7s11 => &[4, 7, 11, 14, 18],
    }
}

/// The root's pitch class placed in `BASS_LO..=BASS_HI`, shifted by `12 * octave`.
pub fn bass_note(root: PitchClass, octave: i8) -> Midi {
    todo!()
}

/// Motion between two ascending voicings.
/// Equal lengths: Σ |a[i] − b[i]| (order-preserving, no crossing).
/// Different lengths: minimum-cost monotone alignment in which every element of both
/// sequences is matched at least once (DTW over the two sorted sequences), summing |Δ| per
/// matched pair.
pub fn motion(a: &[Midi], b: &[Midi]) -> u32 {
    todo!()
}

/// Every valid upper voicing of `chord` for `octave`: one note per template pitch class,
/// all inside `BAND_LO..=BAND_HI` (+12·octave), ascending and distinct, adjacent gaps
/// ≤ `MAX_GAP`. Returned in ascending lexicographic order.
pub fn candidates(chord: Chord, octave: i8) -> Vec<Vec<Midi>> {
    todo!()
}

/// Greedy voice leader for live play.
#[derive(Clone, Debug)]
pub struct VoiceLeader {
    /// Off: close position, each template interval added to the root placed in 60..=71
    /// (+12·octave), sorted. On: least-motion choice among `candidates`.
    pub smooth: bool,
    pub octave: i8,
    prev: Option<Voicing>,
}

impl Default for VoiceLeader {
    fn default() -> Self {
        VoiceLeader { smooth: true, octave: 0, prev: None }
    }
}

impl VoiceLeader {
    pub fn new() -> VoiceLeader {
        VoiceLeader::default()
    }

    /// Start from a known voicing, as if it had just been played.
    pub fn seeded(v: Voicing) -> VoiceLeader {
        VoiceLeader { prev: Some(v), ..VoiceLeader::default() }
    }

    pub fn reset(&mut self) {
        self.prev = None;
    }

    /// Voice `chord`. Smooth mode picks the candidate with the least `motion` from the
    /// previous upper voices; ties go to (1) more voices left exactly where they were, then
    /// (2) the smaller largest single-voice move, then (3) the lexicographically lower
    /// voicing. With no previous voicing, it picks the candidate whose mean pitch is closest
    /// to the band centre (68 + 12·octave), ties to the lexicographically lower.
    pub fn next(&mut self, chord: Chord) -> Voicing {
        todo!()
    }
}

/// Loop mode: voice a cyclic progression at once, minimising the total upper-voice `motion`
/// over every change including the wrap from the last chord back to the first. Bass notes
/// come from `bass_note`. Returns one voicing per chord, in order.
pub fn voice_cycle(chords: &[Chord], octave: i8) -> Vec<Voicing> {
    todo!()
}
