//! Live MIDI output: plain chords on one channel, or MPE with one voice per member channel
//! (architecture.md §9.2).
//!
//! Channels are 0-based here (0 = "channel 1"). MPE uses the lower zone: master channel 0,
//! member channels 1..=7, voice `v` on channel `1 + v`. A gliding voice holds its note and
//! bends; when its slide lands it is re-struck at the new note with the bend reset, so no
//! single bend ever has to span more than one glide.

/// Every message we emit is a 3-byte channel message.
pub type Msg = [u8; 3];

/// Per-member pitch-bend range in semitones (the MPE default).
pub const BEND_RANGE: u8 = 48;

/// 14-bit bend for `semitones` at ±`range`: `8192 + round(semitones / range · 8192)`,
/// clamped to 0..=16383.
pub fn bend_value(semitones: f32, range: f32) -> u16 {
    let v = (8192.0 + (semitones / range * 8192.0).round()) as i32;
    v.clamp(0, 16383) as u16
}

/// Pitch-bend message: [0xE0 | ch, value & 0x7F, value >> 7].
pub fn bend_msg(ch: u8, value: u16) -> Msg {
    [0xE0 | ch, value as u8 & 0x7F, (value >> 7) as u8]
}

/// RPN 0 (pitch-bend sensitivity) on `ch`: CC101=0, CC100=0, CC6=semis, CC38=0, then the
/// null RPN CC101=127, CC100=127.
pub fn rpn_bend_range(ch: u8, semis: u8) -> Vec<Msg> {
    vec![
        [0xB0 | ch, 101, 0], [0xB0 | ch, 100, 0], [0xB0 | ch, 6, semis], [0xB0 | ch, 38, 0],
        [0xB0 | ch, 101, 127], [0xB0 | ch, 100, 127],
    ]
}

/// MPE Configuration Message for a lower zone with `members` member channels, sent on
/// channel 0: CC101=0, CC100=6, CC6=members.
pub fn mcm_lower_zone(members: u8) -> Vec<Msg> {
    vec![[0xB0, 101, 0], [0xB0, 100, 6], [0xB0, 6, members]]
}

#[derive(Clone, Debug)]
pub struct MpeOut {
    voice_state: [Option<VoiceState>; 8],
}

#[derive(Clone, Debug)]
struct VoiceState {
    held_note: u8,
    last_bend: u16,
}

impl Default for MpeOut {
    fn default() -> Self {
        MpeOut::new()
    }
}

impl MpeOut {
    pub fn new() -> MpeOut {
        MpeOut {
            voice_state: [(); 8].map(|_| None),
        }
    }

    /// Channel for voice `v`: `1 + v`.
    pub fn channel(voice: usize) -> u8 {
        (1 + voice) as u8
    }

    /// Zone setup: `mcm_lower_zone(7)`, then `rpn_bend_range(ch, BEND_RANGE)` for channels
    /// 1..=7 in order.
    pub fn setup(&self) -> Vec<Msg> {
        let mut msgs = mcm_lower_zone(7);
        for v in 1..=7 {
            msgs.extend(rpn_bend_range(v, BEND_RANGE));
        }
        msgs
    }

    /// Start `voice` sounding `note`: note off (velocity 0) for the note it holds, if any;
    /// bend reset to centre (8192); note on.
    pub fn strike(&mut self, voice: usize, note: u8, vel: u8) -> Vec<Msg> {
        let ch = Self::channel(voice);
        let mut msgs = Vec::new();
        if let Some(vs) = &self.voice_state[voice] {
            msgs.push([0x80 | ch, vs.held_note, 0]);
        }
        msgs.push(bend_msg(ch, 8192));
        msgs.push([0x90 | ch, note, vel]);
        self.voice_state[voice] = Some(VoiceState { held_note: note, last_bend: 8192 });
        msgs
    }

    /// Move a sounding voice to fractional MIDI `pitch` by bending relative to its held
    /// note. `None` if the voice holds no note or the bend value is unchanged since the last
    /// one sent on its channel.
    pub fn pitch(&mut self, voice: usize, pitch: f32) -> Option<Msg> {
        let ch = Self::channel(voice);
        let vs = self.voice_state[voice].as_ref()?;
        let semis = pitch - vs.held_note as f32;
        let value = bend_value(semis, BEND_RANGE as f32);
        if value == vs.last_bend {
            return None;
        }
        self.voice_state[voice].as_mut().unwrap().last_bend = value;
        Some(bend_msg(ch, value))
    }

    /// Note off for the voice's held note (if any); it then holds nothing.
    pub fn release(&mut self, voice: usize) -> Vec<Msg> {
        let ch = Self::channel(voice);
        if let Some(vs) = self.voice_state[voice].take() {
            vec![[0x80 | ch, vs.held_note, 0]]
        } else {
            Vec::new()
        }
    }

    /// `release` for every voice, lowest first.
    pub fn all_off(&mut self) -> Vec<Msg> {
        let mut msgs = Vec::new();
        for v in 0..8 {
            if self.voice_state[v].is_some() {
                msgs.extend(self.release(v));
            }
        }
        msgs
    }
}

/// Plain-notes mode: whole chords on one channel, no glide.
#[derive(Clone, Debug)]
pub struct PlainOut {
    pub channel: u8,
    held: Vec<u8>,
}

impl PlainOut {
    pub fn new(channel: u8) -> PlainOut {
        PlainOut { channel, held: Vec::new() }
    }

    /// Move to a new chord: note offs (velocity 0) for held notes not in `notes`, ascending,
    /// then note ons for `notes` not already held, ascending. Notes held in both are left
    /// alone.
    pub fn chord(&mut self, notes: &[u8], vel: u8) -> Vec<Msg> {
        let mut msgs = Vec::new();
        let mut sorted_notes = notes.to_vec();
        sorted_notes.sort_unstable();
        let mut sorted_held = self.held.clone();
        sorted_held.sort_unstable();
        for n in &sorted_held {
            if !notes.contains(n) {
                msgs.push([0x80 | self.channel, *n, 0]);
            }
        }
        for n in &sorted_notes {
            if !sorted_held.contains(n) {
                msgs.push([0x90 | self.channel, *n, vel]);
            }
        }
        self.held = sorted_notes;
        msgs
    }

    /// Note offs for everything held, ascending.
    pub fn all_off(&mut self) -> Vec<Msg> {
        let mut notes = self.held.clone();
        notes.sort_unstable();
        self.held.clear();
        notes.into_iter().map(|n| [0x80 | self.channel, n, 0]).collect()
    }
}
