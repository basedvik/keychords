# T11b: app UI: export and import

Wire T11a's functions into the Rune UI in `crates/app/src/main.rs`. Rune is a pre-release UI framework you have not seen: learn it from `crates/app/src/main.rs` itself (its `button`, `row`, `col` and `on_press` usage) and from the example at `.research/rune/examples/drop-files/` (dropping files onto an element). Do not guess at APIs; grep `.research/rune/crates/rune/src` when unsure.

## Add
1. In the loop strip (`loop_strip`), three buttons after "½ bar": **Export MIDI**, **Export audio**, **Import MIDI**.
2. Exports write to `~/Music/keychords/` (create it), named `keychords-YYYYMMDD-HHMMSS.mid` / `.wav` (use `std::time::SystemTime`; no new dependencies, so format the timestamp yourself from the Unix time). Set `s.status` to "Saved <path>" or the error.
3. Import MIDI imports the newest `.mid` in `~/Music/keychords/import/` (create the folder if missing; status says so if it's empty).
4. Dropping a `.mid` file anywhere on the window imports it (see the drop-files example).
5. Keep `cargo run -p app -- --snap logs/ui.png --scene demo` working; the new buttons must appear in it.

## Files you may edit
- crates/app/src/main.rs

## Verify
```
cargo build -p app && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -q -p app -- --snap logs/ui.png --scene demo
```
(There are no UI unit tests; the reviewer will look at the snapshot.)

## Rules for to_smf
1. Format 1, PPQ 480. Slot k starts at tick `round(k * slot_beats * 480)`; the loop ends at `slots * slot_beats * 480`.
2. Track 0 (conductor): TrackName "keychords", Tempo (`round(60_000_000 / bpm)`), TimeSig 4/4 (num 4, den_pow2 2, clocks 24, n32 8), a Marker with the chord name at every slot whose voicing differs from the previous slot's (slot 0 always gets one if it sounds), and SeqSpecific(STATE_TAG followed by `state`) at tick 0.
3. One track per voice index v that ever sounds (v = 0 is the bass), on channel `1 + v` (MPE member channels). Each voice track starts at tick 0 with RPN 0 pitch-bend range: CC101=0, CC100=0, CC6=R, CC38=0, where R = 48, or 96 if any slide in the loop is wider than 48 semitones.
4. "Struck as it lands": at each slot start where a voice's note changes (or the voice starts sounding), send NoteOff for its previous note (velocity 0, if any), PitchBend 8192, then NoteOn (velocity 100) for the new note. A voice that is silent in a slot gets a NoteOff at that slot's start.
5. Glides: when a voice moves from note a to note b at a slot start s, and glide_ms > 0, the bend ramps over the glide window *before* s: from tick `s - glide_ticks` to s, where `glide_ticks = round(glide_ms / 1000 * bpm / 60 * 480)` (clamped so it never starts before the previous strike). Emit a PitchBend every 10 ticks inside the window, value `8192 + round((b - a) * ease(x) / R * 8192)` clamped to 0..=16383, with `ease(x) = 3x^2 - 2x^3`, x = progress 0..1. The bend is relative to the held note a. Then the strike at s (rule 4) resets it.
6. At the loop end, NoteOff for every held note.
7. A voice that keeps the same note across slots is not re-struck.

## Files you may edit
- crates/midi/src/export.rs (new)
- crates/midi/src/lib.rs (only to add `pub mod export;`)
- crates/midi/tests/export.rs (new: your tests)

## Verify
```
cargo test -p midi --test export && cargo clippy -p midi --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit" (creating them if needed). Do not touch other crates, existing tests, or Cargo.toml files, and add no dependencies.
- **You write the tests.** Put them in the test file listed. Cover the rules in this spec, not just the happy path: a reviewer will read your tests as carefully as your code.
- Keep the style of the surrounding crates: plain, idiomatic Rust, short doc comments. Refer to enum variants by name; write any non-ASCII literal with `\u{...}` escapes.
- Work incrementally: write a little, run the verify command, fix, repeat. Keep each reply short.
- Loop until the verify command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, what your tests cover, the final test summary line, and anything in this spec you found ambiguous.
