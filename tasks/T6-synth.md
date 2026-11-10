# T6: synth — 7-voice built-in sound engine

Implement every `todo!()` in `crates/synth/src/lib.rs`. Read the doc comments and `crates/synth/tests/acceptance.rs` first. Suggested design: a fixed array of 7 voices, each with phase accumulators, a smoothed pitch (one-pole, ~2 ms), and a linear ADSR (attack/decay/sustain/release per patch). Mix voices, scale by 1/4, apply master gain, then `tanh` soft-clip; write the same mono mix to both channels (or a slight per-voice pan). Pluck/EPiano/Bells decay to silence even while held; `strike` restarts the envelope. Keep `render` allocation-free. Start with Saw and Organ, get the in-tune test passing, then add the others.

## How to work
Keep each reply short and write code early. Do **not** plan the whole module in one reply; implement one function or group, run the verify command, read the failures, fix, repeat.
- Refer to enum variants by name (e.g. `Patch::Saw`), not by index.
- Write any non-ASCII string literal with `\u{...}` escapes.

## Files you may edit
- crates/synth/src/lib.rs

## Verify
```
cargo test -p synth --release && cargo clippy -p synth --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit". Do not touch tests, other crates, Cargo.toml files, or add dependencies.
- Never edit anything under `tests/`. The tests are the spec; if one looks wrong, say so in your final message instead of changing it.
- Replace every `todo!()` in your files with a real implementation. Keep the public signatures exactly as they are.
- Keep the style of the surrounding code: plain, idiomatic Rust, short doc comments only where they add something.
- Loop until the verification command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, the final test output summary line, and anything you think is wrong in the spec.
