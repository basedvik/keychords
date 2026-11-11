# keychords: playful UI redesign

**Subject:** a chord toy for bedroom producers and short-video makers who want lush R&B and neo-soul chords without theory or a DAW.
**Primary job:** open it, press two keys, sound great, keep going on instinct.
**References surveyed:**
- Patatap: every key press answers instantly with colour and motion; palettes swap whole moods.
- Teenage Engineering EP-133: function-coloured tactile keys, a calculator-like body, a screen that dances.
- Chrome Music Lab Chords: playable on load; colour encodes harmonic relationships.
- Propellerhead Figure: flat polygons, large friendly text, a big marker for the current chord, nothing decorative.

## Concept: the computer keyboard is the instrument

The on-screen keyboard stops being a legend and becomes the **hero object**: a light, tactile device body with chunky caps that sink and glow when pressed. Above it sits one dark **screen**, the single bold element, where the voices glow as ribbons in the colour of the current root. Everything else stays quiet.

## Tokens

### Colour
| Token | Value | Use |
|---|---|---|
| body | `#E7E4F1` (lavender felt) | Window background, the device body |
| ink | `#26223F` | Primary text and cap legends |
| ink-soft | `#6E6890` | Secondary text, unused caps |
| cap | `#F8F7FC` | Key cap face |
| cap-edge | `#C9C4DC` | 3 px cap bottom edge (the "depth" of a key) |
| screen | `#1E1A38` | The dark inset screen behind the ribbons |

**Root hues (harmony as colour):** each of the 12 roots gets a hue placed around the **circle of fifths**, so keys a fifth apart get neighbouring colours:
`root_color(pc) = oklch(0.72, 0.16, 20 + 30 * fifths_index(pc))`, where `fifths_index(pc) = (pc * 7) % 12` (C = 0, G = 1, D = 2, …).

The current root's hue tints the ribbons, the chord readout and the root cap.

**Chord families (4 tints on the chord caps):**
| Family | Qualities | Tint |
|---|---|---|
| triads | maj, m, dim, aug, sus2, sus4 | `oklch(0.93, 0.045, 250)` |
| sevenths | maj7, m7, dom7, m7♭5, dim7, 7sus4 | `oklch(0.93, 0.05, 305)` |
| colour | add9, maj6, m6, 6/9 | `oklch(0.94, 0.05, 160)` |
| extended | maj9, m9, dom9, dom13, m11, 7♭9, 7♯9, maj7♯11 | `oklch(0.92, 0.06, 45)` |

### Type
Rune's embedded sans family throughout, with personality carried by **scale and weight**, not extra families:
- the chord readout is huge and heavy, root letter in its root hue
- cap legends are medium
- everything else is small and quiet

Sentence case only: no all-caps labels, no middle-dot meta strings.

### Layout
```
┌────────────────────────────────────────────────────────────────────────┐
│ keychords        Neo-soul keys  Night drive  Lo-fi pluck  …    ▮▮▮▯ level│  ← vibes: one click sets the whole mood
├───────────────────────┬────────────────────────────────────────────────┤
│                       │ ╭──────────────────── screen ────────────────╮ │
│  Dm9                  │ │   ribbons glide in the root's hue           │ │
│  D3 A3 F4 C5 E5       │ │                                     ●now    │ │
│  Try next  G13  Cmaj9 │ ╰─────────────────────────────────────────────╯ │
├───────────────────────┴────────────────────────────────────────────────┤
│   ┌──┐┌──┐┌──┐ …   chunky caps: roots in their hues (left hand),        │
│   └──┘└──┘└──┘     chord caps in family tints (right hand); pressed caps│
│                    sink 3 px and glow; suggested next chords breathe    │
├────────────────────────────────────────────────────────────────────────┤
│  glide ━━━●━━   strum ━●━━━━   reverb ━━━━●━   octave ◀ 0 ▶   loop ▷  midi│
└────────────────────────────────────────────────────────────────────────┘
```
Left-aligned text; the keyboard centred in the body; controls in one quiet row.

## Principles
1. **Press, see, hear in the same frame.** Every press changes the screen colour, sinks a cap and moves a ribbon. Nothing waits.
2. **Colour is harmony.** Root hues follow the circle of fifths; chord caps are tinted by family. You learn the instrument by looking at it.
3. **Zero-friction start.** It opens on a vibe with a gentle hint ("Press D, then ,") breathing on those two caps until the first chord plays.
4. **Suggest, don't instruct.** Up to three "Try next" chords, chosen for smooth voice leading, glow on the keyboard and appear as chips you can click.
5. **One bold thing.** The dark screen with glowing ribbons; everything else stays a calm light device.

## Review against generic defaults (what changed and why)
- **Near-black UI with a single bright accent** (the current UI): replaced with a light device body and one dark screen, so the boldness lives in exactly one place.
- **Identical rounded cards for every section:** removed. The only repeated unit is the key cap, and it repeats because it *is* a physical key.
- **All-caps eyebrows ("PLAY LIVE") and middle-dot status strings:** removed. Mode is shown by the active tab; status is plain words.
- **Stepper buttons (− value +):** replaced by drag sliders, faster to play with and visibly continuous.
- **Copying the reference product's dark-navy, gold-line look:** avoided. Root colours follow the circle of fifths, which is specific to this instrument.
