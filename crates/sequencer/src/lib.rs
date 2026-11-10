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
        assert!(matches!(bars, 4 | 8 | 16 | 32), "bars must be 4, 8, 16, or 32");
        assert!(matches!(per_bar, 1 | 2), "per_bar must be 1 or 2");
        let count = (bars * per_bar) as usize;
        Loop {
            bars,
            per_bar,
            slots: vec![Slot::Empty; count],
        }
    }

    /// 4/4: 4.0 beats per bar divided by `per_bar`.
    pub fn slot_beats(&self) -> f64 {
        4.0 / self.per_bar as f64
    }

    pub fn len_beats(&self) -> f64 {
        self.bars as f64 * 4.0
    }

    /// Change the bar count, keeping existing slots; new slots are `Empty`.
    pub fn set_bars(&mut self, bars: u8) {
        let new_count = (bars * self.per_bar) as usize;
        self.bars = bars;
        if new_count > self.slots.len() {
            self.slots.resize(new_count, Slot::Empty);
        } else {
            self.slots.truncate(new_count);
        }
    }

    /// 1 → 2: every slot is duplicated into both halves of its bar.
    /// 2 → 1: each bar keeps its first half.
    pub fn set_per_bar(&mut self, per_bar: u8) {
        if per_bar == self.per_bar {
            return;
        }
        let old = self.per_bar;
        self.per_bar = per_bar;
        if old == 1 && per_bar == 2 {
            let mut out = Vec::with_capacity(self.slots.len() * 2);
            for &s in &self.slots {
                out.push(s);
                out.push(s);
            }
            self.slots = out;
        } else {
            let bar_count = self.bars;
            let mut out = Vec::with_capacity(bar_count as usize);
            for b in 0..bar_count {
                out.push(self.slots[(b * old) as usize]);
            }
            self.slots = out;
        }
    }

    /// The chord sounding during slot `i`: its own chord; `Empty` inherits from the nearest
    /// earlier non-`Empty` slot, wrapping around the end of the loop; `Silent` (own or
    /// inherited) gives `None`; a loop with no non-`Empty` slot gives `None`.
    pub fn resolved(&self, i: usize) -> Option<Chord> {
        let n = self.slots.len();
        if n == 0 {
            return None;
        }
        match self.slots[i] {
            Slot::Chord(ch) => return Some(ch),
            Slot::Silent => return None,
            Slot::Empty => {}
        }
        for offset in 1..=n {
            let j = (i + n - offset) % n;
            match self.slots[j] {
                Slot::Chord(ch) => return Some(ch),
                Slot::Silent => return None,
                Slot::Empty => {}
            }
        }
        None
    }

    /// Slot indices where the sounding chord changes from the previous slot's (cyclic, so
    /// slot 0 is compared with the last slot), ascending. A change into or out of silence
    /// counts. A loop that sounds one chord throughout has none.
    pub fn changes(&self) -> Vec<usize> {
        let n = self.slots.len();
        let mut out = Vec::new();
        for i in 0..n {
            let prev = (i + n - 1) % n;
            if self.resolved(i) != self.resolved(prev) {
                out.push(i);
            }
        }
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clock {
    pub bpm: f64,
    pub sample_rate: f64,
}

impl Clock {
    pub fn samples_per_beat(&self) -> f64 {
        self.sample_rate * 60.0 / self.bpm
    }

    /// `round(beat * samples_per_beat)`.
    pub fn beat_to_sample(&self, beat: f64) -> u64 {
        (beat * self.samples_per_beat()).round() as u64
    }
}

/// Slot boundaries that fall in `from..to` (samples) for a loop whose beat 0 was at sample
/// `origin`, as `(sample, slot index)`, ascending, repeating cyclically. Boundary k (k ≥ 0
/// counted from `origin`) is at `origin + beat_to_sample(k · slot_beats)` and belongs to slot
/// `k % slots.len()`. Every boundary is listed, whether or not the chord changes there.
pub fn boundaries(lp: &Loop, clock: &Clock, origin: u64, from: u64, to: u64) -> Vec<(u64, usize)> {
    let slot_beats = lp.slot_beats();
    let n = lp.slots.len();
    let mut result = Vec::new();
    let mut k = 0u64;

    loop {
        let sample = origin + clock.beat_to_sample(k as f64 * slot_beats);
        if sample >= from {
            break;
        }
        k += 1;
    }

    loop {
        let sample = origin + clock.beat_to_sample(k as f64 * slot_beats);
        if sample >= to {
            break;
        }
        result.push((sample, (k as usize) % n));
        k += 1;
    }

    result
}
