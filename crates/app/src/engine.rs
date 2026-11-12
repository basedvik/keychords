//! The real-time engine: everything the audio callback does, with no device attached so it
//! can be driven from tests. The UI thread sends `Command`s; the callback calls `process`.

use glide::{Glider, Params, MAX_VOICES};
use midi::mpe::{MpeOut, Msg, PlainOut};
use synth::{Patch, Synth};

use crate::reverb::Reverb;

/// Master make-up gain before the soft limiter.
const MASTER_GAIN: f32 = 2.2;

/// Pitches are pushed to the synth and MIDI out once per control block.
pub const CONTROL_BLOCK: usize = 32;
const VELOCITY: u8 = 100;

/// A chord as the engine plays it: bass first, then upper voices ascending.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Notes {
    pub len: u8,
    pub notes: [u8; MAX_VOICES],
}

impl Notes {
    pub fn from_slice(s: &[u8]) -> Notes {
        let mut n = Notes::default();
        let len = s.len().min(MAX_VOICES);
        n.notes[..len].copy_from_slice(&s[..len]);
        n.len = len as u8;
        n
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.notes[..self.len as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiMode {
    Off,
    Plain { channel: u8 },
    Mpe,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Live play: glide into this chord now.
    Play(Notes),
    /// Loop play: land on this chord at an absolute sample.
    PlayAt(Notes, u64),
    Release,
    Glide { glide_ms: f32, strum_ms: f32 },
    Patch(Patch),
    OwnSound(bool),
    Midi(MidiMode),
    /// 0..=1 reverb mix.
    Reverb(f32),
}

pub struct Engine {
    sample_rate: f32,
    now: u64,
    glider: Glider,
    synth: Synth,
    reverb: Reverb,
    own_sound: bool,
    midi_mode: MidiMode,
    mpe: MpeOut,
    plain: PlainOut,
    /// Per voice, whether its latest landing has been struck on MIDI.
    struck: [Option<u64>; MAX_VOICES],
    sounding: [bool; MAX_VOICES],
    /// Chords waiting for a loop boundary, so plain-notes MIDI changes on the beat.
    plain_pending: Option<(Notes, u64)>,
}

impl Engine {
    pub fn new(sample_rate: f32) -> Engine {
        let params = Params { glide_ms: 220.0, strum_ms: 0.0, sample_rate };
        let mut synth = Synth::new(sample_rate);
        synth.set_volume_db(0.0);
        Engine {
            sample_rate,
            now: 0,
            glider: Glider::new(params),
            synth,
            reverb: Reverb::new(sample_rate),
            own_sound: true,
            midi_mode: MidiMode::Off,
            mpe: MpeOut::new(),
            plain: PlainOut::new(0),
            struck: [None; MAX_VOICES],
            sounding: [false; MAX_VOICES],
            plain_pending: None,
        }
    }

    /// Each voice's pitch right now (fractional MIDI), `None` when silent.
    pub fn voice_pitches(&self) -> [Option<f32>; MAX_VOICES] {
        std::array::from_fn(|v| self.glider.pitch(v, self.now))
    }

    /// Absolute sample position of the next sample `process` will render.
    pub fn now(&self) -> u64 {
        self.now
    }

    pub fn handle(&mut self, cmd: Command, midi: &mut Vec<Msg>) {
        match cmd {
            Command::Play(n) => {
                self.glider.play_now(n.as_slice(), self.now);
                if let MidiMode::Plain { .. } = self.midi_mode {
                    midi.extend(self.plain.chord(n.as_slice(), VELOCITY));
                }
            }
            Command::PlayAt(n, at) => {
                self.glider.play_at(n.as_slice(), at);
                if let MidiMode::Plain { .. } = self.midi_mode {
                    self.plain_pending = Some((n, at));
                }
            }
            Command::Release => {
                self.glider.release(self.now);
                midi.extend(self.mpe.all_off());
                midi.extend(self.plain.all_off());
                self.struck = [None; MAX_VOICES];
                self.plain_pending = None;
            }
            Command::Glide { glide_ms, strum_ms } => {
                self.glider.set_params(Params { glide_ms, strum_ms, sample_rate: self.sample_rate });
            }
            Command::Patch(p) => self.synth.set_patch(p),
            Command::Reverb(mix) => self.reverb.mix = mix.clamp(0.0, 1.0),
            Command::OwnSound(on) => self.own_sound = on,
            Command::Midi(mode) => {
                midi.extend(self.mpe.all_off());
                midi.extend(self.plain.all_off());
                self.struck = [None; MAX_VOICES];
                self.midi_mode = mode;
                match mode {
                    MidiMode::Mpe => midi.extend(self.mpe.setup()),
                    MidiMode::Plain { channel } => self.plain = PlainOut::new(channel),
                    MidiMode::Off => {}
                }
            }
        }
    }

    /// Render one buffer. MIDI produced during it is appended to `midi` (allocation-free
    /// while `midi` has capacity).
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], midi: &mut Vec<Msg>) {
        let mut off = 0;
        while off < left.len() {
            let end = (off + CONTROL_BLOCK).min(left.len());
            let t = self.now + off as u64;
            self.control(t, midi);
            self.synth.render(&mut left[off..end], &mut right[off..end]);
            off = end;
        }
        if self.own_sound {
            self.reverb.process(left, right);
            // Make-up gain into a soft limiter: loud on laptop speakers, never clipping.
            for v in left.iter_mut().chain(right.iter_mut()) {
                *v = (*v * MASTER_GAIN).tanh() * 0.9;
            }
        } else {
            left.fill(0.0);
            right.fill(0.0);
        }
        self.now += left.len() as u64;
    }

    fn control(&mut self, t: u64, midi: &mut Vec<Msg>) {
        if let Some((n, at)) = self.plain_pending {
            if t >= at {
                midi.extend(self.plain.chord(n.as_slice(), VELOCITY));
                self.plain_pending = None;
            }
        }
        for v in 0..MAX_VOICES {
            let pitch = self.glider.pitch(v, t);
            if pitch.is_some() != self.sounding[v] {
                self.sounding[v] = pitch.is_some();
                if pitch.is_none() && self.midi_mode == MidiMode::Mpe {
                    midi.extend(self.mpe.release(v));
                    self.struck[v] = None;
                }
            }
            self.synth.set_voice(v, pitch);
            let (Some(p), MidiMode::Mpe) = (pitch, self.midi_mode) else { continue };
            // Struck as it lands: once a slide has landed, re-strike at the new note; until
            // then, bend from the note already held.
            let lands = self.glider.lands_at(v);
            let landed = lands.is_some_and(|l| t >= l);
            if landed && self.struck[v] != lands {
                midi.extend(self.mpe.strike(v, p.round() as u8, VELOCITY));
                self.struck[v] = lands;
            } else if let Some(m) = self.mpe.pitch(v, p) {
                midi.push(m);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn run(e: &mut Engine, frames: usize, midi: &mut Vec<Msg>) -> Vec<f32> {
        let (mut l, mut r) = (vec![0.0; frames], vec![0.0; frames]);
        for (lc, rc) in l.chunks_mut(512).zip(r.chunks_mut(512)) {
            e.process(lc, rc, midi);
        }
        l
    }

    #[test]
    fn plays_and_glides_audio() {
        let mut e = Engine::new(SR);
        let mut midi = Vec::new();
        e.handle(Command::Play(Notes::from_slice(&[50, 57, 65, 72, 76])), &mut midi);
        let a = run(&mut e, 24_000, &mut midi);
        assert!(a.iter().any(|v| v.abs() > 1e-3), "sound");
        e.handle(Command::Play(Notes::from_slice(&[43, 57, 65, 71, 76])), &mut midi);
        run(&mut e, 24_000, &mut midi);
        assert!(midi.is_empty(), "MIDI off by default");
    }

    #[test]
    fn mpe_bends_then_restrikes_on_landing() {
        let mut e = Engine::new(SR);
        let mut midi = Vec::new();
        e.handle(Command::Midi(MidiMode::Mpe), &mut midi);
        assert_eq!(midi.len(), 3 + 7 * 6, "zone setup");
        midi.clear();
        e.handle(Command::Play(Notes::from_slice(&[48])), &mut midi);
        run(&mut e, 4800, &mut midi);
        assert!(midi.contains(&[0x91, 48, VELOCITY]), "first chord struck at once");
        midi.clear();
        e.handle(Command::Play(Notes::from_slice(&[50])), &mut midi);
        run(&mut e, 24_000, &mut midi);
        let bends = midi.iter().filter(|m| m[0] == 0xE1).count();
        assert!(bends > 10, "bends during the 220 ms slide: {bends}");
        let land = midi.iter().position(|m| *m == [0x91, 50, VELOCITY]).expect("struck at 50");
        assert!(midi[..land].contains(&[0x81, 48, 0]), "old note off before the restrike");
    }

    #[test]
    fn plain_midi_changes_on_the_loop_boundary() {
        let mut e = Engine::new(SR);
        let mut midi = Vec::new();
        e.handle(Command::Midi(MidiMode::Plain { channel: 0 }), &mut midi);
        e.handle(Command::PlayAt(Notes::from_slice(&[48, 64, 67]), 9_600), &mut midi);
        run(&mut e, 9_000, &mut midi);
        assert!(midi.is_empty(), "not yet");
        run(&mut e, 1_000, &mut midi);
        assert_eq!(midi, vec![[0x90, 48, VELOCITY], [0x90, 64, VELOCITY], [0x90, 67, VELOCITY]]);
    }

    #[test]
    fn own_sound_off_is_silent_but_still_sends_midi() {
        let mut e = Engine::new(SR);
        let mut midi = Vec::new();
        e.handle(Command::OwnSound(false), &mut midi);
        e.handle(Command::Midi(MidiMode::Plain { channel: 2 }), &mut midi);
        e.handle(Command::Play(Notes::from_slice(&[60, 64, 67])), &mut midi);
        let a = run(&mut e, 4800, &mut midi);
        assert!(a.iter().all(|v| *v == 0.0));
        assert_eq!(midi.len(), 3);
    }
}
