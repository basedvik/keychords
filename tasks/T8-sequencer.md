# T8: sequencer — loop slots and clock

Implement every `todo!()` in `crates/sequencer/src/lib.rs`. The doc comments are the spec; `crates/sequencer/tests/acceptance.rs` pins down the edge cases (inheritance across the wrap, silence, boundaries before `origin`). For `boundaries`, compute the first boundary index k with `origin + beat_to_sample(k * slot_beats) >= from` and step forward while the sample is `< to`.

## How to work
Keep each reply short and write code early. Do **not** plan the whole module in one reply; implement one function or group, run the verify command, read the failures, fix, repeat.
- Refer to enum variants by name (e.g. `Patch::Saw`), not by index.
- Write any non-ASCII string literal with `\u{...}` escapes.

## Files you may edit
- crates/sequencer/src/lib.rs

## Verify
```
cargo test -p sequencer && cargo clippy -p sequencer --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit". Do not touch tests, other crates, Cargo.toml files, or add dependencies.
- Never edit anything under `tests/`. The tests are the spec; if one looks wrong, say so in your final message instead of changing it.
- Replace every `todo!()` in your files with a real implementation. Keep the public signatures exactly as they are.
- Keep the style of the surrounding code: plain, idiomatic Rust, short doc comments only where they add something.
- Loop until the verification command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, the final test output summary line, and anything you think is wrong in the spec.
