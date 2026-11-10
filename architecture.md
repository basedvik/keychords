# Swan Glide: System Architecture Breakdown

A teardown of **Swan Glide** by Swanek Audio (v1.8.0, $20), reconstructed from public material only: the product page, the site's JavaScript bundle (which holds the full page copy, specs, FAQ and EULA), two high-resolution UI screenshots, and the vendor's demo video. No binary was decompiled.

Every claim carries one of three tags:

| Tag | Meaning |
|---|---|
| **[V]** Vendor | Stated by Swanek Audio on the product page, FAQ, spec table, EULA or video description. |
| **[O]** Observed | Measured or read directly from the official screenshots, such as pitches read off the draw-mode grid. |
| **[I]** Inferred | My engineering reconstruction. Plausible and consistent with the evidence, but not confirmed. |

---

## 1. What it is, in one paragraph

Swan Glide is a **chord instrument with a computer-keyboard interface and per-voice portamento**. One letter key picks the root (A–G, with accidental modifiers). A second key picks one of 24 chord qualities. The engine then turns that pair into a **7-voice voicing (a bass plus up to six upper voices)** and, rather than retriggering, **slides each voice continuously to its nearest note in the new chord**. The effect is the "neo-soul / R&B glide chords" sound popular on short-form video. **[V]** It runs in three modes:

- **Play live:** a performance instrument.
- **Loop:** a 4–32 bar chord sequencer locked to the DAW transport.
- **Draw:** a piano-roll editor for individual voices and glide start points.

It produces sound in two ways: through 23 built-in synth sounds, or as MIDI and MPE out to drive other instruments. **[V]**

---

## 2. Product surface

| Area | Detail | Src |
|---|---|---|
| Formats | VST3 (Win/Mac), AU (Mac), Standalone app (both) | V |
| No | AAX (no Pro Tools), CLAP, Linux | V (by omission) |
| Plugin type | Instrument with **MIDI in and MIDI out** | V |
| Arch | 64-bit; macOS universal binary (arm64 + x86_64) | V |
| OS | Windows 10/11 x64; macOS 11+ | V |
| Framework | **JUCE**, Steinberg VST3 SDK (EULA credits) | V |
| Mac packaging | One signed and notarized `.pkg` that installs AU + VST3 + app | V |
| Win packaging | Unsigned `.zip`; the user copies `Swan Glide.vst3` to `C:\Program Files\Common Files\VST3` by hand, so SmartScreen warns | V |
| Integrity | SHA-256 checksums for each build are published in the FAQ | V |
| Licensing | **No key, no activation, no demo mode.** Possession of the download is the license. | V |
| Delivery | Stripe Payment Link, then a webhook, then an email (Resend) with Mac and Windows links that always point at the newest build | V + I |
| Updates | About box has "Check for updates" plus an automatic daily check that can be turned off; a dot on the swan icon signals a new version | V |
| Bug reports | "Report a problem" opens a `mailto:` prefilled with version, OS and DAW, and copies the same text to the clipboard as a fallback | V |
| Roadmap | iOS version "coming soon" (video description), most likely AUv3 | V / I |
| Version history | 1.7.0 was the previous release; 1.8.0 is current | V |

> **Inconsistency worth noting:** the EULA says "Swan Glide … has no network features", but the FAQ describes an automatic daily update check. One of the two is stale. The update check is almost certainly a single HTTPS GET for a version manifest. **[I]**

---

## 3. High-level architecture

```
                ┌──────────────────────────── INPUT LAYER ─────────────────────────────┐
 Computer keys ─┤ Layout-aware key mapper (letters by character, others by position)   │
 Mouse clicks  ─┤ On-screen key map (every root/chord key is clickable)                │
 MIDI in       ─┤ Split-point decoder: below split = root PC, split+0..23 = chord type │
 MIDI file     ─┤ Importer: exact round-trip for own files, chord recognition for others│
                └───────────────┬──────────────────────────────────────────────────────┘
                                │  ChordEvent{root, quality, time}
                ┌───────────────▼──────────────── MODE / SEQUENCING ───────────────────┐
                │ Play live: event → immediate voicing change                         │
                │ Loop:     bar grid (4/8/16/32 bars × 1–2 slots) synced to host PPQ  │
                │ Draw:     per-voice note + glide-start overrides on top of the loop │
                └───────────────┬──────────────────────────────────────────────────────┘
                                │  target chord (+ lookahead in Loop mode)
                ┌───────────────▼───────────── HARMONY ENGINE ─────────────────────────┐
                │ Chord dictionary (24 qualities × 12 roots)                          │
                │ Voicer: bass in fixed register + upper-voice candidate voicings      │
                │ Voice leader: pick the voicing and assignment with least total motion│
                │ ("Smooth voice leading" toggle)                                     │
                └───────────────┬──────────────────────────────────────────────────────┘
                                │  per-voice targets: pitch[v], glideStart[v], strumOffset[v]
                ┌───────────────▼───────────── GLIDE SCHEDULER ────────────────────────┐
                │ Per-voice pitch ramps, 0–800 ms, eased                              │
                │ Strum: 0–160 ms onset stagger, bass first                           │
                └───────┬─────────────────────────────────────────┬────────────────────┘
                        │                                         │
        ┌───────────────▼──────────┐               ┌──────────────▼──────────────────┐
        │ Internal synth           │               │ MIDI out                        │
        │ 7 voices × 23 patches    │               │ Plain: notes on one channel     │
        │ + volume + reverb send   │               │ MPE: 1 voice per member channel,│
        │ ("Play its own sound")   │               │      glides as per-note bend    │
        └───────────────┬──────────┘               └──────────────┬──────────────────┘
                        ▼                                         ▼
                 Audio out (+ WAV export)            Host MIDI out bus (+ .mid export)
```

---

## 4. Input layer

### 4.1 Computer-keyboard map (US layout) **[O]** from the UI screenshot

```
 `     1:Pad  2:Saw  3:E-piano 4:Organ 5:Pluck 6:Bells │ 7:maj  8:m   9:dim  0:aug  -:dim7  =:7sus4
   Q:oct−  W:oct+  E:[E]  R:–  T:–                     │ Y:7♯9  U:maj7 I:m7  O:dom7 P:m7♭5 [:m6  ]:6/9
    A:[A]  S:stop  D:[D]  F:[F]  G:[G]                 │ H:7♭9  J:sus2 K:sus4 L:add9 ;:maj6  ':m11
 Shift:+letter=♯  Z:flat♭  X:sharp♯  C:[C]  V:–  B:[B] │ N:maj7♯11 M:maj9 ,:m9 .:dom9 /:dom13
                         Space: DAW play / stop
```

How the keyboard is divided:

- **Roots** live on the left hand, on the seven note letters wherever they sit physically: E, A, D, F, G, C, B. Accidentals come from **Shift+letter** (♯) or the **Z** (♭) and **X** (♯) modifiers.
- **Chord qualities** live on the right hand: 6 + 7 + 6 + 5 = **24 keys**. Each row roughly holds one family:
  - number row: triads plus dim7 and 7sus4
  - top row: sevenths
  - home row: sus, add and 6th chords
  - bottom row: extended chords
- **Utilities:**
  - `1`–`6` choose sound presets
  - `Q`/`W` shift the octave
  - `S` is "stop", a silent slot or rest in Loop mode
  - Arrow keys select the bar; `Tab` toggles half-bar slots
  - `Ctrl+Z` undoes
- Two-key gesture: **letter, then chord key**. With the option "A letter replays the last chord type", a letter alone re-triggers the previous quality on a new root. That allows one-key-per-chord playing once the quality is set. **[V]**

### 4.2 Keyboard-layout independence **[V]** (mechanism **[I]**)

The FAQ describes a two-rule mapping:

1. **Letters map by character.** A is A wherever the layout puts it, so German QWERTZ swaps Z and Y, and French AZERTY moves A, Z, Q, W and M.
2. **Every other key maps by physical position**, as if it were a US key in that spot. German `Ü`, `Ö` and `Ä` therefore become m6, maj6 and m11.
3. A role that would land on a dead/accent key (German `´`, French `^`) **moves to the key beside Enter**.
4. The on-screen key map is redrawn for the user's actual layout.

**[I]** JUCE's `KeyPress` is character-oriented, so this almost certainly goes below JUCE:

- **Windows:** hardware scan codes (`WM_KEYDOWN` lParam bits 16–23), with `ToUnicodeEx` against the active HKL for letter identity.
- **macOS:** positional virtual key codes (`kVK_ANSI_*`), with `UCKeyTranslate` on the current keyboard layout for letter identity. Handling dead keys needs exactly this layer.

### 4.3 Keyboard focus inside a DAW **[V]**

Plugin windows compete with the host for keystrokes:

- A focus banner ("Click here to play with your keyboard" / "Keyboard ready") shows whether the plugin has the keys.
- The FAQ's workarounds: turn off the host's computer-keyboard-to-MIDI feature (press `M` in Ableton Live), or use REAPER's "Send all keyboard input to plug-in".
- **[I]** Space is labelled "DAW play / stop". The likely mechanism is that the plugin does *not* consume Space, so the host still receives it. Plugins can't reliably start a host transport through VST3 or AU.

### 4.4 MIDI input: split-point protocol **[V]**

```
            split (default MIDI 48, shown as "C3 · 48")
                │
  ◄── roots ────┼──────────── chord types (24 semitones) ────────────►
  any note: its │ 48 maj  49 m    50 dim  51 aug  52 maj7  53 m7
  pitch class   │ 54 dom7 55 m7♭5 56 sus2 57 sus4 58 add9  59 maj6
  = root        │ 60 maj9 61 m9   62 dom9 63 dom13 64 dim7 65 7sus4
                │ 66 m6   67 6/9  68 m11  69 7♭9  70 7♯9  71 maj7♯11
```

- A chord is **two simultaneous notes of equal length**: one root, one quality. The vendor's example: Dm7 is D (below split) plus F above the split (53, m7).
- The split point is adjustable ("MIDI keyboard split: roots below, chords from").
- The 24-key order matches the internal `chordTypes` array in the site bundle. That suggests the array index *is* the chord ID throughout the code. **[I]**
- Octave naming uses middle C (60) = C4, so 48 = C3. FL Studio calls the same key C4; the FAQ calls this out.
- **In Loop mode, incoming MIDI notes write chords into the loop** rather than just playing them. Leave the DAW piano roll empty in Loop mode. **[V]**

---

## 5. Chord dictionary

The 24 qualities **[V]**, with the standard interval sets **[I]**. The vendor doesn't publish spellings, and the actual voicings omit tones (see §6):

| # | Key | Name | Intervals (semitones) | Tones |
|---|---|---|---|---|
| 0 | 7 | maj | 0 4 7 | 3 |
| 1 | 8 | m | 0 3 7 | 3 |
| 2 | 9 | dim | 0 3 6 | 3 |
| 3 | 0 | aug | 0 4 8 | 3 |
| 4 | U | maj7 | 0 4 7 11 | 4 |
| 5 | I | m7 | 0 3 7 10 | 4 |
| 6 | O | dom7 | 0 4 7 10 | 4 |
| 7 | P | m7♭5 | 0 3 6 10 | 4 |
| 8 | J | sus2 | 0 2 7 | 3 |
| 9 | K | sus4 | 0 5 7 | 3 |
| 10 | L | add9 | 0 4 7 14 | 4 |
| 11 | ; | maj6 | 0 4 7 9 | 4 |
| 12 | M | maj9 | 0 4 7 11 14 | 5 |
| 13 | , | m9 | 0 3 7 10 14 | 5 |
| 14 | . | dom9 | 0 4 7 10 14 | 5 |
| 15 | / | dom13 | 0 4 7 10 14 (17) 21 | 6–7 |
| 16 | - | dim7 | 0 3 6 9 | 4 |
| 17 | = | 7sus4 | 0 5 7 10 | 4 |
| 18 | [ | m6 | 0 3 7 9 | 4 |
| 19 | ] | 6/9 | 0 4 7 9 14 | 5 |
| 20 | ' | m11 | 0 3 7 10 14 17 | 6 |
| 21 | H | 7♭9 | 0 4 7 10 13 | 5 |
| 22 | Y | 7♯9 | 0 4 7 10 15 | 5 |
| 23 | N | maj7♯11 | 0 4 7 11 (14) 18 | 5–6 |

**Why "7 voices: a bass and six above it":** the largest chord, a full 13th (1 3 5 ♭7 9 11 13), has exactly 7 distinct pitch classes. 7 voices is therefore the polyphony ceiling, not a fixed count. **[I]**

---

## 6. Harmony engine: voicing and voice leading

This is the core intellectual property. The vendor's description is just "each [voice] gliding to its next note by the shortest way" and "glides every voice to its nearest note in the next chord". **[V]** The screenshots are good enough to reverse-engineer what it actually does.

### 6.1 Ground truth read off the Draw-mode grid **[O]**

On the draw grid, C3, C4 and C5 sit at y = 803, 581 and 359 px. That gives **18.5 px per semitone**. Reading every voice of the example loop (Dm9, G13, Cmaj9, A7♭9):

| Voice | Dm9 | G13 | Cmaj9 | A7♭9 | (wraps to Dm9) |
|---|---|---|---|---|---|
| Top | **E5** (9) | **E5** (13) | **E5** (3) | **E5** (5) | E5 |
| 4 | C5 (♭7) | B4 (3) ↓1 | B4 (7) | B♭4 (♭9) ↓1 | C5 ↑2 |
| 3 | F4 (♭3) | F4 (♭7) | G4 (5) ↑2 | G4 (♭7) | F4 ↓2 |
| 2 | A3 (5) | A3 (9) | D4 (9) ↑5 | C♯4 (3) ↓1 | A3 ↓4 |
| Bass | D3 | G2 ↓7 | C3 ↑5 | A2 ↓3 | D3 ↑5 |

What this table shows:

1. **The top voice is a held common tone (E5) through all four chords.** That is the signature "ultraviolet chorus" sound: E is the 9th, then 13th, then 3rd, then 5th. The engine finds and keeps common tones.
2. **Upper voices move by 0–2 semitones almost everywhere.** Upper-voice motion totals 1, 7 and 2 semitones across the three changes.
3. **Voices never cross.** Each change matches sorted old pitches to sorted new pitches. This is order-preserving assignment, the classic result: for sorted sets, monotone matching minimises the summed |Δ|.
4. **The bass does *not* take the shortest path.** D3→G2 is −7 when +5 (to G3) was nearer. The bass is placed by register instead, with roots landing in a window of roughly G2–D3. Bass and upper voices are solved separately. **[I]**
5. **Voicings are curated, not "all chord tones".** G13 is played as G–A–F–B–E (root, 9, ♭7, 3, 13), omitting the 5th and 11th. Dm9 is the open spread D–A–F–C–E. These are pianist voicings (rootless upper structure over a root bass). This example uses 5 voices, not 7.
6. **The upper voices sit in a band of about A3–E5.** The sibling MIDI-pack product quotes "roughly C2 to F5" for its voicings. The same voicing library is probably shared. **[I]**

### 6.2 Likely algorithm **[I]**

```text
on chord change (root r, quality q), given current upper voicing U (sorted pitches):
  bass   := pitch class r placed in the fixed bass window (≈ G2..F#3), then shifted by Octave
  tones  := voicing template for q (which tones to keep or omit and how many voices),
            from a curated per-quality table rather than the raw interval set
  candidates := every octave placement of `tones` inside the upper band [lo, hi]
                that respects spacing rules (no seconds below ~C4, max span, etc.)
  for each candidate C (sorted):
      cost(C) := Σ |C[i] − U[i]|                      # order-preserving matching
               + w_top  · |C.top − U.top|             # protect the melody / top note
               + w_drift· |mean(C) − bandCentre|      # stop slow register drift
               + spacing / omission penalties
  U' := argmin cost  (if "Smooth voice leading" is off: a fixed root-position voicing instead)
  if |U'| ≠ |U|: split or merge voices (a voice "lands" on a doubled or released tone)
```

- The search is tiny: about 6 tones × 2–3 octaves ≈ 3⁶ = 729 candidates. Brute force per chord change takes microseconds, so no Hungarian algorithm or dynamic programming is needed.
- **Loop mode can do better than greedy.** It knows the whole progression, so it can solve the cycle globally with a Viterbi pass over candidate voicings. The wrap from the last chord back to the first is smooth in the screenshot, which hints at cyclic optimisation. Play live can only be greedy.
- A drift term (or a hard band) is essential: pure least-motion leading wanders up or down over a long progression. The fixed E5 top voice and the A3–E5 band suggest one is there.

### 6.3 Open questions

- How changing voice counts are handled, for example triad (3) → 13th (7). A voice could fade in from silence, start on a doubled tone and glide apart, or get dropped.
- Whether the per-quality voicing templates change with context (for example, dom13 is thinned when the chord before it is dense).

---

## 7. Glide and strum scheduler

| Param | Range | Default in screenshot | Src |
|---|---|---|---|
| Glide between chords | 0–800 ms | 220 ms | V / O |
| Strum | 0–160 ms, bass first | 0 ms | V / O |
| Volume | dB | −6.0 dB | O |
| Reverb | % | 22 % | O |
| Octave | integer (Q/W) | 0 | O |

### 7.1 Timing semantics **[O]**

In both screenshots **glides end on the bar line**. Each slide starts about the glide time *before* the downbeat, and the new chord *arrives* on the beat. In the Draw view, held notes stop roughly 32 px short of the next bar. Bars are 344 px wide; at 120 BPM that makes 220 ms ≈ 38 px, which fits.

- **Loop mode = anticipatory glide.** The scheduler looks ahead to the next slot and starts each ramp at `barTime − glideTime`, so the harmony lands on the grid.
- **Play live** can't look ahead, so the ramp must start at the key-press. **[I]**
- **Draw mode** exposes each ramp's start point as a draggable dot, so glide length can differ per voice and per change. **[V]**
- Every slot can override any voice's pitch. A double-click restores the computed note. **[V]**

### 7.2 Curve **[O/I]**

The drawn slides are S-shaped (ease-in-out), not linear. Interpolating in semitones, a log-frequency space, is the musically correct choice and is almost certainly what's done. In the internal synth this is a per-voice smoothed pitch parameter. In MPE it is a stream of 14-bit pitch-bend messages (§9).

### 7.3 Envelopes across a glide **[V/I]**

- "Ring until the next chord" decides whether voices sustain after the keys are released. **[V]**
- On a glide the oscillators keep running and are not retriggered. That legato behaviour is the whole point.
- For percussive patches (Pluck, Koto, Kalimba, Music box) it is unclear whether a glide re-excites the voice. The MIDI exporter re-strikes "as it lands" (§9.2), which hints the internal synth may too. **[I]**

---

## 8. Modes and sequencing

| Mode | Behaviour | Src |
|---|---|---|
| **Play live** | Key or MIDI chord events change the sounding chord immediately. MIDI-in uses the split protocol. | V |
| **Loop** | 4, 8, 16 or 32 bars, with 1 chord per bar or 2 (½-bar toggle) or a silent slot. Follows the host's tempo and bar lines, and Space drives the DAW transport. Workflow: select a slot, press letter + chord key, and the cursor moves to the next slot. | V / O |
| **Draw** | Piano-roll view of the loop: move single notes up or down, drag glide-start dots, double-click to reset. | V / O |

Host sync **[I]**: in `processBlock` the plugin reads JUCE's `AudioPlayHead::PositionInfo`:

- `ppqPosition`, `bpm` and `isPlaying` drive playback
- the loop position is `fmod(ppqPosition, bars × 4)` (the export says 4/4)
- slot boundaries become sample-accurate events inside the block, so glide ramps start at `boundary − glideSamples` (lookahead across blocks)

In the standalone app an internal clock replaces the host.

Utilities **[O]**:

- Undo, Redo and Clear (one undo also reverts an import)
- Load example loop
- Copy the loop (most likely clipboard text)
- Theme (custom colour themes persist across updates)

---

## 9. Output paths

### 9.1 Internal synth **[V]**

- **23 sounds:** Glide pad, Plain sawtooth, Electric piano, Organ, Pluck, Glass bells, Koto, Music box, Banjo, Sax section, Clarinet choir, Pizzicato strings, 12-string guitar, Vibraphone, Felt piano, Handpan, Kalimba, Harp, Choir, String section, French horns, Theremin, Neo-soul guitar.
- 7-voice polyphony, master volume and a reverb send. 24-bit **WAV export** of the loop, rendered offline. **[V]**
- "Play its own sound" off turns it into a pure MIDI controller.
- **[I]** It isn't clear whether these sounds are synthesised or sampled. Swanek's main product, Swan Synth, is a node-based synth, so in-house synthesis (subtractive, FM or modal) is the likely engine for most patches. Acoustic ones such as Sax section, Choir and 12-string could be lightweight samples.

### 9.2 MIDI out **[V]** (wire details **[I]**)

| Setting | Behaviour |
|---|---|
| **Plain notes** | Chord notes as they are voiced, on a chosen channel (1–16). Works with any synth. **No glide.** |
| **MPE** | Each voice gets its own member channel and slides with per-channel pitch bend, so an MPE synth (Vital, Surge XT) glides exactly like the internal engine. |

**[I]** MPE wire format:

- Lower zone: master channel 1, voices on member channels 2–8, which is enough for 7 voices.
- Each voice holds its note-on and ramps pitch bend toward the target. Bend range 48 semitones per member channel is the MPE spec default.
- **[V/I]** At the landing point a voice is re-struck at the new pitch and its bend reset to 0 ("struck as it lands"). That keeps every individual glide inside ±48 even across long progressions.
- Resolution: 14 bits over ±48 semitones ≈ 0.59 cents per step, so the bends are inaudibly smooth.

### 9.3 MIDI file export **[V]**

- **One track per voice**; each note is struck where it lands, and glides are written as MPE pitch bend.
- Each track requests a **pitch-bend range of 48 semitones** (96 when any drawn glide is wider), almost certainly via RPN 0 (CC101/100 = 0, CC6 = range). An ordinary synth that honours RPN, or is set to match, slides correctly. At the default ±2 the slides collapse into slight bends.
- **Lossless round trip:** "Import MIDI brings it back exactly", including chords, drawn notes, glides, and the Glide, Strum, Smooth voice leading, Octave, length and tempo settings. The plugin must embed its own state in the file. **[I]** That is most likely a sequencer-specific meta event (`FF 7F`) or text events holding serialised state, with chord-name markers alongside. The sibling MIDI packs write "a chord-name marker at every change".

### 9.4 MIDI file import (foreign files) **[V]** (algorithm **[I]**)

- Read "as chords, half a bar at a time, up to 32 bars", **channel 10 (drums) ignored**. Drag-and-drop onto the window works.
- **[I]** For each half-bar window:
  1. build a duration-weighted pitch-class histogram
  2. score it against the 24 × 12 chord templates
  3. use the lowest sounding note as the root/bass hint
  4. keep the best match, or write a silent slot if nothing is sounding
- That is standard template-matching chord recognition. Quantising to half bars matches the loop grid's finest slot exactly.

---

## 10. UI composition **[O]**

```
┌ Title · [Play live | Loop | Draw] ·········· [Keyboard-focus banner] [Theme] [Swan/About] ┐
├ Chord readout ("D m9", status "LOOP · STOPPED", hint text) │ Voice-line view (Loop) or   │
│                                                           │ pitch grid editor (Draw)    │
├ [Play loop] [4 bars ▾] [½ bar] · "Follows the DAW's tempo" │ bar slots 1..N              │
├ On-screen key map: 4 rows, roots highlighted gold, chord keys blue, utilities grey       │
├ Sound ▾ · checkboxes │ Glide / Strum / Volume / Reverb │ Octave · MIDI ch · Split · MIDI out ▾│
│ Undo Redo Clear      │                                 │ Load example loop · Copy the loop │
└ Export audio · Export MIDI · Import MIDI                                                  ┘
```

- One visual language connects all three modes: **each voice is a coloured line**, and a glide is a curve between plateaus.
- The key map draws itself for the user's actual layout and doubles as a mouse input surface.
- **[I]** This is a custom-drawn JUCE `Component` tree with a single vector-rendered editor. There are no standard JUCE look-and-feel widgets in sight.

---

## 11. State and persistence **[V/I]**

- Plugin state goes into the DAW project through `getStateInformation` / `setStateInformation`: loop, settings, draw overrides and mode.
- "Your songs, loops and themes carry over" an update. That points to a **versioned state schema** with forward migration, plus user-level files (themes, perhaps a song library) stored outside the project in the user's app-data folder. **[I]**
- Exported `.mid` files double as a portable save format (§9.3).

---

## 12. Commercial and delivery stack

Read from the site bundle and privacy policy. **[V]**

| Concern | Choice |
|---|---|
| Website | React SPA built with Vite and Tailwind; each route is prerendered to its own HTML for SEO (`seo:start/seo:end` markers come from `routes.js`) |
| Hosting | Vercel (also `/api/broadcast` serverless) |
| Payments | Stripe Payment Links (`buy.stripe.com/...`) |
| Email | Resend (download links, receipts) |
| Backend | Supabase (community accounts, uploaded patches, in-app problem reports for other products) |
| Binaries | Other Swanek products sit on public GitHub Releases (`greilly424/*-releases`). **Swan Glide's links appear only in the purchase email**, which suggests private or tokenised URLs. |
| DRM | None. The deliberate tradeoff is zero friction over piracy resistance, at a $20 price. |

---

## 13. Engineering assessment

**What is clever**

1. **Separating the input into root and quality.** Twelve root keys plus twenty-four quality keys cover 288 chords from two keystrokes, and the same factorisation drives MIDI in, the key map and the internal chord ID. It is one clean abstraction used everywhere.
2. **Lookahead glides that land on the beat** in Loop mode. Most portamento lands late; this lands on the grid.
3. **Leading upper voices by least motion over a register-placed bass.** This mirrors how keyboardists actually comp, and it is what makes cheap progressions sound "expensive".
4. **MPE as the glide transport.** Per-voice pitch bend is the only standard way to slide independent notes in MIDI. Using the spec's default range of 48 makes it work with Vital and Surge out of the box.
5. **Self-describing MIDI export.** The `.mid` file is both a DAW-playable render and a lossless save file.

**Risks and limitations**

- **Keyboard capture inside hosts** is inherently fragile; the FAQ has to explain it for each DAW.
- **AU MIDI output:** Logic Pro doesn't route MIDI out of an instrument AU to other tracks, so "drive your other synths" likely works only in VST3 hosts and the standalone app on Mac. **[I]**
- **No AAX**, so no Pro Tools. **Windows builds are unsigned**, which means SmartScreen friction.
- Fixed 4/4 grid with at most 2 chords per bar: no odd meters, pushes or anticipations, except per-voice glide starts in Draw mode.
- Plain-MIDI out drops the glide entirely; the defining feature needs MPE or a bend-range-aware synth.
- Network behaviour in the documentation is inconsistent (§2).

---

## 14. Sources

- Product page: https://swanekaudio.com/swan-glide. The HTML shell holds SEO metadata and a JSON-LD `SoftwareApplication` block (version 1.8.0, price, OS).
- Site bundle: `https://swanekaudio.com/assets/index-BlPIJEk_.js`. The product object (`name`, `version`, `chordTypes`, `sounds`, `specs`, `faq`), the purchase-success flow and the EULA text are all in it.
- Screenshots: https://swanekaudio.com/swan-glide-ui.png (Loop mode) and https://swanekaudio.com/swan-glide-draw.png (Draw mode).
- Demo video: https://www.youtube.com/watch?v=azFEOOtuq1E ("Swan Glide", @swanekaudio, 86 s; the description lists features and the iOS roadmap).
- Downloads and support pages: https://swanekaudio.com/download, https://swanekaudio.com/support
- Third-party project inspired by it: https://github.com/broeib/swan-glide-visualizer (a web visualiser using "minimal-displacement Viterbi voice-leading", a useful reference implementation of the same idea; not Swanek's code).
