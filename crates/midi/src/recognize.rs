//! Chord recognition over an SMF: split the notes into half-bar windows and match each
//! window's pitch-class histogram against every quality in `chords`.

use chords::{Chord, Quality};
use std::collections::HashMap;

use crate::smf::{Event, Smf};

/// Chords at each half bar (4/4: every 2 beats, using smf.ppq), up to 32 bars (64 windows),
/// None where nothing sounds or nothing matches.
pub fn recognize(smf: &Smf) -> Vec<Option<Chord>> {
    let notes = collect_notes(smf);
    if notes.is_empty() {
        return Vec::new();
    }
    let window = (2 * u32::from(smf.ppq)).max(1);
    let last_end = notes.iter().map(|n| n.end).max().unwrap();
    let count = last_end.div_ceil(window).min(64) as usize;
    (0..count)
        .map(|w| {
            let (a, b) = (window * w as u32, window * (w as u32 + 1));
            let (lowest, weights) = window_stats(&notes, a, b)?;
            best_chord(&weights, Some(lowest))
        })
        .collect()
}

/// Best chord for a duration-weighted pitch-class histogram and the lowest pitch class sounding.
///
/// The template is the quality's distinct pitch classes (duplicates, e.g. the 14th on a
/// 2nd, count once). Score: sum of weights on template pcs, minus 0.5 times the sum off
/// template, minus 0.1 times the number of template pcs with zero weight times the max
/// weight; plus 0.25 times the max weight when the root is the lowest sounding pitch
/// class. Highest score wins; ties go to the quality with fewer notes, then the lower
/// root. `None` when all weights are zero.
pub fn best_chord(weights: &[f32; 12], lowest: Option<u8>) -> Option<Chord> {
    let max = weights.iter().cloned().fold(0.0f32, f32::max);
    if max == 0.0 {
        return None;
    }
    let off_total: f32 = weights.iter().sum();
    let mut best: Option<(f32, Chord, usize, u8)> = None;
    for root in 0..12u8 {
        for quality in Quality::ALL {
            let mut pcs: [bool; 12] = [false; 12];
            for &iv in quality.intervals() {
                pcs[(root + iv % 12) as usize % 12] = true;
            }
            let mut on = 0.0f32;
            let mut missing = 0usize;
            for pc in 0..12 {
                if pcs[pc] {
                    on += weights[pc];
                    if weights[pc] == 0.0 {
                        missing += 1;
                    }
                }
            }
            let off = off_total - on;
            let mut score = on - 0.5 * off - 0.1 * missing as f32 * max;
            if lowest.is_some_and(|l| l == root) {
                score += 0.25 * max;
            }
            let nnotes = pcs.iter().filter(|&&p| p).count();
            let better = match best {
                None => true,
                Some((bs, _, bnotes, broot)) => {
                    if (score - bs).abs() > 1e-6 {
                        score > bs
                    } else if nnotes != bnotes {
                        nnotes < bnotes
                    } else {
                        root < broot
                    }
                }
            };
            if better {
                best = Some((score, Chord::new(root, quality), nnotes, root));
            }
        }
    }
    best.map(|(_, c, _, _)| c)
}

struct Note {
    note: u8,
    start: u32,
    end: u32,
}

/// All non-drum notes as (note, start tick, end tick). NoteOn pairs with the next NoteOff
/// (or NoteOn velocity 0) of the same channel and note; a note still held at the end ends
/// at the last event.
fn collect_notes(smf: &Smf) -> Vec<Note> {
    let mut events: Vec<(u32, Event)> = smf
        .tracks
        .iter()
        .flat_map(|t| t.events.iter().cloned())
        .filter(|(_, e)| !matches!(e, Event::NoteOn { ch: 9, .. } | Event::NoteOff { ch: 9, .. }))
        .collect();
    events.sort_by_key(|&(tick, _)| tick);
    let last_tick = events.iter().map(|&(t, _)| t).max().unwrap_or(0);
    let mut held: HashMap<(u8, u8), u32> = HashMap::new();
    let mut out = Vec::new();
    for (tick, ev) in &events {
        let key = match ev {
            Event::NoteOn { ch, note, vel: 0 } | Event::NoteOff { ch, note, .. } => {
                held.remove(&(*ch, *note)).map(|start| Note {
                    note: *note,
                    start,
                    end: *tick,
                })
            }
            Event::NoteOn { ch, note, vel } if *vel > 0 => {
                held.insert((*ch, *note), *tick);
                None
            }
            _ => None,
        };
        if let Some(n) = key {
            out.push(n);
        }
    }
    for ((ch, note), start) in held {
        let _ = ch;
        out.push(Note { note, start, end: last_tick });
    }
    out
}

/// (lowest pitch class sounding, duration weights per pitch class) in `[a, b)`; None if
/// nothing sounds.
fn window_stats(notes: &[Note], a: u32, b: u32) -> Option<(u8, [f32; 12])> {
    let mut weights = [0.0f32; 12];
    let mut lowest: Option<u8> = None;
    for n in notes {
        let ov = n.end.saturating_sub(a).min(b.saturating_sub(n.start));
        if ov == 0 {
            continue;
        }
        weights[(n.note % 12) as usize] += ov as f32;
        lowest = Some(lowest.map_or(n.note, |l| l.min(n.note)) % 12);
    }
    lowest.map(|l| (l, weights))
}
