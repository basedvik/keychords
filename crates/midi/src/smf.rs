//! Standard MIDI Files, format 0 and 1.
//!
//! Writing always produces format 1 with an `MThd` header (length 6, format 1, track count,
//! ticks-per-quarter `ppq`), then one `MTrk` chunk per track. Events are stored with absolute
//! tick times; the writer sorts each track by tick (stable, so same-tick events keep their
//! order), emits delta times as variable-length quantities, never uses running status, and
//! appends End of Track (FF 2F 00) unless the track already ends with one.
//! The parser accepts running status, format 0 and 1, and skips unknown meta and SysEx
//! events. Note On with velocity 0 parses as `NoteOff { vel: 0 }`.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    NoteOn { ch: u8, note: u8, vel: u8 },
    NoteOff { ch: u8, note: u8, vel: u8 },
    /// 14-bit value, 8192 = centre.
    PitchBend { ch: u8, value: u16 },
    Cc { ch: u8, cc: u8, val: u8 },
    /// Microseconds per quarter note (FF 51 03).
    Tempo(u32),
    /// FF 58 04: numerator, denominator as a power of two, clocks per click, 32nds per quarter.
    TimeSig { num: u8, den_pow2: u8, clocks: u8, n32: u8 },
    /// FF 06.
    Marker(String),
    /// FF 03.
    TrackName(String),
    /// FF 7F: sequencer-specific payload.
    SeqSpecific(Vec<u8>),
    /// FF 2F 00.
    EndOfTrack,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Track {
    /// (absolute tick, event)
    pub events: Vec<(u32, Event)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Smf {
    pub ppq: u16,
    pub tracks: Vec<Track>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SmfError {
    BadHeader,
    Truncated,
    BadEvent { track: usize, offset: usize },
}

/// Append `v` (≤ 0x0FFF_FFFF) as a MIDI variable-length quantity.
pub fn write_vlq(v: u32, out: &mut Vec<u8>) {
    todo!()
}

/// Read a VLQ from the start of `bytes`: (value, bytes consumed). `None` if truncated or
/// longer than 4 bytes.
pub fn read_vlq(bytes: &[u8]) -> Option<(u32, usize)> {
    todo!()
}

impl Smf {
    pub fn to_bytes(&self) -> Vec<u8> {
        todo!()
    }

    /// Parse; every returned track ends with `(tick, EndOfTrack)`.
    pub fn parse(bytes: &[u8]) -> Result<Smf, SmfError> {
        todo!()
    }
}
