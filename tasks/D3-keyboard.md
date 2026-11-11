# D3: the keyboard as the hero (Rune UI)

Rewrite `keymap_panel` in `crates/app/src/main.rs` into the tactile keyboard described in `design.md` (read it first, especially Tokens and Principles). Use the tokens and helpers in `crates/app/src/theme.rs` and suggestions from `crates/app/src/hints.rs`.

Rune is a pre-release UI framework you have not seen. Learn it from `crates/app/src/main.rs` (look at `voice_canvas`, `button`, `meter`) and grep `.research/rune/crates/rune/src` for anything else. Do not guess APIs.

## Behaviour
1. **Caps:** each key is a cap: `theme::CAP` face, radius 10, a 3 px bottom edge in `theme::CAP_EDGE` (draw it as a slightly taller rounded element behind the face, offset down), about 68×58 pt.
   - **Root caps** show their letter large (weight 700) in `theme::root_color` of that letter's pitch class.
   - **Chord caps** use `theme::family_tint(family(q))` as the face and show the chord label (`q.key_label()`).
   - **Utility caps** (octave, stop, flat, sharp, vibes 1–6) keep the plain cap face with `theme::INK_SOFT` legends.
   - **Unused caps** (backquote, R, T, V) are drawn at 40% opacity.
2. **Held keys:** track held keys in `State` (a small `Vec`/set of `KeyInput`s or legend chars). In `on_key`, add the key. Add a key-up handler (`El::on_key_up` exists in `.research/rune/crates/rune/src/input/canvas_input.rs`; attach it to the root `col` in `view`, make that element `focusable(true)` and `autofocus()`) that removes it. A held cap sinks: drop the bottom edge to 1 px and move the face down 2 pt; its face brightens (root caps take a 25%-alpha wash of their root colour).
3. **Current chord:** the cap of the current root gets a 2 pt border in its root colour; the current quality's cap gets a 2 pt border in `theme::INK`.
4. **Suggestions glow:** take `hints::suggest(current, octave, smooth, 3)` when a chord has been played. Their root caps and chord caps get a soft halo: a 4 pt border in the root colour at 45% alpha. Use the suggestion's own root colour for its chord cap.
5. **Start hint:** before the first chord is played, the D cap and the comma cap pulse gently (alpha 0.5 → 1.0 on a ~1.4 s cycle using the seconds value `poll` receives; store it in `State`), and a line above the keyboard reads "Press D, then , to play your first chord". It disappears after the first chord.
6. Clicking a cap still plays it (keep `on_press` calling `s.press(input)`).

## Files you may edit
- crates/app/src/main.rs

## Verify
```
cargo build -p app && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings -A dead-code && cargo run -q -p app -- --snap logs/d3.png --scene demo
```
The reviewer will look at `logs/d3.png`; make sure the demo snapshot shows root hues, family tints, current-chord borders and suggestion halos.

## Rules
- Only edit crates/app/src/main.rs. Keep every existing behaviour (keys, loop strip, controls) working.
- Work incrementally: one behaviour at a time, run the verify command, look at the snapshot, continue.
- When done, reply with what you built, the verify output summary, and anything ambiguous.
