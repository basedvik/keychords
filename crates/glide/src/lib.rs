//! Per-voice glide and strum scheduling (architecture.md §7).
//!
//! Voices are indexed bass first: voice 0 is the bass, voices 1.. are the upper voices in
//! ascending order. Time is in samples. Pitch is fractional MIDI (60.5 = a quarter tone
//! above middle C). Each voice keeps only its latest slide: querying a time before the most
//! recent slide's start returns that slide's starting pitch.

/// Most voices a chord can have: a bass and six above it.
pub const MAX_VOICES: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// 0..=800.
    pub glide_ms: f32,
    /// 0..=160: total roll from the bass to the top voice.
    pub strum_ms: f32,
    pub sample_rate: f32,
}

/// `round(ms * sample_rate / 1000)`.
pub fn ms_to_samples(ms: f32, sample_rate: f32) -> u64 {
    (ms * sample_rate / 1000.0).round() as u64
}

/// Smoothstep `3x² − 2x³`, with `x` clamped to 0..=1.
pub fn ease(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Voice `i` of `n`'s strum delay: `round(strum_ms·sr/1000 · i/(n−1))`, or 0 when `n ≤ 1`.
pub fn strum_offset(i: usize, n: usize, strum_ms: f32, sample_rate: f32) -> u64 {
    if n <= 1 {
        return 0;
    }
    let total = strum_ms * sample_rate / 1000.0;
    (total * i as f32 / (n - 1) as f32).round() as u64
}

/// Rules shared by `play_now` and `play_at`, for a new chord of `n` notes:
/// - Each voice's slide runs from its pitch at the slide's start to its new note, eased with
///   `ease`, over `glide` samples. A slide that interrupts another starts from wherever the
///   voice is at that moment. Glide 0 jumps at the slide start.
/// - First chord (nothing sounding, at start or after `release`): no slides. Voice i simply
///   becomes active at its onset (the slide start it would have had) at its note, and
///   `pitch` is `None` before that.
/// - More voices than before: each new voice starts from the pitch of the previously
///   sounding voice nearest to its target note (ties to the lower), measured at its slide
///   start, and slides like the others.
/// - Fewer voices than before: each surplus voice holds its pitch and goes inactive
///   (`None`) from the moment the chord's last slide lands, i.e. the greatest `lands_at` of
///   the new voices.
#[derive(Clone, Debug)]
pub struct Glider {
    params: Params,
    /// The latest slide per voice. For a first chord this is the onset, with `from == to`.
    slides: [Option<Slide>; MAX_VOICES],
    /// Sample from which the voice is inactive (after `release` or a voice drop).
    inactive_from: [Option<u64>; MAX_VOICES],
    /// Number of notes in the most recent chord (0 before any chord / after `release`).
    voice_count: usize,
}

#[derive(Clone, Copy, Debug)]
struct Slide {
    from: f32,
    to: f32,
    /// Sample from which the voice is active at `from`.
    onset: u64,
    start: u64,
    end: u64,
}

/// The chord in flight when a new one is scheduled.
#[derive(Clone, Copy)]
struct Snapshot {
    slides: [Option<Slide>; MAX_VOICES],
    inactive_from: [Option<u64>; MAX_VOICES],
    count: usize,
}

impl Snapshot {
    /// The pitch voice `j` would have at sample `t` under this chord.
    fn pitch_at(&self, j: usize, t: u64) -> Option<f32> {
        let s = self.slides.get(j)?.as_ref()?;
        if let Some(inactive) = self.inactive_from[j] {
            if t >= inactive {
                return None;
            }
        }
        if t < s.onset {
            return None;
        }
        if t >= s.end {
            return Some(s.to);
        }
        if t < s.start || s.start == s.end {
            return Some(s.from);
        }
        let x = (t - s.start) as f32 / (s.end - s.start) as f32;
        Some(s.from + (s.to - s.from) * ease(x))
    }
}

impl Glider {
    pub fn new(params: Params) -> Glider {
        Glider {
            params,
            slides: [None; MAX_VOICES],
            inactive_from: [None; MAX_VOICES],
            voice_count: 0,
        }
    }

    pub fn set_params(&mut self, params: Params) {
        self.params = params;
    }

    /// Live play: voice i's slide starts at `now + strum_offset(i)` and lands `glide` later.
    pub fn play_now(&mut self, notes: &[u8], now: u64) {
        let n = notes.len();
        let first_chord = self.voice_count == 0;
        let glide = ms_to_samples(self.params.glide_ms, self.params.sample_rate);
        let strum = self.params.strum_ms;
        let sr = self.params.sample_rate;
        // Snapshot the previous state so each voice's `from` is measured against the chord
        // being replaced, not the one being written.
        let snap = Snapshot {
            slides: self.slides,
            inactive_from: self.inactive_from,
            count: self.voice_count,
        };
        let starts: Vec<u64> = (0..n)
            .map(|i| now + strum_offset(i, n, strum, sr))
            .collect();
        for (i, &start) in starts.iter().enumerate() {
            self.schedule(notes, i, start, glide, first_chord, &snap);
        }
        self.finish_chord(n, snap.count);
    }

    /// Loop play: voice i lands at `land_at + strum_offset(i)`; its slide starts `glide`
    /// earlier (saturating at 0).
    pub fn play_at(&mut self, notes: &[u8], land_at: u64) {
        let n = notes.len();
        let first_chord = self.voice_count == 0;
        let glide = ms_to_samples(self.params.glide_ms, self.params.sample_rate);
        let strum = self.params.strum_ms;
        let sr = self.params.sample_rate;
        let snap = Snapshot {
            slides: self.slides,
            inactive_from: self.inactive_from,
            count: self.voice_count,
        };
        let lands: Vec<u64> = (0..n)
            .map(|i| land_at + strum_offset(i, n, strum, sr))
            .collect();
        for (i, &land) in lands.iter().enumerate() {
            let start = land.saturating_sub(glide);
            let span = land - start;
            self.schedule(notes, i, start, span, first_chord, &snap);
        }
        self.finish_chord(n, snap.count);
    }

    /// Every voice goes inactive from `at`; the next chord is a first chord.
    pub fn release(&mut self, at: u64) {
        for v in 0..MAX_VOICES {
            self.inactive_from[v] = Some(at);
        }
        self.voice_count = 0;
    }

    /// Voice `voice`'s pitch at sample `t`, or `None` when it is not sounding.
    pub fn pitch(&self, voice: usize, t: u64) -> Option<f32> {
        let slide = self.slides.get(voice)?;
        let slide = slide.as_ref()?;
        if let Some(inactive) = self.inactive_from[voice] {
            if t >= inactive {
                return None;
            }
        }
        if t < slide.onset {
            return None;
        }
        if t >= slide.end {
            return Some(slide.to);
        }
        if t < slide.start || slide.start == slide.end {
            return Some(slide.from);
        }
        let x = (t - slide.start) as f32 / (slide.end - slide.start) as f32;
        Some(slide.from + (slide.to - slide.from) * ease(x))
    }

    /// Number of notes in the most recent chord (0 after `release` or before any chord).
    pub fn voices(&self) -> usize {
        self.voice_count
    }

    /// Sample at which `voice`'s latest slide lands (its onset for a first chord).
    pub fn lands_at(&self, voice: usize) -> Option<u64> {
        self.slides.get(voice)?.map(|s| if s.start == s.end { s.onset } else { s.end })
    }

    fn schedule(
        &mut self,
        notes: &[u8],
        i: usize,
        start: u64,
        glide: u64,
        first_chord: bool,
        snap: &Snapshot,
    ) {
        let end = if first_chord {
            // A first chord's slide is degenerate: start == end == onset, landing at onset.
            start
        } else {
            start + glide
        };
        let to = notes[i] as f32;
        let old_count = snap.count;
        let prev_pitch = |j: usize| snap.pitch_at(j, start);
        // A voice becomes active at the slide start it would have had.
        let onset = if first_chord {
            start
        } else {
            snap.slides[i].map_or(0, |s| s.onset)
        };
        let from = if first_chord {
            to
        } else if i < old_count {
            prev_pitch(i).unwrap_or(to)
        } else {
            // New voice: the previously sounding voice nearest to the target, ties to the lower.
            let mut best = 0usize;
            let mut best_d = f32::MAX;
            for j in 0..old_count {
                let pj = match prev_pitch(j) {
                    Some(p) => p,
                    None => continue,
                };
                let d = (pj - to).abs();
                if d < best_d {
                    best_d = d;
                    best = j;
                }
            }
            prev_pitch(best).unwrap_or(to)
        };
        self.slides[i] = Some(Slide { onset, from, to, start, end });
        self.inactive_from[i] = None;
    }

    /// Apply the per-chord bookkeeping: surplus-voice drop and the new voice count.
    fn finish_chord(&mut self, n: usize, old_count: usize) {
        if old_count > n {
            // Surplus voices hold their pitch until the chord's last slide lands.
            let max_land = (0..n)
                .filter_map(|j| self.slides[j].map(|s| s.end))
                .max()
                .unwrap_or(0);
            for j in 0..MAX_VOICES {
                if j >= n {
                    self.inactive_from[j] = Some(max_land);
                }
            }
        }
        self.voice_count = n;
    }
}
