# D5: Controller::apply_vibe and Controller::play_chord

Implement the two `todo!()` methods in `crates/app/src/controller.rs`. Their doc comments are the spec; the tests in the `vibe_tests` module at the bottom of the file check them. Read the existing `key()` and private `chord()` methods first and reuse them.

- `apply_vibe`: set `self.patch`, `self.glide_ms`, `self.strum_ms`, `self.leader.octave`, re-voice the loop (call the existing private `revoice_loop`), and return exactly `vec![Command::Patch(..), self.glide(), Command::Reverb(v.reverb)]`.
- `play_chord`: set `self.root = Some(chord.root)` and `self.quality = Some(chord.quality)`, then return `self.chord(chord, now)`.

## Files you may edit
- crates/app/src/controller.rs (only the bodies of `apply_vibe` and `play_chord`)

## Verify
```
cargo test -p app vibe_tests && cargo test -p app
```

## Rules
- Do not edit any test module or anything else. Keep the signatures.
- When done, reply with the test summary.
