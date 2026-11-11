# D4: the device layout (Rune UI)

Restyle the rest of the window to the layout and tokens in `design.md` (read the Layout wireframe and Principles first). Learn Rune from `crates/app/src/main.rs` and `.research/rune/crates/rune/src`; do not guess APIs.

## Changes, all in `crates/app/src/main.rs`
1. **Body:** window background `theme::BODY`; primary text `theme::INK`, secondary `theme::INK_SOFT`. Remove the dark `tok::theme(true)` surfaces except the screen.
2. **Header row:** "keychords" (weight 700) on the left, then the six vibes from `vibes::vibes()` as pill buttons (the active one filled in `theme::INK` with `theme::CAP` text). Clicking a vibe calls `s.ctl.apply_vibe(&v)` and sends the commands it returns (also set `s.reverb = v.reverb`). Keys 1–6 select vibes too (in `on_key`, a `KeyAction::Sound(i)` now applies `vibes()[i]` via `apply_vibe`). The level meter and status sit on the right. The status has no middle dots: "Sound on, MIDI port keychords" or the error text.
3. **Screen:** `voice_canvas` sits in a `theme::SCREEN` inset (radius 14). Its ribbons use the **current root's** `theme::root_color`, from lightest for the top voice to most saturated for the bass: blend toward white by `0.08 * (voices - 1 - v)`. The C guide lines stay faint.
4. **Readout (left of the screen, on the body, no card):** chord name huge (size 64, weight 800) in `theme::INK` with the root letter in its root colour; voice notes below in `theme::INK_SOFT`; then "Try next" followed by up to 3 chip buttons from `hints::suggest`, each labelled with the chord name and playing that chord when clicked via `s.ctl.play_chord(chord, s.now())` (send the commands it returns). Remove the "PLAY LIVE" label; the active mode tab says it.
5. **Controls row:** replace the − value + steppers for glide, strum and reverb with **drag sliders**: a 140 pt track (`theme::CAP_EDGE`) with a filled part (`theme::INK`) and a 14 pt round thumb; drag horizontally to set the value (look for `on_drag` in Rune; the sketch example at `.research/rune/examples/sketch/` uses it). Show the value as text after the label ("Glide 220 ms"). Keep octave as ◀ value ▶ and keep the toggles, styled as quiet outline pills.
6. Mode tabs ("Play live", "Loop") become a two-segment control in the header.

## Files you may edit
- crates/app/src/main.rs

## Verify
```
cargo build -p app && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings -A dead-code && cargo run -q -p app -- --snap logs/d4.png --scene demo
```
The reviewer will compare `logs/d4.png` against the wireframe in design.md.

## Rules
- Only the files listed. Keep all existing behaviour working.
- Work incrementally; look at the snapshot after each step.
- When done, reply with what you built, the verify summary, and anything ambiguous.
