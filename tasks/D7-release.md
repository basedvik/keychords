# D7: Controller::key_up (ring until the next chord, off)

Implement the `todo!()` in `Controller::key_up` in `crates/app/src/controller.rs`. The doc comment is the spec; `release_tests` checks it. Read `key()` first.

Steps: if `self.ring` is true or the mode is not `Mode::Live`, return an empty vec. Otherwise resolve the input with `self.keymap.resolve(input)`. If it is `KeyAction::Root { letter, .. }` whose `letter.pitch_class()` equals the current chord's root (`self.current()`), or `KeyAction::Quality(q)` equal to the current chord's quality, then `self.leader.reset()` and return `vec![Command::Release]`. Otherwise return an empty vec.

## Files you may edit
- crates/app/src/controller.rs (only the body of `key_up`)

## Verify
```
cargo test -p app release_tests && cargo test -p app
```
Do not edit any test module. When done, reply with the test summary.
