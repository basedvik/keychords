//! Input mapping: computer keys → actions, and the MIDI split-point chord protocol.
//!
//! Layout rule (architecture.md §4.2): a key whose character is an ASCII letter resolves by
//! that letter wherever the layout puts it; every other key resolves by its physical
//! position, as if it were the US key in that spot. An action whose US position is a dead
//! (accent) key on the user's layout moves to `Phys::Backslash`, the key beside Enter.

use chords::{Chord, PitchClass, Quality};

/// Physical key positions, named after the US key in that position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phys {
    Backquote,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Digit0,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Space,
    /// Any letter position; letters resolve by character, never by position.
    Letter,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Letter {
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

impl Letter {
    /// C=0 D=2 E=4 F=5 G=7 A=9 B=11.
    pub fn pitch_class(self) -> PitchClass {
        todo!()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    /// A root letter; `sharp` is true when Shift was held.
    Root { letter: Letter, sharp: bool },
    Quality(Quality),
    /// Sound preset slot 0..=5 on keys 1..=6.
    Sound(u8),
    OctaveDown,
    OctaveUp,
    /// Silent slot / stop.
    Stop,
    Flat,
    Sharp,
    /// Space: play / stop.
    Transport,
}

/// One key press as the UI reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyInput {
    /// The character the layout produced, if any (unshifted or shifted, either case).
    pub character: Option<char>,
    pub phys: Phys,
    pub shift: bool,
}

/// US-layout map.
///
/// Letters (by character, case-insensitive):
///   A B C D E F G → Root; Q → OctaveDown; W → OctaveUp; S → Stop; Z → Flat; X → Sharp;
///   Y 7♯9, U maj7, I m7, O dom7, P m7♭5, H 7♭9, J sus2, K sus4, L add9, N maj7♯11, M maj9;
///   R T V → nothing.
/// Positions:
///   Digit1..Digit6 → Sound(0..5); Digit7 maj; Digit8 m; Digit9 dim; Digit0 aug; Minus dim7;
///   Equal 7sus4; BracketLeft m6; BracketRight 6/9; Semicolon maj6; Quote m11; Comma m9;
///   Period dom9; Slash dom13; Space → Transport; Backquote, Backslash, Other → nothing.
#[derive(Clone, Debug, Default)]
pub struct KeyMap {
    dead: Vec<Phys>,
}

impl KeyMap {
    /// `dead` lists positions that are dead/accent keys on the user's layout.
    pub fn new(dead: &[Phys]) -> KeyMap {
        KeyMap { dead: dead.to_vec() }
    }

    pub fn resolve(&self, input: KeyInput) -> Option<KeyAction> {
        todo!()
    }
}

/// Decodes the two-note MIDI chord protocol: any note below `split` gives the root
/// (its pitch class); notes `split..split+24` give the quality (`Quality::from_index(note -
/// split)`); notes at `split+24` and above are ignored. A chord is emitted by whichever of
/// the pair arrives second, and again whenever either half changes while the other is held.
#[derive(Clone, Debug)]
pub struct SplitDecoder {
    pub split: u8,
    root: Option<u8>,
    quality: Option<u8>,
}

impl SplitDecoder {
    pub fn new(split: u8) -> SplitDecoder {
        SplitDecoder { split, root: None, quality: None }
    }

    pub fn note_on(&mut self, note: u8) -> Option<Chord> {
        todo!()
    }

    /// Returns true when this releases a held half of the current chord.
    pub fn note_off(&mut self, note: u8) -> bool {
        todo!()
    }
}
