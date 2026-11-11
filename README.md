# keychords

A standalone Rust app (Rune UI) that plays full chords from two computer keys — a letter key for the root, another key for the chord type — and glides every voice into the next chord; it sends MIDI to a DAW over a virtual port.

## Build and run

**Toolchain:** see `rust-toolchain.toml` (Rust 1.99.1 with clippy, rustfmt).

```bash
cargo run -p app --release          # launch the app
cargo test --workspace               # run all tests
cargo run -p app -- --snap out.png --scene demo   # headless screenshot
```

## Playing

### Key map

**Root letters (A-G):** any of A B C D E F G selects the root note.

**Accidentals:**
| Key | Action |
|-----|--------|
| Z   | Flat (lower root by semitone) |
| X   | Sharp (raise root by semitone) |
| Shift+A…G | Sharp the root letter (equivalent to X while root is held) |

**Octave:**
| Key | Action |
|-----|--------|
| Q   | Octave down |
| W   | Octave up |

**Utility:**
| Key | Action |
|-----|--------|
| S   | Stop |
| Space | Play / stop transport |

**Sound presets (1-6):**
| Key | Preset |
|-----|--------|
| 1   | Patch 0 |
| 2   | Patch 1 |
| 3   | Patch 2 |
| 4   | Patch 3 |
| 5   | Patch 4 |
| 6   | Patch 5 |

**Chord keys (24 keys, mapped to Q-P row + number/number-row):**

| Key | Label | Quality |
|-----|-------|---------|
| 7 (number row) | maj | Maj |
| 8 | m | Min |
| 9 | dim | Dim |
| 0 | aug | Aug |
| - | dim7 | Dim7 |
| = | 7sus4 | Dom7Sus4 |
| [ | m6 | Min6 |
| ] | 6/9 | SixNine |
| Q | sus2 | Sus2 |
| W | sus4 | Sus4 |
| E | (no action) | — |
| R | (no action) | — |
| Y | 7#9 | Dom7s9 |
| U | maj7 | Maj7 |
| I | m7 | Min7 |
| O | dom7 | Dom7 |
| P | m7b5 | Min7b5 |
| H | 7b9 | Dom7b9 |
| J | sus2 | Sus2 |
| K | sus4 | Sus4 |
| L | add9 | Add9 |
| N | maj7#11 | Maj7s11 |
| M | maj9 | Maj9 |
| , | m9 | Min9 |
| . | dom9 | Dom9 |
| / | dom13 | Dom13 |
| ; | maj6 | Maj6 |
| ' | m11 | Min11 |

### Loop mode controls

| Control | Key |
|---------|-----|
| Move cursor left/right | Left / Right arrow |
| Toggle half-bar timing | Tab |
| Play/stop loop | Space |
| Cycle bar length (4/8/16/32) | UI button (no key) |

## MIDI out

**Modes (cycle with the MIDI button in the UI):**

- **Off** — no MIDI output
- **Plain** — all voices as plain notes on a single MIDI channel (1-16)
- **MPE** — one voice per member channel (channels 2-8), pitch-bend range 48 semitones; each voice is re-struck as the glide lands

macOS ships with a virtual port named `keychords`. Windows requires loopMIDI to create a virtual MIDI port.

## Crates

| Crate | Description |
|-------|-------------|
| `app` | Standalone Rune UI application: main entry point, audio device glue, controller, engine integration |
| `chords` | Chord dictionary: 24 chord qualities on any of 12 roots |
| `keymap` | Input mapping: computer keys to actions, and the MIDI split-point chord protocol |
| `sequencer` | Loop mode: a grid of chord slots played against an internal clock |
| `synth` | Built-in sound engine: up to 7 voices, one patch at a time |
| `voicing` | Voicing and voice leading: bass plus 2-5 upper voices, least total motion |
| `midi` | MIDI: Standard MIDI Files, live MPE and plain-note output, loop export, chord recognition |
| `glide` | Per-voice glide and strum scheduling: fractional MIDI pitch interpolation |

## How it was built

Written by local models (chiz and a Mac mini over a tailnet) orchestrated and reviewed by Claude. See `deploy-plan.md` and `model-quality.md` for details on the development process.
