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
use rune::tok::{self, type_};
use rune::{canvas, col, el, row, text, AppSpec, Color, Cx, El, Key, KeyEvent, LineCap, Path, PhysicalKeyCode, Shape, TextStyle, Tx};

use controller::{Controller, Mode};
use engine::{Command, MidiMode};

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
}

impl State {
    fn init() -> State {
        // `--snap` renders headlessly; don't open devices for it.
        let headless = std::env::args().any(|a| a == "--snap");
        let (audio, status) = if headless {
            (None, "headless".to_string())
        } else {
            match audio::start() {
                Ok(a) => {
                    let port = a.midi_port.clone().unwrap_or_else(|| "no MIDI port".into());
                    let s = format!("{:.0} Hz · MIDI: {port}", a.sample_rate);
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
            s.press(input);
        }
        _ => return false,
    }
    true
}

fn poll(s: &mut State, _: f64, _: &mut Tx) -> bool {
    let now = s.now();
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

const VOICE_COLORS: [&str; 7] = ["#7B93FF", "#F3D9A4", "#F6C066", "#F7A75C", "#FFE6B8", "#FFF5E3", "#E9B3FF"];
const ROOT_COLOR: &str = "#F3C350";
const CHORD_COLOR: &str = "#8FB2FF";

fn c(h: &str) -> Color {
    hex(h).expect("valid colour")
}

fn big() -> TextStyle {
    TextStyle::new(56.0, 700.0)
}

fn button(label: impl Into<String>, on: bool, f: impl Fn(&mut State) + 'static) -> El {
    let t = tok::theme(true);
    el().padding(8.0).px(12.0).radius(6.0)
        .fill(if on { tok::accent(0, true) } else { t.card })
        .border(1.0, t.card_border)
        .on_press(f)
        .child(text(label).font(type_::LABEL).color(t.text))
}

fn header(s: &State) -> El {
    let t = tok::theme(true);
    let live = s.ctl.mode == Mode::Live;
    row().w_full().items_center().gap(12.0)
        .child(text("keychords").font(TextStyle::new(22.0, 700.0)).color(t.text))
        .child(button("Play live", live, |s: &mut State| { let c = s.ctl.set_mode(Mode::Live); s.send(c); }))
        .child(button("Loop", !live, |s: &mut State| { let c = s.ctl.set_mode(Mode::Loop); s.send(c); }))
        .child(el().grow(1.0))
        .child(meter(s.meter))
        .child(text(s.status.clone()).font(type_::LABEL).color(t.text_secondary))
}

/// A horizontal output-level bar: green, then amber near full scale.
fn meter(level: f32) -> El {
    let t = tok::theme(true);
    let w = 120.0;
    let fill = (level.clamp(0.0, 1.0) * w).max(2.0);
    let col = if level > 0.85 { c("#F6A04D") } else { c("#5FD38D") };
    el().size(w, 8.0).radius(4.0).fill(t.control_track).clip_rounded(4.0)
        .child(el().size(fill, 8.0).radius(4.0).fill(col))
}

fn chord_panel(s: &State) -> El {
    let t = tok::theme(true);
    let (name, notes) = match s.ctl.current() {
        Some((ch, v)) => {
            let mut ns = vec![v.bass];
            ns.extend(&v.upper);
            (ch.name(), ns.iter().map(|n| note_name(*n)).collect::<Vec<_>>().join("  "))
        }
        None => ("–".to_string(), "Pick a root with a letter key, then press a chord key.".to_string()),
    };
    col().w(300.0).gap(10.0).padding(18.0).radius(10.0).fill(t.card)
        .child(text(if s.ctl.mode == Mode::Live { "PLAY LIVE" } else { "LOOP" }).font(type_::SECTION).color(t.text_secondary))
        .child(text(name).font(big()).color(t.text))
        .child(text(notes).font(type_::BODY).color(t.text_secondary))
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
    let t = tok::theme(true);
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

    let mut shape = Shape::new();
    // Faint guide at every C from C2 to C6.
    for c in (36..=84).step_by(12) {
        let y = pitch_y(c as f32);
        shape = shape.line([0.0, y], [VB_W, y], 1.0, LineCap::Butt, c_alpha(t.separator, 0.5));
    }
    for i in 0..hist.len() {
        let here = notes_of(&hist[i].1);
        let start = at[i];
        let next = hist.get(i + 1).map(|(_, v)| notes_of(v));
        let end = at.get(i + 1).copied().unwrap_or(now);
        for (v, &p) in here.iter().enumerate() {
            let y0 = pitch_y(p as f32);
            let col = c(VOICE_COLORS[v.min(6)]);
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
    shape = shape.line([VB_W - 1.0, 0.0], [VB_W - 1.0, VB_H], 1.5, LineCap::Butt, c_alpha(t.text_secondary, 0.6));
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
            shape = shape.line([cx, cy], [cx, cy], 9.0, LineCap::Round, c(VOICE_COLORS[v.min(6)]));
        }
    }
    canvas(shape.view_box(VB_W, VB_H)).key("voices").grow(1.0).h(260.0).radius(10.0).fill(t.card).clip_rounded(10.0)
}

fn c_alpha(col: Color, a: f32) -> Color {
    Color { a: col.a * a, ..col }
}

fn loop_strip(s: &State) -> El {
    let t = tok::theme(true);
    let head = s.ctl.playhead(s.now());
    let mut strip = row().w_full().gap(6.0);
    for i in 0..s.ctl.lp.slots.len() {
        let label = match s.ctl.lp.slots[i] {
            sequencer::Slot::Chord(ch) => ch.name(),
            sequencer::Slot::Silent => "stop".into(),
            sequencer::Slot::Empty => "·".into(),
        };
        let mut cell = el().grow(1.0).h(44.0).radius(6.0).center()
            .fill(if head == Some(i) { tok::accent(0, true) } else { t.card })
            .on_press(move |s: &mut State| s.ctl.cursor = i)
            .child(text(label).font(type_::TITLE).color(t.text));
        if s.ctl.cursor == i {
            cell = cell.border(2.0, c(ROOT_COLOR));
        }
        strip = strip.child(cell);
    }
    let bars = s.ctl.lp.bars;
    let playing = s.ctl.playing;
    col().w_full().gap(8.0)
        .child(row().gap(8.0).items_center()
            .child(button(if playing { "Stop loop" } else { "Play loop" }, playing, |s: &mut State| {
                let now = s.now();
                let c = s.ctl.key(KeyInput { character: Some(' '), phys: Phys::Space, shift: false }, now);
                s.send(c);
            }))
            .child(button(format!("{bars} bars"), false, |s: &mut State| {
                let next = match s.ctl.lp.bars { 4 => 8, 8 => 16, 16 => 32, _ => 4 };
                s.ctl.set_bars(next);
            }))
            .child(button("½ bar", s.ctl.lp.per_bar == 2, |s: &mut State| s.ctl.toggle_half()))
            .child(text(format!("{:.0} BPM · ←/→ pick a slot · Tab half bars · Space play", s.ctl.clock.bpm))
                .font(type_::LABEL).color(t.text_secondary)))
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

fn keymap_panel(s: &State) -> El {
    let t = tok::theme(true);
    let current = s.ctl.current().map(|(ch, _)| *ch);
    let mut rows = col().gap(6.0);
    for (r, keys) in ROWS.iter().enumerate() {
        let mut line = row().gap(6.0).ml(r as f32 * 22.0);
        for &(cap, phys, ch) in keys.iter() {
            let input = KeyInput { character: ch, phys, shift: false };
            let action = s.ctl.keymap.resolve(input);
            let (label, accent) = match action {
                Some(KeyAction::Root { letter, .. }) => {
                    let on = s.ctl.root == Some(letter.pitch_class());
                    (String::new(), on.then(|| c(ROOT_COLOR)))
                }
                Some(KeyAction::Quality(q)) => {
                    let on = current.is_some_and(|ch| ch.quality == q) || s.ctl.quality == Some(q);
                    (q.key_label().to_string(), on.then(|| c(CHORD_COLOR)))
                }
                Some(KeyAction::Sound(i)) => {
                    let p = synth::Patch::ALL[i as usize];
                    (p.label().to_string(), (s.ctl.patch == p).then(|| tok::accent(2, true)))
                }
                Some(KeyAction::OctaveDown) => ("octave −".into(), None),
                Some(KeyAction::OctaveUp) => ("octave +".into(), None),
                Some(KeyAction::Stop) => ("stop".into(), None),
                Some(KeyAction::Flat) => ("flat ♭".into(), None),
                Some(KeyAction::Sharp) => ("sharp ♯".into(), None),
                Some(KeyAction::Transport) | None => (String::new(), None),
            };
            let is_root = matches!(action, Some(KeyAction::Root { .. }));
            let mut key = col().size(64.0, 50.0).padding(6.0).radius(6.0).gap(2.0)
                .fill(t.card).border(1.0, accent.unwrap_or(if is_root { c("#5A4A20") } else { t.card_border }))
                .on_press(move |s: &mut State| s.press(input))
                .child(text(cap).font(if is_root { TextStyle::new(17.0, 700.0) } else { type_::LABEL })
                    .color(if is_root { c(ROOT_COLOR) } else { t.text_secondary }));
            if !label.is_empty() {
                key = key.child(text(label).font(type_::LABEL).color(accent.unwrap_or(t.text)));
            }
            line = line.child(key);
        }
        rows = rows.child(line);
    }
    rows
}

fn controls(s: &State) -> El {
    let t = tok::theme(true);
    let midi = match s.midi {
        MidiMode::Off => "MIDI out: off".to_string(),
        MidiMode::Plain { channel } => format!("MIDI out: notes ch {}", channel + 1),
        MidiMode::Mpe => "MIDI out: MPE".to_string(),
    };
    let stepper = |label: String, dec: fn(&mut State), inc: fn(&mut State)| {
        row().gap(4.0).items_center()
            .child(button("−", false, dec))
            .child(text(label).font(type_::LABEL).color(t.text))
            .child(button("+", false, inc))
    };
    row().w_full().gap(14.0).items_center()
        .child(stepper(format!("Glide {:.0} ms", s.ctl.glide_ms),
            |s| { s.ctl.glide_ms = (s.ctl.glide_ms - 20.0).max(0.0); let g = s.ctl.glide(); s.send(vec![g]); },
            |s| { s.ctl.glide_ms = (s.ctl.glide_ms + 20.0).min(800.0); let g = s.ctl.glide(); s.send(vec![g]); }))
        .child(stepper(format!("Strum {:.0} ms", s.ctl.strum_ms),
            |s| { s.ctl.strum_ms = (s.ctl.strum_ms - 10.0).max(0.0); let g = s.ctl.glide(); s.send(vec![g]); },
            |s| { s.ctl.strum_ms = (s.ctl.strum_ms + 10.0).min(160.0); let g = s.ctl.glide(); s.send(vec![g]); }))
        .child(stepper(format!("Octave {:+}", s.ctl.leader.octave),
            |s| s.press(KeyInput { character: Some('q'), phys: Phys::Letter, shift: false }),
            |s| s.press(KeyInput { character: Some('w'), phys: Phys::Letter, shift: false })))
        .child(stepper(format!("Vol {:.0} dB", s.volume_db),
            |s| { s.volume_db = (s.volume_db - 3.0).max(-48.0); let v = s.volume_db; s.send(vec![Command::VolumeDb(v)]); },
            |s| { s.volume_db = (s.volume_db + 3.0).min(0.0); let v = s.volume_db; s.send(vec![Command::VolumeDb(v)]); }))
        .child(stepper(format!("Reverb {:.0}%", s.reverb * 100.0),
            |s| { s.reverb = (s.reverb - 0.1).max(0.0); let r = s.reverb; s.send(vec![Command::Reverb(r)]); },
            |s| { s.reverb = (s.reverb + 0.1).min(1.0); let r = s.reverb; s.send(vec![Command::Reverb(r)]); }))
        .child(button("Smooth voice leading", s.ctl.leader.smooth, |s: &mut State| {
            s.ctl.leader.smooth = !s.ctl.leader.smooth;
        }))
        .child(button("Letter replays chord", s.ctl.replay_quality, |s: &mut State| {
            s.ctl.replay_quality = !s.ctl.replay_quality;
        }))
        .child(button("Own sound", s.own_sound, |s: &mut State| {
            s.own_sound = !s.own_sound;
            let o = s.own_sound;
            s.send(vec![Command::OwnSound(o)]);
        }))
        .child(button(midi, s.midi != MidiMode::Off, |s: &mut State| s.cycle_midi()))
}

fn view(s: &State, _: &mut Cx<State>) -> El {
    let t = tok::theme(true);
    let mut body = col().w_full().h_full().padding(24.0).gap(18.0).fill(t.bg)
        .child(header(s))
        .child(row().w_full().gap(18.0).child(chord_panel(s)).child(voice_canvas(s)));
    if s.ctl.mode == Mode::Loop {
        body = body.child(loop_strip(s));
    }
    body.child(keymap_panel(s)).child(controls(s))
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
