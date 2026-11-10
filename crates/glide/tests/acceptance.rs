use glide::*;

// 1 kHz sample rate: one sample per millisecond keeps the arithmetic readable.
fn p(glide: f32, strum: f32) -> Params {
    Params { glide_ms: glide, strum_ms: strum, sample_rate: 1000.0 }
}
fn close(a: Option<f32>, b: f32) -> bool {
    a.is_some_and(|a| (a - b).abs() < 1e-4)
}

#[test]
fn helpers() {
    assert_eq!(ms_to_samples(220.0, 48000.0), 10560);
    assert_eq!(ms_to_samples(0.5, 1000.0), 1, "rounds half up");
    assert_eq!(ease(0.0), 0.0);
    assert_eq!(ease(1.0), 1.0);
    assert!((ease(0.5) - 0.5).abs() < 1e-6);
    assert!((ease(0.25) - 0.15625).abs() < 1e-6);
    assert_eq!(ease(-1.0), 0.0);
    assert_eq!(ease(2.0), 1.0);
    assert_eq!(strum_offset(0, 4, 60.0, 1000.0), 0);
    assert_eq!(strum_offset(1, 4, 60.0, 1000.0), 20);
    assert_eq!(strum_offset(3, 4, 60.0, 1000.0), 60);
    assert_eq!(strum_offset(0, 1, 60.0, 1000.0), 0);
}

#[test]
fn first_chord_does_not_slide() {
    let mut g = Glider::new(p(100.0, 0.0));
    assert_eq!(g.voices(), 0);
    assert_eq!(g.pitch(0, 0), None);
    g.play_now(&[48, 60, 64], 10);
    assert_eq!(g.voices(), 3);
    assert_eq!(g.pitch(0, 9), None);
    assert!(close(g.pitch(0, 10), 48.0));
    assert!(close(g.pitch(2, 500), 64.0));
    assert_eq!(g.pitch(3, 10), None);
    assert_eq!(g.lands_at(1), Some(10));
}

#[test]
fn live_slide_is_eased() {
    let mut g = Glider::new(p(100.0, 0.0));
    g.play_now(&[48, 60, 64], 0);
    g.play_now(&[50, 62, 65], 1000);
    assert!(close(g.pitch(0, 1000), 48.0));
    assert!(close(g.pitch(0, 1050), 49.0));
    assert!(close(g.pitch(0, 1100), 50.0));
    assert!(close(g.pitch(1, 1025), 60.3125));
    assert!(close(g.pitch(2, 2000), 65.0));
    assert_eq!(g.lands_at(0), Some(1100));
}

#[test]
fn loop_slide_lands_on_the_boundary() {
    let mut g = Glider::new(p(100.0, 0.0));
    g.play_now(&[48, 60, 64], 0);
    g.play_at(&[43, 57, 71], 3000);
    assert!(close(g.pitch(0, 2900), 48.0));
    assert!(close(g.pitch(0, 2950), 45.5));
    assert!(close(g.pitch(0, 3000), 43.0));
    assert_eq!(g.lands_at(2), Some(3000));
}

#[test]
fn strum_rolls_bass_first() {
    let mut g = Glider::new(p(100.0, 60.0));
    g.play_now(&[48, 60, 64, 67], 0);
    g.play_now(&[50, 62, 65, 69], 1000);
    assert_eq!(g.lands_at(0), Some(1100));
    assert_eq!(g.lands_at(3), Some(1160));
    assert!(close(g.pitch(3, 1060), 67.0), "top voice has not started yet");
    assert!(close(g.pitch(3, 1110), 68.0));
    g.play_at(&[48, 60, 64, 67], 5000);
    assert_eq!(g.lands_at(0), Some(5000));
    assert_eq!(g.lands_at(3), Some(5060));
}

#[test]
fn zero_glide_jumps() {
    let mut g = Glider::new(p(0.0, 0.0));
    g.play_now(&[48], 0);
    g.play_now(&[55], 100);
    assert!(close(g.pitch(0, 99), 48.0));
    assert!(close(g.pitch(0, 100), 55.0));
}

#[test]
fn interrupting_a_slide_starts_from_where_the_voice_is() {
    let mut g = Glider::new(p(100.0, 0.0));
    g.play_now(&[48], 0);
    g.play_now(&[50], 1000);
    g.play_now(&[43], 1050);
    assert!(close(g.pitch(0, 1050), 49.0));
    assert!(close(g.pitch(0, 1100), 46.0));
    assert!(close(g.pitch(0, 1150), 43.0));
}

#[test]
fn added_voice_starts_from_nearest_old_voice() {
    let mut g = Glider::new(p(100.0, 0.0));
    g.play_now(&[48, 60, 64], 0);
    g.play_now(&[48, 60, 64, 67], 1000);
    assert_eq!(g.voices(), 4);
    assert!(close(g.pitch(3, 1000), 64.0));
    assert!(close(g.pitch(3, 1050), 65.5));
    assert!(close(g.pitch(3, 1100), 67.0));
}

#[test]
fn dropped_voice_holds_then_stops() {
    let mut g = Glider::new(p(100.0, 40.0));
    g.play_now(&[48, 60, 64, 67], 0);
    g.play_now(&[50, 62, 65], 2000);
    assert_eq!(g.voices(), 3);
    // Last new voice (index 2) lands at 2000 + 40 + 100.
    assert!(close(g.pitch(3, 2139), 67.0));
    assert_eq!(g.pitch(3, 2140), None);
}

#[test]
fn release_silences_and_resets() {
    let mut g = Glider::new(p(100.0, 0.0));
    g.play_now(&[48, 60], 0);
    g.release(5000);
    assert!(close(g.pitch(0, 4999), 48.0));
    assert_eq!(g.pitch(0, 5000), None);
    assert_eq!(g.voices(), 0);
    g.play_now(&[55, 59], 6000);
    assert!(close(g.pitch(0, 6000), 55.0), "first chord again: no slide");
}

#[test]
fn set_params_applies_to_later_chords() {
    let mut g = Glider::new(p(100.0, 0.0));
    g.play_now(&[48], 0);
    g.set_params(p(200.0, 0.0));
    g.play_now(&[52], 1000);
    assert_eq!(g.lands_at(0), Some(1200));
}
