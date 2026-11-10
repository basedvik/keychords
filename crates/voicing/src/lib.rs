//! Voicing and voice leading (architecture.md §6).
//!
//! A voicing is a bass note plus 2..=5 upper voices. The bass is the root placed in a fixed
//! register window. Upper voices realise the quality's `upper_template` pitch classes, one
//! note per pitch class, and move to the next chord by least total motion.

use chords::{Chord, Midi, PitchClass, Quality};

/// Lowest bass note before octave shift: G2.
pub const BASS_LO: Midi = 43;
/// Highest bass note before octave shift: F♯3.
pub const BASS_HI: Midi = 54;
/// Upper-voice band before octave shift: A3..=G5.
pub const BAND_LO: Midi = 57;
pub const BAND_HI: Midi = 79;
/// Largest allowed gap between adjacent upper voices, in semitones.
pub const MAX_GAP: u8 = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voicing {
    pub bass: Midi,
    /// Ascending, distinct.
    pub upper: Vec<Midi>,
}

/// Intervals above the root that the upper voices realise (only `% 12` matters).
/// Triads and sus chords keep their root up top; larger chords leave the root to the bass;
/// dom13 drops the 5th and 11th (G13 = G | A F B E).
pub fn upper_template(q: Quality) -> &'static [u8] {
    use Quality::*;
    match q {
        Maj => &[0, 4, 7],
        Min => &[0, 3, 7],
        Dim => &[0, 3, 6],
        Aug => &[0, 4, 8],
        Maj7 => &[4, 7, 11],
        Min7 => &[3, 7, 10],
        Dom7 => &[4, 7, 10],
        Min7b5 => &[3, 6, 10],
        Sus2 => &[0, 2, 7],
        Sus4 => &[0, 5, 7],
        Add9 => &[4, 7, 14],
        Maj6 => &[4, 7, 9],
        Maj9 => &[4, 7, 11, 14],
        Min9 => &[3, 7, 10, 14],
        Dom9 => &[4, 7, 10, 14],
        Dom13 => &[4, 10, 14, 21],
        Dim7 => &[3, 6, 9],
        Dom7Sus4 => &[5, 7, 10],
        Min6 => &[3, 7, 9],
        SixNine => &[4, 7, 9, 14],
        Min11 => &[3, 7, 10, 14, 17],
        Dom7b9 => &[4, 7, 10, 13],
        Dom7s9 => &[4, 7, 10, 15],
        Maj7s11 => &[4, 7, 11, 14, 18],
    }
}

/// The root's pitch class placed in `BASS_LO..=BASS_HI`, shifted by `12 * octave`.
pub fn bass_note(root: PitchClass, octave: i8) -> Midi {
    let offset = (root + 12 - BASS_LO % 12) % 12;
    (BASS_LO as i32 + offset as i32 + 12 * octave as i32) as Midi
}

/// Motion between two ascending voicings.
/// Equal lengths: Σ |a[i] − b[i]| (order-preserving, no crossing).
/// Different lengths: minimum-cost monotone alignment in which every element of both
/// sequences is matched at least once (DTW over the two sorted sequences), summing |Δ| per
/// matched pair.
pub fn motion(a: &[Midi], b: &[Midi]) -> u32 {
    if a.len() == b.len() {
        a.iter().zip(b).map(|(x, y)| x.abs_diff(*y) as u32).sum()
    } else {
        dtw(a, b).0
    }
}

/// Minimum-cost monotone alignment of `a` onto `b`, |Δ| per matched pair, with the
/// matched (i, j) pairs in order.
fn dtw(a: &[Midi], b: &[Midi]) -> (u32, Vec<(usize, usize)>) {
    let (m, n) = (a.len(), b.len());
    if m == 0 || n == 0 {
        return (0, Vec::new());
    }
    const INF: u32 = u32::MAX / 4;
    let mut dp = vec![vec![INF; n]; m];
    let mut from = vec![vec![0u8; n]; m];
    for i in 0..m {
        for j in 0..n {
            let cost = a[i].abs_diff(b[j]) as u32;
            if i == 0 && j == 0 {
                dp[i][j] = cost;
                continue;
            }
            let (mut best, mut dir) = (INF, 0u8);
            if i > 0 && dp[i - 1][j] < best {
                best = dp[i - 1][j];
                dir = 1;
            }
            if j > 0 && dp[i][j - 1] < best {
                best = dp[i][j - 1];
                dir = 2;
            }
            if i > 0 && j > 0 && dp[i - 1][j - 1] < best {
                best = dp[i - 1][j - 1];
                dir = 0;
            }
            dp[i][j] = best + cost;
            from[i][j] = dir;
        }
    }
    let mut path = Vec::with_capacity(m + n);
    let (mut i, mut j) = (m - 1, n - 1);
    loop {
        path.push((i, j));
        if i == 0 && j == 0 {
            break;
        }
        match from[i][j] {
            1 => i -= 1,
            2 => j -= 1,
            _ => {
                i -= 1;
                j -= 1;
            }
        }
    }
    path.reverse();
    (dp[m - 1][n - 1], path)
}

/// Every valid upper voicing of `chord` for `octave`: one note per template pitch class,
/// all inside `BAND_LO..=BAND_HI` (+12·octave), ascending and distinct, adjacent gaps
/// ≤ `MAX_GAP`. Returned in ascending lexicographic order.
pub fn candidates(chord: Chord, octave: i8) -> Vec<Vec<Midi>> {
    let lo = BAND_LO as i32 + 12 * octave as i32;
    let hi = BAND_HI as i32 + 12 * octave as i32;
    let mut out: Vec<Vec<i32>> = vec![Vec::new()];
    for &iv in upper_template(chord.quality) {
        let pc = (chord.root + iv) % 12;
        let mut ps = Vec::new();
        let mut p = pc as i32;
        while p < lo {
            p += 12;
        }
        while p <= hi {
            ps.push(p);
            p += 12;
        }
        let mut next = Vec::new();
        for c in out {
            for &p in &ps {
                let mut nc = c.clone();
                nc.push(p);
                next.push(nc);
            }
        }
        out = next;
    }
    let mut res: Vec<Vec<Midi>> = out
        .into_iter()
        .map(|c| {
            let mut s = c;
            s.sort_unstable();
            s.into_iter().map(|n| n as Midi).collect::<Vec<Midi>>()
        })
        .filter(|c| c.windows(2).all(|w| w[0] < w[1] && w[1] - w[0] <= MAX_GAP))
        .collect();
    res.sort();
    res
}

/// Greedy voice leader for live play.
#[derive(Clone, Debug)]
pub struct VoiceLeader {
    /// Off: close position, each template interval added to the root placed in 60..=71
    /// (+12·octave), sorted. On: least-motion choice among `candidates`.
    pub smooth: bool,
    pub octave: i8,
    prev: Option<Voicing>,
}

impl Default for VoiceLeader {
    fn default() -> Self {
        VoiceLeader { smooth: true, octave: 0, prev: None }
    }
}

impl VoiceLeader {
    pub fn new() -> VoiceLeader {
        VoiceLeader::default()
    }

    /// Start from a known voicing, as if it had just been played.
    pub fn seeded(v: Voicing) -> VoiceLeader {
        VoiceLeader { prev: Some(v), ..VoiceLeader::default() }
    }

    pub fn reset(&mut self) {
        self.prev = None;
    }

    /// Voice `chord`. Smooth mode picks the candidate with the least `motion` from the
    /// previous upper voices; ties go to (1) more voices left exactly where they were, then
    /// (2) the smaller largest single-voice move, then (3) the lexicographically lower
    /// voicing. With no previous voicing, it picks the candidate whose mean pitch is closest
    /// to the band centre (68 + 12·octave), ties to the lexicographically lower.
    pub fn next(&mut self, chord: Chord) -> Voicing {
        let bass = bass_note(chord.root, self.octave);
        let upper = if self.smooth {
            let cands = candidates(chord, self.octave);
            match &self.prev {
                Some(prev) => pick_least_motion(&prev.upper, &cands).clone(),
                None => pick_centre(&cands, self.octave).clone(),
            }
        } else {
            close_position(chord, self.octave)
        };
        let v = Voicing { bass, upper };
        self.prev = Some(v.clone());
        v
    }
}

fn close_position(chord: Chord, octave: i8) -> Vec<Midi> {
    let base = 60 + 12 * octave as i32 + chord.root as i32;
    let mut v: Vec<Midi> = upper_template(chord.quality)
        .iter()
        .map(|&iv| (base + iv as i32) as Midi)
        .collect();
    v.sort_unstable();
    v
}

fn pick_centre(cands: &[Vec<Midi>], octave: i8) -> &Vec<Midi> {
    let centre = 68 + 12 * octave as i32;
    let mut best: &Vec<Midi> = &cands[0];
    let mut best_d = off_centre(best, centre);
    for c in &cands[1..] {
        let d = off_centre(c, centre);
        if d < best_d {
            best = c;
            best_d = d;
        }
    }
    best
}

fn off_centre(c: &[Midi], centre: i32) -> i32 {
    let sum: i32 = c.iter().map(|&n| n as i32).sum();
    (sum - centre * c.len() as i32).abs()
}

fn pick_least_motion<'a>(prev: &[Midi], cands: &'a [Vec<Midi>]) -> &'a Vec<Midi> {
    let mut best: &Vec<Midi> = &cands[0];
    let mut best_s = score(prev, best);
    for c in &cands[1..] {
        let s = score(prev, c);
        if s.0 < best_s.0
            || (s.0 == best_s.0 && (s.1 > best_s.1 || (s.1 == best_s.1 && s.2 < best_s.2)))
        {
            best = c;
            best_s = s;
        }
    }
    best
}

/// (total motion, voices left exactly where they were, largest single-voice move).
fn score(prev: &[Midi], cand: &[Midi]) -> (u32, u32, u32) {
    let m = motion(prev, cand);
    if prev.len() == cand.len() {
        let held = prev.iter().zip(cand).filter(|(p, c)| p == c).count() as u32;
        let largest = prev.iter().zip(cand).map(|(p, c)| p.abs_diff(*c) as u32).max().unwrap_or(0);
        (m, held, largest)
    } else {
        let largest = dtw(prev, cand)
            .1
            .into_iter()
            .map(|(i, j)| prev[i].abs_diff(cand[j]) as u32)
            .max()
            .unwrap_or(0);
        (m, 0, largest)
    }
}

/// Loop mode: voice a cyclic progression at once, minimising the total upper-voice `motion`
/// over every change including the wrap from the last chord back to the first. Bass notes
/// come from `bass_note`. Returns one voicing per chord, in order.
pub fn voice_cycle(chords: &[Chord], octave: i8) -> Vec<Voicing> {
    let n = chords.len();
    if n == 0 {
        return Vec::new();
    }
    let cands: Vec<Vec<Vec<Midi>>> = chords.iter().map(|c| candidates(*c, octave)).collect();
    let basses: Vec<Midi> = chords.iter().map(|c| bass_note(c.root, octave)).collect();
    if n == 1 {
        return vec![Voicing { bass: basses[0], upper: cands[0][0].clone() }];
    }

    let mut best_total = u32::MAX;
    let mut best: (usize, Vec<usize>) = (0, Vec::new());
    for c0 in 0..cands[0].len() {
        let mut dp: Vec<u32> = cands[1].iter().map(|c| motion(&cands[0][c0], c)).collect();
        let mut back: Vec<Vec<usize>> = Vec::new();
        for (i, level) in cands.iter().enumerate().skip(2) {
            let mut next_dp = vec![0u32; level.len()];
            let mut next_back = vec![0usize; level.len()];
            for (j, c) in level.iter().enumerate() {
                let (mut v, mut k) = (u32::MAX, 0usize);
                for (kk, pc) in cands[i - 1].iter().enumerate() {
                    let t = dp[kk] + motion(pc, c);
                    if t < v {
                        v = t;
                        k = kk;
                    }
                }
                next_dp[j] = v;
                next_back[j] = k;
            }
            back.push(next_back);
            dp = next_dp;
        }
        let (mut total, mut last) = (u32::MAX, 0usize);
        for (j, &v) in dp.iter().enumerate() {
            let t = v + motion(&cands[n - 1][j], &cands[0][c0]);
            if t < total {
                total = t;
                last = j;
            }
        }
        if total < best_total {
            best_total = total;
            let mut path = vec![0usize; n - 1];
            path[n - 2] = last;
            for t in (0..n - 2).rev() {
                path[t] = back[t][path[t + 1]];
            }
            best = (c0, path);
        }
    }

    let (c0, path) = best;
    let mut out = vec![Voicing { bass: basses[0], upper: cands[0][c0].clone() }];
    for (i, &idx) in path.iter().enumerate() {
        out.push(Voicing { bass: basses[i + 1], upper: cands[i + 1][idx].clone() });
    }
    out
}
