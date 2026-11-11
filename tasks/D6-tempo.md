# D6: Controller::set_bpm (keep the beat when tempo changes)

Implement the `todo!()` in `Controller::set_bpm` in `crates/app/src/controller.rs`. The doc comment is the spec; the `tempo_tests` module checks it. Read `toggle_play`, `poll` and `playhead` first: `origin` is the engine sample where beat 0 fell, and `playhead` computes the beat as `(now - origin) / samples_per_beat`.

Steps: clamp `bpm` to 60..=180. If the loop is playing, compute the current beat with the old tempo, set the new tempo, then set `origin = now - round(beat * new_samples_per_beat)` (saturating at 0) and `scheduled_to = now`. Otherwise just set the tempo.

## Files you may edit
- crates/app/src/controller.rs (only the body of `set_bpm`)

## Verify
```
cargo test -p app tempo_tests && cargo test -p app
```
Do not edit any test module. When done, reply with the test summary.
