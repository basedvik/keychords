use midi::smf::*;

#[test]
fn vlq_known_values() {
    let cases: [(u32, &[u8]); 7] = [
        (0, &[0x00]),
        (0x40, &[0x40]),
        (0x7F, &[0x7F]),
        (0x80, &[0x81, 0x00]),
        (0x2000, &[0xC0, 0x00]),
        (0x3FFF, &[0xFF, 0x7F]),
        (0x0FFF_FFFF, &[0xFF, 0xFF, 0xFF, 0x7F]),
    ];
    for (v, bytes) in cases {
        let mut out = Vec::new();
        write_vlq(v, &mut out);
        assert_eq!(out, bytes, "write {v:#x}");
        assert_eq!(read_vlq(bytes), Some((v, bytes.len())), "read {v:#x}");
    }
    assert_eq!(read_vlq(&[0x81]), None, "truncated");
    assert_eq!(read_vlq(&[0x81, 0x80, 0x80, 0x80, 0x00]), None, "too long");
    assert_eq!(read_vlq(&[0x81, 0x00, 0x55]), Some((0x80, 2)), "stops at the end of the quantity");
}

#[test]
fn writes_exact_bytes() {
    let smf = Smf {
        ppq: 480,
        tracks: vec![Track { events: vec![
            (0, Event::Tempo(500_000)),
            (0, Event::NoteOn { ch: 0, note: 60, vel: 100 }),
            (480, Event::NoteOff { ch: 0, note: 60, vel: 0 }),
        ] }],
    };
    let expected: Vec<u8> = [
        &b"MThd"[..], &[0, 0, 0, 6, 0, 1, 0, 1, 0x01, 0xE0],
        b"MTrk", &[0, 0, 0, 20],
        &[0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20],
        &[0x00, 0x90, 60, 100],
        &[0x83, 0x60, 0x80, 60, 0],
        &[0x00, 0xFF, 0x2F, 0x00],
    ].concat();
    assert_eq!(smf.to_bytes(), expected);
}

#[test]
fn sorts_by_tick_stably() {
    let smf = Smf { ppq: 96, tracks: vec![Track { events: vec![
        (10, Event::Marker("b".into())),
        (0, Event::Marker("a".into())),
        (10, Event::Marker("c".into())),
    ] }] };
    let back = Smf::parse(&smf.to_bytes()).unwrap();
    let names: Vec<_> = back.tracks[0].events.iter().filter_map(|(t, e)| match e {
        Event::Marker(s) => Some((*t, s.clone())), _ => None }).collect();
    assert_eq!(names, vec![(0, "a".into()), (10, "b".into()), (10, "c".into())]);
}

#[test]
fn round_trips_every_event_kind() {
    let smf = Smf {
        ppq: 480,
        tracks: vec![
            Track { events: vec![
                (0, Event::TrackName("meta".into())),
                (0, Event::Tempo(600_000)),
                (0, Event::TimeSig { num: 4, den_pow2: 2, clocks: 24, n32: 8 }),
                (0, Event::Marker("Dm9".into())),
                (0, Event::SeqSpecific(vec![0x7D, 1, 2, 3])),
                (1920, Event::EndOfTrack),
            ] },
            Track { events: vec![
                (0, Event::Cc { ch: 1, cc: 101, val: 0 }),
                (0, Event::Cc { ch: 1, cc: 100, val: 0 }),
                (0, Event::Cc { ch: 1, cc: 6, val: 48 }),
                (0, Event::NoteOn { ch: 1, note: 62, vel: 90 }),
                (100, Event::PitchBend { ch: 1, value: 8192 + 512 }),
                (200, Event::PitchBend { ch: 1, value: 0x3FFF }),
                (960, Event::NoteOff { ch: 1, note: 62, vel: 64 }),
                (960, Event::EndOfTrack),
            ] },
        ],
    };
    assert_eq!(Smf::parse(&smf.to_bytes()).unwrap(), smf);
}

#[test]
fn parses_running_status_format0_and_velocity_zero() {
    let bytes: Vec<u8> = [
        &b"MThd"[..], &[0, 0, 0, 6, 0, 0, 0, 1, 0, 96],
        b"MTrk", &[0, 0, 0, 14],
        &[0x00, 0x91, 64, 80],      // note on ch1
        &[0x10, 67, 81],            // running status: note on
        &[0x10, 64, 0],             // running status, vel 0 → note off
        &[0x00, 0xFF, 0x2F, 0x00],
    ].concat();
    let smf = Smf::parse(&bytes).unwrap();
    assert_eq!(smf.ppq, 96);
    assert_eq!(smf.tracks[0].events, vec![
        (0, Event::NoteOn { ch: 1, note: 64, vel: 80 }),
        (16, Event::NoteOn { ch: 1, note: 67, vel: 81 }),
        (32, Event::NoteOff { ch: 1, note: 64, vel: 0 }),
        (32, Event::EndOfTrack),
    ]);
}

#[test]
fn skips_unknown_meta_and_sysex() {
    let bytes: Vec<u8> = [
        &b"MThd"[..], &[0, 0, 0, 6, 0, 1, 0, 1, 0, 96],
        b"MTrk", &[0, 0, 0, 19],
        &[0x00, 0xFF, 0x01, 0x02, b'h', b'i'],   // text event: skipped
        &[0x00, 0xF0, 0x02, 0x7E, 0xF7],         // sysex: skipped
        &[0x05, 0x90, 60, 1],
        &[0x00, 0xFF, 0x2F, 0x00],
    ].concat();
    let smf = Smf::parse(&bytes).unwrap();
    assert_eq!(smf.tracks[0].events, vec![
        (5, Event::NoteOn { ch: 0, note: 60, vel: 1 }),
        (5, Event::EndOfTrack),
    ]);
}

#[test]
fn rejects_garbage() {
    assert_eq!(Smf::parse(b"RIFF....").unwrap_err(), SmfError::BadHeader);
    let mut ok = Smf { ppq: 96, tracks: vec![Track::default()] }.to_bytes();
    ok.truncate(ok.len() - 2);
    assert_eq!(Smf::parse(&ok).unwrap_err(), SmfError::Truncated);
}
