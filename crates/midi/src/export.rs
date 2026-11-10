//! Export a voiced chord loop to a Standard MIDI File.
//!
//! `to_smf` writes a format 1 file at [`PPQ`]: a "keychords" conductor track (tempo, 4/4,
//! a chord-name marker at each change, and the app state under [`STATE_TAG`]), then one
//! track per voice on MPE member channel `1 + v`. Voices are struck as they land and glide
//! by bending over the window before the strike, so the file plays in a DAW; `state_of`
//! recovers the state blob so the loop can be loaded back exactly.

use crate::smf::{Event, Smf, Track};

/// A voiced chord loop, ready for export.
#[derive(Clone, Debug, PartialEq)]
pub struct LoopExport {
    pub bpm: f64,
    /// Beats per slot (4.0 = one chord per bar, 2.0 = half bars), 4/4 time.
    pub slot_beats: f64,
    pub glide_ms: f32,
    /// Per slot, the chord's notes bass first (as the app voices them), or None for silence.
    pub voicings: Vec<Option<Vec<u8>>>,
    /// Per slot, a chord name for a marker at each change, or None.
    pub names: Vec<Option<String>>,
    /// Opaque app state, stored verbatim.
    pub state: Vec<u8>,
}

/// Ticks per quarter note.
pub const PPQ: u16 = 480;

/// Tag for the `SeqSpecific` event that carries the app state.
pub const STATE_TAG: &[u8; 4] = b"KCHD";

/// The loop as a Standard MIDI File: format 1, [`PPQ`], conductor track first, then one
/// track per voice that ever sounds.
pub fn to_smf(e: &LoopExport) -> Smf {
    let slots = e.voicings.len();
    let tick_of = |k: usize| (k as f64 * e.slot_beats * f64::from(PPQ)).round() as u32;
    let end_tick = (slots as f64 * e.slot_beats * f64::from(PPQ)).round() as u32;
    let range = slide_range(e);

    let mut conductor: Vec<(u32, Event)> = vec![
        (0, Event::TrackName("keychords".into())),
        (0, Event::Tempo((60_000_000.0 / e.bpm).round() as u32)),
        (0, Event::TimeSig { num: 4, den_pow2: 2, clocks: 24, n32: 8 }),
    ];
    for k in 0..slots {
        let changed = if k == 0 {
            e.voicings[0].is_some()
        } else {
            e.voicings[k] != e.voicings[k - 1]
        };
        if changed {
            if let Some(Some(name)) = e.names.get(k) {
                conductor.push((tick_of(k), Event::Marker(name.clone())));
            }
        }
    }
    let mut state = STATE_TAG.to_vec();
    state.extend_from_slice(&e.state);
    conductor.push((0, Event::SeqSpecific(state)));

    let voices = e
        .voicings
        .iter()
        .filter_map(Option::as_ref)
        .map(|n| n.len())
        .max()
        .unwrap_or(0);
    let mut tracks = vec![Track { events: conductor }];
    for v in 0..voices {
        let mut voice = Voice::new(1 + v as u8, range);
        for k in 0..slots {
            let note = e.voicings[k].as_ref().and_then(|n| n.get(v)).copied();
            voice.slot(note, tick_of(k), e, range);
        }
        voice.release(end_tick);
        tracks.push(Track { events: voice.events });
    }
    Smf { ppq: PPQ, tracks }
}

/// The state blob from a file `to_smf` wrote (a `SeqSpecific` event whose payload starts
/// with [`STATE_TAG`]), else `None`.
pub fn state_of(smf: &Smf) -> Option<Vec<u8>> {
    for track in &smf.tracks {
        for (_, event) in &track.events {
            if let Event::SeqSpecific(payload) = event {
                if payload.starts_with(STATE_TAG) {
                    return Some(payload[STATE_TAG.len()..].to_vec());
                }
            }
        }
    }
    None
}

/// One voice track: RPN 0 setup on its channel, per-slot strikes and glides, release at
/// the loop end.
struct Voice {
    events: Vec<(u32, Event)>,
    ch: u8,
    held: Option<u8>,
    last_strike: u32,
}

impl Voice {
    fn new(ch: u8, range: u8) -> Voice {
        Voice {
            events: vec![
                (0, Event::Cc { ch, cc: 101, val: 0 }),
                (0, Event::Cc { ch, cc: 100, val: 0 }),
                (0, Event::Cc { ch, cc: 6, val: range }),
                (0, Event::Cc { ch, cc: 38, val: 0 }),
            ],
            ch,
            held: None,
            last_strike: 0,
        }
    }

    /// The slot's note (or silence) at slot start `s`.
    fn slot(&mut self, note: Option<u8>, s: u32, e: &LoopExport, range: u8) {
        match (self.held, note) {
            (Some(a), None) => self.events.push((s, Event::NoteOff { ch: self.ch, note: a, vel: 0 })),
            (None, Some(n)) => self.strike(n, s),
            (Some(a), Some(n)) if a != n => {
                self.glide(a, n, s, e, range);
                self.events.push((s, Event::NoteOff { ch: self.ch, note: a, vel: 0 }));
                self.strike(n, s);
            }
            _ => {}
        }
        self.held = note;
    }

    /// Strike `note` at `at`: bend reset to centre, then note on at velocity 100.
    fn strike(&mut self, note: u8, at: u32) {
        self.events.push((at, Event::PitchBend { ch: self.ch, value: 8192 }));
        self.events.push((at, Event::NoteOn { ch: self.ch, note, vel: 100 }));
        self.last_strike = at;
    }

    /// Glide bends for a move `a` -> `b` at slot start `s`: a ramp over the glide window
    /// before `s`, clamped to start no earlier than the previous strike.
    fn glide(&mut self, a: u8, b: u8, s: u32, e: &LoopExport, range: u8) {
        if e.glide_ms <= 0.0 {
            return;
        }
        let glide_ticks = (f64::from(e.glide_ms) / 1000.0 * e.bpm / 60.0 * f64::from(PPQ)).round() as u32;
        let start = s.saturating_sub(glide_ticks).max(self.last_strike);
        if start >= s {
            return;
        }
        let delta = i16::from(b) - i16::from(a);
        let span = f64::from(s - start);
        let mut t = start;
        while t < s {
            let x = f64::from(t - start) / span;
            self.events.push((t, Event::PitchBend { ch: self.ch, value: glide_bend(f64::from(delta), x, range) }));
            t += 10;
        }
    }

    /// Release the held note, if any, at the loop end.
    fn release(&mut self, at: u32) {
        if let Some(n) = self.held {
            self.events.push((at, Event::NoteOff { ch: self.ch, note: n, vel: 0 }));
        }
    }
}

/// Bend at progress `x` (0..1) of a `delta`-semitone slide at range `range`:
/// `8192 + round(delta * ease(x) / range * 8192)`, clamped to 0..=16383, where
/// `ease(x) = 3x^2 - 2x^3`.
fn glide_bend(delta: f64, x: f64, range: u8) -> u16 {
    let ease = 3.0 * x * x - 2.0 * x * x * x;
    (8192.0 + delta * ease / f64::from(range) * 8192.0).round().clamp(0.0, 16383.0) as u16
}

/// 48 semitones, or 96 if any slide in the loop is wider than 48.
fn slide_range(e: &LoopExport) -> u8 {
    let widest = e.voicings.windows(2).fold(0u16, |w, pair| {
        let (Some(p), Some(c)) = (&pair[0], &pair[1]) else {
            return w;
        };
        let d = p.iter().zip(c).map(|(a, b)| (i16::from(*b) - i16::from(*a)).unsigned_abs()).max().unwrap_or(0);
        w.max(d)
    });
    if e.glide_ms > 0.0 && widest > 48 {
        96
    } else {
        48
    }
}
