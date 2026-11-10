# T10: midi::recognize: read chords from any MIDI file

Add `crates/midi/src/recognize.rs` (and `pub mod recognize;` in `crates/midi/src/lib.rs`). It reads an arbitrary Standard MIDI File (already parsed with `crate::smf::Smf::parse`) as a chord progression for the app's loop. The `chords` crate gives `Chord`, `Quality::ALL` and `Quality::intervals()`.

You will need `chords` as a dependency of `midi`: this is the one Cargo.toml change allowed. Add `chords = { workspace = true }` under `[dependencies]` in `crates/midi/Cargo.toml`.

## API (implement exactly)
```rust
use chords::Chord;
/// Chords at each half bar (4/4: every 2 beats, using smf.ppq), up to 32 bars (64 windows),
/// None where nothing sounds or nothing matches.
pub fn recognize(smf: &Smf) -> Vec<Option<Chord>>;
/// Best chord for a duration-weighted pitch-class histogram and the lowest pitch class sounding.
pub fn best_chord(weights: &[f32; 12], lowest: Option<u8>) -> Option<Chord>;
```

## Rules
1. Collect notes from every track and channel **except channel 10 (index 9, drums)**. Pair NoteOn with the next NoteOff (or NoteOn velocity 0) of the same channel and note. A note still held at the end ends at the last event.
2. Windows are 2 beats long (`2 * ppq` ticks) from tick 0. The output length is the number of windows up to the last note end, capped at 64.
3. A window's histogram adds each note's overlap duration (ticks) to its pitch class. lowest = pitch class of the lowest note sounding in the window.
4. `best_chord`: score every (root, quality) over 12 roots and the 24 qualities. Template = the quality's interval pitch classes. score = (sum of weights on template pcs) - 0.5 * (sum of weights off template) - 0.1 * (template pcs with zero weight) * (max weight). Add a bonus of 0.25 * max weight when root == lowest. Highest score wins; ties go to the quality with fewer notes, then the lower root. If all weights are zero, return None.
5. Windows with no sounding notes give None.

## Files you may edit
- crates/midi/src/recognize.rs (new)
- crates/midi/src/lib.rs (only to add `pub mod recognize;`)
- crates/midi/Cargo.toml (only the `chords` dependency line)
- crates/midi/tests/recognize.rs (new: your tests; build small Smf values in code)

## Verify
```
cargo test -p midi --test recognize && cargo clippy -p midi --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit" (creating them if needed). Do not touch other crates, existing tests, or Cargo.toml files, and add no dependencies.
- **You write the tests.** Put them in the test file listed. Cover the rules in this spec, not just the happy path: a reviewer will read your tests as carefully as your code.
- Keep the style of the surrounding crates: plain, idiomatic Rust, short doc comments. Refer to enum variants by name; write any non-ASCII literal with `\u{...}` escapes.
- Work incrementally: write a little, run the verify command, fix, repeat. Keep each reply short.
- Loop until the verify command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, what your tests cover, the final test summary line, and anything in this spec you found ambiguous.
