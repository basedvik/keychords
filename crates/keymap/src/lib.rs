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
        match self {
            Letter::C => 0,
            Letter::D => 2,
            Letter::E => 4,
            Letter::F => 5,
            Letter::G => 7,
            Letter::A => 9,
            Letter::B => 11,
        }
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
        // Dead key: the position itself resolves to nothing.
        if self.dead.contains(&input.phys) {
            return None;
        }
        // Dead key's action moves to Backslash.
        if input.phys == Phys::Backslash {
            if let Some(&dp) = self.dead.iter().find(|p| Self::pos_action(**p).is_some()) {
                return Self::pos_action(dp);
            }
            return None;
        }
        // Letters resolve by character (case-insensitive), ignoring phys.
        if let Some(c) = input.character {
            if c.is_ascii_alphabetic() {
                let ch = c.to_ascii_lowercase();
                return match ch {
                    'a' => Some(KeyAction::Root { letter: Letter::A, sharp: input.shift }),
                    'b' => Some(KeyAction::Root { letter: Letter::B, sharp: input.shift }),
                    'c' => Some(KeyAction::Root { letter: Letter::C, sharp: input.shift }),
                    'd' => Some(KeyAction::Root { letter: Letter::D, sharp: input.shift }),
                    'e' => Some(KeyAction::Root { letter: Letter::E, sharp: input.shift }),
                    'f' => Some(KeyAction::Root { letter: Letter::F, sharp: input.shift }),
                    'g' => Some(KeyAction::Root { letter: Letter::G, sharp: input.shift }),
                    'q' => Some(KeyAction::OctaveDown),
                    'w' => Some(KeyAction::OctaveUp),
                    's' => Some(KeyAction::Stop),
                    'z' => Some(KeyAction::Flat),
                    'x' => Some(KeyAction::Sharp),
                    'y' => Some(KeyAction::Quality(Quality::ALL[22])),
                    'u' => Some(KeyAction::Quality(Quality::ALL[4])),
                    'i' => Some(KeyAction::Quality(Quality::ALL[5])),
                    'o' => Some(KeyAction::Quality(Quality::ALL[6])),
                    'p' => Some(KeyAction::Quality(Quality::ALL[7])),
                    'h' => Some(KeyAction::Quality(Quality::ALL[21])),
                    'j' => Some(KeyAction::Quality(Quality::ALL[8])),
                    'k' => Some(KeyAction::Quality(Quality::ALL[9])),
                    'l' => Some(KeyAction::Quality(Quality::ALL[10])),
                    'n' => Some(KeyAction::Quality(Quality::ALL[23])),
                    'm' => Some(KeyAction::Quality(Quality::ALL[12])),
                    _ => None,
                };
            }
        }
        // Resolve by physical position.
        Self::pos_action(input.phys)
    }

    fn pos_action(phys: Phys) -> Option<KeyAction> {
        match phys {
            Phys::Digit1 => Some(KeyAction::Sound(0)),
            Phys::Digit2 => Some(KeyAction::Sound(1)),
            Phys::Digit3 => Some(KeyAction::Sound(2)),
            Phys::Digit4 => Some(KeyAction::Sound(3)),
            Phys::Digit5 => Some(KeyAction::Sound(4)),
            Phys::Digit6 => Some(KeyAction::Sound(5)),
            Phys::Digit7 => Some(KeyAction::Quality(Quality::ALL[0])),
            Phys::Digit8 => Some(KeyAction::Quality(Quality::ALL[1])),
            Phys::Digit9 => Some(KeyAction::Quality(Quality::ALL[2])),
            Phys::Digit0 => Some(KeyAction::Quality(Quality::ALL[3])),
            Phys::Minus => Some(KeyAction::Quality(Quality::ALL[16])),
            Phys::Equal => Some(KeyAction::Quality(Quality::ALL[17])),
            Phys::BracketLeft => Some(KeyAction::Quality(Quality::ALL[18])),
            Phys::BracketRight => Some(KeyAction::Quality(Quality::ALL[19])),
            Phys::Semicolon => Some(KeyAction::Quality(Quality::ALL[11])),
            Phys::Quote => Some(KeyAction::Quality(Quality::ALL[20])),
            Phys::Comma => Some(KeyAction::Quality(Quality::ALL[13])),
            Phys::Period => Some(KeyAction::Quality(Quality::ALL[14])),
            Phys::Slash => Some(KeyAction::Quality(Quality::ALL[15])),
            Phys::Space => Some(KeyAction::Transport),
            Phys::Backquote | Phys::Backslash | Phys::Letter | Phys::Other => None,
        }
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
        if note < self.split {
            let pc = note % 12;
            self.root = Some(pc);
        } else if note < self.split + 24 {
            self.quality = Some(note - self.split);
        } else {
            // split+24 and above ignored.
        }
        match (self.root, self.quality) {
            (Some(r), Some(qi)) => Quality::ALL.get(qi as usize).map(|q| Chord::new(r, *q)),
            _ => None,
        }
    }

    /// Returns true when this releases a held half of the current chord.
    pub fn note_off(&mut self, note: u8) -> bool {
        if note < self.split {
            let pc = note % 12;
            if self.root == Some(pc) {
                self.root = None;
                return true;
            }
        } else if note < self.split + 24 {
            let idx = note - self.split;
            if Some(idx) == self.quality {
                self.quality = None;
                return true;
            }
        }
        false
    }
}
