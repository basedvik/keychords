# T12: synth: the remaining built-in sounds

`crates/synth/src/lib.rs` has 6 patches. Add 17 more variants to `Patch`, after the existing six and in this order, with these labels (use `\u{...}` escapes only if you need non-ASCII; none of these do):

Koto "Koto", MusicBox "Music box", Banjo "Banjo", SaxSection "Sax section", ClarinetChoir "Clarinet choir", PizzicatoStrings "Pizzicato strings", TwelveString "12-string guitar", Vibraphone "Vibraphone", FeltPiano "Felt piano", Handpan "Handpan", Kalimba "Kalimba", Harp "Harp", Choir "Choir", StringSection "String section", FrenchHorns "French horns", Theremin "Theremin", NeoSoulGuitar "Neo-soul guitar".

`Patch::ALL` becomes all 23 in order; keep the existing six first and unchanged (the app maps keys 1–6 to `ALL[0..6]`).

Each new sound is synthesised in code (no samples): pick a fitting technique per sound (plucked: Karplus–Strong or decaying filtered saw; bowed/blown: detuned saws or pulses with a slow attack and gentle low-pass; mallet/bell: FM or decaying sine partials; theremin: sine with vibrato; choir: several detuned saws through formant-ish band-passes). They only need to be recognisable and musical, not perfect.

## Requirements (your tests must check them for every one of the 23 patches)
- Bounded output (|x| <= 1), never NaN, audible when 7 voices sound, and silent within 5 s after all voices release (the existing test `every_patch_sounds_stays_bounded_and_releases` already loops over `Patch::ALL`, so it covers the new ones automatically: keep it passing).
- Sustained sounds (sax, clarinet, strings, choir, horns, theremin) play in tune: add an in-tune test for them like `sustained_patches_play_in_tune`.
- `render` stays allocation-free.

## Files you may edit
- crates/synth/src/lib.rs
- crates/synth/tests/sounds.rs (new: your tests)

## Verify
```
cargo test -p synth --release && cargo clippy -p synth --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit" (creating them if needed). Do not touch other crates, existing tests, or Cargo.toml files, and add no dependencies.
- **You write the tests.** Put them in the test file listed. Cover the rules in this spec, not just the happy path: a reviewer will read your tests as carefully as your code.
- Keep the style of the surrounding crates: plain, idiomatic Rust, short doc comments. Refer to enum variants by name; write any non-ASCII literal with `\u{...}` escapes.
- Work incrementally: write a little, run the verify command, fix, repeat. Keep each reply short.
- Loop until the verify command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, what your tests cover, the final test summary line, and anything in this spec you found ambiguous.
