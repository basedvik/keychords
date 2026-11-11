# T13: README for keychords

Write `README.md` at the repository root for people who want to build, run and understand keychords. Read the code to get the facts right; do not invent features.

## Sections
1. **What it is** (2–3 sentences): a standalone Rust app (Rune UI) that plays full chords from two computer keys (a letter for the root, another key for the chord type) and glides every voice into the next chord; it sends MIDI to a DAW over a virtual port.
2. **Build and run**: Rust toolchain from `rust-toolchain.toml`; `cargo run -p app --release`; `cargo test --workspace`; headless screenshot with `cargo run -p app -- --snap out.png --scene demo`.
3. **Playing**: the key map: list every root, chord and utility key exactly as `crates/keymap/src/lib.rs` defines them (letters A–G roots, Z/X flat/sharp, Shift+letter sharp, Q/W octave, S stop, 1–6 sounds, the 24 chord keys with their labels from `crates/chords/src/lib.rs`, Space play/stop). Loop mode controls from `crates/app/src/main.rs` (arrow keys, Tab, Space).
4. **MIDI out**: plain notes on one channel, or MPE (one voice per member channel 2–8, pitch-bend range 48, re-struck as each glide lands); macOS gets a virtual port named "keychords"; Windows needs loopMIDI.
5. **Crates**: one line per crate under `crates/` saying what it does, taken from each crate's top doc comment.
6. **How it was built**: one short paragraph: written by local models (chiz and a Mac mini over a tailnet) orchestrated and reviewed by Claude; see `deploy-plan.md` and `model-quality.md`.

## Files you may edit
- README.md (new)

## Verify
```
test -s README.md && grep -q "cargo run -p app" README.md && grep -q "MPE" README.md
```

## Rules
- Only create README.md. Keep it accurate and concise (under ~150 lines). Use `\u{...}`-free plain Markdown; write ♭ and ♯ as "b" and "#" if unsure.
- When done, reply with a one-paragraph summary.
