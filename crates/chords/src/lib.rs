//! Chord dictionary: 24 chord qualities on any of 12 roots.
//!
//! `Quality::ALL` order is load-bearing: a quality's index is its offset above the MIDI
//! keyboard split (split+0 = maj … split+23 = maj7♯11) and its ID everywhere else.

/// Pitch class, 0..12, C = 0.
pub type PitchClass = u8;
/// MIDI note number, middle C = 60 (shown as C4).
pub type Midi = u8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quality {
    Maj,
    Min,
    Dim,
    Aug,
    Maj7,
    Min7,
    Dom7,
    Min7b5,
    Sus2,
    Sus4,
    Add9,
    Maj6,
    Maj9,
    Min9,
    Dom9,
    Dom13,
    Dim7,
    Dom7Sus4,
    Min6,
    SixNine,
    Min11,
    Dom7b9,
    Dom7s9,
    Maj7s11,
}

impl Quality {
    pub const ALL: [Quality; 24] = [
        Quality::Maj,
        Quality::Min,
        Quality::Dim,
        Quality::Aug,
        Quality::Maj7,
        Quality::Min7,
        Quality::Dom7,
        Quality::Min7b5,
        Quality::Sus2,
        Quality::Sus4,
        Quality::Add9,
        Quality::Maj6,
        Quality::Maj9,
        Quality::Min9,
        Quality::Dom9,
        Quality::Dom13,
        Quality::Dim7,
        Quality::Dom7Sus4,
        Quality::Min6,
        Quality::SixNine,
        Quality::Min11,
        Quality::Dom7b9,
        Quality::Dom7s9,
        Quality::Maj7s11,
    ];

    /// Position in `ALL`.
    pub fn index(self) -> usize {
        todo!()
    }

    /// Inverse of `index`; `None` for 24 and above.
    pub fn from_index(i: usize) -> Option<Quality> {
        todo!()
    }

    /// Semitones above the root, ascending, starting with 0. Full textbook spelling
    /// (dom13 includes the 11th as 17); see the table in tests/acceptance.rs.
    pub fn intervals(self) -> &'static [u8] {
        todo!()
    }

    /// The label printed on the chord key: "maj", "m", "dim", "aug", "maj7", "m7", "dom7",
    /// "m7♭5", "sus2", "sus4", "add9", "maj6", "maj9", "m9", "dom9", "dom13", "dim7",
    /// "7sus4", "m6", "6/9", "m11", "7♭9", "7♯9", "maj7♯11".
    pub fn key_label(self) -> &'static str {
        todo!()
    }

    /// Suffix used in chord names: "" (maj), "m", "dim", "aug", "maj7", "m7", "7", "m7♭5",
    /// "sus2", "sus4", "add9", "6", "maj9", "m9", "9", "13", "dim7", "7sus4", "m6", "6/9",
    /// "m11", "7♭9", "7♯9", "maj7♯11".
    pub fn suffix(self) -> &'static str {
        todo!()
    }
}

/// Root names, spelled with sharps: C C♯ D D♯ E F F♯ G G♯ A A♯ B.
pub fn root_name(pc: PitchClass) -> &'static str {
    todo!()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub root: PitchClass,
    pub quality: Quality,
}

impl Chord {
    pub fn new(root: PitchClass, quality: Quality) -> Chord {
        Chord { root: root % 12, quality }
    }

    /// Distinct pitch classes of the chord, in interval order (root first), each
    /// `(root + interval) % 12`.
    pub fn pitch_classes(&self) -> Vec<PitchClass> {
        todo!()
    }

    /// Root name + suffix, e.g. "Dm9", "G13", "Cmaj9", "A7♭9", "F♯m7♭5", "C".
    pub fn name(&self) -> String {
        todo!()
    }
}
