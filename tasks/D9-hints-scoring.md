# D9: better "Try next" scoring

Update the body of `suggest` in `crates/app/src/hints.rs` to follow the **new** numbered steps in its doc comment (steps 4–6 changed: new score terms, and only the first suggestion per root is kept). The `#[cfg(test)]` module in that file is the spec: do not edit it.

## Files you may edit
- crates/app/src/hints.rs (only the body of `suggest`)

## Verify
```
cargo test -p app hints:: && cargo clippy -p app --all-targets -- -D warnings -A dead-code
```
When done, reply with the test summary.
