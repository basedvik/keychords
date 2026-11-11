//! UI-thread logic: key presses → chords → voicings → engine commands. No Rune types here,
//! so it is unit-testable; `main.rs` adapts Rune key events into `keymap::KeyInput`.

#![allow(dead_code)]

use chords::{Chord, PitchClass, Quality};
use keymap::{KeyAction, KeyInput, KeyMap};
use midi::export::{self, LoopExport};
use midi::smf::Smf;
use sequencer::{boundaries, Clock, Loop, Slot};
use synth::Patch;
use voicing::{voice_cycle, VoiceLeader, Voicing};

use crate::engine::{Command, Engine, Notes};

const HISTORY: usize = 16;
/// Extra lookahead beyond the glide time, covering UI frame jitter.
const LOOKAHEAD_MS: f64 = 150.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Live,
    Loop,
}

pub fn notes_of(v: &Voicing) -> Notes {
    let mut all = Vec::with_capacity(1 + v.upper.len());
    all.push(v.bass);
    all.extend_from_slice(&v.upper);
    Notes::from_slice(&all)
}

pub struct Controller {
    pub keymap: KeyMap,
    pub leader: VoiceLeader,
    pub mode: Mode,
    pub root: Option<PitchClass>,
    pub quality: Option<Quality>,
    /// Z / X: one-shot accidental for the next root letter.
    accidental: i8,
    /// "A letter replays the last chord type."
    pub replay_quality: bool,
    pub patch: Patch,
    pub glide_ms: f32,
    pub strum_ms: f32,
    /// Most recent chords played live, oldest first, for the voice-line view.
    pub history: Vec<(Chord, Voicing)>,
    /// When each `history` entry was played, in seconds of the engine clock.
    pub history_at: Vec<f64>,
    pub lp: Loop,
    pub cursor: usize,
    pub playing: bool,
    /// Engine sample at which loop beat 0 fell.
    origin: u64,
    /// Boundaries before this sample have been scheduled.
    scheduled_to: u64,
    /// Per-slot voicing for the loop (None = silent).
    loop_voicings: Vec<Option<Voicing>>,
    pub clock: Clock,
}

impl Controller {
    pub fn new(sample_rate: f64) -> Controller {
        Controller {
            keymap: KeyMap::new(&[]),
            leader: VoiceLeader::new(),
            mode: Mode::Live,
            root: None,
            quality: None,
            accidental: 0,
            replay_quality: false,
            patch: Patch::GlidePad,
            glide_ms: 220.0,
            strum_ms: 0.0,
            history: Vec::new(),
            history_at: Vec::new(),
            lp: Loop::new(4, 1),
            cursor: 0,
            playing: false,
            origin: 0,
            scheduled_to: 0,
            loop_voicings: vec![None; 4],
            clock: Clock { bpm: 90.0, sample_rate },
        }
    }

    pub fn current(&self) -> Option<&(Chord, Voicing)> {
        self.history.last()
    }

    pub fn set_mode(&mut self, mode: Mode) -> Vec<Command> {
        self.mode = mode;
        self.playing = false;
        vec![Command::Release]
    }

    pub fn glide(&self) -> Command {
        Command::Glide { glide_ms: self.glide_ms, strum_ms: self.strum_ms }
    }

    /// A key press; returns the commands for the engine.
    pub fn key(&mut self, input: KeyInput, now: u64) -> Vec<Command> {
        let Some(action) = self.keymap.resolve(input) else { return Vec::new() };
        match action {
            KeyAction::Root { letter, sharp } => {
                let shift = self.accidental + sharp as i8;
                self.accidental = 0;
                let root = (letter.pitch_class() as i8 + shift).rem_euclid(12) as PitchClass;
                self.root = Some(root);
                match (self.replay_quality, self.quality) {
                    (true, Some(q)) => self.chord(Chord::new(root, q), now),
                    _ => Vec::new(),
                }
            }
            KeyAction::Quality(q) => {
                self.quality = Some(q);
                match self.root {
                    Some(root) => self.chord(Chord::new(root, q), now),
                    None => Vec::new(),
                }
            }
            KeyAction::Flat => {
                self.accidental = -1;
                Vec::new()
            }
            KeyAction::Sharp => {
                self.accidental = 1;
                Vec::new()
            }
            KeyAction::OctaveDown | KeyAction::OctaveUp => {
                let step = if action == KeyAction::OctaveUp { 1 } else { -1 };
                self.leader.octave = (self.leader.octave + step).clamp(-2, 2);
                self.revoice_loop();
                Vec::new()
            }
            KeyAction::Sound(i) => {
                self.patch = Patch::ALL[i as usize];
                vec![Command::Patch(self.patch)]
            }
            KeyAction::Stop => match self.mode {
                Mode::Live => {
                    self.leader.reset();
                    vec![Command::Release]
                }
                Mode::Loop => {
                    self.write_slot(Slot::Silent);
                    Vec::new()
                }
            },
            KeyAction::Transport => self.toggle_play(now),
        }
    }

    /// Apply a vibe: patch, glide, strum, octave and reverb at once. Returns the commands for
    /// the engine in this order: Patch, Glide, Reverb. Octave changes re-voice the loop.
    pub fn apply_vibe(&mut self, v: &crate::vibes::Vibe) -> Vec<Command> {
        self.patch = v.patch;
        self.glide_ms = v.glide_ms;
        self.strum_ms = v.strum_ms;
        self.leader.octave = v.octave;
        self.revoice_loop();
        vec![
            Command::Patch(v.patch),
            self.glide(),
            Command::Reverb(v.reverb),
        ]
    }

    /// Change the loop tempo, clamped to 60..=180 BPM. While the loop plays, the current beat
    /// position is kept: `origin` moves so that `now` maps to the same beat as before, and
    /// boundaries not yet scheduled are re-timed from there (`scheduled_to` becomes `now`).
    pub fn set_bpm(&mut self, bpm: f64, now: u64) {
        let bpm = bpm.clamp(60.0, 180.0);
        if self.playing {
            let old_spb = self.clock.samples_per_beat();
            let beat = (now - self.origin) as f64 / old_spb;
            self.clock.bpm = bpm;
            let new_spb = self.clock.samples_per_beat();
            let offset = (beat * new_spb).round() as i64;
            self.origin = now.saturating_sub(offset as u64);
            self.scheduled_to = now;
        } else {
            self.clock.bpm = bpm;
        }
    }

    /// Play `chord` exactly as if its root letter and chord key had been pressed (live: voiced
    /// and sent now; loop: written into the slot under the cursor). Sets `root` and `quality`.
    pub fn play_chord(&mut self, chord: Chord, now: u64) -> Vec<Command> {
        self.root = Some(chord.root);
        self.quality = Some(chord.quality);
        self.chord(chord, now)
    }

    fn chord(&mut self, chord: Chord, now: u64) -> Vec<Command> {
        match self.mode {
            Mode::Live => {
                let v = self.leader.next(chord);
                let cmd = Command::Play(notes_of(&v));
                self.history.push((chord, v));
                self.history_at.push(now as f64 / self.clock.sample_rate);
                if self.history.len() > HISTORY {
                    self.history.remove(0);
                    self.history_at.remove(0);
                }
                vec![cmd]
            }
            Mode::Loop => {
                self.write_slot(Slot::Chord(chord));
                Vec::new()
            }
        }
    }

    fn write_slot(&mut self, slot: Slot) {
        let n = self.lp.slots.len();
        self.lp.slots[self.cursor % n] = slot;
        self.cursor = (self.cursor + 1) % n;
        self.revoice_loop();
    }

    pub fn move_cursor(&mut self, delta: i32) {
        let n = self.lp.slots.len() as i32;
        self.cursor = (self.cursor as i32 + delta).rem_euclid(n) as usize;
    }

    pub fn set_bars(&mut self, bars: u8) {
        self.lp.set_bars(bars);
        self.cursor %= self.lp.slots.len();
        self.revoice_loop();
    }

    pub fn toggle_half(&mut self) {
        let per_bar = if self.lp.per_bar == 1 { 2 } else { 1 };
        self.lp.set_per_bar(per_bar);
        self.cursor %= self.lp.slots.len();
        self.revoice_loop();
    }

    /// Voice the loop as one cycle: the chords at each change are solved together, and each
    /// sounding slot takes the voicing of the latest change at or before it (wrapping).
    fn revoice_loop(&mut self) {
        let n = self.lp.slots.len();
        let mut sounding: Vec<usize> =
            self.lp.changes().into_iter().filter(|&i| self.lp.resolved(i).is_some()).collect();
        if sounding.is_empty() {
            // One chord throughout (no changes) or nothing at all.
            sounding.extend((0..n).find(|&i| self.lp.resolved(i).is_some()));
        }
        let chords: Vec<Chord> = sounding.iter().filter_map(|&i| self.lp.resolved(i)).collect();
        let solved = voice_cycle(&chords, self.leader.octave);
        self.loop_voicings = (0..n)
            .map(|i| {
                self.lp.resolved(i)?;
                let k = sounding.iter().rposition(|&c| c <= i).unwrap_or(sounding.len() - 1);
                solved.get(k).cloned()
            })
            .collect();
    }

    /// The voicing loop slot `i` plays, if it sounds.
    pub fn loop_voicing(&self, i: usize) -> Option<Voicing> {
        self.loop_voicings.get(i).cloned().flatten()
    }

    fn toggle_play(&mut self, now: u64) -> Vec<Command> {
        if self.mode != Mode::Loop {
            return Vec::new();
        }
        self.playing = !self.playing;
        if self.playing {
            self.origin = now + self.clock.beat_to_sample(0.1);
            self.scheduled_to = self.origin;
            Vec::new()
        } else {
            vec![Command::Release]
        }
    }

    /// Called every UI frame with the engine's sample clock: schedules loop boundaries that
    /// fall within the lookahead window.
    pub fn poll(&mut self, now: u64) -> Vec<Command> {
        if !(self.playing && self.mode == Mode::Loop) {
            return Vec::new();
        }
        let ahead = (self.glide_ms as f64 + LOOKAHEAD_MS) * self.clock.sample_rate / 1000.0;
        let until = now + ahead as u64;
        if until <= self.scheduled_to {
            return Vec::new();
        }
        let mut cmds = Vec::new();
        for (at, slot) in boundaries(&self.lp, &self.clock, self.origin, self.scheduled_to, until) {
            let prev = (slot + self.lp.slots.len() - 1) % self.lp.slots.len();
            let first = at == self.origin;
            if !first && self.loop_voicings[slot] == self.loop_voicings[prev] {
                continue;
            }
            match &self.loop_voicings[slot] {
                Some(v) => cmds.push(Command::PlayAt(notes_of(v), at)),
                None => cmds.push(Command::Release),
            }
        }
        self.scheduled_to = until;
        cmds
    }

    /// The slot the loop is in at `now`, for the playhead.
    pub fn playhead(&self, now: u64) -> Option<usize> {
        if !self.playing || now < self.origin {
            return None;
        }
        let beats = (now - self.origin) as f64 / self.clock.samples_per_beat();
        Some((beats / self.lp.slot_beats()) as usize % self.lp.slots.len())
    }

    /// Serialize loop state as UTF-8 text lines (key=value).
    ///
    /// Format: `v=1`, `bars`, `per_bar`, `bpm`, `glide_ms`, `strum_ms`, `smooth` (0/1),
    /// `octave`, and `slots` as a comma-separated list where each slot is `-` (empty),
    /// `s` (silent), or `root:quality_index`.
    pub fn state_bytes(&self) -> Vec<u8> {
        let mut lines = Vec::new();
        lines.push("v=1".to_string());
        lines.push(format!("bars={}", self.lp.bars));
        lines.push(format!("per_bar={}", self.lp.per_bar));
        lines.push(format!("bpm={}", self.clock.bpm));
        lines.push(format!("glide_ms={}", self.glide_ms));
        lines.push(format!("strum_ms={}", self.strum_ms));
        lines.push(format!("smooth={}", if self.leader.smooth { 1 } else { 0 }));
        lines.push(format!("octave={}", self.leader.octave));
        let slot_strs: Vec<String> = self.lp.slots.iter().map(|s| match s {
            Slot::Empty => "-".to_string(),
            Slot::Silent => "s".to_string(),
            Slot::Chord(ch) => format!("{}:{}", ch.root, ch.quality.index()),
        }).collect();
        lines.push(format!("slots={}", slot_strs.join(",")));
        lines.join("\n").into_bytes()
    }

    /// Restore loop state from UTF-8 text (key=value) lines produced by [`state_bytes`].
    ///
    /// Returns `false` on anything malformed, leaving the controller unchanged. Returns
    /// `true` and re-voices the loop on success.
    pub fn restore_state(&mut self, bytes: &[u8]) -> bool {
        let text = match std::str::from_utf8(bytes) {
            Ok(t) => t,
            Err(_) => return false,
        };
        let mut bars: u8 = 0;
        let mut per_bar: u8 = 0;
        let mut bpm: f64 = 0.0;
        let mut glide_ms: f32 = 0.0;
        let mut strum_ms: f32 = 0.0;
        let mut smooth: bool = false;
        let mut octave: i8 = 0;
        let mut slot_strs: Option<Vec<String>> = None;
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { return false };
            match k {
                "v" => {
                    if v != "1" { return false; }
                }
                "bars" => { bars = match v.parse() { Ok(v) => v, Err(_) => return false }; }
                "per_bar" => { per_bar = match v.parse() { Ok(v) => v, Err(_) => return false }; }
                "bpm" => { bpm = match v.parse() { Ok(v) => v, Err(_) => return false }; }
                "glide_ms" => { glide_ms = match v.parse() { Ok(v) => v, Err(_) => return false }; }
                "strum_ms" => { strum_ms = match v.parse() { Ok(v) => v, Err(_) => return false }; }
                "smooth" => {
                    smooth = match v { "1" => true, "0" => false, _ => return false };
                }
                "octave" => { octave = match v.parse() { Ok(v) => v, Err(_) => return false }; }
                "slots" => {
                    slot_strs = Some(v.split(',').map(|s| s.to_string()).collect());
                }
                _ => return false,
            }
        }
        let slots = match slot_strs {
            Some(s) => s,
            None => return false,
        };
        if bars != 4 && bars != 8 && bars != 16 && bars != 32 { return false; }
        if per_bar != 1 && per_bar != 2 { return false; }
        let expected_n: usize = (bars * per_bar) as usize;
        if slots.len() != expected_n { return false; }
        let new_slots: Vec<Slot> = slots.into_iter().filter_map(|s| {
            if s == "-" {
                Some(Slot::Empty)
            } else if s == "s" {
                Some(Slot::Silent)
            } else {
                let (root_s, qi_s) = s.split_once(':')?;
                let root: PitchClass = root_s.parse().ok()?;
                let qi: usize = qi_s.parse().ok()?;
                let quality = Quality::from_index(qi)?;
                Some(Slot::Chord(Chord::new(root, quality)))
            }
        }).collect();
        if new_slots.len() != expected_n { return false; }
        self.lp.bars = bars;
        self.lp.per_bar = per_bar;
        self.lp.slots = new_slots;
        self.clock.bpm = bpm;
        self.glide_ms = glide_ms;
        self.strum_ms = strum_ms;
        self.leader.smooth = smooth;
        self.leader.octave = octave;
        self.cursor = 0;
        self.revoice_loop();
        true
    }

    /// Export the loop as a Standard MIDI File (bytes).
    pub fn export_midi(&self) -> Vec<u8> {
        let voicings: Vec<Option<Vec<u8>>> = (0..self.lp.slots.len())
            .map(|i| self.loop_voicing(i).map(|v| {
                let mut notes = vec![v.bass];
                notes.extend(v.upper);
                notes
            }))
            .collect();
        let names: Vec<Option<String>> = self.lp.slots.iter().map(|s| match s {
            Slot::Chord(ch) => Some(ch.name()),
            _ => None,
        }).collect();
        let export = LoopExport {
            bpm: self.clock.bpm,
            slot_beats: self.lp.slot_beats(),
            glide_ms: self.glide_ms,
            voicings,
            names,
            state: self.state_bytes(),
        };
        let smf = export::to_smf(&export);
        smf.to_bytes()
    }

    /// Import a loop from Standard MIDI File bytes.
    ///
    /// If the file contains our app state, it is restored exactly. Otherwise the MIDI
    /// note events are parsed and chords are written into a new loop of the smallest size
    /// in {4, 8, 16, 32} bars that fits, with half-bar slots (`per_bar = 2`) and
    /// `Slot::Silent` for unfilled slots.
    pub fn import_midi(&mut self, bytes: &[u8]) -> Result<(), String> {
        let smf = Smf::parse(bytes).map_err(|e: midi::smf::SmfError| format!("{:?}", e))?;
        if let Some(state) = export::state_of(&smf) {
            if !self.restore_state(&state) {
                return Err("state parse failed".to_string());
            }
            self.mode = Mode::Loop;
            return Ok(());
        }
        // `recognize` returns one entry per half bar, None where nothing matches.
        let chords = midi::recognize::recognize(&smf);
        let n = chords.len();
        let bars = [4u8, 8, 16, 32].into_iter().find(|&b| (b * 2) as usize >= n).unwrap_or(32);
        let total = (bars * 2) as usize;
        let slots: Vec<Slot> = chords
            .into_iter()
            .map(|c| c.map(Slot::Chord).unwrap_or(Slot::Silent))
            .chain(std::iter::repeat_n(Slot::Silent, total - n))
            .collect();
        self.lp.bars = bars;
        self.lp.per_bar = 2;
        self.lp.slots = slots;
        self.cursor = 0;
        self.revoice_loop();
        self.mode = Mode::Loop;
        Ok(())
    }

    /// Render one pass of the loop plus a 1-second tail as a 24-bit WAV file, offline, with
    /// a fresh [`Engine`] (no MIDI). Sends the same `PlayAt`/`Release` commands, at the same
    /// sample times, as [`Controller::poll`] would for one pass.
    pub fn render_wav(&self, sample_rate: u32) -> Vec<u8> {
        let clock = Clock { bpm: self.clock.bpm, sample_rate: sample_rate as f64 };
        let slot_beats = self.lp.slot_beats();
        let n = self.lp.slots.len();
        let mut cmds: Vec<(u64, Command)> = Vec::new();
        for k in 0..n {
            let at = clock.beat_to_sample(k as f64 * slot_beats);
            let prev = (k + n - 1) % n;
            if k > 0 && self.loop_voicings[k] == self.loop_voicings[prev] {
                continue;
            }
            match &self.loop_voicings[k] {
                Some(v) => cmds.push((at, Command::PlayAt(notes_of(v), at))),
                None => cmds.push((at, Command::Release)),
            }
        }
        let total = clock.beat_to_sample(n as f64 * slot_beats) as usize + sample_rate as usize;

        let mut engine = Engine::new(sample_rate as f32);
        let mut midi = Vec::new();
        let mut left = vec![0.0f32; total];
        let mut right = vec![0.0f32; total];
        // Run the engine up to each command's sample so that `Release` takes effect exactly
        // on its boundary, as in `poll`.
        let (mut off, mut ci) = (0usize, 0usize);
        while ci < cmds.len() {
            let at = cmds[ci].0 as usize;
            while off < at {
                let chunk = (at - off).min(512);
                engine.process(&mut left[off..off + chunk], &mut right[off..off + chunk], &mut midi);
                off += chunk;
            }
            engine.handle(cmds[ci].1, &mut midi);
            ci += 1;
        }
        while off < total {
            let chunk = (total - off).min(512);
            engine.process(&mut left[off..off + chunk], &mut right[off..off + chunk], &mut midi);
            off += chunk;
        }

        crate::wav::wav_24bit(&left, &right, sample_rate)
    }
}

#[cfg(test)]
mod tempo_tests {
    use super::*;

    #[test]
    fn clamps_to_range() {
        let mut c = Controller::new(48_000.0);
        c.set_bpm(30.0, 0);
        assert_eq!(c.clock.bpm, 60.0);
        c.set_bpm(400.0, 0);
        assert_eq!(c.clock.bpm, 180.0);
        c.set_bpm(120.0, 0);
        assert_eq!(c.clock.bpm, 120.0);
    }

    #[test]
    fn keeps_the_beat_position_while_playing() {
        let mut c = Controller::new(1000.0);
        c.clock.bpm = 60.0; // 1000 samples per beat
        c.set_mode(Mode::Loop);
        c.key(KeyInput { character: Some(' '), phys: keymap::Phys::Space, shift: false }, 0);
        let origin = 100; // 0.1 beat after now = 0
        let now = origin + 5000; // 5 beats in: slot 1 of a 4-beat-slot loop
        assert_eq!(c.playhead(now), Some(1));
        c.set_bpm(120.0, now); // 500 samples per beat; origin becomes 5100 - 5 * 500 = 2600
        assert_eq!(c.playhead(now), Some(1), "same slot right after the change");
        // 3 more beats at the new tempo reach beat 8: slot 2.
        assert_eq!(c.playhead(now + 1500), Some(2));
        assert_eq!(c.playhead(now + 1499), Some(1));
    }

    #[test]
    fn stopped_loop_just_changes_tempo() {
        let mut c = Controller::new(48_000.0);
        c.set_bpm(100.0, 12_345);
        assert_eq!(c.clock.bpm, 100.0);
        assert_eq!(c.playhead(12_345), None);
    }
}

#[cfg(test)]
mod vibe_tests {
    use super::*;
    use crate::vibes::vibes;

    #[test]
    fn apply_vibe_sets_everything_and_sends_three_commands() {
        let mut c = Controller::new(48_000.0);
        let v = vibes()[3]; // Glass bells: Bells, 200 ms, 60 ms, 0.5, octave +1
        let cmds = c.apply_vibe(&v);
        assert_eq!(c.patch, synth::Patch::Bells);
        assert_eq!((c.glide_ms, c.strum_ms, c.leader.octave), (200.0, 60.0, 1));
        assert_eq!(cmds, vec![
            Command::Patch(synth::Patch::Bells),
            Command::Glide { glide_ms: 200.0, strum_ms: 60.0 },
            Command::Reverb(0.5),
        ]);
    }

    #[test]
    fn play_chord_live_matches_two_key_presses() {
        let mut keys = Controller::new(48_000.0);
        keys.key(KeyInput { character: Some('d'), phys: keymap::Phys::Letter, shift: false }, 0);
        let by_keys = keys.key(KeyInput { character: Some(','), phys: keymap::Phys::Comma, shift: false }, 0);
        let mut direct = Controller::new(48_000.0);
        let by_call = direct.play_chord(Chord::new(2, chords::Quality::Min9), 0);
        assert_eq!(by_keys, by_call);
        assert_eq!(direct.current(), keys.current());
        assert_eq!((direct.root, direct.quality), (Some(2), Some(chords::Quality::Min9)));
    }

    #[test]
    fn play_chord_in_loop_mode_writes_the_slot() {
        let mut c = Controller::new(48_000.0);
        c.set_mode(Mode::Loop);
        assert!(c.play_chord(Chord::new(7, chords::Quality::Dom13), 0).is_empty());
        assert_eq!(c.lp.slots[0], Slot::Chord(Chord::new(7, chords::Quality::Dom13)));
        assert_eq!(c.cursor, 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keymap::Phys;
    use midi::smf::{Event, Track};

    fn letter(c: char) -> KeyInput {
        KeyInput { character: Some(c), phys: Phys::Letter, shift: false }
    }
    fn pos(p: Phys) -> KeyInput {
        KeyInput { character: None, phys: p, shift: false }
    }

    #[test]
    fn letter_then_chord_key_plays() {
        let mut c = Controller::new(48_000.0);
        assert!(c.key(letter('d'), 0).is_empty());
        let cmds = c.key(pos(Phys::Comma), 0); // m9
        assert_eq!(cmds.len(), 1);
        let (chord, v) = c.current().unwrap();
        assert_eq!(chord.name(), "Dm9");
        assert_eq!(v.bass, 50);
        assert!(matches!(cmds[0], Command::Play(n) if n.as_slice()[0] == 50));
    }

    #[test]
    fn chord_key_alone_reuses_root_and_flat_modifier_applies_once() {
        let mut c = Controller::new(48_000.0);
        c.key(letter('z'), 0);
        c.key(letter('b'), 0); // B♭
        c.key(letter('u'), 0); // maj7
        assert_eq!(c.current().unwrap().0.name(), "A♯maj7");
        c.key(letter('i'), 0); // m7 on the same root
        assert_eq!(c.current().unwrap().0.name(), "A♯m7");
        c.key(letter('b'), 0);
        c.key(letter('i'), 0);
        assert_eq!(c.current().unwrap().0.name(), "Bm7", "flat was one-shot");
    }

    #[test]
    fn replay_quality_plays_on_the_letter() {
        let mut c = Controller::new(48_000.0);
        c.replay_quality = true;
        c.key(letter('c'), 0);
        c.key(letter('m'), 0); // maj9
        let cmds = c.key(letter('g'), 0);
        assert_eq!(cmds.len(), 1);
        assert_eq!(c.current().unwrap().0.name(), "Gmaj9");
    }

    #[test]
    fn loop_mode_writes_slots_and_schedules_boundaries() {
        let mut c = Controller::new(1000.0);
        c.clock.bpm = 120.0; // 500 samples per beat, 2000 per bar
        c.glide_ms = 100.0;
        c.set_mode(Mode::Loop);
        for (l, k) in [('d', Phys::Comma), ('g', Phys::Slash), ('c', Phys::Letter), ('a', Phys::Letter)] {
            c.key(letter(l), 0);
            match k {
                Phys::Letter if l == 'c' => { c.key(letter('m'), 0); }
                Phys::Letter => { c.key(letter('h'), 0); }
                p => { c.key(pos(p), 0); }
            }
        }
        let names: Vec<_> = (0..4).map(|i| c.lp.resolved(i).unwrap().name()).collect();
        assert_eq!(names, ["Dm9", "G13", "Cmaj9", "A7♭9"]);
        assert_eq!(c.cursor, 0, "wrapped");
        c.key(pos(Phys::Space), 0);
        assert!(c.playing);
        let origin = 50; // 0.1 beat at 500 samples per beat
        let first = c.poll(0);
        assert!(matches!(first[..], [Command::PlayAt(_, at)] if at == origin));
        assert!(c.poll(100).is_empty());
        let second = c.poll(origin + 2000 - 250 + 1);
        assert!(matches!(second[..], [Command::PlayAt(_, at)] if at == origin + 2000));
        assert_eq!(c.playhead(origin + 2100), Some(1));
    }

    #[test]
    fn state_bytes_round_trips_into_a_fresh_controller() {
        let mut c = Controller::new(48_000.0);
        c.lp.bars = 8;
        c.lp.per_bar = 2;
        c.lp.slots = vec![
            Slot::Chord(Chord::new(2, Quality::Min9)),
            Slot::Empty,
            Slot::Silent,
            Slot::Chord(Chord::new(7, Quality::Dom13)),
            Slot::Empty,
            Slot::Chord(Chord::new(0, Quality::Maj7)),
            Slot::Silent,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
        ];
        c.clock.bpm = 112.0;
        c.glide_ms = 333.0;
        c.strum_ms = 25.0;
        c.leader.smooth = true;
        c.leader.octave = -1;
        c.revoice_loop();

        // Hand-written expected text: quality indices are Min9 = 13, Dom13 = 15, Maj7 = 4.
        let text = String::from_utf8(c.state_bytes().clone()).unwrap();
        assert_eq!(
            text,
            "v=1\nbars=8\nper_bar=2\nbpm=112\nglide_ms=333\nstrum_ms=25\nsmooth=1\noctave=-1\n\
             slots=2:13,-,s,7:15,-,0:4,s,-,-,-,-,-,-,-,-,-"
        );

        let mut c2 = Controller::new(48_000.0);
        assert!(c2.restore_state(&c.state_bytes()));
        assert_eq!(c2.lp.bars, 8);
        assert_eq!(c2.lp.per_bar, 2);
        assert_eq!(c2.lp.slots, c.lp.slots);
        assert_eq!(c2.clock.bpm, 112.0);
        assert_eq!(c2.glide_ms, 333.0);
        assert_eq!(c2.strum_ms, 25.0);
        assert!(c2.leader.smooth);
        assert_eq!(c2.leader.octave, -1);
        for i in 0..16 {
            assert_eq!(c2.loop_voicing(i), c.loop_voicing(i), "voicing {i}");
        }
    }

    #[test]
    fn restore_state_rejects_malformed_input_without_changes() {
        let mut c = Controller::new(48_000.0);
        c.lp.slots[0] = Slot::Chord(Chord::new(0, Quality::Maj));
        c.revoice_loop();
        let before = c.state_bytes();

        let bad: &[&[u8]] = &[
            b"",
            b"v=2\nbars=4\nper_bar=1\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=0\noctave=0\nslots=-,-,-,-",
            b"v=1\nbars=7\nper_bar=1\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=0\noctave=0\nslots=-,-,-,-",
            b"v=1\nbars=4\nper_bar=3\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=0\noctave=0\nslots=-,-,-,-",
            b"v=1\nbars=4\nper_bar=1\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=0\noctave=0\nslots=-,-,-",
            b"v=1\nbars=4\nper_bar=1\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=0\noctave=0\nslots=-,-,-,300:0",
            b"v=1\nbars=4\nper_bar=1\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=0\noctave=0\nslots=-,-,-,0:24",
            b"v=1\nbars=4\nper_bar=1\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=0\noctave=0\nslots=-,-,-,x",
            b"v=1\nbars=4\nper_bar=1\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=2\noctave=0\nslots=-,-,-,-",
            b"v=1\nbars=4\nper_bar=1\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=0\noctave=0\nslots=-,-,-,-\nfrobnicate=1",
            b"v=1\nbars4\nper_bar=1\nbpm=90\nglide_ms=0\nstrum_ms=0\nsmooth=0\noctave=0\nslots=-,-,-,-",
            b"v=1\n\xff",
        ];
        for (i, b) in bad.iter().enumerate() {
            assert!(!c.restore_state(b), "case {i} accepted");
            assert_eq!(c.state_bytes(), before, "case {i} changed the controller");
        }
    }

    #[test]
    fn export_midi_then_import_restores_the_exact_loop() {
        let mut c = Controller::new(48_000.0);
        c.set_mode(Mode::Loop);
        c.lp.bars = 8;
        c.lp.per_bar = 2;
        c.lp.slots = vec![
            Slot::Chord(Chord::new(2, Quality::Min9)),
            Slot::Chord(Chord::new(7, Quality::Dom13)),
            Slot::Silent,
            Slot::Chord(Chord::new(9, Quality::Maj7)),
            Slot::Empty,
            Slot::Chord(Chord::new(4, Quality::Dim7)),
            Slot::Empty,
            Slot::Chord(Chord::new(11, Quality::Sus4)),
            Slot::Silent,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
        ];
        c.clock.bpm = 96.0;
        c.glide_ms = 240.0;
        c.strum_ms = 30.0;
        c.leader.smooth = true;
        c.leader.octave = 1;
        c.revoice_loop();

        let bytes = c.export_midi();

        let mut c2 = Controller::new(48_000.0);
        c2.import_midi(&bytes).expect("import");
        assert_eq!(c2.mode, Mode::Loop);
        assert_eq!(c2.lp.bars, c.lp.bars);
        assert_eq!(c2.lp.per_bar, c.lp.per_bar);
        assert_eq!(c2.lp.slots, c.lp.slots);
        assert_eq!(c2.clock.bpm, c.clock.bpm);
        assert_eq!(c2.glide_ms, c.glide_ms);
        assert_eq!(c2.strum_ms, c.strum_ms);
        assert_eq!(c2.leader.smooth, c.leader.smooth);
        assert_eq!(c2.leader.octave, c.leader.octave);
        for i in 0..c.lp.slots.len() {
            assert_eq!(c2.loop_voicing(i), c.loop_voicing(i), "voicing {i}");
        }
    }

    #[test]
    fn foreign_midi_imports_as_chords() {
        // C major (C4 E4 G4) for one half bar, then G7 (G3 B3 D4 F4) for the next, at PPQ 480.
        let events: Vec<(u32, Event)> = vec![
            (0, Event::NoteOn { ch: 0, note: 60, vel: 100 }),
            (0, Event::NoteOn { ch: 0, note: 64, vel: 100 }),
            (0, Event::NoteOn { ch: 0, note: 67, vel: 100 }),
            (960, Event::NoteOff { ch: 0, note: 60, vel: 0 }),
            (960, Event::NoteOff { ch: 0, note: 64, vel: 0 }),
            (960, Event::NoteOff { ch: 0, note: 67, vel: 0 }),
            (960, Event::NoteOn { ch: 0, note: 55, vel: 100 }),
            (960, Event::NoteOn { ch: 0, note: 59, vel: 100 }),
            (960, Event::NoteOn { ch: 0, note: 62, vel: 100 }),
            (960, Event::NoteOn { ch: 0, note: 65, vel: 100 }),
            (1920, Event::NoteOff { ch: 0, note: 55, vel: 0 }),
            (1920, Event::NoteOff { ch: 0, note: 59, vel: 0 }),
            (1920, Event::NoteOff { ch: 0, note: 62, vel: 0 }),
            (1920, Event::NoteOff { ch: 0, note: 65, vel: 0 }),
        ];
        let smf = Smf { ppq: 480, tracks: vec![Track { events }] };

        let mut c = Controller::new(48_000.0);
        c.import_midi(&smf.to_bytes()).expect("import");
        assert_eq!(c.mode, Mode::Loop);
        assert_eq!(c.lp.bars, 4, "smallest loop that fits two half bars");
        assert_eq!(c.lp.per_bar, 2);
        assert_eq!(c.lp.slots.len(), 8);
        assert_eq!(c.lp.slots[0], Slot::Chord(Chord::new(0, Quality::Maj)), "C major");
        assert_eq!(c.lp.slots[1], Slot::Chord(Chord::new(7, Quality::Dom7)), "G7");
        for i in 2..8 {
            assert_eq!(c.lp.slots[i], Slot::Silent, "unfilled slot {i}");
        }
    }

    #[test]
    fn render_wav_is_one_pass_plus_tail_and_non_silent() {
        let mut c = Controller::new(8_000.0);
        c.clock.bpm = 120.0; // 4000 samples per beat
        c.set_mode(Mode::Loop);
        c.lp.slots[0] = Slot::Chord(Chord::new(2, Quality::Min9));
        c.lp.slots[1] = Slot::Chord(Chord::new(7, Quality::Maj));
        c.lp.slots[2] = Slot::Chord(Chord::new(0, Quality::Maj7));
        c.lp.slots[3] = Slot::Silent;
        c.revoice_loop();

        let buf = c.render_wav(8_000);
        // 4 slots x 4 beats x 4000 samples/beat = 64000, plus a 1 s (8000) tail.
        assert_eq!(buf.len(), 44 + 6 * (64_000 + 8_000));
        assert!(buf[44..].iter().any(|&b| b != 0), "non-silent render");
    }
}
