# D1: theme tokens and vibe presets (translate the tables)

Implement every `todo!()` in `crates/app/src/theme.rs` and `crates/app/src/vibes.rs`. The doc comments give the exact values; the tests at the bottom of each file check them. Read both files first.

- `theme.rs`: `fifths_index`, `root_color`, `family` (a `match` over all 24 `Quality` variants by name), `family_tint`. Use `rune::oklch(l, c, h)` exactly as the doc comments say.
- `vibes.rs`: `vibes()` returns the 6 `Vibe` values from the table in the doc comment, in order. Use `Patch` variants by name.

## Files you may edit
- crates/app/src/theme.rs
- crates/app/src/vibes.rs

## Verify
```
cargo test -p app theme:: && cargo test -p app vibes::
```

## Rules
- Only edit those two files. Do not edit the `#[cfg(test)]` modules (they are the spec), other files, or Cargo.toml.
- Keep public signatures exactly. Plain idiomatic Rust.
- Loop until the verify command passes. Then reply with what you implemented and the test summary.
