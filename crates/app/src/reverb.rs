//! A stereo Freeverb-style reverb for the master bus: 8 damped combs and 4 all-passes per
//! channel, sized for the sample rate. All buffers are allocated in `new`; `process` never
//! allocates.

const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASSES: [usize; 4] = [556, 441, 341, 225];
const SPREAD: usize = 23;
const FIXED_GAIN: f32 = 0.015;

struct Comb {
    buf: Vec<f32>,
    pos: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Comb {
        Comb { buf: vec![0.0; len.max(1)], pos: 0, store: 0.0 }
    }

    fn tick(&mut self, x: f32, feedback: f32, damp: f32) -> f32 {
        let out = self.buf[self.pos];
        self.store = out * (1.0 - damp) + self.store * damp;
        self.buf[self.pos] = x + self.store * feedback;
        self.pos = (self.pos + 1) % self.buf.len();
        out
    }
}

struct Allpass {
    buf: Vec<f32>,
    pos: usize,
}

impl Allpass {
    fn new(len: usize) -> Allpass {
        Allpass { buf: vec![0.0; len.max(1)], pos: 0 }
    }

    fn tick(&mut self, x: f32) -> f32 {
        let delayed = self.buf[self.pos];
        self.buf[self.pos] = x + delayed * 0.5;
        self.pos = (self.pos + 1) % self.buf.len();
        delayed - x
    }
}

pub struct Reverb {
    combs: [Vec<Comb>; 2],
    allpasses: [Vec<Allpass>; 2],
    /// 0..=1 wet amount.
    pub mix: f32,
    feedback: f32,
    damp: f32,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Reverb {
        let scale = |n: usize| (n as f32 * sample_rate / 44_100.0) as usize;
        let side = |extra: usize| {
            (
                COMBS.iter().map(|&n| Comb::new(scale(n + extra))).collect::<Vec<_>>(),
                ALLPASSES.iter().map(|&n| Allpass::new(scale(n + extra))).collect::<Vec<_>>(),
            )
        };
        let (cl, al) = side(0);
        let (cr, ar) = side(SPREAD);
        // Room size 0.85, damping 0.4: a long, soft space that suits pads.
        Reverb { combs: [cl, cr], allpasses: [al, ar], mix: 0.3, feedback: 0.28 * 0.85 + 0.7, damp: 0.4 * 0.4 }
    }

    /// Mix the reverb into both buffers in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        if self.mix <= 0.0 {
            return;
        }
        let (fb, damp, mix) = (self.feedback, self.damp, self.mix);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let input = (*l + *r) * FIXED_GAIN;
            let mut wet = [0.0f32; 2];
            for (ch, w) in wet.iter_mut().enumerate() {
                let mut acc: f32 = self.combs[ch].iter_mut().map(|c| c.tick(input, fb, damp)).sum();
                for a in self.allpasses[ch].iter_mut() {
                    acc = a.tick(acc);
                }
                *w = acc;
            }
            *l = *l * (1.0 - mix * 0.5) + wet[0] * mix * 3.0;
            *r = *r * (1.0 - mix * 0.5) + wet[1] * mix * 3.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_rings_then_decays_and_stays_bounded() {
        let mut rv = Reverb::new(48_000.0);
        rv.mix = 1.0;
        let n = 48_000 * 6;
        let (mut l, mut r) = (vec![0.0f32; n], vec![0.0f32; n]);
        l[0] = 1.0;
        r[0] = 1.0;
        rv.process(&mut l, &mut r);
        let energy = |x: &[f32]| x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32;
        assert!(energy(&l[4_800..24_000]) > 1e-8, "it rings");
        assert!(energy(&l[n - 4_800..]) < energy(&l[4_800..24_000]) * 1e-2, "and decays");
        assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 2.0));
        assert_ne!(l[10_000], r[10_000], "left and right differ (stereo spread)");
    }

    #[test]
    fn zero_mix_is_bypass() {
        let mut rv = Reverb::new(48_000.0);
        rv.mix = 0.0;
        let (mut l, mut r) = (vec![0.5f32; 64], vec![-0.5f32; 64]);
        rv.process(&mut l, &mut r);
        assert!(l.iter().all(|&v| v == 0.5) && r.iter().all(|&v| v == -0.5));
    }
}
