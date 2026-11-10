# T1: chords — chord dictionary

Implement every `todo!()` in `crates/chords/src/lib.rs`: `Quality::index`, `from_index`, `intervals`, `key_label`, `suffix`, `root_name`, `Chord::pitch_classes`, `Chord::name`.

The exact expected values are in `crates/chords/tests/acceptance.rs` (the TABLE constant and the tests). Read it first. Use `match` tables; return `&'static` slices and strs.

## Files you may edit
- crates/chords/src/lib.rs

## Verify
```
cargo test -p chords && cargo clippy -p chords --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit". Do not touch tests, other crates, Cargo.toml files, or add dependencies.
- Never edit anything under `tests/`. The tests are the spec; if one looks wrong, say so in your final message instead of changing it.
- Replace every `todo!()` in your files with a real implementation. Keep the public signatures exactly as they are.
- Keep the style of the surrounding code: plain, idiomatic Rust, short doc comments only where they add something.
- Loop until the verification command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, the final test output summary line, and anything you think is wrong in the spec.
