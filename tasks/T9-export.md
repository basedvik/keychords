# T9: midi::export: loop to MIDI file and back

Add `crates/midi/src/export.rs` (and `pub mod export;` in `crates/midi/src/lib.rs`). It turns a voiced chord loop into a Standard MIDI File that plays correctly in a DAW *and* carries the app's own state so it can be loaded back exactly. Use the existing `crate::smf` types (`Smf`, `Track`, `Event`); read `smf.rs` and `mpe.rs` first for conventions.

## API (implement exactly)
```rust
pub struct LoopExport {
    pub bpm: f64,
    /// Beats per slot (4.0 = one chord per bar, 2.0 = half bars), 4/4 time.
    pub slot_beats: f64,
    pub glide_ms: f32,
    /// Per slot, the chord's notes bass first (as the app voices them), or None for silence.
    pub voicings: Vec<Option<Vec<u8>>>,
    /// Per slot, a chord name for a marker at each change, or None.
    pub names: Vec<Option<String>>,
    /// Opaque app state, stored verbatim.
    pub state: Vec<u8>,
}
pub const PPQ: u16 = 480;
pub const STATE_TAG: &[u8; 4] = b"KCHD";
pub fn to_smf(e: &LoopExport) -> Smf;
/// The state blob from a file `to_smf` wrote (a SeqSpecific event whose payload starts with STATE_TAG), else None.
pub fn state_of(smf: &Smf) -> Option<Vec<u8>>;
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
