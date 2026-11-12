use chords::{Chord, Quality::*};
use keymap::{KeyAction, KeyAction::*, KeyInput, KeyMap, Letter, Phys, SplitDecoder};

fn letter(c: char) -> KeyInput {
    KeyInput { character: Some(c), phys: Phys::Letter, shift: false }
}
fn pos(p: Phys, c: char) -> KeyInput {
    KeyInput { character: Some(c), phys: p, shift: false }
}

#[test]
fn letter_pitch_classes() {
    let pcs: Vec<_> = [Letter::C, Letter::D, Letter::E, Letter::F, Letter::G, Letter::A, Letter::B]
        .iter().map(|l| l.pitch_class()).collect();
    assert_eq!(pcs, [0, 2, 4, 5, 7, 9, 11]);
}

#[test]
fn us_letters() {
    let m = KeyMap::new(&[]);
    let r = |c| m.resolve(letter(c));
    assert_eq!(r('d'), Some(Root { letter: Letter::D, sharp: false }));
    assert_eq!(r('B'), Some(Root { letter: Letter::B, sharp: false }), "case-insensitive");
    assert_eq!(r('q'), Some(OctaveDown));
    assert_eq!(r('w'), Some(OctaveUp));
    assert_eq!(r('s'), Some(Stop));
    assert_eq!(r('z'), Some(Flat));
    assert_eq!(r('x'), Some(Sharp));
    let chord_letters = [('y', Dom7s9), ('u', Maj7), ('i', Min7), ('o', Dom7), ('p', Min7b5),
        ('h', Dom7b9), ('j', Sus2), ('k', Sus4), ('l', Add9), ('n', Maj7s11), ('m', Maj9)];
    for (c, q) in chord_letters {
        assert_eq!(r(c), Some(KeyAction::Quality(q)), "letter {c}");
    }
    for c in ['r', 't', 'v'] {
        assert_eq!(r(c), None, "letter {c}");
    }
}

#[test]
fn shift_letter_is_sharp() {
    let m = KeyMap::new(&[]);
    let i = KeyInput { character: Some('F'), phys: Phys::Letter, shift: true };
    assert_eq!(m.resolve(i), Some(Root { letter: Letter::F, sharp: true }));
}

#[test]
fn us_positions() {
    let m = KeyMap::new(&[]);
    let unused = [Phys::Digit1, Phys::Digit2, Phys::Digit3, Phys::Digit4, Phys::Digit5, Phys::Digit6];
    for p in unused {
        assert_eq!(m.resolve(pos(p, '?')), None, "{p:?}");
    }
    let chords = [(Phys::Digit7, Maj), (Phys::Digit8, Min), (Phys::Digit9, Dim), (Phys::Digit0, Aug),
        (Phys::Minus, Dim7), (Phys::Equal, Dom7Sus4), (Phys::BracketLeft, Min6),
        (Phys::BracketRight, SixNine), (Phys::Semicolon, Maj6), (Phys::Quote, Min11),
        (Phys::Comma, Min9), (Phys::Period, Dom9), (Phys::Slash, Dom13)];
    for (p, q) in chords {
        assert_eq!(m.resolve(pos(p, '?')), Some(KeyAction::Quality(q)), "{p:?}");
    }
    assert_eq!(m.resolve(pos(Phys::Space, ' ')), Some(Transport));
    assert_eq!(m.resolve(pos(Phys::Backquote, '`')), None);
    assert_eq!(m.resolve(pos(Phys::Backslash, '\\')), None);
}

#[test]
fn all_24_qualities_reachable() {
    let m = KeyMap::new(&[]);
    let mut seen = std::collections::HashSet::new();
    for c in 'a'..='z' {
        if let Some(KeyAction::Quality(q)) = m.resolve(letter(c)) { seen.insert(q); }
    }
    for p in [Phys::Digit7, Phys::Digit8, Phys::Digit9, Phys::Digit0, Phys::Minus, Phys::Equal,
        Phys::BracketLeft, Phys::BracketRight, Phys::Semicolon, Phys::Quote, Phys::Comma,
        Phys::Period, Phys::Slash] {
        if let Some(KeyAction::Quality(q)) = m.resolve(pos(p, '?')) { seen.insert(q); }
    }
    assert_eq!(seen.len(), 24);
}

#[test]
fn german_layout_umlauts_follow_position() {
    // On QWERTZ the US [ ; ' positions print Ü Ö Ä; non-letters resolve by position.
    let m = KeyMap::new(&[]);
    assert_eq!(m.resolve(pos(Phys::BracketLeft, 'ü')), Some(KeyAction::Quality(Min6)));
    assert_eq!(m.resolve(pos(Phys::Semicolon, 'ö')), Some(KeyAction::Quality(Maj6)));
    assert_eq!(m.resolve(pos(Phys::Quote, 'ä')), Some(KeyAction::Quality(Min11)));
    // Z and Y swap places physically, but resolve by letter.
    assert_eq!(m.resolve(letter('z')), Some(Flat));
    assert_eq!(m.resolve(letter('y')), Some(KeyAction::Quality(Dom7s9)));
}

#[test]
fn dead_key_action_moves_beside_enter() {
    // German ´ sits at the US Equal position.
    let m = KeyMap::new(&[Phys::Equal]);
    assert_eq!(m.resolve(pos(Phys::Equal, '´')), None);
    assert_eq!(m.resolve(pos(Phys::Backslash, '#')), Some(KeyAction::Quality(Dom7Sus4)));
}

#[test]
fn split_decoder_fl_example() {
    // "So Dm7 is D3 with F4" in FL naming: MIDI 38 + 53.
    let mut d = SplitDecoder::new(48);
    assert_eq!(d.note_on(38), None);
    assert_eq!(d.note_on(53), Some(Chord::new(2, Min7)));
}

#[test]
fn split_decoder_order_and_changes() {
    let mut d = SplitDecoder::new(48);
    assert_eq!(d.note_on(48 + 12), None, "quality first");
    assert_eq!(d.note_on(31), Some(Chord::new(7, Maj9)), "root second emits");
    assert_eq!(d.note_on(48 + 15), Some(Chord::new(7, Dom13)), "quality change re-emits");
    assert!(d.note_off(48 + 15));
    assert!(!d.note_off(100), "unrelated note");
    assert_eq!(d.note_on(48 + 24), None, "split+24 and above ignored");
}

#[test]
fn split_decoder_custom_split() {
    let mut d = SplitDecoder::new(60);
    assert_eq!(d.note_on(57), None);
    assert_eq!(d.note_on(60 + 23), Some(Chord::new(9, Maj7s11)));
}

#[test]
fn split_decoder_root_is_absolute_pitch_class() {
    // Split on F3 (53), not a C: F2 below it is still an F root.
    let mut d = SplitDecoder::new(53);
    assert_eq!(d.note_on(41), None);
    assert_eq!(d.note_on(53 + 5), Some(Chord::new(5, Min7)));
    assert!(d.note_off(41));
}
