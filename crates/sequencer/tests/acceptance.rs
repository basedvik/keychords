use chords::{Chord, Quality::*};
use sequencer::*;

fn c(root: u8, q: chords::Quality) -> Slot { Slot::Chord(Chord::new(root, q)) }

#[test]
fn new_and_sizes() {
    let lp = Loop::new(4, 1);
    assert_eq!(lp.slots, vec![Slot::Empty; 4]);
    assert_eq!(lp.slot_beats(), 4.0);
    assert_eq!(lp.len_beats(), 16.0);
    let half = Loop::new(8, 2);
    assert_eq!(half.slots.len(), 16);
    assert_eq!(half.slot_beats(), 2.0);
    assert_eq!(half.len_beats(), 32.0);
}

#[test]
#[should_panic]
fn rejects_bad_bar_count() {
    let _ = Loop::new(5, 1);
}

#[test]
#[should_panic]
fn rejects_bad_per_bar() {
    let _ = Loop::new(4, 3);
}

#[test]
fn resize_keeps_slots() {
    let mut lp = Loop::new(4, 1);
    lp.slots[0] = c(2, Min9);
    lp.set_bars(8);
    assert_eq!(lp.slots.len(), 8);
    assert_eq!(lp.slots[0], c(2, Min9));
    assert_eq!(lp.slots[7], Slot::Empty);
    lp.set_bars(4);
    assert_eq!(lp.slots.len(), 4);
}

#[test]
fn per_bar_split_and_merge() {
    let mut lp = Loop::new(4, 1);
    lp.slots = vec![c(2, Min9), c(7, Dom13), Slot::Silent, Slot::Empty];
    lp.set_per_bar(2);
    assert_eq!(lp.slots, vec![c(2, Min9), c(2, Min9), c(7, Dom13), c(7, Dom13),
        Slot::Silent, Slot::Silent, Slot::Empty, Slot::Empty]);
    lp.slots[1] = c(5, Maj7);
    lp.set_per_bar(1);
    assert_eq!(lp.slots, vec![c(2, Min9), c(7, Dom13), Slot::Silent, Slot::Empty]);
}

#[test]
fn resolution_inherits_and_wraps() {
    let mut lp = Loop::new(4, 1);
    assert_eq!(lp.resolved(2), None, "nothing written");
    lp.slots = vec![Slot::Empty, c(7, Dom13), Slot::Empty, Slot::Silent];
    assert_eq!(lp.resolved(0), None, "inherits the silent last slot");
    assert_eq!(lp.resolved(1), Some(Chord::new(7, Dom13)));
    assert_eq!(lp.resolved(2), Some(Chord::new(7, Dom13)));
    assert_eq!(lp.resolved(3), None);
    lp.slots[3] = Slot::Empty;
    assert_eq!(lp.resolved(0), Some(Chord::new(7, Dom13)), "wraps to the last written chord");
}

#[test]
fn changes_are_cyclic() {
    let mut lp = Loop::new(4, 1);
    lp.slots = vec![c(2, Min9), c(7, Dom13), c(0, Maj9), c(9, Dom7b9)];
    assert_eq!(lp.changes(), vec![0, 1, 2, 3]);
    lp.slots = vec![c(2, Min9), Slot::Empty, c(0, Maj9), Slot::Empty];
    assert_eq!(lp.changes(), vec![0, 2]);
    lp.slots = vec![c(2, Min9), Slot::Empty, Slot::Empty, Slot::Empty];
    assert!(lp.changes().is_empty());
    lp.slots = vec![c(2, Min9), Slot::Silent, Slot::Empty, Slot::Empty];
    assert_eq!(lp.changes(), vec![0, 1]);
}

#[test]
fn clock_math() {
    let clk = Clock { bpm: 120.0, sample_rate: 48_000.0 };
    assert_eq!(clk.samples_per_beat(), 24_000.0);
    assert_eq!(clk.beat_to_sample(4.0), 96_000);
    let odd = Clock { bpm: 90.0, sample_rate: 44_100.0 };
    assert_eq!(odd.beat_to_sample(1.0), 29_400);
    assert_eq!(odd.beat_to_sample(1.0 / 3.0), 9_800);
}

#[test]
fn boundaries_repeat_cyclically() {
    let lp = Loop::new(4, 1); // 16 beats, slot every 4 beats
    let clk = Clock { bpm: 120.0, sample_rate: 1000.0 }; // 500 samples per beat, 2000 per slot
    assert_eq!(boundaries(&lp, &clk, 0, 0, 1), vec![(0, 0)]);
    assert_eq!(boundaries(&lp, &clk, 0, 1, 2000), vec![]);
    assert_eq!(boundaries(&lp, &clk, 0, 1, 2001), vec![(2000, 1)]);
    assert_eq!(boundaries(&lp, &clk, 100, 7000, 10200), vec![(8100, 0), (10100, 1)]);
    assert_eq!(boundaries(&lp, &clk, 5000, 0, 5001), vec![(5000, 0)], "nothing before origin");
}
