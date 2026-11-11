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
use rune::color::hex;
use rune::{canvas, col, el, row, text, AppSpec, Color, Cx, El, Key, KeyEvent, LineCap, Path, PhysicalKeyCode, Phase, Shape, TextStyle, Tx};

use controller::{Controller, Mode};
use engine::{Command, MidiMode};

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
            Some(_) => None,
            None => Some(Self::Phys(input.phys)),
        }
    }

    /// The cap the hint points at: the letter D, or the comma.
    fn start_hint(d: bool) -> Self {
        if d { Self::Letter('d') } else { Self::Phys(Phys::Comma) }
    }
}

struct State {
    ctl: Controller,
    audio: Option<audio::Audio>,
    status: String,
    midi: MidiMode,
    own_sound: bool,
    volume_db: f32,
    reverb: f32,
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
            (None, "Sound on, MIDI port keychords".to_string())
        } else {
            match audio::start() {
                Ok(a) => {
                    let port = a.midi_port.clone().unwrap_or_else(|| "no MIDI port".into());
                    let s = if a.midi_port.is_some() {
                        format!("Sound on, MIDI port {port}")
                    } else {
                        "Sound on, MIDI off".to_string()
                    };
                    (Some(a), s)
                }
                Err(e) => (None, format!("audio unavailable: {e}")),
            }
        };
        let sr = audio.as_ref().map_or(48_000.0, |a| a.sample_rate);
        let mut s = State {
            ctl: Controller::new(sr),
            audio,
            status,
            midi: MidiMode::Off,
            own_sound: true,
            volume_db: 0.0,
            reverb: 0.3,
            meter: 0.0,
            held: Vec::new(),
            clock: 0.0,
        };
        let glide = s.ctl.glide();
        s.send(vec![glide]);
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

// ---------------------------------------------------------------- keys

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

fn on_key(s: &mut State, e: &KeyEvent, tx: &mut Tx) -> bool {
    if e.mods.cmd || e.mods.ctrl {
        return false;
    }
    match &e.key {
        Key::Left if s.ctl.mode == Mode::Loop => s.ctl.move_cursor(-1),
        Key::Right if s.ctl.mode == Mode::Loop => s.ctl.move_cursor(1),
        Key::Tab if s.ctl.mode == Mode::Loop => s.ctl.toggle_half(),
        Key::Text(t) => {
            let input = KeyInput {
                character: t.chars().next(),
                phys: phys_of(tx.physical_key()),
                shift: e.mods.shift,
            };
            if s.ctl.keymap.resolve(input).is_none() {
                return false;
            }
            // Keys 1–6 are vibes now.
            if let Some(KeyAction::Sound(i)) = s.ctl.keymap.resolve(input) {
                let v = vibes::vibes()[i as usize];
                s.reverb = v.reverb;
                let cmds = s.ctl.apply_vibe(&v);
                s.send(cmds);
                return true;
            }
            s.press(input);
            if let Some(id) = CapId::of(&input) {
                if !s.held.contains(&id) {
                    s.held.push(id);
                }
            }
        }
        _ => return false,
    }
    true
}

fn on_key_up(s: &mut State, e: &KeyEvent, _: &mut Tx) -> bool {
    if let Key::Text(t) = &e.key {
        let input = KeyInput { character: t.chars().next(), phys: Phys::Other, shift: e.mods.shift };
        if let Some(id) = CapId::of(&input) {
            s.held.retain(|h| *h != id);
        }
        let cmds = s.ctl.key_up(input, s.now());
        s.send(cmds);
    }
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

// ---------------------------------------------------------------- view

fn c(h: &str) -> Color {
    hex(h).expect("valid colour")
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

fn mode_tabs(s: &State) -> El {
    let live = s.ctl.mode == Mode::Live;
    let seg = |label: &'static str, on: bool, mode: Mode| {
        el().px(14.0).py(6.0).radius(8.0)
            .fill(if on { c(theme::INK) } else { Color { a: 0.0, ..c(theme::CAP) } })
            .on_press(move |s: &mut State| { let cmds = s.ctl.set_mode(mode); s.send(cmds); })
            .child(text(label).font(TextStyle::new(14.0, 600.0))
                .color(if on { c(theme::CAP) } else { c(theme::INK_SOFT) }))
    };
    row().radius(10.0).border(1.0, c(theme::CAP_EDGE))
        .child(seg("Play live", live, Mode::Live))
        .child(seg("Loop", !live, Mode::Loop))
}

fn header(s: &State) -> El {
    let vibes = vibes::vibes();
    let mut row_ = row().w_full().items_center().gap(12.0)
        .child(text("keychords").font(TextStyle::new(22.0, 700.0)).color(c(theme::INK)))
        .child(mode_tabs(s));
    for v in vibes {
        let active = s.ctl.patch == v.patch && s.ctl.glide_ms == v.glide_ms && s.ctl.strum_ms == v.strum_ms;
        row_ = row_.child(vibe_pill(v, active));
    }
    row_.child(el().grow(1.0))
}

fn vibe_pill(v: vibes::Vibe, active: bool) -> El {
    el().px(6.0).py(6.0).radius(14.0)
        .fill(if active { c(theme::INK) } else { Color { a: 0.0, ..c(theme::CAP) } })
        .border(1.0, if active { c(theme::INK) } else { c(theme::CAP_EDGE) })
        .on_press(move |s: &mut State| {
            s.reverb = v.reverb;
            let cmds = s.ctl.apply_vibe(&v);
            s.send(cmds);
        })
        .child(text(v.name).font(TextStyle::new(13.0, 500.0))
            .color(if active { c(theme::CAP) } else { c(theme::INK_SOFT) }))
}

/// A horizontal output-level bar: green, then amber near full scale.
fn meter(level: f32) -> El {
    let w = 120.0;
    let fill = (level.clamp(0.0, 1.0) * w).max(2.0);
    let col = if level > 0.85 { c("#F6A04D") } else { c("#5FD38D") };
    el().size(w, 8.0).radius(4.0).fill(c(theme::CAP_EDGE)).clip_rounded(4.0)
        .child(el().size(fill, 8.0).radius(4.0).fill(col))
}

/// The chord readout, on the body: the name huge (root letter in its hue), the voice
/// notes, and up to three "Try next" chips.
fn readout(s: &State) -> El {
    let cur = s.ctl.current();
    let sugg = cur
        .map(|(ch, v)| hints::suggest(&(*ch, v.clone()), s.ctl.leader.octave, s.ctl.leader.smooth, 3))
        .unwrap_or_default();
    let mut col_ = col().w(300.0).gap(10.0);
    match cur {
        Some((ch, v)) => {
            let root_col = theme::root_color(ch.root);
            let root_letter = chords::root_name(ch.root).chars().next().unwrap_or('C');
            let rest = ch.name().trim_start_matches(root_letter).to_string();
            let mut ns = vec![v.bass];
            ns.extend(&v.upper);
            col_ = col_.child(row().gap(2.0)
                .child(text(root_letter.to_string()).font(TextStyle::new(64.0, 800.0)).color(root_col))
                .child(text(rest).font(TextStyle::new(64.0, 800.0)).color(c(theme::INK))));
            col_ = col_.child(text(ns.iter().map(|n| note_name(*n)).collect::<Vec<_>>().join("  "))
                .font(TextStyle::new(15.0, 400.0)).color(c(theme::INK_SOFT)));
        }
        None => {
            col_ = col_.child(text("–").font(TextStyle::new(64.0, 800.0)).color(c(theme::INK)));
            col_ = col_.child(text("Pick a root with a letter key, then press a chord key.")
                .font(TextStyle::new(15.0, 400.0)).color(c(theme::INK_SOFT)));
        }
    }
    if !sugg.is_empty() {
        let mut chips = row().gap(8.0).mt(4.0)
            .child(text("Try next").font(TextStyle::new(15.0, 500.0)).color(c(theme::INK_SOFT)));
        for sg in sugg {
            let chord = sg.chord;
            let label = chord.name();
            chips = chips.child(el().px(10.0).py(4.0).radius(12.0)
                .fill(Color { a: 0.0, ..c(theme::CAP) }).border(1.0, c(theme::CAP_EDGE))
                .on_press(move |s: &mut State| {
                    let cmds = s.ctl.play_chord(chord, s.now());
                    s.send(cmds);
                })
                .child(text(label).font(TextStyle::new(13.0, 500.0)).color(c(theme::INK))));
        }
        col_ = col_.child(chips);
    }
    col_
}

fn note_name(n: u8) -> String {
    format!("{}{}", chords::root_name(n % 12), n as i32 / 12 - 1)
}

/// Seconds of history the live canvas shows, and its coordinate space.
const WINDOW_S: f64 = 8.0;
const VB_W: f32 = 1000.0;
const VB_H: f32 = 300.0;
const PITCH_LO: f32 = 36.0;
const PITCH_HI: f32 = 90.0;

fn pitch_y(p: f32) -> f32 {
    VB_H - (p.clamp(PITCH_LO, PITCH_HI) - PITCH_LO) / (PITCH_HI - PITCH_LO) * VB_H
}

fn notes_of(v: &voicing::Voicing) -> Vec<u8> {
    let mut ns = vec![v.bass];
    ns.extend(&v.upper);
    ns
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

    // The current root's hue: lightest for the top voice, most saturated for the bass.
    let root = s.ctl.root.unwrap_or(0);
    let voice_col = |v: usize| mix_white(theme::root_color(root), 0.04 * v as f32);

    let mut shape = Shape::new();
    // Faint guide at every C from C2 to C6.
    for cp in (36..=84).step_by(12) {
        let y = pitch_y(cp as f32);
        shape = shape.line([0.0, y], [VB_W, y], 1.0, LineCap::Butt, c_alpha(c(theme::CAP), 0.10));
    }
    for i in 0..hist.len() {
        let here = notes_of(&hist[i].1);
        let start = at[i];
        let next = hist.get(i + 1).map(|(_, v)| notes_of(v));
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
    shape = shape.line([VB_W - 1.0, 0.0], [VB_W - 1.0, VB_H], 1.5, LineCap::Butt, c_alpha(c(theme::CAP), 0.4));
    let live: Vec<Option<f32>> = match &s.audio {
        Some(a) => a.live.iter().map(|b| {
            let f = f32::from_bits(b.load(Ordering::Relaxed));
            (!f.is_nan()).then_some(f)
        }).collect(),
        None => hist.last().map(|(_, v)| notes_of(v).iter().map(|&n| Some(n as f32)).collect()).unwrap_or_default(),
    };
    for (v, p) in live.iter().enumerate() {
        if let Some(p) = p {
            let (cx, cy) = (VB_W - 1.0, pitch_y(*p));
            shape = shape.line([cx, cy], [cx, cy], 9.0, LineCap::Round, voice_col(v));
        }
    }
    el().grow(1.0).h(260.0).radius(14.0).fill(c(theme::SCREEN)).clip_rounded(14.0).padding(10.0)
        .child(canvas(shape.view_box(VB_W, VB_H)).key("voices").w_full().h_full())
}

fn c_alpha(col: Color, a: f32) -> Color {
    Color { a: col.a * a, ..col }
}

fn loop_strip(s: &State) -> El {
    let head = s.ctl.playhead(s.now());
    let mut strip = row().w_full().gap(6.0);
    for i in 0..s.ctl.lp.slots.len() {
        let label = match s.ctl.lp.slots[i] {
            sequencer::Slot::Chord(ch) => ch.name(),
            sequencer::Slot::Silent => "stop".into(),
            sequencer::Slot::Empty => "·".into(),
        };
        let mut cell = el().grow(1.0).h(44.0).radius(6.0).center()
            .fill(if head == Some(i) { c(theme::INK) } else { c(theme::CAP) })
            .on_press(move |s: &mut State| s.ctl.cursor = i)
            .child(text(label).font(TextStyle::new(14.0, 500.0))
                .color(if head == Some(i) { c(theme::CAP) } else { c(theme::INK) }));
        if s.ctl.cursor == i {
            cell = cell.border(2.0, c(theme::INK));
        }
        strip = strip.child(cell);
    }
    let bars = s.ctl.lp.bars;
    let playing = s.ctl.playing;
    col().w_full().gap(8.0)
        .child(row().gap(8.0).items_center()
            .child(el().px(12.0).py(6.0).radius(14.0)
                .fill(Color { a: 0.0, ..c(theme::CAP) }).border(1.0, c(theme::CAP_EDGE))
                .on_press(|s: &mut State| {
                    let now = s.now();
                    let cmds = s.ctl.key(KeyInput { character: Some(' '), phys: Phys::Space, shift: false }, now);
                    s.send(cmds);
                })
                .child(text(if playing { "Stop loop" } else { "Play loop" })
                    .font(TextStyle::new(14.0, 500.0)).color(c(theme::INK))))
            .child(el().px(12.0).py(6.0).radius(14.0)
                .fill(Color { a: 0.0, ..c(theme::CAP) }).border(1.0, c(theme::CAP_EDGE))
                .on_press(move |s: &mut State| {
                    let next = match s.ctl.lp.bars { 4 => 8, 8 => 16, 16 => 32, _ => 4 };
                    s.ctl.set_bars(next);
                })
                .child(text(format!("{bars} bars")).font(TextStyle::new(14.0, 500.0)).color(c(theme::INK))))
            .child(el().px(12.0).py(6.0).radius(14.0)
                .fill(Color { a: 0.0, ..c(theme::CAP) }).border(1.0, c(theme::CAP_EDGE))
                .on_press(|s: &mut State| s.ctl.toggle_half())
                .child(text("½ bar").font(TextStyle::new(14.0, 500.0)).color(c(theme::INK))))
            .child(text(format!("{:.0} BPM, ←/→ pick a slot, Tab half bars, Space play", s.ctl.clock.bpm))
                .font(TextStyle::new(14.0, 400.0)).color(c(theme::INK_SOFT))))
        .child(strip)
}

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
/// taller rounded element offset down behind the face), which sinks when held.
fn cap(face: Color, edge: Color, held: bool, input: KeyInput, children: El) -> El {
    let sink = held as u8 as f32 * 2.0;
    let edge_h = if held { 1.0 } else { 3.0 };
    let r = 10.0;
    el().size(68.0, 58.0 + edge_h).radius(r).fill(edge)
        .child(el().size(68.0, 58.0).radius(r).fill(face).mt(sink).center().child(children))
        .on_press(move |s: &mut State| s.press(input))
}

fn keymap_panel(s: &State) -> El {
    let cur = s.ctl.current();
    let cur_root = s.ctl.root;
    let cur_quality = s.ctl.quality;
    let has_chord = !s.ctl.history.is_empty();
    // "Try next": the caps of up to three smooth next chords get a soft halo.
    let sugg = cur
        .map(|(ch, v)| hints::suggest(&(*ch, v.clone()), s.ctl.leader.octave, s.ctl.leader.smooth, 3))
        .unwrap_or_default();
    let hint_pulse: f32 = if has_chord {
        1.0
    } else {
        0.5 + 0.25 * (1.0 + ((s.clock / 1.4).fract() * std::f64::consts::PI * 2.0).cos() as f32)
    };
    let vibes = vibes::vibes();

    let mut rows = col().gap(6.0);
    for (r, keys) in ROWS.iter().enumerate() {
        let mut line = row().gap(6.0).ml(r as f32 * 22.0);
        for &(cap_label, phys, ch) in keys.iter() {
            let input = KeyInput { character: ch, phys, shift: false };
            let action = s.ctl.keymap.resolve(input);
            let id = CapId::of(&input);
            let held = id.is_some_and(|i| s.held.contains(&i));

            // Legend and face colour for this cap.
            let (label, face, legend, font, is_root) = match action {
                Some(KeyAction::Root { letter, sharp }) => {
                    let pc = (letter.pitch_class() as i8 + sharp as i8).rem_euclid(12) as u8;
                    let col = theme::root_color(pc);
                    (String::new(), c(theme::CAP), col, TextStyle::new(20.0, 700.0), true)
                }
                Some(KeyAction::Quality(q)) => {
                    let tint = theme::family_tint(theme::family(q));
                    (q.key_label().to_string(), tint, c(theme::INK), TextStyle::new(13.0, 500.0), false)
                }
                Some(KeyAction::Sound(i)) => {
                    (short_vibe_name(&vibes[i as usize]).to_string(), c(theme::CAP), c(theme::INK_SOFT), TextStyle::new(11.0, 500.0), false)
                }
                Some(KeyAction::OctaveDown) => ("octave −".into(), c(theme::CAP), c(theme::INK_SOFT), TextStyle::new(13.0, 500.0), false),
                Some(KeyAction::OctaveUp) => ("octave +".into(), c(theme::CAP), c(theme::INK_SOFT), TextStyle::new(13.0, 500.0), false),
                Some(KeyAction::Stop) => ("stop".into(), c(theme::CAP), c(theme::INK_SOFT), TextStyle::new(13.0, 500.0), false),
                Some(KeyAction::Flat) => ("flat ♭".into(), c(theme::CAP), c(theme::INK_SOFT), TextStyle::new(13.0, 500.0), false),
                Some(KeyAction::Sharp) => ("sharp ♯".into(), c(theme::CAP), c(theme::INK_SOFT), TextStyle::new(13.0, 500.0), false),
                Some(KeyAction::Transport) | None => (String::new(), c(theme::CAP), c(theme::INK_SOFT), TextStyle::new(13.0, 500.0), false),
            };

            // Unused caps (backquote, R, T, V) dim to 40%.
            let unused = action.is_none()
                || matches!(ch, Some('r') | Some('t') | Some('v'));
            let face = if unused { c_alpha(face, 0.4) } else { face };

            // Face border: current root (its root colour), current quality (ink),
            // suggestion halo (the suggestion's root colour at 70% alpha).
            let mut border: Option<(f32, Color)> = None;
            match action {
                Some(KeyAction::Root { letter, sharp }) => {
                    let pc = (letter.pitch_class() as i8 + sharp as i8).rem_euclid(12) as u8;
                    if cur_root == Some(pc) {
                        border = Some((3.0, theme::root_color(pc)));
                    }
                }
                Some(KeyAction::Quality(q)) if cur_quality == Some(q) => {
                    border = Some((3.0, c(theme::INK)));
                }
                _ => {}
            }
            if border.is_none() {
                if let Some(sug) = sugg.iter().find(|sg| match action {
                    Some(KeyAction::Root { letter, sharp }) => {
                        (letter.pitch_class() as i8 + sharp as i8).rem_euclid(12) as u8 == sg.chord.root
                    }
                    Some(KeyAction::Quality(q)) => sg.chord.quality == q,
                    _ => false,
                }) {
                    border = Some((3.0, c_alpha(theme::root_color(sug.chord.root), 0.7)));
                }
            }

            // Sunk cap: a 25% wash of the root colour over the cap face.
            let face = if held && is_root {
                if let Some(KeyAction::Root { letter, sharp }) = action {
                    let pc = (letter.pitch_class() as i8 + sharp as i8).rem_euclid(12) as u8;
                    mix_white(theme::root_color(pc), 0.75)
                } else {
                    face
                }
            } else {
                face
            };

            // Face content: the key (small, top-left) plus the legend centred.
            // Root caps show only their big letter.
            let key_label: String = match action {
                Some(KeyAction::Transport) => "space".into(),
                _ => ch.map(|x| x.to_string()).unwrap_or_else(|| cap_label.to_string().chars().next().unwrap_or('?').to_string()),
            };
            let mut content = el().size(68.0, 58.0).center();
            if !is_root && !key_label.is_empty() {
                let kc = if unused { c_alpha(c(theme::INK_SOFT), 0.4) } else { c(theme::INK_SOFT) };
                content = content.child(text(key_label)
                    .font(TextStyle::new(11.0, 400.0)).color(kc)
                    .absolute(6.0, 4.0));
            }
            if is_root {
                content = content.child(text(cap_label.to_string())
                    .font(font).color(if unused { c_alpha(legend, 0.4) } else { legend }));
            } else if !label.is_empty() {
                let lc = if unused { c_alpha(legend, 0.4) } else { legend };
                content = content.child(text(label).font(font).color(lc));
            }

            let mut key = cap(face, c(theme::CAP_EDGE), held, input, content);
            if let Some((bw, bc)) = border {
                key = key.border(bw, bc);
            }
            if !has_chord
                && (id == Some(CapId::start_hint(true)) || id == Some(CapId::start_hint(false)))
            {
                key = key.opacity(hint_pulse);
            }
            line = line.child(key);
        }
        rows = rows.child(line);
    }
    rows
}



/// The short name for a vibe on its cap: the first word, with "Glass bells" and
/// "String swell" renamed so they fit the 68 pt cap.
fn short_vibe_name(v: &vibes::Vibe) -> &'static str {
    match v.name {
        "Glass bells" => "Bells",
        "String swell" => "Strings",
        _ => v.name.split(' ').next().unwrap_or(v.name),
    }
}

/// A horizontal drag slider: 140 pt track, filled part, 14 pt round thumb.
fn slider(label: &str, value: f32, min: f32, max: f32, format: String,
          apply: impl Fn(&mut State, f32) + 'static) -> El {
    let frac = ((value - min) / (max - min)).clamp(0.0, 1.0);
    let track_w = 140.0;
    let fill_w = frac * (track_w - 14.0);
    let thumb_x = (track_w - 14.0) * frac;
    let track = el().size(track_w, 14.0).radius(7.0).fill(c(theme::CAP_EDGE))
        .child(el().size(fill_w, 14.0).radius(7.0).fill(c(theme::INK)).absolute(0.0, 0.0))
        .child(el().size(14.0, 14.0).radius(7.0).fill(c(theme::INK)).absolute(thumb_x, 0.0))
        .on_drag(move |s: &mut State, d: &mut rune::Drag, _| match d.phase {
            Phase::Down | Phase::Move => {
                let v = (d.x / d.w).clamp(0.0, 1.0) * (max - min) + min;
                apply(s, v);
            }
            Phase::Up => {}
        });
    row().gap(8.0).items_center()
        .child(text(label).font(TextStyle::new(14.0, 500.0)).color(c(theme::INK)))
        .child(track)
        .child(text(format).font(TextStyle::new(14.0, 400.0)).color(c(theme::INK_SOFT)))
}

fn toggle_pill(label: &str, on: bool, f: impl Fn(&mut State) + 'static) -> El {
    el().px(12.0).py(5.0).radius(14.0)
        .fill(if on { c_alpha(c(theme::INK), 0.08) } else { Color { a: 0.0, ..c(theme::CAP) } })
        .border(1.0, c(theme::CAP_EDGE))
        .on_press(f)
        .child(text(label).font(TextStyle::new(14.0, 500.0))
            .color(if on { c(theme::INK) } else { c(theme::INK_SOFT) }))
}

fn controls(s: &State) -> El {
    let midi = match s.midi {
        MidiMode::Off => "MIDI out off".to_string(),
        MidiMode::Plain { channel } => format!("MIDI out notes ch {}", channel + 1),
        MidiMode::Mpe => "MIDI out MPE".to_string(),
    };
    let octave = s.ctl.leader.octave;
    let line1 = row().w_full().gap(14.0).items_center()
        .child(slider("Glide", s.ctl.glide_ms, 0.0, 800.0, format!("{} ms", s.ctl.glide_ms as i32),
            |s, v| { s.ctl.glide_ms = v; let g = s.ctl.glide(); s.send(vec![g]); }))
        .child(slider("Strum", s.ctl.strum_ms, 0.0, 160.0, format!("{} ms", s.ctl.strum_ms as i32),
            |s, v| { s.ctl.strum_ms = v; let g = s.ctl.glide(); s.send(vec![g]); }))
        .child(slider("Reverb", s.reverb, 0.0, 1.0, format!("{}%", (s.reverb * 100.0) as i32),
            |s, v| { s.reverb = v; s.send(vec![Command::Reverb(v)]); }))
        .child(row().gap(6.0).items_center()
            .child(text("Octave").font(TextStyle::new(14.0, 500.0)).color(c(theme::INK)))
            .child(text("◀".to_string()).font(TextStyle::new(14.0, 500.0)).color(c(theme::INK_SOFT))
                .on_press(|s: &mut State| s.press(KeyInput { character: Some('q'), phys: Phys::Letter, shift: false })))
            .child(text(format!("{octave:+}")).font(TextStyle::new(14.0, 500.0)).color(c(theme::INK)))
            .child(text("▶".to_string()).font(TextStyle::new(14.0, 500.0)).color(c(theme::INK_SOFT))
                .on_press(|s: &mut State| s.press(KeyInput { character: Some('w'), phys: Phys::Letter, shift: false }))))
        .child(el().grow(1.0));
    let line2 = row().w_full().gap(10.0).items_center()
        .child(toggle_pill("Smooth voice leading", s.ctl.leader.smooth, |s: &mut State| {
            s.ctl.leader.smooth = !s.ctl.leader.smooth;
        }))
        .child(toggle_pill("Letter replays chord", s.ctl.replay_quality, |s: &mut State| {
            s.ctl.replay_quality = !s.ctl.replay_quality;
        }))
        .child(toggle_pill("Own sound", s.own_sound, |s: &mut State| {
            s.own_sound = !s.own_sound;
            let o = s.own_sound;
            s.send(vec![Command::OwnSound(o)]);
        }))
        .child(toggle_pill(&midi, s.midi != MidiMode::Off, |s: &mut State| s.cycle_midi()))
        .child(toggle_pill("Ring", s.ctl.ring, |s: &mut State| {
            s.ctl.ring = !s.ctl.ring;
        }))
        .child(el().grow(1.0))
        .child(meter(s.meter))
        .child(text(s.status.clone()).font(TextStyle::new(14.0, 400.0)).color(c(theme::INK_SOFT)));
    col().w_full().gap(10.0).child(line1).child(line2)
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let mut body = col().w_full().h_full().padding(24.0).gap(18.0).fill(c(theme::BODY))
        .child(header(s))
        .child(row().w_full().gap(18.0).items_center().child(readout(s)).child(voice_canvas(s)));
    if s.ctl.mode == Mode::Loop {
        body = body.child(loop_strip(s));
    }
    if s.ctl.history.is_empty() {
        body = body.child(text("Press D, then , to play your first chord")
            .font(TextStyle::new(15.0, 400.0)).color(c(theme::INK_SOFT)));
    }
    body = body.child(keymap_panel(s)).child(controls(s));
    body.focusable(true).autofocus().on_key_up(on_key_up)
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
        scenes: &[("demo", demo)],
        poll: Some(poll),
        on_key: Some(on_key),
        setup: Some(setup),
        ..AppSpec::new("keychords", (1180.0, 860.0), State::init, view)
    });
}
