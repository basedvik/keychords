# D2: "Try next" chord suggestions (you write the tests)

Add `crates/app/src/hints.rs` (and `mod hints;` in `crates/app/src/main.rs`): given the chord just played, suggest a few next chords that voice-lead smoothly and sound lush, so a player can follow the glow without theory.

## API (implement exactly)
```rust
use chords::{Chord, Quality};
use voicing::Voicing;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub chord: Chord,
    /// Upper-voice motion from the current voicing (voicing::motion).
    pub motion: u32,
}

/// Up to `n` suggestions after `current` (the chord just played and its voicing).
pub fn suggest(current: &(Chord, Voicing), octave: i8, smooth: bool, n: usize) -> Vec<Suggestion>;
```

## Algorithm (follow exactly)
1. For every root 0..12 and every `Quality::ALL` quality, build `chord = Chord::new(root, q)`. Skip the current chord itself.
2. Voice it the way the app would: `let mut vl = voicing::VoiceLeader::seeded(current.1.clone()); vl.octave = octave; vl.smooth = smooth; let v = vl.next(chord);`
3. `motion = voicing::motion(&current.1.upper, &v.upper)`.
4. Score (lower is better) as an `i32`: `motion as i32 - lush`, where `lush = 2` if `q.intervals().len() >= 4` else `0`. Rich chords are preferred.
5. Sort by `(score, fifths distance between roots, quality index)`, where fifths distance is `min(d, 12 - d)` with `d = (fifths_index(chord.root) + 12 - fifths_index(current.0.root)) % 12` and `fifths_index(pc) = (pc * 7) % 12` (use `crate::theme::fifths_index` if it already exists, else a private copy). Take the first `n`.

## Tests (you write them in a `#[cfg(test)]` module in hints.rs)
Work every expected value out by hand from the algorithm before writing it. Cover at least:
- never suggests the current chord
- returns exactly `n` (for n = 3) and is sorted by score
- from the reference voicing Dm9 `{ bass: 50, upper: [57, 65, 72, 76] }` at octave 0, smooth on: G13 (`Chord::new(7, Dom13)`) is among the top 3 (its motion is 1, score -1)
- every suggestion's `motion` equals `voicing::motion` recomputed in the test
- `n = 0` gives an empty list

## Files you may edit
- crates/app/src/hints.rs (new)
- crates/app/src/main.rs (only to add `mod hints;`)

## Verify
```
cargo test -p app hints:: && cargo clippy -p app --all-targets -- -D warnings -A dead-code
```

## Rules
- Only edit the files listed. Plain idiomatic Rust; refer to enum variants by name.
- Work incrementally: write a little, run the verify command, fix, repeat.
- When done, reply with: what you implemented, what your tests cover, the test summary line, and anything ambiguous.
