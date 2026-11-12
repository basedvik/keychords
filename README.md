# keychords

A standalone Rust app (Rune UI) that plays full chords from two computer keys — a letter (A–G) for the root and another key for the chord type — and glides every voice into the next chord with smooth voice leading; it sends MIDI out over a virtual port to a DAW or synth.

## Build it yourself

No installer, no account, no downloaded release needed — just Rust and the repo.

1. Install Rust from [rustup.rs](https://rustup.rs) if you don't have it. The `rust-toolchain.toml` pins Rust 1.99.1, and rustup will fetch it automatically.
2. Clone and enter the repo:

   ```bash
   git clone https://github.com/basedvik/keychords.git
   cd keychords
   ```

3. Build the standalone binary:

   ```bash
   cargo build -p app --release
   ```

   The first build pulls Rune from git and takes a few minutes.

4. Run it:

   ```bash
   ./target/release/keychords
   ```

That binary is self-contained — copy it wherever you like. On Linux you'll need ALSA headers first (`sudo apt install libasound2-dev`, or `sudo dnf install alsa-lib-devel` on Fedora); macOS needs nothing extra.

Handy extras:

```bash
cargo test --workspace              # all tests
cargo run -p app --release          # build and run in one step
cargo run -p app -- --snap out.png --scene demo  # headless screenshot
```

## Playing

### Root keys

| Key | Action |
|-----|--------|
| A–G | Select a root (letter names; case-insensitive) |
| Z   | Flat the next root letter you press (one-shot) |
| X   | Sharp the next root letter you press (one-shot) |
| Shift+letter | That root, sharp |

### Octave

| Key | Action |
|-----|--------|
| Q   | Octave down |
| W   | Octave up |

Octave ranges from -2 to +2.

### Stop

| Key | Action |
|-----|--------|
| S   | Stop (live mode) / write silent slot (loop mode) |

### Play / stop transport

| Key | Action |
|-----|--------|
| Space | Play / stop (loop mode) |

### Chord keys (24 keys on the right-hand side of the keyboard)

Each key sets the chord quality. Combined with a root letter, it plays the chord.

| Key | Label | Quality |
|-----|-------|---------|
| 7   | maj   | Major triad |
| 8   | m     | Minor triad |
| 9   | dim   | Diminished triad |
| 0   | aug   | Augmented triad |
| -   | dim7  | Diminished 7th |
| =   | 7sus4 | Dominant 7 sus4 |
| [   | m6    | Minor 6th |
| ]   | 6/9   | Six-nine |
| Y   | 7#9   | Dominant 7 sharp 9 |
| U   | maj7  | Major 7th |
| I   | m7    | Minor 7th |
| O   | dom7  | Dominant 7th |
| P   | m7b5  | Half-diminished 7th |
| H   | 7b9   | Dominant 7 flat 9 |
| J   | sus2  | Suspended 2nd |
| K   | sus4  | Suspended 4th |
| L   | add9  | Add 9 |
| N   | maj7#11 | Major 7 sharp 11 |
| M   | maj9  | Major 9th |
| ;   | maj6  | Major 6th |
| '   | m11   | Minor 11th |
| ,   | m9    | Minor 9th |
| .   | dom9  | Dominant 9th |
| /   | dom13 | Dominant 13th |

### Sound presets

The header vibe pills select vibes; each sets the sound, glide, strum, reverb and octave at once. The app opens on **Night drive**. The synth has 23 sounds in total.

| Vibe name | Patch |
|-----------|-------|
| Neo-soul keys | E-piano |
| Night drive | GlidePad |
| Lo-fi pluck | Pluck |
| Glass bells | Bells |
| Choir cloud | Choir |
| String swell | StringSection |

### Loop mode controls

Click **Loop** to switch to the loop sequencer. Controls:

| Key | Action |
|-----|--------|
| Left / Right arrow | Move cursor in loop |
| Tab | Toggle half-bar granularity |
| Space | Play / stop loop |

Loop bars cycle 4 → 8 → 16 → 32 via the UI button.

## MIDI out

Cycle modes with the "MIDI out" button:

- **Off** (default): no MIDI sent.
- **Plain**: all voices as plain notes on channel 1. No glide over MIDI: notes change instantly.
- **MPE**: one voice per member channel 2–8, pitch-bend range 48 semitones, re-struck as each glide lands.

The app creates a virtual MIDI port named `keychords` on macOS (and Linux); on Windows, install loopMIDI and the app connects to its port.

## Crates

- **chords** — Chord dictionary: 24 chord qualities on any of 12 roots. `Quality::ALL` order is load-bearing.
- **keymap** — Input mapping: computer keys to actions, and the MIDI split-point chord protocol.
- **voicing** — Voice leading: computes smooth voicings between chords, including the voice cycle for loops.
- **midi** — Standard MIDI Files, live MPE and plain-note output, loop export, chord recognition.
- **glide** — Per-voice glide and strum scheduling. Smoothstep interpolation between chord tones.
- **synth** — Built-in sound engine: up to 7 voices, one patch at a time. 23 patches from Pad to Neo-soul guitar.
- **sequencer** — Loop sequencer: configurable bar counts (4/8/16/32), half-bar slots, slot state.
- **app** — Standalone Rune UI app: keychords binary with keyboard input, audio output, and MIDI.

## How it was built

Built by local models (chiz and a Mac mini over a tailnet), orchestrated and reviewed in opencode with DeepSeek V4.1 Flash; see `deploy-plan.md` and `model-quality.md`.

## Contributors

- chiz (Qwen 3.8 27B)
- mini (Qwen 3.6 35B A3B, Mac mini over a tailnet)
- opencode (DeepSeek V4.1 Flash) — UI rewrite, input handling, and sound-preset selector polish
