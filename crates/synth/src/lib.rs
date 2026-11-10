//! Built-in sound engine: up to 7 voices, one patch at a time.
//!
//! The caller drives each voice's pitch (fractional MIDI) every control block; the synth
//! renders stereo audio. No allocation in `render`.

use std::f32::consts::TAU;

pub const MAX_VOICES: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Patch {
    /// Two detuned saws through a soft low-pass, slow attack.
    GlidePad,
    /// One band-limited (polyBLEP) sawtooth.
    Saw,
    /// Sine plus a decaying second harmonic, piano-like envelope.
    EPiano,
    /// Sum of sines at 1×, 2× and 3× the fundamental, organ envelope (instant on/off).
    Organ,
    /// Bright decaying tone (Karplus–Strong or filtered decaying saw).
    Pluck,
    /// Two-operator FM, ratio 1:3.5, long decay.
    Bells,
}

impl Patch {
    pub const ALL: [Patch; 6] =
        [Patch::GlidePad, Patch::Saw, Patch::EPiano, Patch::Organ, Patch::Pluck, Patch::Bells];

    /// Key-map label: "Pad", "Saw", "E-piano", "Organ", "Pluck", "Bells".
    pub fn label(self) -> &'static str {
        match self {
            Patch::GlidePad => "Pad",
            Patch::Saw => "Saw",
            Patch::EPiano => "E-piano",
            Patch::Organ => "Organ",
            Patch::Pluck => "Pluck",
            Patch::Bells => "Bells",
        }
    }

    /// Envelope parameters per patch.
    fn env_params(self) -> (f32, f32, f32, f32, bool) {
        match self {
            Patch::GlidePad => (0.25, 0.3, 0.8, 0.6, false),
            Patch::Saw => (0.005, 0.1, 0.9, 0.15, false),
            Patch::EPiano => (0.003, 1.2, 0.0, 0.3, true),
            Patch::Organ => (0.002, 0.01, 1.0, 0.03, false),
            Patch::Pluck => (0.002, 0.6, 0.0, 0.2, true),
            Patch::Bells => (0.002, 2.0, 0.0, 0.8, true),
        }
    }
}

/// 440 · 2^((p − 69) / 12).
pub fn midi_to_hz(p: f32) -> f32 {
    440.0 * 2.0_f32.powf((p - 69.0) / 12.0)
}

const VOICE_GAIN: f32 = 0.18;

#[derive(Clone, Copy, Debug, Default)]
struct Voice {
    pitch: f32,       // current, smoothed
    target: f32,
    on: bool,
    env: f32,         // 0..1
    stage: u8,        // 0=idle, 1=attack, 2=decay, 3=sustain, 4=release
    phase: [f32; 3],  // oscillator phases 0..1
    lp: f32,          // one-pole filter state
}

pub struct Synth {
    sample_rate: f32,
    voices: [Voice; MAX_VOICES],
    patch: Patch,
    gain: f32,          // linear gain from dB
    pitch_k: f32,       // pitch smoothing constant
}

impl Synth {
    pub fn new(sample_rate: f32) -> Synth {
        let pitch_k = 1.0 - (-1.0 / (0.002 * sample_rate)).exp();
        Synth {
            sample_rate,
            voices: [Voice::default(); MAX_VOICES],
            patch: Patch::GlidePad,
            gain: 1.0,
            pitch_k,
        }
    }

    pub fn set_patch(&mut self, patch: Patch) {
        self.patch = patch;
    }

    /// Master gain in dB (0 = unity).
    pub fn set_volume_db(&mut self, db: f32) {
        self.gain = 10f32.powf(db / 20.0);
    }

    /// `Some(pitch)`: the voice sounds at `pitch`; going from `None` to `Some` triggers its
    /// envelope. `None`: the voice releases. Pitch changes while sounding do not retrigger;
    /// they are smoothed over ~2 ms so per-block updates do not zipper.
    pub fn set_voice(&mut self, voice: usize, pitch: Option<f32>) {
        let v = &mut self.voices[voice];
        v.target = pitch.unwrap_or(v.target);

        if let Some(p) = pitch {
            if v.stage == 0 || v.stage == 4 {
                // Not currently sounding — start attack
                v.pitch = p;
                v.stage = 1; // attack
                v.on = true;
            }
            v.target = p;
        } else {
            // Key released
            if v.stage >= 1 && v.stage <= 3 {
                v.stage = 4; // release
            }
        }
    }

    /// Retrigger a sounding voice's envelope without changing its pitch.
    pub fn strike(&mut self, voice: usize) {
        let v = &mut self.voices[voice];
        if v.on && v.stage >= 1 && v.stage <= 4 {
            v.stage = 1;
            v.env = 0.0;
        }
    }

    /// Render into both buffers (same length), overwriting them. Output stays within
    /// −1..=1 (soft-clip the mix) and is never NaN.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        let sr = self.sample_rate;
        let patch = self.patch;
        let params = patch.env_params();
        let (atk, dcy, sus, rel, decays_while_held) = params;
        let gain = self.gain;

        for i in 0..left.len() {
            let mut sum: f32 = 0.0;

            for v in &mut self.voices {
                if v.stage == 0 {
                    continue;
                }

                // Pitch smoothing
                v.pitch += (v.target - v.pitch) * self.pitch_k;
                let freq = midi_to_hz(v.pitch);
                let dt = freq / sr;

                // Advance oscillator phases (harmonics use multipliers for Organ)
                match patch {
                    Patch::Organ => {
                        v.phase[0] = (v.phase[0] + dt).rem_euclid(1.0);
                        v.phase[1] = (v.phase[1] + 2.0 * dt).rem_euclid(1.0);
                        v.phase[2] = (v.phase[2] + 3.0 * dt).rem_euclid(1.0);
                    }
                    _ => {
                        for pj in &mut v.phase {
                            *pj = (*pj + dt).rem_euclid(1.0);
                        }
                    }
                }

                // Envelope update
                match v.stage {
                    1 => {
                        v.env += 1.0 / (atk * sr);
                        if v.env >= 1.0 {
                            v.env = 1.0;
                            v.stage = 2;
                        }
                    }
                    2 => {
                        if sus == 0.0 && decays_while_held {
                            v.env -= v.env / (dcy * sr);
                            if v.env <= 0.0 {
                                v.env = 0.0;
                                v.stage = 0;
                                v.on = false;
                            }
                        } else {
                            let step = (1.0 - sus) / (dcy * sr);
                            v.env -= step;
                            if v.env <= sus {
                                v.env = sus;
                                v.stage = 3;
                            }
                        }
                    }
                    3 => {
                        if sus == 0.0 && decays_while_held {
                            v.env -= v.env / (dcy * sr);
                            if v.env <= 0.0 {
                                v.env = 0.0;
                                v.stage = 0;
                                v.on = false;
                            }
                        }
                    }
                    4 => {
                        v.env -= 1.0 / (rel * sr);
                        if v.env <= 0.0 {
                            v.env = 0.0;
                            v.stage = 0;
                            v.on = false;
                        }
                    }
                    _ => {}
                }

                let env = v.env;
                let p0 = v.phase[0];
                let p1 = v.phase[1];
                let p2 = v.phase[2];

                match patch {
                    Patch::GlidePad => {
                        let s0 = saw(p0, dt);
                        let s1 = saw(p1, dt * 1.006);
                        let x = (s0 + s1) * 0.5;
                        v.lp += (x - v.lp) * 0.08;
                        sum += v.lp * env * VOICE_GAIN;
                    }
                    Patch::Saw => {
                        sum += saw(p0, dt) * env * VOICE_GAIN;
                    }
                    Patch::EPiano => {
                        sum += (sine(p0) + 0.3 * env * sine(p1)) * env * VOICE_GAIN;
                    }
                    Patch::Organ => {
                        sum += (sine(p0) + 0.5 * sine(p1) + 0.25 * sine(p2)) / 1.75 * env * VOICE_GAIN;
                    }
                    Patch::Pluck => {
                        let x = saw(p0, dt);
                        let filt = 0.05 + 0.5 * env;
                        v.lp += (x - v.lp) * filt;
                        sum += v.lp * env * VOICE_GAIN;
                    }
                    Patch::Bells => {
                        let m = sine(p1);
                        sum += sine(p0 + 0.3 * env * m) * env * VOICE_GAIN;
                    }
                }
            }

            // Apply master gain and soft clip, write both channels
            let x = (sum * gain).tanh();
            left[i] = x;
            right[i] = x;
        }
    }
}

/// PolyBLEP band-limiting for sawtooth at phase `t` (0..1) with phase step `dt`.
fn poly_blep(t: f32, dt: f32) -> f32 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// Band-limited sawtooth from phase t ∈ [0,1).
fn saw(t: f32, dt: f32) -> f32 {
    let base = 2.0 * t - 1.0;
    base - poly_blep(t, dt)
}

/// Unit sine from phase t ∈ [0,1).
fn sine(t: f32) -> f32 {
    (t * TAU).sin()
}
