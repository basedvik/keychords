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
    todo!()
}

/// Smoothstep `3x² − 2x³`, with `x` clamped to 0..=1.
pub fn ease(x: f32) -> f32 {
    todo!()
}

/// Voice `i` of `n`'s strum delay: `round(strum_ms·sr/1000 · i/(n−1))`, or 0 when `n ≤ 1`.
pub fn strum_offset(i: usize, n: usize, strum_ms: f32, sample_rate: f32) -> u64 {
    todo!()
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
    // Private state is yours to design.
}

impl Glider {
    pub fn new(params: Params) -> Glider {
        todo!()
    }

    pub fn set_params(&mut self, params: Params) {
        todo!()
    }

    /// Live play: voice i's slide starts at `now + strum_offset(i)` and lands `glide` later.
    pub fn play_now(&mut self, notes: &[u8], now: u64) {
        todo!()
    }

    /// Loop play: voice i lands at `land_at + strum_offset(i)`; its slide starts `glide`
    /// earlier (saturating at 0).
    pub fn play_at(&mut self, notes: &[u8], land_at: u64) {
        todo!()
    }

    /// Every voice goes inactive from `at`; the next chord is a first chord.
    pub fn release(&mut self, at: u64) {
        todo!()
    }

    /// Voice `voice`'s pitch at sample `t`, or `None` when it is not sounding.
    pub fn pitch(&self, voice: usize, t: u64) -> Option<f32> {
        todo!()
    }

    /// Number of notes in the most recent chord (0 after `release` or before any chord).
    pub fn voices(&self) -> usize {
        todo!()
    }

    /// Sample at which `voice`'s latest slide lands (its onset for a first chord).
    pub fn lands_at(&self, voice: usize) -> Option<u64> {
        todo!()
    }
}
