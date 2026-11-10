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
        match self {
            Quality::Maj => 0,
            Quality::Min => 1,
            Quality::Dim => 2,
            Quality::Aug => 3,
            Quality::Maj7 => 4,
            Quality::Min7 => 5,
            Quality::Dom7 => 6,
            Quality::Min7b5 => 7,
            Quality::Sus2 => 8,
            Quality::Sus4 => 9,
            Quality::Add9 => 10,
            Quality::Maj6 => 11,
            Quality::Maj9 => 12,
            Quality::Min9 => 13,
            Quality::Dom9 => 14,
            Quality::Dom13 => 15,
            Quality::Dim7 => 16,
            Quality::Dom7Sus4 => 17,
            Quality::Min6 => 18,
            Quality::SixNine => 19,
            Quality::Min11 => 20,
            Quality::Dom7b9 => 21,
            Quality::Dom7s9 => 22,
            Quality::Maj7s11 => 23,
        }
    }

    /// Inverse of `index`; `None` for 24 and above.
    pub fn from_index(i: usize) -> Option<Quality> {
        match i {
            0 => Some(Quality::Maj),
            1 => Some(Quality::Min),
            2 => Some(Quality::Dim),
            3 => Some(Quality::Aug),
            4 => Some(Quality::Maj7),
            5 => Some(Quality::Min7),
            6 => Some(Quality::Dom7),
            7 => Some(Quality::Min7b5),
            8 => Some(Quality::Sus2),
            9 => Some(Quality::Sus4),
            10 => Some(Quality::Add9),
            11 => Some(Quality::Maj6),
            12 => Some(Quality::Maj9),
            13 => Some(Quality::Min9),
            14 => Some(Quality::Dom9),
            15 => Some(Quality::Dom13),
            16 => Some(Quality::Dim7),
            17 => Some(Quality::Dom7Sus4),
            18 => Some(Quality::Min6),
            19 => Some(Quality::SixNine),
            20 => Some(Quality::Min11),
            21 => Some(Quality::Dom7b9),
            22 => Some(Quality::Dom7s9),
            23 => Some(Quality::Maj7s11),
            _ => None,
        }
    }

    /// Semitones above the root, ascending, starting with 0. Full textbook spelling
    /// (dom13 includes the 11th as 17); see the table in tests/acceptance.rs.
    pub fn intervals(self) -> &'static [u8] {
        match self {
            Quality::Maj => &[0, 4, 7],
            Quality::Min => &[0, 3, 7],
            Quality::Dim => &[0, 3, 6],
            Quality::Aug => &[0, 4, 8],
            Quality::Maj7 => &[0, 4, 7, 11],
            Quality::Min7 => &[0, 3, 7, 10],
            Quality::Dom7 => &[0, 4, 7, 10],
            Quality::Min7b5 => &[0, 3, 6, 10],
            Quality::Sus2 => &[0, 2, 7],
            Quality::Sus4 => &[0, 5, 7],
            Quality::Add9 => &[0, 4, 7, 14],
            Quality::Maj6 => &[0, 4, 7, 9],
            Quality::Maj9 => &[0, 4, 7, 11, 14],
            Quality::Min9 => &[0, 3, 7, 10, 14],
            Quality::Dom9 => &[0, 4, 7, 10, 14],
            Quality::Dom13 => &[0, 4, 7, 10, 14, 17, 21],
            Quality::Dim7 => &[0, 3, 6, 9],
            Quality::Dom7Sus4 => &[0, 5, 7, 10],
            Quality::Min6 => &[0, 3, 7, 9],
            Quality::SixNine => &[0, 4, 7, 9, 14],
            Quality::Min11 => &[0, 3, 7, 10, 14, 17],
            Quality::Dom7b9 => &[0, 4, 7, 10, 13],
            Quality::Dom7s9 => &[0, 4, 7, 10, 15],
            Quality::Maj7s11 => &[0, 4, 7, 11, 14, 18],
        }
    }

    /// The label printed on the chord key: "maj", "m", "dim", "aug", "maj7", "m7", "dom7",
    /// "m7♭5", "sus2", "sus4", "add9", "maj6", "maj9", "m9", "dom9", "dom13", "dim7",
    /// "7sus4", "m6", "6/9", "m11", "7♭9", "7♯9", "maj7♯11".
    pub fn key_label(self) -> &'static str {
        match self {
            Quality::Maj => "maj",
            Quality::Min => "m",
            Quality::Dim => "dim",
            Quality::Aug => "aug",
            Quality::Maj7 => "maj7",
            Quality::Min7 => "m7",
            Quality::Dom7 => "dom7",
            Quality::Min7b5 => "m7\u{266d}5",
            Quality::Sus2 => "sus2",
            Quality::Sus4 => "sus4",
            Quality::Add9 => "add9",
            Quality::Maj6 => "maj6",
            Quality::Maj9 => "maj9",
            Quality::Min9 => "m9",
            Quality::Dom9 => "dom9",
            Quality::Dom13 => "dom13",
            Quality::Dim7 => "dim7",
            Quality::Dom7Sus4 => "7sus4",
            Quality::Min6 => "m6",
            Quality::SixNine => "6/9",
            Quality::Min11 => "m11",
            Quality::Dom7b9 => "7\u{266d}9",
            Quality::Dom7s9 => "7\u{266f}9",
            Quality::Maj7s11 => "maj7\u{266f}11",
        }
    }

    /// Suffix used in chord names: "" (maj), "m", "dim", "aug", "maj7", "m7", "7", "m7♭5",
    /// "sus2", "sus4", "add9", "6", "maj9", "m9", "9", "13", "dim7", "7sus4", "m6", "6/9",
    /// "m11", "7♭9", "7♯9", "maj7♯11".
    pub fn suffix(self) -> &'static str {
        match self {
            Quality::Maj => "",
            Quality::Min => "m",
            Quality::Dim => "dim",
            Quality::Aug => "aug",
            Quality::Maj7 => "maj7",
            Quality::Min7 => "m7",
            Quality::Dom7 => "7",
            Quality::Min7b5 => "m7\u{266d}5",
            Quality::Sus2 => "sus2",
            Quality::Sus4 => "sus4",
            Quality::Add9 => "add9",
            Quality::Maj6 => "6",
            Quality::Maj9 => "maj9",
            Quality::Min9 => "m9",
            Quality::Dom9 => "9",
            Quality::Dom13 => "13",
            Quality::Dim7 => "dim7",
            Quality::Dom7Sus4 => "7sus4",
            Quality::Min6 => "m6",
            Quality::SixNine => "6/9",
            Quality::Min11 => "m11",
            Quality::Dom7b9 => "7\u{266d}9",
            Quality::Dom7s9 => "7\u{266f}9",
            Quality::Maj7s11 => "maj7\u{266f}11",
        }
    }
}

/// Root names, spelled with sharps: C C♯ D D♯ E F F♯ G G♯ A A♯ B.
pub fn root_name(pc: PitchClass) -> &'static str {
    ["C", "C\u{266f}", "D", "D\u{266f}", "E", "F", "F\u{266f}", "G", "G\u{266f}", "A", "A\u{266f}", "B"][pc as usize]
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
        self.quality.intervals().iter().map(|&i| (self.root + i) % 12).collect()
    }

    /// Root name + suffix, e.g. "Dm9", "G13", "Cmaj9", "A7♭9", "F♯m7♭5", "C".
    pub fn name(&self) -> String {
        format!("{}{}", root_name(self.root), self.quality.suffix())
    }
}
