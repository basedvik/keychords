use chords::{root_name, Chord, Quality};

const TABLE: [(Quality, &[u8], &str, &str); 24] = [
    (Quality::Maj, &[0, 4, 7], "maj", ""),
    (Quality::Min, &[0, 3, 7], "m", "m"),
    (Quality::Dim, &[0, 3, 6], "dim", "dim"),
    (Quality::Aug, &[0, 4, 8], "aug", "aug"),
    (Quality::Maj7, &[0, 4, 7, 11], "maj7", "maj7"),
    (Quality::Min7, &[0, 3, 7, 10], "m7", "m7"),
    (Quality::Dom7, &[0, 4, 7, 10], "dom7", "7"),
    (Quality::Min7b5, &[0, 3, 6, 10], "m7♭5", "m7♭5"),
    (Quality::Sus2, &[0, 2, 7], "sus2", "sus2"),
    (Quality::Sus4, &[0, 5, 7], "sus4", "sus4"),
    (Quality::Add9, &[0, 4, 7, 14], "add9", "add9"),
    (Quality::Maj6, &[0, 4, 7, 9], "maj6", "6"),
    (Quality::Maj9, &[0, 4, 7, 11, 14], "maj9", "maj9"),
    (Quality::Min9, &[0, 3, 7, 10, 14], "m9", "m9"),
    (Quality::Dom9, &[0, 4, 7, 10, 14], "dom9", "9"),
    (Quality::Dom13, &[0, 4, 7, 10, 14, 17, 21], "dom13", "13"),
    (Quality::Dim7, &[0, 3, 6, 9], "dim7", "dim7"),
    (Quality::Dom7Sus4, &[0, 5, 7, 10], "7sus4", "7sus4"),
    (Quality::Min6, &[0, 3, 7, 9], "m6", "m6"),
    (Quality::SixNine, &[0, 4, 7, 9, 14], "6/9", "6/9"),
    (Quality::Min11, &[0, 3, 7, 10, 14, 17], "m11", "m11"),
    (Quality::Dom7b9, &[0, 4, 7, 10, 13], "7♭9", "7♭9"),
    (Quality::Dom7s9, &[0, 4, 7, 10, 15], "7♯9", "7♯9"),
    (Quality::Maj7s11, &[0, 4, 7, 11, 14, 18], "maj7♯11", "maj7♯11"),
];

#[test]
fn table_matches_for_every_quality() {
    for (i, (q, ints, label, suffix)) in TABLE.iter().enumerate() {
        assert_eq!(Quality::ALL[i], *q, "ALL order at {i}");
        assert_eq!(q.index(), i, "{q:?}.index()");
        assert_eq!(Quality::from_index(i), Some(*q));
        assert_eq!(q.intervals(), *ints, "{q:?} intervals");
        assert_eq!(q.key_label(), *label, "{q:?} key label");
        assert_eq!(q.suffix(), *suffix, "{q:?} suffix");
    }
    assert_eq!(Quality::from_index(24), None);
}

#[test]
fn root_names_use_sharps() {
    let names: Vec<_> = (0..12).map(root_name).collect();
    assert_eq!(names, ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"]);
}

#[test]
fn chord_names() {
    assert_eq!(Chord::new(2, Quality::Min9).name(), "Dm9");
    assert_eq!(Chord::new(7, Quality::Dom13).name(), "G13");
    assert_eq!(Chord::new(0, Quality::Maj9).name(), "Cmaj9");
    assert_eq!(Chord::new(9, Quality::Dom7b9).name(), "A7♭9");
    assert_eq!(Chord::new(6, Quality::Min7b5).name(), "F♯m7♭5");
    assert_eq!(Chord::new(0, Quality::Maj).name(), "C");
    assert_eq!(Chord::new(14, Quality::Min).name(), "Dm", "root wraps mod 12");
}

#[test]
fn pitch_classes_wrap_and_keep_interval_order() {
    assert_eq!(Chord::new(2, Quality::Min9).pitch_classes(), vec![2, 5, 9, 0, 4]);
    assert_eq!(Chord::new(7, Quality::Dom13).pitch_classes(), vec![7, 11, 2, 5, 9, 0, 4]);
    assert_eq!(Chord::new(11, Quality::Maj).pitch_classes(), vec![11, 3, 6]);
}
