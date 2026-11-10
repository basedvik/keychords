//! Loop mode: a grid of chord slots played against an internal clock (architecture.md §8).

use chords::Chord;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// Not written: keeps sounding the chord before it.
    Empty,
    /// The "stop" key: nothing sounds.
    Silent,
    Chord(Chord),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loop {
    /// 4, 8, 16 or 32.
    pub bars: u8,
    /// 1 or 2 chords per bar.
    pub per_bar: u8,
    /// `bars * per_bar` slots.
    pub slots: Vec<Slot>,
}

impl Loop {
    /// All slots `Empty`. Panics unless bars ∈ {4, 8, 16, 32} and per_bar ∈ {1, 2}.
    pub fn new(bars: u8, per_bar: u8) -> Loop {
        todo!()
    }

    /// 4/4: 4.0 beats per bar divided by `per_bar`.
    pub fn slot_beats(&self) -> f64 {
        todo!()
    }

    pub fn len_beats(&self) -> f64 {
        todo!()
    }

    /// Change the bar count, keeping existing slots; new slots are `Empty`.
    pub fn set_bars(&mut self, bars: u8) {
        todo!()
    }

    /// 1 → 2: every slot is duplicated into both halves of its bar.
    /// 2 → 1: each bar keeps its first half.
    pub fn set_per_bar(&mut self, per_bar: u8) {
        todo!()
    }

    /// The chord sounding during slot `i`: its own chord; `Empty` inherits from the nearest
    /// earlier non-`Empty` slot, wrapping around the end of the loop; `Silent` (own or
    /// inherited) gives `None`; a loop with no non-`Empty` slot gives `None`.
    pub fn resolved(&self, i: usize) -> Option<Chord> {
        todo!()
    }

    /// Slot indices where the sounding chord changes from the previous slot's (cyclic, so
    /// slot 0 is compared with the last slot), ascending. A change into or out of silence
    /// counts. A loop that sounds one chord throughout has none.
    pub fn changes(&self) -> Vec<usize> {
        todo!()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clock {
    pub bpm: f64,
    pub sample_rate: f64,
}

impl Clock {
    pub fn samples_per_beat(&self) -> f64 {
        todo!()
    }

    /// `round(beat * samples_per_beat)`.
    pub fn beat_to_sample(&self, beat: f64) -> u64 {
        todo!()
    }
}

/// Slot boundaries that fall in `from..to` (samples) for a loop whose beat 0 was at sample
/// `origin`, as `(sample, slot index)`, ascending, repeating cyclically. Boundary k (k ≥ 0
/// counted from `origin`) is at `origin + beat_to_sample(k · slot_beats)` and belongs to slot
/// `k % slots.len()`. Every boundary is listed, whether or not the chord changes there.
pub fn boundaries(lp: &Loop, clock: &Clock, origin: u64, from: u64, to: u64) -> Vec<(u64, usize)> {
    todo!()
}
