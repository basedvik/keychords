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
    let v = v.min(0x0FFF_FFFF);
    let mut n = 1;
    let mut x = v;
    while x >= 0x80 {
        x >>= 7;
        n += 1;
    }
    let mut bytes = [0u8; 4];
    for i in (0..n).rev() {
        let mut b = ((v >> (7 * i)) & 0x7F) as u8;
        if i > 0 {
            b |= 0x80;
        }
        bytes[i] = b;
    }
    for i in (0..n).rev() {
        out.push(bytes[i]);
    }
}

/// Read a VLQ from the start of `bytes`: (value, bytes consumed). `None` if truncated or
/// longer than 4 bytes.
pub fn read_vlq(bytes: &[u8]) -> Option<(u32, usize)> {
    let mut value = 0u32;
    for (i, &b) in bytes.iter().enumerate() {
        if i >= 4 {
            return None;
        }
        value = (value << 7) | (b & 0x7F) as u32;
        if b & 0x80 == 0 {
            return Some((value, i + 1));
        }
    }
    None
}

impl Smf {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"MThd");
        out.extend_from_slice(&6u32.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&(self.tracks.len() as u16).to_be_bytes());
        out.extend_from_slice(&self.ppq.to_be_bytes());
        for track in &self.tracks {
            let mut events = track.events.clone();
            events.sort_by_key(|&(tick, _)| tick);
            if !events.last().is_some_and(|(_, e)| matches!(e, Event::EndOfTrack)) {
                let last_tick = events.last().map_or(0, |(t, _)| *t);
                events.push((last_tick, Event::EndOfTrack));
            }
            let mut body = Vec::new();
            let mut prev_tick: u32 = 0;
            for (tick, event) in &events {
                write_vlq(tick - prev_tick, &mut body);
                write_event(event, &mut body);
                prev_tick = *tick;
            }
            out.extend_from_slice(b"MTrk");
            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            out.extend_from_slice(&body);
        }
        out
    }

    /// Parse; every returned track ends with `(tick, EndOfTrack)`.
    pub fn parse(bytes: &[u8]) -> Result<Smf, SmfError> {
        if bytes.len() < 14 || &bytes[0..4] != b"MThd" {
            return Err(SmfError::BadHeader);
        }
        let len = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if len != 6 || bytes.len() < 14 {
            return Err(SmfError::BadHeader);
        }
        let _format = u16::from_be_bytes([bytes[8], bytes[9]]);
        let ntracks = u16::from_be_bytes([bytes[10], bytes[11]]);
        let ppq = u16::from_be_bytes([bytes[12], bytes[13]]);
        let mut pos = 14usize;
        let mut tracks = Vec::new();
        for t in 0..ntracks as usize {
            if pos + 8 > bytes.len() || &bytes[pos..pos + 4] != b"MTrk" {
                return Err(SmfError::BadHeader);
            }
            let len = u32::from_be_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]]) as usize;
            pos += 8;
            if pos + len > bytes.len() {
                return Err(SmfError::Truncated);
            }
            let track_bytes = &bytes[pos..pos + len];
            pos += len;
            tracks.push(parse_track(track_bytes, t)?);
        }
        Ok(Smf { ppq, tracks })
    }
}

fn parse_track(bytes: &[u8], track: usize) -> Result<Track, SmfError> {
    let mut events: Vec<(u32, Event)> = Vec::new();
    let mut tick: u32 = 0;
    let mut running: Option<u8> = None;
    let mut i = 0usize;
    while i < bytes.len() {
        let offset = i;
        let (delta, n) = read_vlq(&bytes[i..]).ok_or(SmfError::Truncated)?;
        tick = tick.saturating_add(delta);
        i += n;
        if i >= bytes.len() {
            return Err(SmfError::Truncated);
        }
        let status_byte = bytes[i];
        if status_byte >= 0x80 {
            if status_byte & 0xF0 == 0xC0 || status_byte & 0xF0 == 0xD0 {
                running = Some(status_byte);
                i += 1;
                if i >= bytes.len() {
                    return Err(SmfError::Truncated);
                }
                let d = bytes[i];
                i += 1;
                events.push((tick, event_for(status_byte, d, d)));
            } else if status_byte & 0xF0 == 0xA0 {
                running = Some(status_byte);
                i += 1;
                if i + 1 >= bytes.len() {
                    return Err(SmfError::Truncated);
                }
                let (a, b) = (bytes[i], bytes[i + 1]);
                i += 2;
                events.push((tick, event_for(status_byte, a, b)));
            } else if status_byte & 0xF0 == 0xF0 {
                running = None;
                i += 1;
                if status_byte == 0xFF {
                    if i >= bytes.len() {
                        return Err(SmfError::Truncated);
                    }
                    let meta_type = bytes[i];
                    i += 1;
                    let (len, n) = read_vlq(&bytes[i..]).ok_or(SmfError::Truncated)?;
                    i += n;
                    if i + len as usize > bytes.len() {
                        return Err(SmfError::Truncated);
                    }
                    let data = &bytes[i..i + len as usize];
                    match (meta_type, data) {
                        (0x51, [a, b, c]) => events.push((tick, Event::Tempo(((u32::from(*a)) << 16) | ((u32::from(*b)) << 8) | u32::from(*c)))),
                        (0x58, [a, b, c, d]) => events.push((
                            tick,
                            Event::TimeSig { num: *a, den_pow2: *b, clocks: *c, n32: *d },
                        )),
                        (0x06, d) | (0x03, d) => {
                            let s = String::from_utf8_lossy(d).into_owned();
                            events.push((tick, if meta_type == 0x06 { Event::Marker(s) } else { Event::TrackName(s) }));
                        }
                        (0x7F, d) => events.push((tick, Event::SeqSpecific(d.to_vec()))),
                        (0x2F, []) => events.push((tick, Event::EndOfTrack)),
                        _ => {}
                    }
                    i += len as usize;
                } else {
                    let (len, n) = read_vlq(&bytes[i..]).ok_or(SmfError::Truncated)?;
                    i += n;
                    if i + len as usize > bytes.len() {
                        return Err(SmfError::Truncated);
                    }
                    i += len as usize;
                }
            } else {
                running = Some(status_byte);
                match status_byte & 0xF0 {
                    0x80 | 0x90 | 0xB0 | 0xE0 => {
                        i += 1;
                        if i + 1 >= bytes.len() {
                            return Err(SmfError::Truncated);
                        }
                        let (a, b) = (bytes[i], bytes[i + 1]);
                        i += 2;
                        events.push((tick, event_for(status_byte, a, b)));
                    }
                    _ => {
                        i += 1;
                        if i >= bytes.len() {
                            return Err(SmfError::Truncated);
                        }
                        let d = bytes[i];
                        i += 1;
                        events.push((tick, event_for(status_byte, d, d)));
                    }
                }
            }
        } else {
            // Running status: reuse the previous channel status.
            let Some(status) = running else {
                return Err(SmfError::BadEvent { track, offset });
            };
            match status & 0xF0 {
                0xA0 => {
                    if i + 1 >= bytes.len() {
                        return Err(SmfError::Truncated);
                    }
                    let (a, b) = (bytes[i], bytes[i + 1]);
                    i += 2;
                    events.push((tick, event_for(status, a, b)));
                }
                0xC0 | 0xD0 => {
                    if i >= bytes.len() {
                        return Err(SmfError::Truncated);
                    }
                    let d = bytes[i];
                    i += 1;
                    events.push((tick, event_for(status, d, d)));
                }
                _ => {
                    if i + 1 >= bytes.len() {
                        return Err(SmfError::Truncated);
                    }
                    let (a, b) = (bytes[i], bytes[i + 1]);
                    i += 2;
                    events.push((tick, event_for(status, a, b)));
                }
            }
        }
    }
    Ok(Track { events })
}

fn event_for(status: u8, a: u8, b: u8) -> Event {
    let ch = status & 0x0F;
    match status & 0xF0 {
        0x90 => {
            if b == 0 {
                Event::NoteOff { ch, note: a, vel: 0 }
            } else {
                Event::NoteOn { ch, note: a, vel: b }
            }
        }
        0x80 => Event::NoteOff { ch, note: a, vel: b },
        0xB0 => Event::Cc { ch, cc: a, val: b },
        0xE0 => Event::PitchBend { ch, value: ((a as u16) << 7) | b as u16 },
        _ => Event::EndOfTrack,
    }
}

fn write_event(event: &Event, out: &mut Vec<u8>) {
    match event {
        Event::NoteOn { ch, note, vel } => {
            out.push(0x90 | ch);
            out.push(*note);
            out.push(*vel);
        }
        Event::NoteOff { ch, note, vel } => {
            out.push(0x80 | ch);
            out.push(*note);
            out.push(*vel);
        }
        Event::PitchBend { ch, value } => {
            out.push(0xE0 | ch);
            out.push(((value >> 7) & 0x7F) as u8);
            out.push((value & 0x7F) as u8);
        }
        Event::Cc { ch, cc, val } => {
            out.push(0xB0 | ch);
            out.push(*cc);
            out.push(*val);
        }
        Event::Tempo(us) => {
            out.push(0xFF);
            out.push(0x51);
            out.push(0x03);
            out.push((us >> 16) as u8);
            out.push((us >> 8) as u8);
            out.push(*us as u8);
        }
        Event::TimeSig { num, den_pow2, clocks, n32 } => {
            out.push(0xFF);
            out.push(0x58);
            out.push(0x04);
            out.push(*num);
            out.push(*den_pow2);
            out.push(*clocks);
            out.push(*n32);
        }
        Event::Marker(s) | Event::TrackName(s) => {
            out.push(0xFF);
            out.push(if matches!(event, Event::Marker(_)) { 0x06 } else { 0x03 });
            let bytes = s.as_bytes();
            write_vlq(bytes.len() as u32, out);
            out.extend_from_slice(bytes);
        }
        Event::SeqSpecific(data) => {
            out.push(0xFF);
            out.push(0x7F);
            write_vlq(data.len() as u32, out);
            out.extend_from_slice(data);
        }
        Event::EndOfTrack => {
            out.push(0xFF);
            out.push(0x2F);
            out.push(0x00);
        }
    }
}
