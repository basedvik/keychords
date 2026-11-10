//! UI-thread logic: key presses → chords → voicings → engine commands. No Rune types here,
//! so it is unit-testable; `main.rs` adapts Rune key events into `keymap::KeyInput`.

use chords::{Chord, PitchClass, Quality};
use keymap::{KeyAction, KeyInput, KeyMap};
use sequencer::{boundaries, Clock, Loop, Slot};
use synth::Patch;
use voicing::{voice_cycle, VoiceLeader, Voicing};

use crate::engine::{Command, Notes};

const HISTORY: usize = 8;
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

    fn chord(&mut self, chord: Chord, _now: u64) -> Vec<Command> {
        match self.mode {
            Mode::Live => {
                let v = self.leader.next(chord);
                let cmd = Command::Play(notes_of(&v));
                self.history.push((chord, v));
                if self.history.len() > HISTORY {
                    self.history.remove(0);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use keymap::Phys;

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
}
