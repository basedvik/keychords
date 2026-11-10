# T2: keymap — computer-key map and MIDI split decoder

Implement every `todo!()` in `crates/keymap/src/lib.rs`: `Letter::pitch_class`, `KeyMap::resolve`, `SplitDecoder::note_on`, `SplitDecoder::note_off`.

The doc comments in that file are the spec (the US map is listed on `KeyMap`). Expected behaviour is in `crates/keymap/tests/acceptance.rs`. Read both first.

Key points:
- If `input.character` is an ASCII letter, resolve by the letter (case-insensitive), ignoring `phys`. Otherwise resolve by `phys`.
- Dead keys: if `input.phys` is in `self.dead`, that key resolves to nothing; the action whose US position is that dead key is produced by `Phys::Backslash` instead.
- The `chords` crate is being implemented in parallel: **do not call `Quality::from_index`, `index`, or other `chords` methods with `todo!()` bodies.** Use `Quality::ALL[i]` / `Quality::ALL.get(i)` and `Chord::new`, which already work.

## Files you may edit
- crates/keymap/src/lib.rs

## Verify
```
cargo test -p keymap && cargo clippy -p keymap --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit". Do not touch tests, other crates, Cargo.toml files, or add dependencies.
- Never edit anything under `tests/`. The tests are the spec; if one looks wrong, say so in your final message instead of changing it.
- Replace every `todo!()` in your files with a real implementation. Keep the public signatures exactly as they are.
- Keep the style of the surrounding code: plain, idiomatic Rust, short doc comments only where they add something.
- Loop until the verification command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, the final test output summary line, and anything you think is wrong in the spec.
