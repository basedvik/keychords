use midi::mpe::*;

#[test]
fn bend_values() {
    assert_eq!(bend_value(0.0, 48.0), 8192);
    assert_eq!(bend_value(48.0, 48.0), 16383, "clamped");
    assert_eq!(bend_value(-48.0, 48.0), 0);
    assert_eq!(bend_value(12.0, 48.0), 10240);
    assert_eq!(bend_value(-1.0, 48.0), 8021);
    assert_eq!(bend_value(1.0, 2.0), 12288);
    assert_eq!(bend_msg(3, 10240), [0xE3, 0x00, 0x50]);
    assert_eq!(bend_msg(0, 8192), [0xE0, 0x00, 0x40]);
}

#[test]
fn rpn_and_mcm() {
    assert_eq!(rpn_bend_range(2, 48), vec![
        [0xB2, 101, 0], [0xB2, 100, 0], [0xB2, 6, 48], [0xB2, 38, 0],
        [0xB2, 101, 127], [0xB2, 100, 127],
    ]);
    assert_eq!(mcm_lower_zone(7), vec![[0xB0, 101, 0], [0xB0, 100, 6], [0xB0, 6, 7]]);
}

#[test]
fn setup_configures_zone_then_members() {
    let s = MpeOut::new().setup();
    assert_eq!(s.len(), 3 + 7 * 6);
    assert_eq!(&s[..3], &mcm_lower_zone(7)[..]);
    assert_eq!(&s[3..9], &rpn_bend_range(1, 48)[..]);
    assert_eq!(&s[s.len() - 6..], &rpn_bend_range(7, 48)[..]);
    assert_eq!(MpeOut::channel(0), 1);
    assert_eq!(MpeOut::channel(6), 7);
}

#[test]
fn glide_then_land() {
    let mut m = MpeOut::new();
    assert_eq!(m.pitch(0, 50.0), None, "nothing held");
    assert_eq!(m.strike(0, 48, 100), vec![[0xE1, 0x00, 0x40], [0x91, 48, 100]]);
    // Slide 48 → 50: bends relative to the held 48.
    assert_eq!(m.pitch(0, 49.0), Some(bend_msg(1, bend_value(1.0, 48.0))));
    assert_eq!(m.pitch(0, 49.0), None, "unchanged value is not resent");
    assert_eq!(m.pitch(0, 50.0), Some(bend_msg(1, bend_value(2.0, 48.0))));
    // Lands: re-struck at 50 with the bend reset.
    assert_eq!(m.strike(0, 50, 90), vec![[0x81, 48, 0], [0xE1, 0x00, 0x40], [0x91, 50, 90]]);
    assert_eq!(m.pitch(0, 50.0), None, "centre was just sent by the strike");
    assert_eq!(m.release(0), vec![[0x81, 50, 0]]);
    assert_eq!(m.release(0), Vec::<Msg>::new(), "already released");
}

#[test]
fn voices_use_their_own_channels() {
    let mut m = MpeOut::new();
    m.strike(0, 48, 100);
    m.strike(2, 64, 100);
    assert_eq!(m.pitch(2, 63.0), Some(bend_msg(3, bend_value(-1.0, 48.0))));
    assert_eq!(m.all_off(), vec![[0x81, 48, 0], [0x83, 64, 0]]);
}

#[test]
fn plain_chords_diff() {
    let mut p = PlainOut::new(0);
    assert_eq!(p.chord(&[64, 60, 67], 100), vec![[0x90, 60, 100], [0x90, 64, 100], [0x90, 67, 100]]);
    assert_eq!(p.chord(&[60, 65, 69], 90), vec![[0x80, 64, 0], [0x80, 67, 0], [0x90, 65, 90], [0x90, 69, 90]]);
    assert_eq!(p.chord(&[60, 65, 69], 90), Vec::<Msg>::new(), "same chord: nothing");
    assert_eq!(p.all_off(), vec![[0x80, 60, 0], [0x80, 65, 0], [0x80, 69, 0]]);
    let mut ch5 = PlainOut::new(5);
    assert_eq!(ch5.chord(&[60], 1), vec![[0x95, 60, 1]]);
}
