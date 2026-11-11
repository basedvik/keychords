//! "Try next": a few next chords that voice-lead smoothly from the current one and sound lush.

use chords::{Chord, PitchClass, Quality};
use voicing::{Voicing, VoiceLeader, motion};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub chord: Chord,
    /// Upper-voice motion from the current voicing (`voicing::motion`).
    pub motion: u32,
}

/// Up to `n` suggestions after `current` (the chord just played and its voicing).
///
/// 1. For every root 0..12 and every `Quality::ALL` quality, `chord = Chord::new(root, q)`;
///    skip the current chord.
/// 2. Voice it as the app would: `VoiceLeader::seeded(current.1.clone())` with `octave` and
///    `smooth` set, then `.next(chord)`.
/// 3. `motion = voicing::motion(&current.1.upper, &v.upper)`.
/// 4. `score = motion as i32 - lush`, `lush = 2` when `q.intervals().len() >= 4`, else 0.
/// 5. Sort by `(score, fifths distance between roots, quality index)`, where fifths distance
///    is `min(d, 12 - d)` with `d = (fifths_index(root) + 12 - fifths_index(current root)) % 12`
///    (`crate::theme::fifths_index`). Take the first `n`.
pub fn suggest(current: &(Chord, Voicing), octave: i8, smooth: bool, n: usize) -> Vec<Suggestion> {
    let (cur_chord, cur_voicing) = current;
    let cur_root = cur_chord.root;
    let mut out: Vec<(i32, u8, usize, Suggestion)> = Vec::new();
    for root in 0..12 {
        let root = root as PitchClass;
        for q in Quality::ALL {
            let chord = Chord::new(root, q);
            if chord == *cur_chord {
                continue;
            }
            let mut vl = VoiceLeader::seeded(cur_voicing.clone());
            vl.octave = octave;
            vl.smooth = smooth;
            let v = vl.next(chord);
            let m = motion(&cur_voicing.upper, &v.upper);
            let lush = if q.intervals().len() >= 4 { 2 } else { 0 };
            let score = m as i32 - lush;
            let d = (crate::theme::fifths_index(root) + 12 - crate::theme::fifths_index(cur_root)) % 12;
            let fifths_dist = d.min(12 - d);
            out.push((score, fifths_dist, q.index(), Suggestion { chord, motion: m }));
        }
    }
    out.sort_by_key(|a| (a.0, a.1, a.2));
    out.into_iter().take(n).map(|(_, _, _, s)| s).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dm9() -> (Chord, Voicing) {
        (Chord::new(2, Quality::Min9), Voicing { bass: 50, upper: vec![57, 65, 72, 76] })
    }

    fn score(s: &Suggestion) -> i32 {
        s.motion as i32 - if s.chord.quality.intervals().len() >= 4 { 2 } else { 0 }
    }

    #[test]
    fn returns_n_sorted_by_score_and_never_the_current_chord() {
        let cur = dm9();
        let out = suggest(&cur, 0, true, 3);
        assert_eq!(out.len(), 3);
        assert!(out.iter().all(|s| s.chord != cur.0));
        assert!(out.windows(2).all(|w| score(&w[0]) <= score(&w[1])));
        let many = suggest(&cur, 0, true, 50);
        assert_eq!(many.len(), 50);
        assert!(many.windows(2).all(|w| score(&w[0]) <= score(&w[1])));
    }

    #[test]
    fn g13_follows_dm9() {
        let out = suggest(&dm9(), 0, true, 3);
        let g13 = out.iter().find(|s| s.chord == Chord::new(7, Quality::Dom13));
        assert_eq!(g13.map(|s| s.motion), Some(1), "G13 is in the top 3 with motion 1: {out:?}");
    }

    #[test]
    fn motion_is_what_the_voice_leader_would_do() {
        let cur = dm9();
        for s in suggest(&cur, 0, true, 10) {
            let mut vl = voicing::VoiceLeader::seeded(cur.1.clone());
            let v = vl.next(s.chord);
            assert_eq!(s.motion, voicing::motion(&cur.1.upper, &v.upper), "{:?}", s.chord);
        }
    }

    #[test]
    fn zero_asked_is_empty() {
        assert!(suggest(&dm9(), 0, true, 0).is_empty());
    }

    #[test]
    fn total_candidates_is_all_chords_but_one() {
        assert_eq!(suggest(&dm9(), 0, true, 1000).len(), 12 * 24 - 1);
    }
}
