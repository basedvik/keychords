# T11a: app: WAV export, loop state, MIDI export/import logic

Pure logic in the `app` crate, no UI. Read `crates/app/src/controller.rs` and `crates/app/src/engine.rs` first, and `crates/midi/src/export.rs` and `crates/midi/src/recognize.rs` (the latter may not exist yet; if it doesn't, skip the recognition parts and say so).

## 1. `crates/app/src/wav.rs` (new)
`pub fn wav_24bit(left: &[f32], right: &[f32], sample_rate: u32) -> Vec<u8>`: a canonical RIFF/WAVE file, PCM (format 1), 2 channels, 24-bit little-endian, samples clamped to -1..=1 and scaled by 8_388_607. Header fields: RIFF size, "WAVE", "fmt " (16 bytes: format, channels, rate, byte rate, block align 6, bits 24), "data" size.

## 2. Loop state (in `controller.rs`)
`pub fn state_bytes(&self) -> Vec<u8>` and `pub fn restore_state(&mut self, bytes: &[u8]) -> bool`: serialise as UTF-8 text lines `key=value`: `v=1`, `bars`, `per_bar`, `bpm`, `glide_ms`, `strum_ms`, `smooth` (0/1), `octave`, and `slots` as a comma-separated list where each slot is `-` (empty), `s` (silent) or `root:quality_index` (e.g. `2:13`). `restore_state` returns false (changing nothing) on anything malformed, and re-voices the loop on success.

## 3. Export / import (in `controller.rs`)
- `pub fn export_midi(&self) -> Vec<u8>`: build a `midi::export::LoopExport` from the loop (bpm, slot_beats, glide_ms, per-slot voicings bass first via `loop_voicing`, chord names at each slot, `state_bytes()`), `to_smf`, `to_bytes`.
- `pub fn import_midi(&mut self, bytes: &[u8]) -> Result<(), String>`: parse; if `midi::export::state_of` finds our state, `restore_state` it exactly; otherwise use `midi::recognize::recognize` and write the chords into a loop of the smallest size in {4, 8, 16, 32} bars that fits, half-bar slots (`per_bar = 2`), `None` → `Slot::Silent`. Either way switch to `Mode::Loop`.
- `pub fn render_wav(&self, sample_rate: u32) -> Vec<u8>`: render one pass of the loop offline with a fresh `Engine` (no MIDI), sending `PlayAt` for each slot boundary exactly as `poll` would, plus a 1-second tail, then `wav::wav_24bit`.

## Tests (you write them, as `#[cfg(test)]` modules in the files above)
WAV header bytes and length for a known input; clamping; state round-trip (write a loop, `state_bytes`, restore into a fresh controller, identical slots and settings); malformed state rejected without changes; `export_midi` then `import_midi` restores the exact loop; a foreign MIDI file (build one with `midi::smf` in the test: e.g. C major then G7 as plain notes) imports as chords; `render_wav` produces a non-silent file of the expected length.

## Files you may edit
- crates/app/src/wav.rs (new)
- crates/app/src/controller.rs (add the functions above; don't change existing behaviour)
- crates/app/src/main.rs (only to add `mod wav;`)

## Verify
```
cargo test -p app && cargo clippy -p app --all-targets -- -D warnings
```

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
