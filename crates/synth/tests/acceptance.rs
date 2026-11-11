use synth::*;

const SR: f32 = 48_000.0;

fn run(s: &mut Synth, secs: f32) -> (Vec<f32>, Vec<f32>) {
    let n = (SR * secs) as usize;
    let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
    for (lc, rc) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
        s.render(lc, rc);
    }
    (l, r)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

/// Lag (in samples) of the strongest normalised autocorrelation peak in 20..=600.
fn period(x: &[f32]) -> usize {
    let best = (20..=600usize).map(|lag| {
        let (mut num, mut den) = (0.0f64, 1e-12f64);
        for i in 0..x.len() - lag {
            num += (x[i] * x[i + lag]) as f64;
            den += (x[i] * x[i]) as f64;
        }
        (lag, num / den)
    });
    // The first local maximum within 3% of the global best: avoids octave errors on
    // harmonic-rich tones, and the local-maximum check stops a broad, sine-like peak from
    // being read early (cos(2*pi*4/109) is already within 3% of the peak at 109).
    let all: Vec<_> = best.collect();
    let top = all.iter().map(|&(_, c)| c).fold(f64::MIN, f64::max);
    (1..all.len() - 1)
        .find(|&i| all[i].1 >= top * 0.97 && all[i].1 >= all[i - 1].1 && all[i].1 >= all[i + 1].1)
        .map(|i| all[i].0)
        .unwrap()
}

#[test]
fn labels_and_pitch() {
    let labels: Vec<_> = Patch::ALL.iter().map(|p| p.label()).collect();
    assert_eq!(
        labels,
        [
            "Pad", "Saw", "E-piano", "Organ", "Pluck", "Bells", "Koto", "Music box", "Banjo",
            "Sax section", "Clarinet choir", "Pizzicato strings", "12-string guitar",
            "Vibraphone", "Felt piano", "Handpan", "Kalimba", "Harp", "Choir",
            "String section", "French horns", "Theremin", "Neo-soul guitar",
        ]
    );
    assert!((midi_to_hz(69.0) - 440.0).abs() < 1e-3);
    assert!((midi_to_hz(81.0) - 880.0).abs() < 1e-2);
    assert!((midi_to_hz(57.0) - 220.0).abs() < 1e-3);
}

#[test]
fn silent_until_a_voice_sounds() {
    let mut s = Synth::new(SR);
    let (l, r) = run(&mut s, 0.1);
    assert!(l.iter().chain(&r).all(|&v| v == 0.0));
}

#[test]
fn every_patch_sounds_stays_bounded_and_releases() {
    for patch in Patch::ALL {
        let mut s = Synth::new(SR);
        s.set_patch(patch);
        for (v, p) in [48.0, 60.0, 64.0, 67.0, 71.0, 74.0, 76.0].iter().enumerate() {
            s.set_voice(v, Some(*p));
        }
        let (l, r) = run(&mut s, 0.5);
        assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() <= 1.0), "{patch:?} bounded");
        assert!(rms(&l[2400..]) > 1e-3 && rms(&r[2400..]) > 1e-3, "{patch:?} audible");
        for v in 0..MAX_VOICES {
            s.set_voice(v, None);
        }
        let _ = run(&mut s, 5.0);
        let (l, _) = run(&mut s, 0.1);
        assert!(rms(&l) < 1e-4, "{patch:?} decays after release");
    }
}

#[test]
fn sustained_patches_play_in_tune() {
    for patch in [Patch::Saw, Patch::Organ, Patch::GlidePad, Patch::EPiano] {
        let mut s = Synth::new(SR);
        s.set_patch(patch);
        s.set_voice(0, Some(69.0));
        let (l, _) = run(&mut s, 0.6);
        let lag = period(&l[9600..19200]);
        assert!((lag as i32 - 109).abs() <= 1, "{patch:?} A4 period {lag}, want ~109");
        s.set_voice(0, Some(81.0));
        let (l, _) = run(&mut s, 0.3);
        let lag = period(&l[4800..14400]);
        assert!((lag as i32 - 55).abs() <= 1, "{patch:?} A5 period {lag}, want ~54.5");
    }
}

#[test]
fn volume_scales_output() {
    let mut loud = Synth::new(SR);
    let mut quiet = Synth::new(SR);
    quiet.set_volume_db(-120.0);
    for s in [&mut loud, &mut quiet] {
        s.set_patch(Patch::Organ);
        s.set_voice(0, Some(60.0));
    }
    let (l, _) = run(&mut loud, 0.2);
    let (q, _) = run(&mut quiet, 0.2);
    assert!(rms(&l) > 1e-2);
    assert!(q.iter().all(|v| v.abs() < 1e-4));
}

#[test]
fn strike_retriggers_a_decaying_voice() {
    let mut s = Synth::new(SR);
    s.set_patch(Patch::Pluck);
    s.set_voice(0, Some(60.0));
    let _ = run(&mut s, 2.0);
    let (before, _) = run(&mut s, 0.05);
    s.strike(0);
    let (after, _) = run(&mut s, 0.05);
    assert!(rms(&after) > 4.0 * rms(&before), "strike brings the pluck back");
}
