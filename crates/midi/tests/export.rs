use midi::export::{state_of, to_smf, LoopExport, PPQ, STATE_TAG};
use midi::smf::{Event, Smf, Track};

/// 120 bpm, one beat per slot, no glide, non-trivial state.
fn export(voicings: Vec<Option<Vec<u8>>>) -> LoopExport {
    let names = vec![None; voicings.len()];
    LoopExport {
        bpm: 120.0,
        slot_beats: 1.0,
        glide_ms: 0.0,
        voicings,
        names,
        state: vec![0xDE, 0xAD, 0xBE, 0xEF, 7],
    }
}

/// RPN 0 pitch-bend range setup, in the order the spec lists it.
fn setup_ccs(ch: u8, range: u8) -> Vec<(u32, Event)> {
    vec![
        (0, Event::Cc { ch, cc: 101, val: 0 }),
        (0, Event::Cc { ch, cc: 100, val: 0 }),
        (0, Event::Cc { ch, cc: 6, val: range }),
        (0, Event::Cc { ch, cc: 38, val: 0 }),
    ]
}

fn bends_of(smf: &Smf, track: usize) -> Vec<(u32, u16)> {
    smf.tracks[track]
        .events
        .iter()
        .filter_map(|(t, e)| match e {
            Event::PitchBend { value, .. } => Some((*t, *value)),
            _ => None,
        })
        .collect()
}

fn cc6(smf: &Smf, track: usize) -> u8 {
    smf.tracks[track]
        .events
        .iter()
        .find_map(|(t, e)| match e {
            Event::Cc { cc: 6, val, .. } if *t == 0 => Some(*val),
            _ => None,
        })
        .unwrap()
}

fn ease(x: f64) -> f64 {
    3.0 * x * x - 2.0 * x * x * x
}

/// The spec's bend formula at the default 48-semitone range.
fn bend48(delta: f64, x: f64) -> u16 {
    (8192.0 + delta * ease(x) / 48.0 * 8192.0).round() as u16
}

#[test]
fn writes_the_conductor_track() {
    let mut e = export(vec![
        Some(vec![36, 52, 64]),
        Some(vec![38, 55, 64]),
        None,
        Some(vec![36, 52, 64]),
    ]);
    e.names = vec![Some("C".into()), Some("D".into()), None, Some("C".into())];
    let smf = to_smf(&e);
    assert_eq!(smf.ppq, 480);
    assert_eq!(smf.tracks.len(), 4, "conductor + 3 voices");
    let mut state_payload = STATE_TAG.to_vec();
    state_payload.extend_from_slice(&e.state);
    assert_eq!(smf.tracks[0].events, vec![
        (0, Event::TrackName("keychords".into())),
        (0, Event::Tempo(500_000)),
        (0, Event::TimeSig { num: 4, den_pow2: 2, clocks: 24, n32: 8 }),
        (0, Event::Marker("C".into())),
        (480, Event::Marker("D".into())),
        (1440, Event::Marker("C".into())),
        (0, Event::SeqSpecific(state_payload.clone())),
    ]);
    let bytes = smf.to_bytes();
    assert_eq!(&bytes[8..10], &[0, 1], "format 1");
    // The writer sorts by tick (stable), so the tick-0 state event lands right after the
    // slot 0 marker; parsing appends End of Track at the last tick.
    let mut expected = smf.tracks[0].events.clone();
    expected.sort_by_key(|&(t, _)| t);
    expected.push((1440, Event::EndOfTrack));
    assert_eq!(Smf::parse(&bytes).unwrap().tracks[0].events, expected);
}

#[test]
fn strikes_voices_as_they_land() {
    let e = export(vec![
        Some(vec![36, 52, 64]),
        Some(vec![38, 55, 64]),
        None,
        Some(vec![36, 52, 64]),
    ]);
    let smf = to_smf(&e);
    // Bass: struck at 0, moved at 480 (old note off first), silenced at 960, re-struck at
    // 1440 with no note off, released at the loop end.
    let mut bass = setup_ccs(1, 48);
    bass.extend([
        (0, Event::PitchBend { ch: 1, value: 8192 }),
        (0, Event::NoteOn { ch: 1, note: 36, vel: 100 }),
        (480, Event::NoteOff { ch: 1, note: 36, vel: 0 }),
        (480, Event::PitchBend { ch: 1, value: 8192 }),
        (480, Event::NoteOn { ch: 1, note: 38, vel: 100 }),
        (960, Event::NoteOff { ch: 1, note: 38, vel: 0 }),
        (1440, Event::PitchBend { ch: 1, value: 8192 }),
        (1440, Event::NoteOn { ch: 1, note: 36, vel: 100 }),
        (1920, Event::NoteOff { ch: 1, note: 36, vel: 0 }),
    ]);
    assert_eq!(smf.tracks[1].events, bass);
    // Held through slot 1 without re-strike; re-entry after silence strikes fresh.
    let mut top = setup_ccs(3, 48);
    top.extend([
        (0, Event::PitchBend { ch: 3, value: 8192 }),
        (0, Event::NoteOn { ch: 3, note: 64, vel: 100 }),
        (960, Event::NoteOff { ch: 3, note: 64, vel: 0 }),
        (1440, Event::PitchBend { ch: 3, value: 8192 }),
        (1440, Event::NoteOn { ch: 3, note: 64, vel: 100 }),
        (1920, Event::NoteOff { ch: 3, note: 64, vel: 0 }),
    ]);
    assert_eq!(smf.tracks[3].events, top);
}

#[test]
fn markers_only_where_the_voicing_changes_and_there_is_a_name() {
    let mut e = export(vec![
        None,
        Some(vec![36]),
        Some(vec![36]),
        Some(vec![43]),
        Some(vec![43]),
        None,
    ]);
    e.names = vec![
        Some("silent slot 0".into()),
        Some("C".into()),
        Some("unchanged".into()),
        Some("G".into()),
        None,
        None,
    ];
    let markers: Vec<(u32, String)> = to_smf(&e)
        .tracks[0]
        .events
        .iter()
        .filter_map(|(t, e)| match e {
            Event::Marker(s) => Some((*t, s.clone())),
            _ => None,
        })
        .collect();
    // Slot 0 is silent, slot 2 is unchanged, and the change at slot 5 has no name.
    assert_eq!(markers, vec![(480, "C".into()), (1440, "G".into())]);
}

#[test]
fn tempo_rounds_to_the_nearest_microsecond() {
    let mut e = export(vec![Some(vec![60])]);
    e.bpm = 117.0;
    let tempo = to_smf(&e)
        .tracks[0]
        .events
        .iter()
        .find_map(|(_, e)| match e {
            Event::Tempo(us) => Some(*us),
            _ => None,
        })
        .unwrap();
    assert_eq!(tempo, 512_821);
}

#[test]
fn glides_ramp_the_bend_before_the_strike() {
    let mut e = export(vec![Some(vec![60]), Some(vec![65]), Some(vec![65])]);
    e.glide_ms = 100.0;
    let smf = to_smf(&e);
    // glide_ticks = round(0.1 * 2 * 480) = 96, so the ramp runs [384, 480).
    let mut expected: Vec<(u32, u16)> = vec![(0, 8192)];
    for t in (384u32..480).step_by(10) {
        expected.push((t, bend48(5.0, f64::from(t - 384) / 96.0)));
    }
    expected.push((480, 8192));
    assert_eq!(bends_of(&smf, 1), expected);
    // Hand-checked: centre at x = 0; 8192 + 843.75 at x = 90/96.
    assert_eq!(bends_of(&smf, 1)[1], (384, 8192));
    assert_eq!(bends_of(&smf, 1)[10], (474, 9036));
    // The strike at the slot start: old note off, bend reset, new note on.
    let at_480: Vec<Event> = smf.tracks[1]
        .events
        .iter()
        .filter_map(|(t, ev)| (*t == 480).then(|| ev.clone()))
        .collect();
    assert_eq!(at_480, vec![
        Event::NoteOff { ch: 1, note: 60, vel: 0 },
        Event::PitchBend { ch: 1, value: 8192 },
        Event::NoteOn { ch: 1, note: 65, vel: 100 },
    ]);
}

#[test]
fn downward_glides_bend_below_centre() {
    let mut e = export(vec![Some(vec![65]), Some(vec![60])]);
    e.glide_ms = 100.0;
    let bends = bends_of(&to_smf(&e), 1);
    assert!(bends.iter().all(|&(_, v)| v <= 8192), "{bends:?}");
    // 8192 - 843.75 at x = 90/96.
    assert_eq!(bends[bends.len() - 2], (474, 7348));
}

#[test]
fn the_glide_window_starts_no_earlier_than_the_previous_strike() {
    let mut e = export(vec![None, Some(vec![60]), Some(vec![65])]);
    e.glide_ms = 2000.0; // glide_ticks = 960
    let smf = to_smf(&e);
    let bends = bends_of(&smf, 1);
    // 60 is struck at 480; the ramp for 60 -> 65 would run from 0 but is clamped to 480.
    assert!(bends.iter().all(|&(t, _)| t >= 480), "{bends:?}");
    let mut expected: Vec<(u32, u16)> = vec![(480, 8192), (480, 8192)]; // strike reset, then x = 0
    for t in (490u32..960).step_by(10) {
        expected.push((t, bend48(5.0, f64::from(t - 480) / 480.0)));
    }
    expected.push((960, 8192));
    assert_eq!(bends, expected);
}

#[test]
fn a_zero_glide_restrikes_without_ramps() {
    let e = export(vec![Some(vec![60]), Some(vec![70]), Some(vec![70])]);
    assert_eq!(bends_of(&to_smf(&e), 1), vec![(0, 8192), (480, 8192)]);
}

#[test]
fn the_bend_range_widens_to_96_for_wide_slides() {
    let mut e = export(vec![Some(vec![30]), Some(vec![80])]);
    e.glide_ms = 100.0;
    assert_eq!(cc6(&to_smf(&e), 1), 96, "+50 is wider than 48");
    e.voicings = vec![Some(vec![30]), Some(vec![78])];
    assert_eq!(cc6(&to_smf(&e), 1), 48, "exactly 48 is not wider");
    e.voicings = vec![Some(vec![30]), Some(vec![127])];
    e.glide_ms = 0.0;
    assert_eq!(cc6(&to_smf(&e), 1), 48, "no glide means no slides");
}

#[test]
fn glide_values_clamp_to_the_bend_range() {
    let mut e = export(vec![Some(vec![20]), Some(vec![120])]);
    e.glide_ms = 1000.0; // 480-tick window, +100 semitones at range 96
    let smf = to_smf(&e);
    assert_eq!(cc6(&smf, 1), 96);
    let ramp: Vec<u16> = bends_of(&smf, 1)
        .into_iter()
        .take_while(|&(t, _)| t < 480)
        .map(|(_, v)| v)
        .collect();
    assert!(ramp.windows(2).all(|w| w[1] >= w[0]), "the ramp never reverses: {ramp:?}");
    assert_eq!(ramp.last(), Some(&16383), "the top of the range clamps");
}

#[test]
fn glide_values_depend_only_on_the_delta() {
    let mut e = export(vec![Some(vec![60, 70]), Some(vec![65, 75])]);
    e.glide_ms = 100.0;
    let smf = to_smf(&e);
    assert_eq!(bends_of(&smf, 1), bends_of(&smf, 2), "same delta, same bends");
}

#[test]
fn the_loop_ends_at_slots_times_slot_beats() {
    let mut e = export(vec![Some(vec![36, 52]), Some(vec![36, 52]), None, Some(vec![36])]);
    e.slot_beats = 0.01; // 4.8 ticks per slot
    let smf = to_smf(&e);
    // Slot starts round to 0, 5, 10, 14; the loop ends at round(4 * 4.8) = 19.
    let mut bass = setup_ccs(1, 48);
    bass.extend([
        (0, Event::PitchBend { ch: 1, value: 8192 }),
        (0, Event::NoteOn { ch: 1, note: 36, vel: 100 }),
        (10, Event::NoteOff { ch: 1, note: 36, vel: 0 }),
        (14, Event::PitchBend { ch: 1, value: 8192 }),
        (14, Event::NoteOn { ch: 1, note: 36, vel: 100 }),
        (19, Event::NoteOff { ch: 1, note: 36, vel: 0 }),
    ]);
    assert_eq!(smf.tracks[1].events, bass);
    // Voice 1 is silent at the end, so it holds nothing to release there.
    let mut upper = setup_ccs(2, 48);
    upper.extend([
        (0, Event::PitchBend { ch: 2, value: 8192 }),
        (0, Event::NoteOn { ch: 2, note: 52, vel: 100 }),
        (10, Event::NoteOff { ch: 2, note: 52, vel: 0 }),
    ]);
    assert_eq!(smf.tracks[2].events, upper);
}

#[test]
fn one_track_per_voice_that_ever_sounds() {
    let e = export(vec![
        Some(vec![36, 52]),
        Some(vec![36, 52, 64]),
        Some(vec![36, 52, 64, 76]),
    ]);
    let smf = to_smf(&e);
    assert_eq!(smf.tracks.len(), 5, "conductor + 4 voices");
    let ons: Vec<(u32, u8, u8)> = smf.tracks[4]
        .events
        .iter()
        .filter_map(|(t, ev)| match ev {
            Event::NoteOn { ch, note, .. } => Some((*t, *ch, *note)),
            _ => None,
        })
        .collect();
    assert_eq!(ons, vec![(960, 4, 76)], "voice 3 first sounds at slot 2, on channel 4");
}

#[test]
fn state_round_trips_verbatim() {
    let e = export(vec![Some(vec![60])]);
    let smf = to_smf(&e);
    assert_eq!(state_of(&smf), Some(e.state.clone()));
    let back = Smf::parse(&smf.to_bytes()).unwrap();
    assert_eq!(state_of(&back), Some(e.state.clone()));
}

#[test]
fn state_of_returns_none_without_the_tag() {
    for payload in [
        vec![],
        b"KCH".to_vec(),
        vec![0x00, b'K', b'C', b'H', b'D', 7],
        b"XCHD".to_vec(),
    ] {
        let smf = Smf {
            ppq: PPQ,
            tracks: vec![Track { events: vec![(0, Event::SeqSpecific(payload))] }],
        };
        assert_eq!(state_of(&smf), None);
    }
    assert_eq!(state_of(&Smf { ppq: PPQ, tracks: vec![] }), None);
    let bare_tag = Smf {
        ppq: PPQ,
        tracks: vec![Track { events: vec![(0, Event::SeqSpecific(b"KCHD".to_vec()))] }],
    };
    assert_eq!(state_of(&bare_tag), Some(vec![]), "the tag alone is an empty state");
    let two = Smf {
        ppq: PPQ,
        tracks: vec![Track { events: vec![
            (0, Event::SeqSpecific([b'K', b'C', b'H', b'D', 1].to_vec())),
            (10, Event::SeqSpecific([b'K', b'C', b'H', b'D', 2].to_vec())),
        ] }],
    };
    assert_eq!(state_of(&two), Some(vec![1]), "the first tagged event wins");
    let elsewhere = Smf {
        ppq: PPQ,
        tracks: vec![
            Track::default(),
            Track { events: vec![(0, Event::SeqSpecific([b'K', b'C', b'H', b'D', 9].to_vec()))] },
        ],
    };
    assert_eq!(state_of(&elsewhere), Some(vec![9]), "any track may carry it");
}
