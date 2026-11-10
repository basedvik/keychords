# T4: glide — per-voice slide and strum scheduler

Implement every `todo!()` in `crates/glide/src/lib.rs`. The doc comments (especially the rules on `Glider`) are the spec; `crates/glide/tests/acceptance.rs` uses a 1 kHz sample rate so one sample = 1 ms. Design the private state yourself: a per-voice record of the current slide (from, to, start, end), an active flag or deactivation time, and the count of the latest chord is enough. Do it in this order: helpers (`ms_to_samples`, `ease`, `strum_offset`); then `new`/`pitch`/`play_now` for first chords and equal-size chords; then `play_at`; then voice-count changes; then `release` and `set_params`.

## How to work
Keep each reply short and write code early. Do **not** plan the whole module in one reply; implement one function or group, run the verify command, read the failures, fix, repeat.
- Refer to enum variants by name (e.g. `Patch::Saw`), not by index.
- Write any non-ASCII string literal with `\u{...}` escapes.

## Files you may edit
- crates/glide/src/lib.rs

## Verify
```
cargo test -p glide && cargo clippy -p glide --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit". Do not touch tests, other crates, Cargo.toml files, or add dependencies.
- Never edit anything under `tests/`. The tests are the spec; if one looks wrong, say so in your final message instead of changing it.
- Replace every `todo!()` in your files with a real implementation. Keep the public signatures exactly as they are.
- Keep the style of the surrounding code: plain, idiomatic Rust, short doc comments only where they add something.
- Loop until the verification command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, the final test output summary line, and anything you think is wrong in the spec.
