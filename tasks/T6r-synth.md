# T6 (retry): synth: translate this design into Rust

Implement every `todo!()` in `crates/synth/src/lib.rs` by **translating the design below**. Do not invent a different architecture. A previous attempt failed by over-engineering (it used a `rand` crate that is not a dependency, mixed f32/f64, and fought the borrow checker). Use **only f32** and **no external crates**.

## Design (translate step by step)
1. Constants: `const VOICE_GAIN: f32 = 0.18;` A `Voice` struct (derive Clone, Copy, Debug, Default) with fields: `pitch: f32` (current, smoothed), `target: f32`, `on: bool`, `env: f32` (0..1), `stage: u8` (0 idle, 1 attack, 2 decay, 3 sustain, 4 release), `phase: [f32; 3]` (oscillator phases 0..1), `lp: f32` (one-pole filter state).
2. `Synth` fields: `sample_rate: f32`, `voices: [Voice; MAX_VOICES]`, `patch: Patch`, `gain: f32` (linear, from dB: `10f32.powf(db / 20.0)`), default gain for 0 dB is 1.0.
3. Per patch, a `fn params(patch) -> (attack_s, decay_s, sustain, release_s, decays_while_held: bool)`:
   GlidePad (0.25, 0.3, 0.8, 0.6, false); Saw (0.005, 0.1, 0.9, 0.15, false); EPiano (0.003, 1.2, 0.0, 0.3, true); Organ (0.002, 0.01, 1.0, 0.03, false); Pluck (0.002, 0.6, 0.0, 0.2, true); Bells (0.002, 2.0, 0.0, 0.8, true).
   "decays_while_held" means sustain is 0 and the decay stage continues to 0 while the key is held.
4. Envelope per sample (rate-based, linear): attack: env += 1/(attack_s*sr) until 1, then decay; decay: env -= (1-sustain)/(decay_s*sr) (or env/(decay_s*sr) if sustain is 0) until sustain (or 0); release: env -= env_at_release/(release_s*sr); simplest acceptable: env -= 1/(release_s*sr) clamped at 0, then stage 0 and on=false.
5. `set_voice(v, Some(p))`: if the voice was not sounding (stage 0 or 4), set pitch = p immediately, stage = 1 (attack from current env), `on = true`; always set target = p. `set_voice(v, None)`: if stage is 1..=3, stage = 4.
6. `strike(v)`: if the voice is on, stage = 1 and env = 0.
7. Pitch smoothing per sample: `pitch += (target - pitch) * k` with `k = 1 - (-1.0 / (0.002 * sr)).exp()` (computed once in `new`). Frequency `f = midi_to_hz(pitch)`, phase increment `dt = f / sr`.
8. Oscillators (phase in 0..1, advance with `phase = (phase + dt).fract()`):
   - saw(phase, dt) = `2*phase - 1 - poly_blep(phase, dt)`, where `poly_blep(t, dt)`: if t < dt { let x = t/dt; x+x - x*x - 1 } else if t > 1-dt { let x = (t-1)/dt; x*x + x+x + 1 } else { 0 }.
   - sine(phase) = `(phase * TAU).sin()`.
   Per patch sample (before envelope):
   - GlidePad: average of saw at phase[0] (dt) and saw at phase[1] (dt * 1.006), then one-pole low-pass `lp += (x - lp) * 0.08`; output lp.
   - Saw: saw(phase[0]).
   - EPiano: sine(phase[0]) + 0.3 * env * sine(phase[1]) with phase[1] at 2*dt.
   - Organ: (sine(p0) + 0.5*sine(p1 at 2*dt) + 0.25*sine(p2 at 3*dt)) / 1.75.
   - Pluck: saw(phase[0]) through `lp += (x - lp) * (0.05 + 0.5 * env)`; output lp.
   - Bells: FM: modulator m = sine(phase[1]) with phase[1] at 3.5*dt; carrier sine(phase[0] + 0.3 * env * m).
9. `render(left, right)`: for each sample, sum over voices with stage != 0 of `osc * env * VOICE_GAIN`, multiply by `gain`, soft clip `x.tanh()`, write to both channels. Allocation-free.
10. `midi_to_hz(p)` = `440.0 * 2f32.powf((p - 69.0) / 12.0)`. `Patch::label` per the doc comment.

## How to work
Write the whole file from this design in one go, run the verify command, fix compile errors, then fix failing tests. Keep replies short.

## Files you may edit
- crates/synth/src/lib.rs

## Verify
```
cargo test -p synth --release && cargo clippy -p synth --all-targets -- -D warnings
```

## Rules
- Only edit crates/synth/src/lib.rs. Do not touch tests, other crates or Cargo.toml; add no dependencies.
- Keep the public signatures exactly as they are.
- When done, reply with: what you implemented, the final test summary line, and anything wrong in the spec.
