# T3: voicing — voice leading engine

Implement every `todo!()` in `crates/voicing/src/lib.rs`: `bass_note`, `motion`, `candidates`, `VoiceLeader::next`, `voice_cycle`.

The doc comments on each function are the spec; the tests in `crates/voicing/tests/acceptance.rs` encode ground truth read from a reference product (Dm9 → G13 → Cmaj9 → A7♭9). Read both fully before writing code.

Notes:
- `candidates`: for each template interval take pitch class `(root + interval) % 12`, enumerate every octave placement inside the band, keep combinations that are ascending, distinct and whose adjacent gaps are ≤ MAX_GAP, return sorted lexicographically. Brute force is fine (≤ 5 voices, ≤ 3 placements each).
- `motion` for unequal lengths is DTW over the two sorted sequences where every element of both is matched at least once; cost is the sum of |Δ| over matched pairs. Equal lengths are 1:1 by index.
- `next`: implement the tie-break order exactly as documented; the Cmaj9 step of the screenshot test depends on it. "Voices left where they were" = count of indices i where prev[i] == cand[i] (only meaningful for equal lengths; count 0 otherwise).
- `voice_cycle`: exact optimisation. For each candidate of the first chord, run a Viterbi/DP over the remaining chords and add the wrap-around cost back to that first candidate; keep the best total. Loops can be up to 64 chords; this is fast enough.
- Do not call `chords` methods whose bodies are `todo!()` (another worker is implementing that crate). Use `Chord`'s public fields and `upper_template`.

## How to work
Work incrementally and keep each reply short. Do **not** plan the whole crate in one go: a previous attempt spent its entire reply budget reasoning and wrote nothing.
1. Implement `bass_note`, `motion` and `candidates`; run the verify command.
2. Implement `VoiceLeader::next`; run it again.
3. Implement `voice_cycle`; run it again, then fix clippy.
Write code early; let the compiler and the tests tell you what is wrong.

## Files you may edit
- crates/voicing/src/lib.rs

## Verify
```
cargo test -p voicing && cargo clippy -p voicing --all-targets -- -D warnings
```

## Rules
- Only edit the files listed under "Files you may edit". Do not touch tests, other crates, Cargo.toml files, or add dependencies.
- Never edit anything under `tests/`. The tests are the spec; if one looks wrong, say so in your final message instead of changing it.
- Replace every `todo!()` in your files with a real implementation. Keep the public signatures exactly as they are.
- Keep the style of the surrounding code: plain, idiomatic Rust, short doc comments only where they add something.
- Loop until the verification command passes with zero failures and zero clippy warnings.
- When done, reply with: what you implemented, the final test output summary line, and anything you think is wrong in the spec.
