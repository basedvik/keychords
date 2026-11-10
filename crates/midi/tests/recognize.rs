use chords::{Chord, Quality};
use midi::recognize::{best_chord, recognize};
use midi::smf::{Event, Smf, Track};

const PPQ: u32 = 120;

fn chord(root: u8, quality: Quality) -> Chord {
    Chord::new(root, quality)
}

/// One track: on every given (ch, note, start, end in beats).
fn make(ch: u8, notes: &[(u8, u32, u32)]) -> Smf {
    let mut events = Vec::new();
    for (note, s, e) in notes {
        events.push((s * PPQ, Event::NoteOn { ch, note: *note, vel: 100 }));
        events.push((e * PPQ, Event::NoteOff { ch, note: *note, vel: 0 }));
    }
    events.push((0, Event::EndOfTrack));
    Smf {
        ppq: PPQ as u16,
        tracks: vec![Track { events }],
    }
}

/// C major triad (C4 E4 G4) held from beat a to beat b, on channel `ch`.
fn c_major(a: u32, b: u32, ch: u8) -> Vec<(u32, Event)> {
    let mut ev = Vec::new();
    for note in [60u8, 64, 67] {
        ev.push((a * PPQ, Event::NoteOn { ch, note, vel: 100 }));
        ev.push((b * PPQ, Event::NoteOff { ch, note, vel: 0 }));
    }
    ev
}

#[test]
fn empty_smf_gives_no_windows() {
    let smf = Smf { ppq: 240, tracks: vec![Track { events: vec![(0, Event::EndOfTrack)] }] };
    assert_eq!(recognize(&smf), Vec::<Option<Chord>>::new());
}

#[test]
fn c_major_recognized_with_lowest_bonus() {
    let smf = make(0, &[(60, 0, 2), (64, 0, 2), (67, 0, 2)]);
    assert_eq!(recognize(&smf), vec![Some(chord(0, Quality::Maj))]);
}

#[test]
fn c_major_seventh_recognized() {
    let smf = make(0, &[(60, 0, 2), (64, 0, 2), (67, 0, 2), (71, 0, 2)]);
    assert_eq!(recognize(&smf), vec![Some(chord(0, Quality::Maj7))]);
}

#[test]
fn a_minor_recognized() {
    let smf = make(0, &[(69, 0, 2), (72, 0, 2), (76, 0, 2)]);
    assert_eq!(recognize(&smf), vec![Some(chord(9, Quality::Min))]);
}

#[test]
fn g7_and_f7_recognized_as_dom7() {
    // G7: G B D F (F = 65). The major-7th spelling (G B D F# = 66) is a different chord:
    // maj7(G).
    let smf = make(0, &[(55, 0, 2), (59, 0, 2), (62, 0, 2), (65, 0, 2)]);
    assert_eq!(recognize(&smf), vec![Some(chord(7, Quality::Dom7))]);
    // F7: F A C Eb (53 57 60 63).
    let smf = make(0, &[(53, 0, 2), (57, 0, 2), (60, 0, 2), (63, 0, 2)]);
    assert_eq!(recognize(&smf), vec![Some(chord(5, Quality::Dom7))]);
    // Gmaj7: G B D F#.
    let smf = make(0, &[(55, 0, 2), (59, 0, 2), (62, 0, 2), (66, 0, 2)]);
    assert_eq!(recognize(&smf), vec![Some(chord(7, Quality::Maj7))]);
}

#[test]
fn drums_channel_10_ignored() {
    let mut smf = make(0, &[(60, 0, 2), (64, 0, 2), (67, 0, 2)]);
    // A kick on ch 9 (pc 0) must not change the recognition.
    smf.tracks[0].events.push((0, Event::NoteOn { ch: 9, note: 36, vel: 127 }));
    smf.tracks[0].events.push((2 * PPQ, Event::NoteOff { ch: 9, note: 36, vel: 0 }));
    assert_eq!(recognize(&smf), vec![Some(chord(0, Quality::Maj))]);
    // Drums alone: no notes left.
    let drums = Smf {
        ppq: PPQ as u16,
        tracks: vec![Track {
            events: vec![
                (0, Event::NoteOn { ch: 9, note: 36, vel: 127 }),
                (2 * PPQ, Event::NoteOff { ch: 9, note: 36, vel: 0 }),
                (2 * PPQ, Event::EndOfTrack),
            ],
        }],
    };
    assert_eq!(recognize(&drums), Vec::<Option<Chord>>::new());
}

#[test]
fn note_on_zero_velocity_closes_the_note() {
    let smf = Smf {
        ppq: PPQ as u16,
        tracks: vec![Track {
            events: c_major(0, 2, 0)
                .into_iter()
                .map(|(t, e)| match e {
                    Event::NoteOff { ch, note, .. } => (t, Event::NoteOn { ch, note, vel: 0 }),
                    other => (t, other),
                })
                .chain(std::iter::once((4 * PPQ, Event::EndOfTrack)))
                .collect(),
        }],
    };
    assert_eq!(recognize(&smf), vec![Some(chord(0, Quality::Maj))]);
}

#[test]
fn note_held_at_end_runs_to_last_event() {
    let smf = Smf {
        ppq: PPQ as u16,
        tracks: vec![Track {
            events: c_major(0, 3, 0)
                .into_iter()
                .filter(|(_, e)| !matches!(e, Event::NoteOff { .. }))
                .chain(std::iter::once((5 * PPQ, Event::EndOfTrack)))
                .collect(),
        }],
    };
    // No NoteOff: the notes run to the last event (beat 5), so all three half-bar
    // windows hold C major (the last only for 120 of its 240 ticks).
    assert_eq!(recognize(&smf), vec![Some(chord(0, Quality::Maj)); 3]);
}

#[test]
fn notes_across_tracks_merge() {
    let mut smf = make(0, &[(60, 0, 2)]);
    smf.tracks.push(Track {
        events: vec![
            (0, Event::NoteOn { ch: 1, note: 64, vel: 100 }),
            (0, Event::NoteOn { ch: 2, note: 67, vel: 100 }),
            (2 * PPQ, Event::NoteOff { ch: 1, note: 64, vel: 0 }),
            (2 * PPQ, Event::NoteOff { ch: 2, note: 67, vel: 0 }),
            (2 * PPQ, Event::EndOfTrack),
        ],
    });
    assert_eq!(recognize(&smf), vec![Some(chord(0, Quality::Maj))]);
}

#[test]
fn window_length_is_two_beats_in_ppq_ticks() {
    // A whole bar (4 beats) of C major: the notes end exactly at the end of the second
    // half-bar, so both windows are full C major -- no trailing empty window.
    let smf = make(0, &[(60, 0, 4), (64, 0, 4), (67, 0, 4)]);
    assert_eq!(recognize(&smf), vec![Some(chord(0, Quality::Maj)), Some(chord(0, Quality::Maj))]);
    // With ppq doubled the window doubles in ticks, so the same 2-beat chord still fills
    // exactly one window.
    let mut smf = make(0, &[(60, 0, 2), (64, 0, 2), (67, 0, 2)]);
    smf.ppq = 240;
    assert_eq!(recognize(&smf), vec![Some(chord(0, Quality::Maj))]);
}

#[test]
fn window_crossing_note_is_weighted_by_overlap() {
    // Notes from beat 1 to beat 3 cross the boundary at beat 2: each window holds a full
    // C major, so both are recognized.
    let smf = make(0, &[(60, 1, 3), (64, 1, 3), (67, 1, 3)]);
    let out = recognize(&smf);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0], Some(chord(0, Quality::Maj)));
    assert_eq!(out[1], Some(chord(0, Quality::Maj)));
    // A chord that starts mid-window is still recognized in that window: C major from
    // beat 1 to beat 3 is full in both windows, and the lowest note (C4) is the root.
    let mid = make(0, &[(60, 1, 3), (64, 1, 3), (67, 1, 3)]);
    assert_eq!(recognize(&mid)[0], Some(chord(0, Quality::Maj)));
}

#[test]
fn length_is_windows_up_to_last_note_end() {
    // A note held 80 beats: the last end (tick 9600) falls in window 40, so the output is
    // 40 windows (0..39), well under the cap.
    let smf = Smf {
        ppq: PPQ as u16,
        tracks: vec![Track {
            events: vec![
                (0, Event::NoteOn { ch: 0, note: 60, vel: 100 }),
                (80 * PPQ, Event::EndOfTrack),
            ],
        }],
    };
    let out = recognize(&smf);
    assert_eq!(out.len(), 40);
    // A lone C still matches: C major has the root on template, two missing pcs, and the
    // lowest-pc bonus, in every window.
    assert!(out.iter().all(|c| *c == Some(chord(0, Quality::Maj))));
}

#[test]
fn capped_at_64_windows() {
    // A note held 130 beats: the last end (tick 15600) falls in window 65, so the
    // windows-up-to-the-end count (65) is capped at 64.
    let smf = Smf {
        ppq: PPQ as u16,
        tracks: vec![Track {
            events: vec![
                (0, Event::NoteOn { ch: 0, note: 60, vel: 100 }),
                (130 * PPQ, Event::EndOfTrack),
            ],
        }],
    };
    assert_eq!(recognize(&smf).len(), 64);
}

#[test]
fn lowest_note_breaks_root_ties() {
    // A and E at equal weight: six 3-note qualities cover both pcs at the same score
    // (on 2, one missing: 1.9) -- maj/min/sus2/sus4 on A, sus2 on D, sus4 on E. The
    // lowest-pc bonus (0.25) then picks between the roots.
    let mut w = [0.0; 12];
    w[9] = 1.0;
    w[4] = 1.0;
    assert_eq!(best_chord(&w, Some(9)), Some(chord(9, Quality::Maj)));
    // No lowest info: the tie goes to the lower root (D before E, E before A).
    assert_eq!(best_chord(&w, None), Some(chord(2, Quality::Sus2)));
    // Lowest pc 2 flips the bonus to D.
    assert_eq!(best_chord(&w, Some(2)), Some(chord(2, Quality::Sus2)));
}

#[test]
fn all_zero_weights_give_none() {
    let w = [0.0; 12];
    assert_eq!(best_chord(&w, Some(0)), None);
    assert_eq!(best_chord(&w, None), None);
}

#[test]
fn off_template_weights_are_penalized() {
    // G B D F (pcs 7, 11, 2, 5): dom7(G) covers all four pcs (4 + 0.25 = 4.25). maj(G)
    // leaves the F off-template (3 - 0.5 = 2.5, + 0.25) and maj7(G) also misses F#
    // (3 - 0.5 - 0.1 = 2.4, + 0.25), so the off-template penalty keeps both below dom7.
    let mut w = [0.0; 12];
    w[7] = 1.0;
    w[11] = 1.0;
    w[2] = 1.0;
    w[5] = 1.0;
    assert_eq!(best_chord(&w, Some(7)), Some(chord(7, Quality::Dom7)));
}

#[test]
fn missing_pc_penalty_makes_min7_beat_min() {
    // A minor triad plus G: min7(A) covers all four pcs (4.25); min(A) leaves the G
    // off-template (3 - 0.5 = 2.5, + 0.25). min7 wins even though it has an extra note.
    let mut w = [0.0; 12];
    w[9] = 1.0;
    w[0] = 1.0;
    w[4] = 1.0;
    w[7] = 1.0;
    assert_eq!(best_chord(&w, Some(9)), Some(chord(9, Quality::Min7)));
}

#[test]
fn missing_pc_penalty_makes_sus2_beat_maj() {
    // C D G: sus2(C) covers all three pcs (3.25); add9(C) covers them too but misses E
    // (3 - 0.1 = 2.9, + 0.25); maj(C) has the D off-template and misses E (2 - 0.5 - 0.1
    // = 1.4, + 0.25). The missing-pc penalty keeps sus2 on top.
    let mut w = [0.0; 12];
    w[0] = 1.0;
    w[2] = 1.0;
    w[7] = 1.0;
    assert_eq!(best_chord(&w, Some(0)), Some(chord(0, Quality::Sus2)));
}

#[test]
fn fewer_notes_win_score_ties() {
    // Only pc 0 sounds: every quality has its root on template, so the score is
    // 1 - 0.1 * missing * 1. Fewer missing pcs (i.e. fewer notes) wins; the single-pc
    // template does not exist, so the best is a 3-note quality. Among those, the tie is
    // broken by quality order in ALL, then lower root: maj(0) [0,4,7] with 2 missing
    // scores 0.8, same as every other 3-note quality with 2 missing pcs, and maj comes
    // first in ALL.
    let mut w = [0.0; 12];
    w[0] = 1.0;
    assert_eq!(best_chord(&w, Some(0)), Some(chord(0, Quality::Maj)));
}

#[test]
fn d_seventh_recognized() {
    // D F# A C: dom7(D), root D.
    let smf = make(0, &[(50, 0, 2), (54, 0, 2), (57, 0, 2), (60, 0, 2)]);
    assert_eq!(recognize(&smf), vec![Some(chord(2, Quality::Dom7))]);
}

#[test]
fn best_chord_picks_the_covering_quality() {
    // E7: E G# B D.
    let mut w = [0.0; 12];
    w[4] = 1.0;
    w[8] = 1.0;
    w[11] = 1.0;
    w[2] = 1.0;
    assert_eq!(best_chord(&w, Some(4)), Some(chord(4, Quality::Dom7)));
}
