# D8: layout polish from the D4 review

Read `docs/rune-notes.md` first. Do not read files outside the repo. All changes in `crates/app/src/main.rs`.

The reviewer rendered the demo snapshot at the default window size (1180 × 860). Fix exactly these:

1. **Vibe names overflow the number caps** ("Neo-soul keys" spills into the next cap). On caps 1–6, show a short name in size 11: Neo-soul, Night, Lo-fi, Bells, Choir, Strings. Map from `vibes::vibes()[i].name` by taking the first word, except "Glass bells" → "Bells" and "String swell" → "Strings".
2. **The header runs off the right edge.** Keep the title, mode switch and vibe pills on the left; move the level meter and status into the controls row at the bottom right. Shrink the vibe pills (padding 6, size 13 text) so the header fits 1180 pt.
3. **The controls row runs off the right edge** ("Own sound" and "MIDI out" are cut off). Make the controls a two-line block: line 1 the three sliders and octave; line 2 the toggles (Smooth voice leading, Letter replays chord, Own sound, MIDI out, and a new "Ring" toggle for `s.ctl.ring`), then the level meter and status on the right.
4. **Upper ribbons are nearly white.** Blend toward white by `0.04 * v` (not 0.08) so every ribbon keeps the root's hue.
5. **Ring toggle:** when Ring is off, releasing keys should stop the chord. Call `s.ctl.key_up(input, now)` from the existing key-up handler and send the returned commands (it already exists in `controller.rs`).
6. Make sure nothing in the demo snapshot is clipped at 1180 pt wide.

## Files you may edit
- crates/app/src/main.rs

## Verify
```
cargo build -p app && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings -A dead-code && cargo run -q -p app -- --snap logs/d8.png --scene demo
```
Keep all existing behaviour. When done, reply with what you changed and the verify summary.
