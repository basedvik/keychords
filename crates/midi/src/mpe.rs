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
    todo!()
}

/// Pitch-bend message: [0xE0 | ch, value & 0x7F, value >> 7].
pub fn bend_msg(ch: u8, value: u16) -> Msg {
    todo!()
}

/// RPN 0 (pitch-bend sensitivity) on `ch`: CC101=0, CC100=0, CC6=semis, CC38=0, then the
/// null RPN CC101=127, CC100=127.
pub fn rpn_bend_range(ch: u8, semis: u8) -> Vec<Msg> {
    todo!()
}

/// MPE Configuration Message for a lower zone with `members` member channels, sent on
/// channel 0: CC101=0, CC100=6, CC6=members.
pub fn mcm_lower_zone(members: u8) -> Vec<Msg> {
    todo!()
}

#[derive(Clone, Debug)]
pub struct MpeOut {
    // Private state is yours to design.
}

impl Default for MpeOut {
    fn default() -> Self {
        MpeOut::new()
    }
}

impl MpeOut {
    pub fn new() -> MpeOut {
        todo!()
    }

    /// Channel for voice `v`: `1 + v`.
    pub fn channel(voice: usize) -> u8 {
        todo!()
    }

    /// Zone setup: `mcm_lower_zone(7)`, then `rpn_bend_range(ch, BEND_RANGE)` for channels
    /// 1..=7 in order.
    pub fn setup(&self) -> Vec<Msg> {
        todo!()
    }

    /// Start `voice` sounding `note`: note off (velocity 0) for the note it holds, if any;
    /// bend reset to centre (8192); note on.
    pub fn strike(&mut self, voice: usize, note: u8, vel: u8) -> Vec<Msg> {
        todo!()
    }

    /// Move a sounding voice to fractional MIDI `pitch` by bending relative to its held
    /// note. `None` if the voice holds no note or the bend value is unchanged since the last
    /// one sent on its channel.
    pub fn pitch(&mut self, voice: usize, pitch: f32) -> Option<Msg> {
        todo!()
    }

    /// Note off for the voice's held note (if any); it then holds nothing.
    pub fn release(&mut self, voice: usize) -> Vec<Msg> {
        todo!()
    }

    /// `release` for every voice, lowest first.
    pub fn all_off(&mut self) -> Vec<Msg> {
        todo!()
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
        todo!()
    }

    /// Note offs for everything held, ascending.
    pub fn all_off(&mut self) -> Vec<Msg> {
        todo!()
    }
}
