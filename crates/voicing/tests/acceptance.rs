use chords::{Chord, Quality::*};
use voicing::*;

// Ground truth read off the vendor's draw-mode screenshot (architecture.md §6.1).
fn dm9() -> Chord { Chord::new(2, Min9) }
fn g13() -> Chord { Chord::new(7, Dom13) }
fn cmaj9() -> Chord { Chord::new(0, Maj9) }
fn a7b9() -> Chord { Chord::new(9, Dom7b9) }

#[test]
fn bass_window() {
    assert_eq!(bass_note(2, 0), 50, "D3");
    assert_eq!(bass_note(7, 0), 43, "G2");
    assert_eq!(bass_note(0, 0), 48, "C3");
    assert_eq!(bass_note(9, 0), 45, "A2");
    assert_eq!(bass_note(6, 0), 54, "F♯3");
    assert_eq!(bass_note(2, 1), 62);
    assert_eq!(bass_note(2, -1), 38);
}

#[test]
fn motion_equal_and_unequal() {
    assert_eq!(motion(&[57, 65, 71, 76], &[62, 67, 71, 76]), 7);
    assert_eq!(motion(&[60, 64, 67], &[60, 64, 67, 71]), 4);
    assert_eq!(motion(&[60, 64, 67, 71], &[60, 64, 67]), 4);
    assert_eq!(motion(&[60], &[55, 65]), 10);
    assert_eq!(motion(&[], &[]), 0);
}

#[test]
fn candidates_respect_rules() {
    let cs = candidates(cmaj9(), 0);
    assert!(cs.contains(&vec![62, 67, 71, 76]));
    assert!(cs.contains(&vec![59, 67, 74, 76]));
    assert!(!cs.iter().any(|c| c.contains(&55)), "G3 is below the band");
    for c in &cs {
        assert_eq!(c.len(), 4);
        assert!(c.windows(2).all(|w| w[0] < w[1] && w[1] - w[0] <= MAX_GAP));
        assert!(c.iter().all(|&n| (BAND_LO..=BAND_HI).contains(&n)));
        let mut pcs: Vec<_> = c.iter().map(|n| n % 12).collect();
        pcs.sort();
        assert_eq!(pcs, vec![2, 4, 7, 11]);
    }
    let mut sorted = cs.clone();
    sorted.sort();
    assert_eq!(cs, sorted, "lexicographic order");
    let up = candidates(cmaj9(), 1);
    assert!(up.contains(&vec![74, 79, 83, 88]), "octave shifts the band");
}

#[test]
fn greedy_matches_screenshot() {
    let mut vl = VoiceLeader::seeded(Voicing { bass: 50, upper: vec![57, 65, 72, 76] });
    assert_eq!(vl.next(g13()), Voicing { bass: 43, upper: vec![57, 65, 71, 76] });
    // 62,67,71,76 ties 59,67,74,76 at motion 7; it holds two voices still, so it wins.
    assert_eq!(vl.next(cmaj9()), Voicing { bass: 48, upper: vec![62, 67, 71, 76] });
    assert_eq!(vl.next(a7b9()), Voicing { bass: 45, upper: vec![61, 67, 70, 76] });
}

#[test]
fn voice_count_can_change() {
    let mut vl = VoiceLeader::seeded(Voicing { bass: 48, upper: vec![62, 67, 71, 76] });
    let v = vl.next(Chord::new(5, Maj));
    assert_eq!(v.bass, 53);
    assert_eq!(v.upper.len(), 3);
    let mut pcs: Vec<_> = v.upper.iter().map(|n| n % 12).collect();
    pcs.sort();
    assert_eq!(pcs, vec![0, 5, 9]);
}

#[test]
fn first_chord_is_deterministic_and_valid() {
    let a = VoiceLeader::new().next(dm9());
    let b = VoiceLeader::new().next(dm9());
    assert_eq!(a, b);
    assert_eq!(a.bass, 50);
    assert!(candidates(dm9(), 0).contains(&a.upper));
}

#[test]
fn not_smooth_is_close_position() {
    let mut vl = VoiceLeader::new();
    vl.smooth = false;
    assert_eq!(vl.next(cmaj9()).upper, vec![64, 67, 71, 74]);
    assert_eq!(vl.next(dm9()).upper, vec![65, 69, 72, 76]);
    vl.octave = -1;
    assert_eq!(vl.next(cmaj9()), Voicing { bass: 36, upper: vec![52, 55, 59, 62] });
}

#[test]
fn cycle_is_at_least_as_smooth_as_the_screenshot() {
    let prog = [dm9(), g13(), cmaj9(), a7b9()];
    let vs = voice_cycle(&prog, 0);
    assert_eq!(vs.len(), 4);
    let mut total = 0;
    for i in 0..4 {
        assert_eq!(vs[i].bass, bass_note(prog[i].root, 0));
        assert!(candidates(prog[i], 0).contains(&vs[i].upper), "chord {i} valid");
        total += motion(&vs[i].upper, &vs[(i + 1) % 4].upper);
    }
    // The screenshot's own voicings cost 1 + 7 + 2 + 8 = 18 around the loop.
    assert!(total <= 18, "cyclic motion {total}");
}

#[test]
fn cycle_handles_short_loops() {
    assert!(voice_cycle(&[], 0).is_empty());
    let one = voice_cycle(&[cmaj9()], 0);
    assert_eq!(one.len(), 1);
}
