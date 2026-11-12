//! keychords standalone app (Rune UI).

mod audio;
mod controller;
mod reverb;
mod theme;
mod vibes;
mod engine;
mod hints;
mod wav;

use std::sync::atomic::Ordering;
use std::time::Duration;

use keymap::{KeyAction, KeyInput, Phys};
use rune::{canvas, col, el, row, text, AppSpec, Color, Cx, El, Key, KeyEvent, LineCap, Path, PhysicalKeyCode, Phase, Shape, TextStyle, Tx};

use controller::{Controller, Mode};
use engine::{Command, MidiMode};
use hints::Suggestion;

// ---------------------------------------------------------------- constants

const CAP_W: f32 = 70.0;
const CAP_H: f32 = 60.0;
const CAP_DEPTH: f32 = 3.0;
const CAP_SINK: f32 = 2.0;
const CAP_RADIUS: f32 = 10.0;
const CAP_GAP: f32 = 6.0;
const ROW_STAGGER: f32 = 22.0;
const TRACK_W: f32 = 140.0;
const THUMB: f32 = 14.0;
const METER_W: f32 = 100.0;

const TITLE: TextStyle = TextStyle::new(22.0, 700.0);
const TAB: TextStyle = TextStyle::new(14.0, 600.0);
const VIBE: TextStyle = TextStyle::new(13.0, 500.0);
const LABEL: TextStyle = TextStyle::new(14.0, 500.0);
const META: TextStyle = TextStyle::new(14.0, 400.0);
const CAP_KEY: TextStyle = TextStyle::new(11.0, 400.0);
const CAP_LEGEND: TextStyle = TextStyle::new(13.0, 500.0);
const ROOT_LETTER: TextStyle = TextStyle::new(20.0, 700.0);
const READOUT: TextStyle = TextStyle::new(64.0, 800.0);
const BODY_TEXT: TextStyle = TextStyle::new(15.0, 400.0);
const CHIP: TextStyle = TextStyle::new(13.0, 500.0);
const SLOT: TextStyle = TextStyle::new(13.0, 500.0);
const STEP: TextStyle = TextStyle::new(15.0, 600.0);

fn clear_cap() -> Color {
    Color { a: 0.0, ..theme::cap() }
}

fn c_alpha(col: Color, a: f32) -> Color {
    Color { a: col.a * a, ..col }
}

/// Blend `c` toward white by `t` (0..=1).
fn mix_white(col: Color, t: f32) -> Color {
    let w = 255.0f32;
    Color {
        r: col.r + (w - col.r) * t,
        g: col.g + (w - col.g) * t,
        b: col.b + (w - col.b) * t,
        a: col.a,
    }
}

// ---------------------------------------------------------------- input ids

/// A keyboard position: letters by character, everything else by physical key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CapId {
    Letter(char),
    Phys(Phys),
}

impl CapId {
    fn of(input: &KeyInput) -> Option<Self> {
        match input.character {
            Some(c) if c.is_ascii_alphabetic() => Some(Self::Letter(c.to_ascii_lowercase())),
            _ => match input.phys {
                Phys::Letter | Phys::Other | Phys::Space => None,
                phys => Some(Self::Phys(phys)),
            },
        }
    }
}

fn phys_of(code: Option<PhysicalKeyCode>) -> Phys {
    use PhysicalKeyCode as K;
    match code {
        Some(K::Backquote) => Phys::Backquote,
        Some(K::Digit1) => Phys::Digit1,
        Some(K::Digit2) => Phys::Digit2,
        Some(K::Digit3) => Phys::Digit3,
        Some(K::Digit4) => Phys::Digit4,
        Some(K::Digit5) => Phys::Digit5,
        Some(K::Digit6) => Phys::Digit6,
        Some(K::Digit7) => Phys::Digit7,
        Some(K::Digit8) => Phys::Digit8,
        Some(K::Digit9) => Phys::Digit9,
        Some(K::Digit0) => Phys::Digit0,
        Some(K::Minus) => Phys::Minus,
        Some(K::Equal) => Phys::Equal,
        Some(K::BracketLeft) => Phys::BracketLeft,
        Some(K::BracketRight) => Phys::BracketRight,
        Some(K::Backslash | K::IntlBackslash) => Phys::Backslash,
        Some(K::Semicolon) => Phys::Semicolon,
        Some(K::Quote) => Phys::Quote,
        Some(K::Comma) => Phys::Comma,
        Some(K::Period) => Phys::Period,
        Some(K::Slash) => Phys::Slash,
        Some(K::Space) => Phys::Space,
        Some(
            K::KeyA | K::KeyB | K::KeyC | K::KeyD | K::KeyE | K::KeyF | K::KeyG | K::KeyH | K::KeyI
            | K::KeyJ | K::KeyK | K::KeyL | K::KeyM | K::KeyN | K::KeyO | K::KeyP | K::KeyQ | K::KeyR
            | K::KeyS | K::KeyT | K::KeyU | K::KeyV | K::KeyW | K::KeyX | K::KeyY | K::KeyZ,
        ) => Phys::Letter,
        _ => Phys::Other,
    }
}

/// One key event as the app reports it, with the physical position filled in.
fn key_input(e: &KeyEvent, tx: &mut Tx) -> KeyInput {
    match &e.key {
        Key::Text(t) => KeyInput {
            character: t.chars().next(),
            phys: phys_of(tx.physical_key()),
            shift: e.mods.shift,
        },
        _ => KeyInput { character: None, phys: Phys::Other, shift: e.mods.shift },
    }
}

// ---------------------------------------------------------------- state

struct State {
    ctl: Controller,
    audio: Option<audio::Audio>,
    status: String,
    midi: MidiMode,
    own_sound: bool,
    /// Output level for the meter, 0..=1, with a falling decay.
    meter: f32,
    /// Keys currently held, for the sunk-cap look.
    held: Vec<CapId>,
    /// Wall seconds from `poll`, for the start-hint pulse.
    clock: f64,
}

impl State {
    fn init() -> State {
        // `--snap` renders headlessly; don't open devices for it.
        let headless = std::env::args().any(|a| a == "--snap");
        let (audio, status) = if headless {
            (None, "Snapshot preview".to_string())
        } else {
            match audio::start() {
                Ok(a) => (Some(a), "Sound on".to_string()),
                Err(e) => (None, format!("Audio unavailable: {e}")),
            }
        };
        let sr = audio.as_ref().map_or(48_000.0, |a| a.sample_rate);
        let mut s = State {
            ctl: Controller::new(sr),
            audio,
            status,
            midi: MidiMode::Off,
            own_sound: true,
            meter: 0.0,
            held: Vec::new(),
            clock: 0.0,
        };
        let glide = s.ctl.glide();
        s.send(vec![Command::Patch(s.ctl.patch), glide, Command::Reverb(s.ctl.reverb)]);
        s
    }

    fn now(&self) -> u64 {
        self.audio.as_ref().map_or(0, |a| a.now.load(Ordering::Relaxed))
    }

    fn send(&mut self, cmds: Vec<Command>) {
        if let Some(a) = self.audio.as_mut() {
            for c in cmds {
                let _ = a.commands.push(c);
            }
        }
    }

    fn press(&mut self, input: KeyInput) {
        let now = self.now();
        let cmds = self.ctl.key(input, now);
        self.send(cmds);
    }

    fn cycle_midi(&mut self) {
        self.midi = match self.midi {
            MidiMode::Off => MidiMode::Plain { channel: 0 },
            MidiMode::Plain { .. } => MidiMode::Mpe,
            MidiMode::Mpe => MidiMode::Off,
        };
        let m = self.midi;
        self.send(vec![Command::Midi(m)]);
    }
}

fn on_key(s: &mut State, e: &KeyEvent, tx: &mut Tx) -> bool {
    if e.mods.cmd || e.mods.ctrl {
        return false;
    }
    match &e.key {
        Key::Left if s.ctl.mode == Mode::Loop => s.ctl.move_cursor(-1),
        Key::Right if s.ctl.mode == Mode::Loop => s.ctl.move_cursor(1),
        Key::Tab if s.ctl.mode == Mode::Loop => s.ctl.toggle_half(),
        _ => {
            let input = key_input(e, tx);
            if s.ctl.keymap.resolve(input).is_none() { return false; }
            s.press(input);
            if let Some(id) = CapId::of(&input) {
                if !s.held.contains(&id) {
                    s.held.push(id);
                }
            }
        }
    }
    true
}

fn on_key_up(s: &mut State, e: &KeyEvent, tx: &mut Tx) -> bool {
    let input = key_input(e, tx);
    if let Some(id) = CapId::of(&input) {
        s.held.retain(|h| *h != id);
    }
    let cmds = s.ctl.key_up(input, s.now());
    s.send(cmds);
    false
}

fn poll(s: &mut State, secs: f64, _: &mut Tx) -> bool {
    let now = s.now();
    s.clock = secs;
    let cmds = s.ctl.poll(now);
    s.send(cmds);
    if let Some(a) = &s.audio {
        let peak = f32::from_bits(a.peak.swap(0, Ordering::Relaxed));
        s.meter = peak.max(s.meter * 0.88);
    }
    s.ctl.playing || s.audio.is_some()
}

fn setup(_: &mut State, waker: rune::Waker) {
    // Loop playback is scheduled from `poll`, which only runs on frames.
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(15));
        waker();
    });
}

// ---------------------------------------------------------------- shared controls

#[derive(Clone, Copy, PartialEq, Eq)]
enum PillKind {
    /// Navigation: the active segment is solid ink.
    Solid,
    /// An option that can be on or off; on is a raised cap with an ink edge.
    Toggle,
}

fn pill(label: impl Into<String>, style: TextStyle, kind: PillKind, active: bool) -> El {
    let (fill, border, color) = match (kind, active) {
        (PillKind::Solid, true) => (theme::ink(), theme::ink(), theme::cap()),
        (PillKind::Solid, false) => (clear_cap(), theme::cap_edge(), theme::ink_soft()),
        (PillKind::Toggle, true) => (theme::cap(), theme::ink(), theme::ink()),
        (PillKind::Toggle, false) => (clear_cap(), theme::cap_edge(), theme::ink_soft()),
    };
    let (px, py, radius) = match kind {
        PillKind::Solid => (10.0, 5.0, 14.0),
        PillKind::Toggle => (12.0, 5.0, 14.0),
    };
    el().px(px).py(py).radius(radius).fill(fill).border(1.0, border)
        .child(text(label).font(style).color(color))
}

fn toggle(label: &str, on: bool, press: impl Fn(&mut State) + 'static) -> El {
    pill(label, LABEL, PillKind::Toggle, on).on_press(press)
}

fn divider() -> El {
    el().size(1.0, 22.0).fill(c_alpha(theme::cap_edge(), 0.8))
}

// ---------------------------------------------------------------- header

fn mode_tabs(s: &State) -> El {
    let live = s.ctl.mode == Mode::Live;
    let seg = |label: &'static str, on: bool, mode: Mode| {
        pill(label, TAB, PillKind::Solid, on)
            .px(14.0).py(6.0).radius(9.0)
            .on_press(move |s: &mut State| { let cmds = s.ctl.set_mode(mode); s.send(cmds); })
    };
    row().radius(11.0).padding(3.0).gap(2.0).border(1.0, theme::cap_edge())
        .child(seg("Play live", live, Mode::Live))
        .child(seg("Loop", !live, Mode::Loop))
}

fn header(s: &State) -> El {
    let mut vibes_row = row().gap(6.0).items_center();
    for (i, v) in vibes::vibes().iter().enumerate() {
        let active = s.ctl.vibe == Some(i);
        let label = v.name;
        vibes_row = vibes_row.child(
            pill(label, VIBE, PillKind::Solid, active)
                .px(10.0).py(5.0)
                .on_press(move |s: &mut State| {
                    let cmds = s.ctl.apply_vibe_at(i);
                    s.send(cmds);
                }),
        );
    }
    row().w_full().gap(12.0).items_center()
        .child(text("keychords").font(TITLE).color(theme::ink()))
        .child(mode_tabs(s))
        .child(el().grow(1.0))
        .child(vibes_row)
}

// ---------------------------------------------------------------- readout

fn suggestions(s: &State) -> Vec<Suggestion> {
    s.ctl.current()
        .map(|(ch, v)| hints::suggest(&(*ch, v.clone()), s.ctl.leader.octave, s.ctl.leader.smooth, 3))
        .unwrap_or_default()
}

fn note_name(n: u8) -> String {
    format!("{}{}", chords::root_name(n % 12), n as i32 / 12 - 1)
}

fn chord_chip(sg: &Suggestion) -> El {
    let chord = sg.chord;
    let tint = theme::root_color(chord.root);
    el().px(9.0).py(4.0).radius(12.0)
        .fill(clear_cap()).border(1.0, c_alpha(tint, 0.55))
        .on_press(move |s: &mut State| {
            let cmds = s.ctl.play_chord(chord, s.now());
            s.send(cmds);
        })
        .child(row().gap(6.0).items_center()
            .child(el().size(8.0, 8.0).radius(4.0).fill(tint))
            .child(text(chord.name()).font(CHIP).color(theme::ink())))
}

fn readout(s: &State, sugg: &[Suggestion]) -> El {
    let mut col_ = col().w(300.0).gap(10.0);
    match s.ctl.current() {
        Some((ch, v)) => {
            let root = chords::root_name(ch.root);
            let suffix = ch.name().strip_prefix(root).unwrap_or("").to_string();
            let notes = controller::notes_of(v);
            let notes = notes.as_slice().iter().map(|&n| note_name(n)).collect::<Vec<_>>().join("  ");
            col_ = col_
                .child(row().gap(2.0)
                    .child(text(root).font(READOUT).color(theme::root_color(ch.root)))
                    .child(text(suffix).font(READOUT).color(theme::ink())))
                .child(text(notes).font(BODY_TEXT).color(theme::ink_soft()));
        }
        None => {
            col_ = col_
                .child(text("–").font(READOUT).color(theme::ink()))
                .child(text("No chord yet").font(BODY_TEXT).color(theme::ink_soft()));
        }
    }
    if !sugg.is_empty() {
        let mut chips = row().gap(8.0);
        for sg in sugg {
            chips = chips.child(chord_chip(sg));
        }
        col_ = col_
            .child(text("Try next").font(BODY_TEXT).color(theme::ink_soft()))
            .child(chips);
    }
    col_
}

// ---------------------------------------------------------------- screen

/// Seconds of history the live canvas shows, and its coordinate space.
const WINDOW_S: f64 = 8.0;
const VB_W: f32 = 1000.0;
const VB_H: f32 = 300.0;
const PITCH_LO: f32 = 36.0;
const PITCH_HI: f32 = 90.0;

fn pitch_y(p: f32) -> f32 {
    VB_H - (p.clamp(PITCH_LO, PITCH_HI) - PITCH_LO) / (PITCH_HI - PITCH_LO) * VB_H
}

/// The live voice lines: a scrolling timeline where each voice holds its note, then curves
/// into its next note over the glide time, ending at a "now" line with a dot per sounding voice.
fn voice_canvas(s: &State) -> El {
    let hist = &s.ctl.history;
    let at = &s.ctl.history_at;
    // Live: the engine clock. Headless or silent: just after the last chord.
    let now = match &s.audio {
        Some(a) => a.now.load(Ordering::Relaxed) as f64 / a.sample_rate,
        None => at.last().map_or(0.0, |&l| l + 1.5),
    };
    let left = now - WINDOW_S;
    let x = |time: f64| ((time - left) / WINDOW_S * VB_W as f64) as f32;
    let glide = s.ctl.glide_ms as f64 / 1000.0;

    // The current root's hue, stepping lighter for each voice above the bass.
    let root = s.ctl.root.unwrap_or(0);
    let voices = hist.last().map_or(7, |(_, v)| 1 + v.upper.len());
    let voice_col = |v: usize| theme::voice_color(root, v, voices);

    let mut shape = Shape::new();
    // Faint guide at every C from C2 to C6.
    for cp in (36..=84).step_by(12) {
        let y = pitch_y(cp as f32);
        shape = shape.line([0.0, y], [VB_W, y], 1.0, LineCap::Butt, c_alpha(theme::cap(), 0.12));
    }
    for i in 0..hist.len() {
        let here = controller::notes_of(&hist[i].1);
        let here = here.as_slice();
        let start = at[i];
        let next = hist.get(i + 1).map(|(_, v)| controller::notes_of(v));
        let next = next.as_ref().map(|n| n.as_slice());
        let end = at.get(i + 1).copied().unwrap_or(now);
        for (v, &p) in here.iter().enumerate() {
            let y0 = pitch_y(p as f32);
            let col = voice_col(v);
            // Hold from this chord's landing until the next chord starts.
            // A held line begins where the slide into it lands (the first chord has no slide).
            let held_from = if i == 0 { start } else { start + glide.max(0.02) };
            let mut path = Path::new().move_to([x(held_from), y0]).line_to([x(end), y0]);
            if let Some(nx) = next.as_ref().and_then(|n| n.get(v)) {
                // S-curve into the next note over the glide time.
                let (x0, x1, y1) = (x(end), x(end + glide.max(0.02)), pitch_y(*nx as f32));
                let mid = (x0 + x1) / 2.0;
                path = path.cubic_to([mid, y0], [mid, y1], [x1, y1]);
            }
            shape = shape.stroke_path(&path, if v == 0 { 3.0 } else { 2.5 }, LineCap::Round, col);
        }
    }
    // Now line, and a dot per sounding voice at its live pitch.
    shape = shape.line([VB_W - 1.0, 0.0], [VB_W - 1.0, VB_H], 1.5, LineCap::Butt, c_alpha(theme::cap(), 0.4));
    let live: Vec<Option<f32>> = match &s.audio {
        Some(a) => a.live.iter().map(|b| {
            let f = f32::from_bits(b.load(Ordering::Relaxed));
            (!f.is_nan()).then_some(f)
        }).collect(),
        None => hist.last()
            .map(|(_, v)| controller::notes_of(v).as_slice().iter().map(|&n| Some(n as f32)).collect())
            .unwrap_or_default(),
    };
    let voices = live.len().max(voices);
    for (v, p) in live.iter().enumerate() {
        if let Some(p) = p {
            let (cx, cy) = (VB_W - 1.0, pitch_y(*p));
            shape = shape.line([cx, cy], [cx, cy], 9.0, LineCap::Round, theme::voice_color(root, v, voices));
        }
    }
    el().grow(1.0).h(260.0).radius(14.0).fill(theme::screen()).clip_rounded(14.0).padding(10.0)
        .child(canvas(shape.view_box(VB_W, VB_H)).key("voices").w_full().h_full())
}

// ---------------------------------------------------------------- loop strip

fn loop_strip(s: &State) -> El {
    let head = s.ctl.playhead(s.now());
    let per_bar = s.ctl.lp.per_bar as usize;
    let bar = head.map(|i| i / per_bar + 1);
    let mut strip = row().w_full().gap(4.0);
    for i in 0..s.ctl.lp.slots.len() {
        let label = match s.ctl.lp.slots[i] {
            sequencer::Slot::Chord(ch) => ch.name(),
            sequencer::Slot::Silent => "silent".into(),
            sequencer::Slot::Empty => "–".into(),
        };
        let playing = head == Some(i);
        let selected = s.ctl.cursor == i;
        let mut cell = el().grow(1.0).h(42.0).radius(6.0).center()
            .fill(if playing { theme::ink() } else { clear_cap() })
            .on_press(move |s: &mut State| s.ctl.cursor = i)
            .child(text(label).font(SLOT)
                .color(if playing { theme::cap() } else { theme::ink() }));
        cell = cell.border(if selected { 2.0 } else { 1.0 },
            if selected { theme::ink() } else { theme::cap_edge() });
        strip = strip.child(cell);
    }

    let bars = s.ctl.lp.bars;
    let playing = s.ctl.playing;
    let mut bar_pills = row().gap(4.0);
    for b in [4u8, 8, 16, 32] {
        bar_pills = bar_pills.child(
            pill(format!("{b}"), META, PillKind::Toggle, bars == b)
                .on_press(move |s: &mut State| s.ctl.set_bars(b)),
        );
    }
    let meta = match bar {
        Some(bar) => format!("{:.0} BPM, bar {bar} of {bars}", s.ctl.clock.bpm),
        None => format!("{:.0} BPM, {bars} bars, ←/→ pick a slot", s.ctl.clock.bpm),
    };
    col().w_full().gap(10.0)
        .child(row().w_full().gap(12.0).items_center()
            .child(pill(if playing { "Stop loop" } else { "Play loop" }, LABEL, PillKind::Toggle, playing)
                .on_press(|s: &mut State| {
                    let now = s.now();
                    let cmds = s.ctl.key(KeyInput { character: Some(' '), phys: Phys::Space, shift: false }, now);
                    s.send(cmds);
                }))
            .child(bar_pills)
            .child(toggle("Half bars", per_bar == 2, |s: &mut State| s.ctl.toggle_half()))
            .child(el().grow(1.0))
            .child(text(meta).font(META).color(theme::ink_soft())))
        .child(strip)
}

// ---------------------------------------------------------------- keyboard

const ROWS: [&[(&str, Phys, Option<char>)]; 4] = [
    &[("`", Phys::Backquote, None), ("1", Phys::Digit1, None), ("2", Phys::Digit2, None), ("3", Phys::Digit3, None),
      ("4", Phys::Digit4, None), ("5", Phys::Digit5, None), ("6", Phys::Digit6, None), ("7", Phys::Digit7, None),
      ("8", Phys::Digit8, None), ("9", Phys::Digit9, None), ("0", Phys::Digit0, None), ("-", Phys::Minus, None),
      ("=", Phys::Equal, None)],
    &[("Q", Phys::Letter, Some('q')), ("W", Phys::Letter, Some('w')), ("E", Phys::Letter, Some('e')),
      ("R", Phys::Letter, Some('r')), ("T", Phys::Letter, Some('t')), ("Y", Phys::Letter, Some('y')),
      ("U", Phys::Letter, Some('u')), ("I", Phys::Letter, Some('i')), ("O", Phys::Letter, Some('o')),
      ("P", Phys::Letter, Some('p')), ("[", Phys::BracketLeft, None), ("]", Phys::BracketRight, None)],
    &[("A", Phys::Letter, Some('a')), ("S", Phys::Letter, Some('s')), ("D", Phys::Letter, Some('d')),
      ("F", Phys::Letter, Some('f')), ("G", Phys::Letter, Some('g')), ("H", Phys::Letter, Some('h')),
      ("J", Phys::Letter, Some('j')), ("K", Phys::Letter, Some('k')), ("L", Phys::Letter, Some('l')),
      (";", Phys::Semicolon, None), ("'", Phys::Quote, None)],
    &[("Z", Phys::Letter, Some('z')), ("X", Phys::Letter, Some('x')), ("C", Phys::Letter, Some('c')),
      ("V", Phys::Letter, Some('v')), ("B", Phys::Letter, Some('b')), ("N", Phys::Letter, Some('n')),
      ("M", Phys::Letter, Some('m')), (",", Phys::Comma, None), (".", Phys::Period, None), ("/", Phys::Slash, None)],
];

/// The tactile cap: a `theme::CAP` face with a `theme::CAP_EDGE` bottom edge (a slightly
/// taller rounded element behind the face). A held cap sinks two points into the edge.
fn cap(face: Color, held: bool, input: KeyInput, children: El) -> El {
    let sink = if held { CAP_SINK } else { 0.0 };
    el().size(CAP_W, CAP_H + CAP_DEPTH).radius(CAP_RADIUS).fill(theme::cap_edge())
        .child(el().size(CAP_W, CAP_H).radius(CAP_RADIUS).fill(face).mt(sink).center().child(children))
        .on_press(move |s: &mut State| s.press(input))
}

fn keymap_panel(s: &State, sugg: &[Suggestion]) -> El {
    let cur_root = s.ctl.root;
    let cur_quality = s.ctl.quality;
    let has_chord = !s.ctl.history.is_empty();
    let hint_pulse: f32 = if has_chord {
        1.0
    } else {
        0.5 + 0.25 * (1.0 + ((s.clock / 1.4).fract() * std::f64::consts::PI * 2.0).cos() as f32)
    };

    let mut rows = col().gap(CAP_GAP);
    for (r, keys) in ROWS.iter().enumerate() {
        let mut line = row().gap(CAP_GAP).ml(r as f32 * ROW_STAGGER);
        for &(cap_label, phys, ch) in keys.iter() {
            let input = KeyInput { character: ch, phys, shift: false };
            let action = s.ctl.keymap.resolve(input);
            let id = CapId::of(&input);
            let held = id.is_some_and(|i| s.held.contains(&i));

            // Legend, face tint and type for this cap.
            let (label, face, legend, font, is_root) = match action {
                Some(KeyAction::Root { letter, sharp }) => {
                    let pc = (letter.pitch_class() as i8 + sharp as i8).rem_euclid(12) as u8;
                    (String::new(), theme::cap(), theme::root_color(pc), ROOT_LETTER, true)
                }
                Some(KeyAction::Quality(q)) => {
                    (q.key_label().to_string(), theme::family_tint(theme::family(q)), theme::ink(), CAP_LEGEND, false)
                }
                Some(KeyAction::OctaveDown) => ("octave −".into(), theme::cap(), theme::ink_soft(), CAP_LEGEND, false),
                Some(KeyAction::OctaveUp) => ("octave +".into(), theme::cap(), theme::ink_soft(), CAP_LEGEND, false),
                Some(KeyAction::Stop) => ("stop".into(), theme::cap(), theme::ink_soft(), CAP_LEGEND, false),
                Some(KeyAction::Flat) => ("flat ♭".into(), theme::cap(), theme::ink_soft(), CAP_LEGEND, false),
                Some(KeyAction::Sharp) => ("sharp ♯".into(), theme::cap(), theme::ink_soft(), CAP_LEGEND, false),
                Some(KeyAction::Transport) | None => (String::new(), theme::cap(), theme::ink_soft(), CAP_LEGEND, false),
            };

            // Unused caps (backquote, R, T, V) dim to 40%.
            let unused = action.is_none();
            let face = if unused { c_alpha(face, 0.4) } else { face };

            // Face border: current root (its root colour), current quality (ink),
            // suggestion halo (the suggestion's root colour, softer than the current state).
            let mut border: Option<(f32, Color)> = None;
            match action {
                Some(KeyAction::Root { letter, sharp }) => {
                    let pc = (letter.pitch_class() as i8 + sharp as i8).rem_euclid(12) as u8;
                    if cur_root == Some(pc) {
                        border = Some((3.0, theme::root_color(pc)));
                    }
                }
                Some(KeyAction::Quality(q)) if cur_quality == Some(q) => {
                    border = Some((3.0, theme::ink()));
                }
                _ => {}
            }
            if border.is_none() {
                if let Some(sg) = sugg.iter().find(|sg| match action {
                    Some(KeyAction::Root { letter, sharp }) => {
                        (letter.pitch_class() as i8 + sharp as i8).rem_euclid(12) as u8 == sg.chord.root
                    }
                    Some(KeyAction::Quality(q)) => sg.chord.quality == q,
                    _ => false,
                }) {
                    border = Some((2.5, c_alpha(theme::root_color(sg.chord.root), 0.55)));
                }
            }

            // Held caps brighten; root caps take a wash of their root colour.
            let face = if held {
                match action {
                    Some(KeyAction::Root { .. }) => mix_white(face, 0.75),
                    _ => mix_white(face, 0.45),
                }
            } else {
                face
            };

            // Face content: the physical key (small, top-left) plus the function legend.
            // Root caps show only their big letter.
            let key_label = match action {
                Some(KeyAction::Transport) => "space".into(),
                _ => ch.map(|x| x.to_ascii_uppercase().to_string())
                    .unwrap_or_else(|| cap_label.chars().next().map(|c| c.to_string()).unwrap_or_default()),
            };
            let mut content = el().size(CAP_W, CAP_H).center();
            if !is_root && !key_label.is_empty() {
                let key_col = if unused { c_alpha(theme::ink_soft(), 0.4) } else { theme::ink_soft() };
                content = content.child(text(key_label).font(CAP_KEY).color(key_col).absolute(6.0, 5.0));
            }
            if is_root {
                content = content.child(text(cap_label)
                    .font(font).color(if unused { c_alpha(legend, 0.4) } else { legend }));
            } else if !label.is_empty() {
                let legend = if unused { c_alpha(legend, 0.4) } else { legend };
                content = content.child(text(label).font(font).color(legend));
            }

            let mut key = cap(face, held, input, content);
            if let Some((width, color)) = border {
                key = key.border(width, color);
            }
            if !has_chord
                && (id == Some(CapId::Letter('d')) || id == Some(CapId::Phys(Phys::Comma)))
            {
                key = key.opacity(hint_pulse);
            }
            line = line.child(key);
        }
        rows = rows.child(line);
    }
    rows
}

// ---------------------------------------------------------------- footer

/// A horizontal drag slider: a 140 pt track with a filled part and a 14 pt thumb.
fn slider(label: &str, value: f32, min: f32, max: f32, format: String,
          apply: impl Fn(&mut State, f32) + 'static) -> El {
    let frac = ((value - min) / (max - min)).clamp(0.0, 1.0);
    let fill_w = frac * (TRACK_W - THUMB);
    let thumb_x = (TRACK_W - THUMB) * frac;
    let track = el().size(TRACK_W, THUMB).radius(7.0).fill(theme::cap_edge())
        .child(el().size(fill_w, THUMB).radius(7.0).fill(theme::ink()).absolute(0.0, 0.0))
        .child(el().size(THUMB, THUMB).radius(7.0).fill(theme::ink()).absolute(thumb_x, 0.0));
    let hit = el().size(TRACK_W, 24.0).center().child(track)
        .on_drag(move |s: &mut State, d: &mut rune::Drag, _| match d.phase {
            Phase::Down | Phase::Move => {
                let v = (d.x / d.w).clamp(0.0, 1.0) * (max - min) + min;
                apply(s, v);
            }
            Phase::Up => {}
        });
    row().gap(8.0).items_center()
        .child(el().w(52.0).child(text(label).font(LABEL).color(theme::ink())))
        .child(hit)
        .child(el().w(56.0).child(text(format).font(META).color(theme::ink_soft())))
}

fn step_button(glyph: &str, key: char) -> El {
    el().size(22.0, 22.0).radius(11.0).center().fill(clear_cap()).border(1.0, theme::cap_edge())
        .on_press(move |s: &mut State| s.press(KeyInput { character: Some(key), phys: Phys::Letter, shift: false }))
        .child(text(glyph).font(STEP).color(theme::ink()))
}

fn octave_stepper(octave: i8) -> El {
    row().items_center().gap(4.0).radius(14.0).px(4.0).py(3.0).border(1.0, theme::cap_edge())
        .child(step_button("−", 'q'))
        .child(el().w(34.0).center().child(text(format!("{octave:+}")).font(LABEL).color(theme::ink())))
        .child(step_button("+", 'w'))
}

/// A horizontal output-level bar: green, then amber near full scale.
fn meter(level: f32) -> El {
    let fill = level.clamp(0.0, 1.0) * METER_W;
    let color = if level > 0.85 { theme::meter_hot() } else { theme::meter_low() };
    row().gap(8.0).items_center()
        .child(text("Level").font(META).color(theme::ink_soft()))
        .child(el().size(METER_W, 8.0).radius(4.0).fill(theme::cap_edge()).clip_rounded(4.0)
            .child(el().size(fill, 8.0).radius(4.0).fill(color)))
}

fn controls(s: &State) -> El {
    let midi = match s.midi {
        MidiMode::Off => "MIDI off".to_string(),
        MidiMode::Plain { channel } => format!("MIDI notes ch {}", channel + 1),
        MidiMode::Mpe => "MIDI MPE".to_string(),
    };
    let line1 = row().w_full().gap(16.0).items_center()
        .child(slider("Glide", s.ctl.glide_ms, 0.0, 800.0, format!("{} ms", s.ctl.glide_ms as i32),
            |s, v| { s.ctl.glide_ms = v; let g = s.ctl.glide(); s.send(vec![g]); }))
        .child(slider("Strum", s.ctl.strum_ms, 0.0, 160.0, format!("{} ms", s.ctl.strum_ms as i32),
            |s, v| { s.ctl.strum_ms = v; let g = s.ctl.glide(); s.send(vec![g]); }))
        .child(slider("Reverb", s.ctl.reverb, 0.0, 1.0, format!("{}%", (s.ctl.reverb * 100.0) as i32),
            |s, v| { s.ctl.reverb = v; s.send(vec![Command::Reverb(v)]); }))
        .child(divider())
        .child(row().gap(8.0).items_center()
            .child(text("Octave").font(LABEL).color(theme::ink()))
            .child(octave_stepper(s.ctl.leader.octave)))
        .child(el().grow(1.0));
    let line2 = row().w_full().gap(8.0).items_center()
        .child(toggle("Smooth voice leading", s.ctl.leader.smooth, |s: &mut State| {
            s.ctl.leader.smooth = !s.ctl.leader.smooth;
        }))
        .child(toggle("Letter replays chord", s.ctl.replay_quality, |s: &mut State| {
            s.ctl.replay_quality = !s.ctl.replay_quality;
        }))
        .child(toggle("Ring", s.ctl.ring, |s: &mut State| {
            s.ctl.ring = !s.ctl.ring;
        }))
        .child(toggle("Own sound", s.own_sound, |s: &mut State| {
            s.own_sound = !s.own_sound;
            let on = s.own_sound;
            s.send(vec![Command::OwnSound(on)]);
        }))
        .child(toggle(&midi, s.midi != MidiMode::Off, |s: &mut State| s.cycle_midi()))
        .child(el().grow(1.0))
        .child(meter(s.meter))
        .child(text(s.status.clone()).font(META).color(theme::ink_soft()));
    col().w_full().gap(12.0)
        .child(el().w_full().h(1.0).fill(c_alpha(theme::cap_edge(), 0.8)))
        .child(line1)
        .child(line2)
}

// ---------------------------------------------------------------- view

fn view(s: &State, _: &mut Cx<State>) -> El {
    let sugg = suggestions(s);
    let mut body = col().w_full().h_full().padding(24.0).gap(18.0).fill(theme::body())
        .child(header(s))
        .child(row().w_full().gap(18.0).items_center()
            .child(readout(s, &sugg))
            .child(voice_canvas(s)));
    if s.ctl.mode == Mode::Loop {
        body = body.child(loop_strip(s));
    }
    if s.ctl.history.is_empty() && s.ctl.mode == Mode::Live {
        body = body.child(row().w_full().justify_center()
            .child(text("Press D, then , to play your first chord")
                .font(BODY_TEXT).color(theme::ink_soft())));
    }
    body = body
        .child(row().w_full().justify_center().child(keymap_panel(s, &sugg)))
        .child(el().grow(1.0))
        .child(controls(s));
    body.focusable(true).autofocus().on_key_up(on_key_up)
}

fn empty_demo(_: &mut State) {}

fn loop_demo(s: &mut State) {
    demo(s);
    s.ctl.mode = Mode::Loop;
    let chords: Vec<_> = s.ctl.history.iter().take(3).map(|(ch, _)| *ch).collect();
    for (i, ch) in chords.iter().enumerate() {
        s.ctl.lp.slots[i] = sequencer::Slot::Chord(*ch);
    }
    s.ctl.lp.slots[2] = sequencer::Slot::Silent;
    s.ctl.cursor = 1;
}

fn demo(s: &mut State) {
    for (l, k) in [('d', ','), ('g', '/'), ('c', 'm'), ('a', 'h')] {
        let phys = |ch: char| match ch {
            ',' => Phys::Comma,
            '/' => Phys::Slash,
            _ => Phys::Letter,
        };
        s.press(KeyInput { character: Some(l), phys: Phys::Letter, shift: false });
        s.press(KeyInput { character: Some(k), phys: phys(k), shift: false });
    }
    let n = s.ctl.history_at.len();
    for (i, t) in s.ctl.history_at.iter_mut().enumerate() {
        *t = (i as f64 - n as f64) * 1.6 + 4.0;
    }
}

/// `--audio-test`: open the device exactly as the app does, play a chord, report what reached it.
fn audio_test() {
    match audio::start() {
        Err(e) => println!("audio::start failed: {e}"),
        Ok(mut a) => {
            println!("device: {} | {} Hz | {} ch | midi: {:?}", a.device_name, a.sample_rate, a.channels, a.midi_port);
            let _ = a.commands.push(Command::Play(engine::Notes::from_slice(&[50, 57, 65, 72, 76])));
            for i in 0..8 {
                std::thread::sleep(Duration::from_millis(250));
                let peak = f32::from_bits(a.peak.swap(0, Ordering::Relaxed));
                println!("t={:.2}s clock={} peak={peak:.4}", (i + 1) as f32 * 0.25, a.now.load(Ordering::Relaxed));
            }
        }
    }
}

fn main() {
    if std::env::args().any(|a| a == "--audio-test") {
        return audio_test();
    }
    rune::run(AppSpec::<State> {
        scenes: &[("demo", demo), ("empty", empty_demo), ("loop", loop_demo)],
        poll: Some(poll),
        on_key: Some(on_key),
        setup: Some(setup),
        ..AppSpec::new("keychords", (1180.0, 860.0), State::init, view)
    });
}

#[cfg(test)]
mod input_tests {
    use super::*;

    fn input(character: Option<char>, phys: Phys, shift: bool) -> KeyInput {
        KeyInput { character, phys, shift }
    }

    #[test]
    fn letters_are_keyed_by_lowercased_character() {
        assert_eq!(CapId::of(&input(Some('D'), Phys::Letter, true)), Some(CapId::Letter('d')));
    }

    #[test]
    fn punctuation_is_keyed_by_physical_position() {
        assert_eq!(CapId::of(&input(None, Phys::Comma, false)), Some(CapId::Phys(Phys::Comma)));
        assert_eq!(CapId::of(&input(None, Phys::Slash, false)), Some(CapId::Phys(Phys::Slash)));
    }

    #[test]
    fn positions_without_caps_have_no_id() {
        assert_eq!(CapId::of(&input(None, Phys::Letter, false)), None);
        assert_eq!(CapId::of(&input(None, Phys::Other, false)), None);
        assert_eq!(CapId::of(&input(None, Phys::Space, false)), None);
    }
}
