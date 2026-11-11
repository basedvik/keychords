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
/// 4. `fd` = fifths distance between roots: `min(d, 12 - d)` with
///    `d = (fifths_index(root) + 12 - fifths_index(current root)) % 12` (`crate::theme::fifths_index`).
/// 5. `score = motion - lush + same_quality + same_root + |fd - 1|` (all `i32`), where
///    `lush = 2` when `q.intervals().len() >= 4` else 0; `same_quality = 4` when `q` equals the
///    current quality else 0; `same_root = 3` when `root` equals the current root else 0.
///    (Prefers smooth, rich chords a fifth away: ii-V, V-I.)
/// 6. Sort by `(score, fd, quality index)`, keep only the **first suggestion for each root**,
///    then take the first `n`.
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
            let same_quality = if q == cur_chord.quality { 4 } else { 0 };
            let same_root = if root == cur_root { 3 } else { 0 };
            let d = (crate::theme::fifths_index(root) + 12 - crate::theme::fifths_index(cur_root)) % 12;
            let fifths_dist: i32 = d.min(12 - d) as i32;
            let score = m as i32 - lush + same_quality + same_root + (fifths_dist - 1).abs();
            out.push((score, d.min(12 - d), q.index(), Suggestion { chord, motion: m }));
        }
    }
    out.sort_by_key(|a| (a.0, a.1, a.2));
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<_> = out.into_iter().filter_map(|(s, fd, _, sugg)| {
        let root = sugg.chord.root;
        if seen.insert(root) {
            Some((s, fd, sugg))
        } else {
            None
        }
    }).collect();
    deduped.into_iter().take(n).map(|(_, _, s)| s).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dm9() -> (Chord, Voicing) {
        (Chord::new(2, Quality::Min9), Voicing { bass: 50, upper: vec![57, 65, 72, 76] })
    }

    fn a7b9() -> (Chord, Voicing) {
        (Chord::new(9, Quality::Dom7b9), Voicing { bass: 45, upper: vec![61, 64, 70, 79] })
    }

    fn score(cur: &(Chord, Voicing), s: &Suggestion) -> i32 {
        let fi = crate::theme::fifths_index;
        let d = (fi(s.chord.root) + 12 - fi(cur.0.root)) % 12;
        let fd = d.min(12 - d) as i32;
        s.motion as i32 - if s.chord.quality.intervals().len() >= 4 { 2 } else { 0 }
            + if s.chord.quality == cur.0.quality { 4 } else { 0 }
            + if s.chord.root == cur.0.root { 3 } else { 0 }
            + (fd - 1).abs()
    }

    #[test]
    fn dm9_suggests_g13_first() {
        let out = suggest(&dm9(), 0, true, 3);
        assert_eq!(out[0].chord, Chord::new(7, Quality::Dom13), "ii-V: {out:?}");
    }

    #[test]
    fn a7b9_suggests_dmaj9_first() {
        let out = suggest(&a7b9(), 0, true, 3);
        assert_eq!(out[0].chord, Chord::new(2, Quality::Maj9), "V-I: {out:?}");
    }

    #[test]
    fn one_per_root_sorted_and_never_the_current_chord() {
        for cur in [dm9(), a7b9()] {
            let out = suggest(&cur, 0, true, 5);
            assert_eq!(out.len(), 5);
            assert!(out.iter().all(|s| s.chord != cur.0));
            let mut roots: Vec<_> = out.iter().map(|s| s.chord.root).collect();
            roots.sort();
            roots.dedup();
            assert_eq!(roots.len(), 5, "distinct roots: {out:?}");
            assert!(out.windows(2).all(|w| score(&cur, &w[0]) <= score(&cur, &w[1])), "{out:?}");
        }
    }

    #[test]
    fn motion_is_what_the_voice_leader_would_do() {
        let cur = dm9();
        for s in suggest(&cur, 0, true, 12) {
            let mut vl = voicing::VoiceLeader::seeded(cur.1.clone());
            let v = vl.next(s.chord);
            assert_eq!(s.motion, voicing::motion(&cur.1.upper, &v.upper), "{:?}", s.chord);
        }
    }

    #[test]
    fn at_most_one_per_root_in_total_and_zero_is_empty() {
        assert_eq!(suggest(&dm9(), 0, true, 1000).len(), 12);
        assert!(suggest(&dm9(), 0, true, 0).is_empty());
    }
}
