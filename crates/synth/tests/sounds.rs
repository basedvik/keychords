//! Tests for the 17 patches added in T12, covering the spec's rules for every one of the
//! 23: bounded, NaN-free output; audibility with 7 voices; silence within 5 s of release;
//! in-tune sustained patches; distinct timbres; and allocation-free `render`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::ThreadId;

use synth::*;

const SR: f32 = 48_000.0;
const VOICE_PITCHES: [f32; MAX_VOICES] = [48.0, 60.0, 64.0, 67.0, 71.0, 74.0, 76.0];

/// The process allocator with a per-thread allocation counter, so `render` can be checked
/// for allocation-free behaviour without interference from parallel tests. Each thread
/// claims one slot, keyed by its thread id; the allocator never touches thread-locals
/// (which could re-enter it).
struct Slot {
    id: AtomicUsize, // 0 = unclaimed, else the claiming thread's id hash
    count: AtomicUsize,
}

impl Slot {
    const fn new() -> Slot {
        Slot { id: AtomicUsize::new(0), count: AtomicUsize::new(0) }
    }
}

static SLOTS: [Slot; 128] = [const { Slot::new() }; 128];

/// Fixed-seed polynomial hash: enough to spread thread ids across slots.
struct SimpleHash(u64);

impl Hasher for SimpleHash {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = self.0.wrapping_mul(31).wrapping_add(b as u64);
        }
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

fn tid_hash(id: &ThreadId) -> usize {
    let mut h = SimpleHash(0x9E37_79B9);
    id.hash(&mut h);
    h.finish() as usize | 1 // nonzero, so unclaimed slots stay distinguishable
}

/// This thread's allocation count; the slot is claimed on first allocation.
fn thread_allocs() -> usize {
    let h = tid_hash(&std::thread::current().id());
    let mut i = h & 127;
    for _ in 0..128 {
        let slot = &SLOTS[i];
        if slot.id.load(Ordering::Relaxed) == h {
            return slot.count.load(Ordering::Relaxed);
        }
        i = (i + 1) & 127;
    }
    panic!("no slot claimed for this thread")
}

struct Counting;

#[global_allocator]
static ALLOCATOR: Counting = Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let h = tid_hash(&std::thread::current().id());
        let mut i = h & 127;
        for _ in 0..128 {
            let slot = &SLOTS[i];
            let cur = slot.id.load(Ordering::Relaxed);
            if cur == h {
                slot.count.fetch_add(1, Ordering::Relaxed);
                break;
            }
            if cur == 0 && slot.id.compare_exchange(0, h, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
                slot.count.fetch_add(1, Ordering::Relaxed);
                break;
            }
            i = (i + 1) & 127;
        }
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}

fn run(s: &mut Synth, secs: f32) -> (Vec<f32>, Vec<f32>) {
    let n = (SR * secs) as usize;
    let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
    for (lc, rc) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
        s.render(lc, rc);
    }
    (l, r)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

/// Lag (in samples) of the strongest normalised autocorrelation peak in 20..=600, as in the
/// acceptance tests.
fn period(x: &[f32]) -> usize {
    let best = (20..=600usize).map(|lag| {
        let (mut num, mut den) = (0.0f64, 1e-12f64);
        for i in 0..x.len() - lag {
            num += (x[i] * x[i + lag]) as f64;
            den += (x[i] * x[i]) as f64;
        }
        (lag, num / den)
    });
    let all: Vec<_> = best.collect();
    let top = all.iter().map(|&(_, c)| c).fold(f64::MIN, f64::max);
    (1..all.len() - 1)
        .find(|&i| all[i].1 >= top * 0.97 && all[i].1 >= all[i - 1].1 && all[i].1 >= all[i + 1].1)
        .map(|i| all[i].0)
        .unwrap()
}

fn sound_all_voices(s: &mut Synth) {
    for (v, p) in VOICE_PITCHES.iter().enumerate() {
        s.set_voice(v, Some(*p));
    }
}

/// All 23 patches, in spec order: the original six unchanged, then the 17 new ones with
/// their spec labels.
#[test]
fn added_patches_follow_the_original_six_in_order() {
    assert_eq!(Patch::ALL.len(), 23);
    assert_eq!(
        Patch::ALL[..6],
        [Patch::GlidePad, Patch::Saw, Patch::EPiano, Patch::Organ, Patch::Pluck, Patch::Bells]
    );
    assert_eq!(
        Patch::ALL[6..],
        [
            Patch::Koto,
            Patch::MusicBox,
            Patch::Banjo,
            Patch::SaxSection,
            Patch::ClarinetChoir,
            Patch::PizzicatoStrings,
            Patch::TwelveString,
            Patch::Vibraphone,
            Patch::FeltPiano,
            Patch::Handpan,
            Patch::Kalimba,
            Patch::Harp,
            Patch::Choir,
            Patch::StringSection,
            Patch::FrenchHorns,
            Patch::Theremin,
            Patch::NeoSoulGuitar,
        ]
    );
    assert_eq!(
        Patch::ALL.iter().map(|p| p.label()).collect::<Vec<_>>(),
        [
            "Pad", "Saw", "E-piano", "Organ", "Pluck", "Bells", "Koto", "Music box", "Banjo",
            "Sax section", "Clarinet choir", "Pizzicato strings", "12-string guitar",
            "Vibraphone", "Felt piano", "Handpan", "Kalimba", "Harp", "Choir",
            "String section", "French horns", "Theremin", "Neo-soul guitar",
        ]
    );
    for (i, p) in Patch::ALL.iter().enumerate() {
        assert!(Patch::ALL[..i].iter().all(|q| q != p), "duplicate {p:?} in ALL");
    }
}

/// Spec rule 1, for every patch: |x| <= 1, never NaN, audible with all 7 voices sounding,
/// and silent within 5 s after all voices release.
#[test]
fn every_patch_is_bounded_audible_and_releases() {
    for patch in Patch::ALL {
        let mut s = Synth::new(SR);
        s.set_patch(patch);
        sound_all_voices(&mut s);
        let (l, r) = run(&mut s, 0.5);
        assert!(
            l.iter().chain(r.iter()).all(|v| v.is_finite() && v.abs() <= 1.0),
            "{patch:?}: output bounded and finite"
        );
        assert!(rms(&l[2400..]) > 1e-3 && rms(&r[2400..]) > 1e-3, "{patch:?}: audible");
        for v in 0..MAX_VOICES {
            s.set_voice(v, None);
        }
        let _ = run(&mut s, 5.0);
        let (l, _) = run(&mut s, 0.1);
        assert!(rms(&l) < 1e-4, "{patch:?}: silent 5 s after release");
    }
}

/// Worst case for rule 1: release a single voice from near-peak envelope; it must still be
/// silent within 5 s.
#[test]
fn release_from_peak_envelope_is_silent_within_five_seconds() {
    for patch in Patch::ALL {
        let mut s = Synth::new(SR);
        s.set_patch(patch);
        s.set_voice(0, Some(60.0));
        let _ = run(&mut s, 0.1);
        s.set_voice(0, None);
        let _ = run(&mut s, 5.0);
        let (l, _) = run(&mut s, 0.05);
        assert!(
            l.iter().all(|v| v.is_finite() && v.abs() <= 1.0),
            "{patch:?}: bounded after release"
        );
        assert!(rms(&l) < 1e-4, "{patch:?}: not silent 5 s after peak release");
    }
}

/// Even at +12 dB the soft clip keeps every patch within |x| <= 1 and finite.
#[test]
fn boost_does_not_exceed_bounds() {
    for patch in Patch::ALL {
        let mut s = Synth::new(SR);
        s.set_volume_db(12.0);
        s.set_patch(patch);
        sound_all_voices(&mut s);
        let (l, r) = run(&mut s, 0.2);
        assert!(
            l.iter().chain(r.iter()).all(|v| v.is_finite() && v.abs() <= 1.0),
            "{patch:?}: bounded at +12 dB"
        );
    }
}

/// Spec rule 2: the sustained (blown/bowed/theremin) patches track their fundamental.
/// At 48 kHz, A4 is ~109 samples/period and A5 ~55.
#[test]
fn sustained_patches_play_in_tune() {
    for patch in [
        Patch::SaxSection,
        Patch::ClarinetChoir,
        Patch::StringSection,
        Patch::Choir,
        Patch::FrenchHorns,
        Patch::Theremin,
    ] {
        let mut s = Synth::new(SR);
        s.set_patch(patch);
        s.set_voice(0, Some(69.0));
        let (l, _) = run(&mut s, 0.6);
        let lag = period(&l[9600..19200]);
        assert!((lag as i32 - 109).abs() <= 1, "{patch:?}: A4 period {lag}, want ~109");
        s.set_voice(0, Some(81.0));
        let (l, _) = run(&mut s, 0.3);
        let lag = period(&l[4800..14400]);
        assert!((lag as i32 - 55).abs() <= 1, "{patch:?}: A5 period {lag}, want ~55");
    }
}

/// Spec rule 3: `render` performs no heap allocations, including the Karplus–Strong
/// re-prime on a fresh strike. The counter is per-thread, so parallel tests cannot
/// interfere; a tiny tolerance allows for one-off std lazy state on this thread.
#[test]
fn render_performs_no_allocations() {
    let mut l = vec![0.0f32; 256];
    let mut r = vec![0.0f32; 256];
    for patch in Patch::ALL {
        let mut s = Synth::new(SR);
        s.set_patch(patch);
        s.set_voice(0, Some(60.0));
        for _ in 0..2 {
            s.render(&mut l, &mut r);
        }
        let before = thread_allocs();
        s.strike(0);
        s.set_voice(1, Some(64.0));
        for _ in 0..8192 {
            s.render(&mut l, &mut r);
        }
        let after = thread_allocs();
        let delta = after.saturating_sub(before);
        assert!(
            delta <= 4,
            "{patch:?}: render allocated {delta} times over 8192 blocks"
        );
    }
}

/// The Karplus–Strong plucks re-strike on `strike`, like the existing `Pluck`.
#[test]
fn ks_plucks_restrike() {
    for patch in [Patch::Koto, Patch::Banjo] {
        let mut s = Synth::new(SR);
        s.set_patch(patch);
        s.set_voice(0, Some(60.0));
        let _ = run(&mut s, 2.0);
        let (before, _) = run(&mut s, 0.05);
        s.strike(0);
        let (after, _) = run(&mut s, 0.05);
        assert!(
            rms(&after) > 4.0 * rms(&before),
            "{patch:?}: strike re-primed the pluck"
        );
    }
}

/// The 17 new patches are not copies of each other: every pair renders differently.
#[test]
fn new_patches_are_audibly_distinct() {
    let new: [Patch; 17] = [
        Patch::Koto,
        Patch::MusicBox,
        Patch::Banjo,
        Patch::SaxSection,
        Patch::ClarinetChoir,
        Patch::PizzicatoStrings,
        Patch::TwelveString,
        Patch::Vibraphone,
        Patch::FeltPiano,
        Patch::Handpan,
        Patch::Kalimba,
        Patch::Harp,
        Patch::Choir,
        Patch::StringSection,
        Patch::FrenchHorns,
        Patch::Theremin,
        Patch::NeoSoulGuitar,
    ];
    for (i, pa) in new.iter().enumerate() {
        let mut s = Synth::new(SR);
        s.set_patch(*pa);
        s.set_voice(0, Some(60.0));
        let (l, _) = run(&mut s, 0.2);
        for pb in &new[i + 1..] {
            let mut s = Synth::new(SR);
            s.set_patch(*pb);
            s.set_voice(0, Some(60.0));
            let (l2, _) = run(&mut s, 0.2);
            let diff = l
                .iter()
                .zip(&l2)
                .map(|(x, y)| (x - y).abs())
                .max_by(f32::total_cmp)
                .unwrap();
            assert!(diff > 1e-4, "{pa:?} and {pb:?} render identically");
        }
    }
}
