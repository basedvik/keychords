//! Built-in sound engine: up to 7 voices, one patch at a time.
//!
//! The caller drives each voice's pitch (fractional MIDI) every control block; the synth
//! renders stereo audio. No allocation in `render`.

use std::f32::consts::TAU;

pub const MAX_VOICES: usize = 7;

/// Karplus–Strong delay line length: covers the app's bass range at 44.1–48 kHz.
const KS_LEN: usize = 1024;
/// Theremin vibrato: rate in Hz, depth as a fraction of the carrier frequency.
const VIB_RATE: f32 = 5.5;
const VIB_DEPTH: f32 = 0.008;

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
    /// Karplus–Strong pluck, medium decay.
    Koto,
    /// Two beating sine tines plus a faint inharmonic tinkle, long decay.
    MusicBox,
    /// Bright Karplus–Strong pluck, fast decay.
    Banjo,
    /// Three detuned saws, slow attack, gentle low-pass and a breathy top.
    SaxSection,
    /// Three detuned pulses (odd-harmonic reed), slow attack, gentle low-pass.
    ClarinetChoir,
    /// Muffled decaying-saw pluck through two low-pass stages.
    PizzicatoStrings,
    /// Two detuned decaying-saw plucks, a few cents apart.
    TwelveString,
    /// Two-operator FM, ratio 1:2.4, slow decay with a decaying index.
    Vibraphone,
    /// Sines at 1×, 2× and 3× with weak upper partials, felt-piano envelope.
    FeltPiano,
    /// Fundamental with a faint octave and a detuned ghost partial, long sustain.
    Handpan,
    /// Sine partials at 1×, 2.76× and 5.4× (metal tine), long decay.
    Kalimba,
    /// Decaying-saw pluck with a soft attack.
    Harp,
    /// Detuned saws through a low-pass body and a formant-ish resonator.
    Choir,
    /// Three detuned saws through two low-pass stages, slow attack.
    StringSection,
    /// Three detuned saws through two low-pass stages, horn-dark.
    FrenchHorns,
    /// Sine with slow vibrato and a faint second harmonic.
    Theremin,
    /// Decaying-saw pluck through two low-pass stages, clean electric tone.
    NeoSoulGuitar,
}

impl Patch {
    pub const ALL: [Patch; 23] = [
        Patch::GlidePad,
        Patch::Saw,
        Patch::EPiano,
        Patch::Organ,
        Patch::Pluck,
        Patch::Bells,
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

    /// Key-map label: "Pad", "Saw", "E-piano", "Organ", "Pluck", "Bells", ...
    pub fn label(self) -> &'static str {
        match self {
            Patch::GlidePad => "Pad",
            Patch::Saw => "Saw",
            Patch::EPiano => "E-piano",
            Patch::Organ => "Organ",
            Patch::Pluck => "Pluck",
            Patch::Bells => "Bells",
            Patch::Koto => "Koto",
            Patch::MusicBox => "Music box",
            Patch::Banjo => "Banjo",
            Patch::SaxSection => "Sax section",
            Patch::ClarinetChoir => "Clarinet choir",
            Patch::PizzicatoStrings => "Pizzicato strings",
            Patch::TwelveString => "12-string guitar",
            Patch::Vibraphone => "Vibraphone",
            Patch::FeltPiano => "Felt piano",
            Patch::Handpan => "Handpan",
            Patch::Kalimba => "Kalimba",
            Patch::Harp => "Harp",
            Patch::Choir => "Choir",
            Patch::StringSection => "String section",
            Patch::FrenchHorns => "French horns",
            Patch::Theremin => "Theremin",
            Patch::NeoSoulGuitar => "Neo-soul guitar",
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
            Patch::Koto => (0.002, 1.0, 0.0, 0.25, true),
            Patch::MusicBox => (0.002, 2.5, 0.0, 0.5, true),
            Patch::Banjo => (0.002, 0.5, 0.0, 0.2, true),
            Patch::SaxSection => (0.12, 0.15, 0.85, 0.25, false),
            Patch::ClarinetChoir => (0.1, 0.15, 0.85, 0.25, false),
            Patch::PizzicatoStrings => (0.02, 1.2, 0.0, 0.3, true),
            Patch::TwelveString => (0.002, 0.9, 0.0, 0.25, true),
            Patch::Vibraphone => (0.003, 3.0, 0.0, 0.7, true),
            Patch::FeltPiano => (0.002, 1.4, 0.0, 0.4, true),
            Patch::Handpan => (0.01, 4.0, 0.4, 0.8, false),
            Patch::Kalimba => (0.002, 1.8, 0.0, 0.5, true),
            Patch::Harp => (0.03, 1.0, 0.0, 0.35, true),
            Patch::Choir => (0.3, 0.2, 0.85, 0.5, false),
            Patch::StringSection => (0.18, 0.2, 0.85, 0.3, false),
            Patch::FrenchHorns => (0.15, 0.2, 0.8, 0.3, false),
            Patch::Theremin => (0.08, 0.1, 0.9, 0.15, false),
            Patch::NeoSoulGuitar => (0.004, 0.8, 0.0, 0.3, true),
        }
    }

    /// Karplus–Strong plucks: the delay line is (re)primed on trigger and on `strike`.
    fn is_ks(self) -> bool {
        matches!(self, Patch::Koto | Patch::Banjo)
    }
}

/// 440 · 2^((p − 69) / 12).
pub fn midi_to_hz(p: f32) -> f32 {
    440.0 * 2.0_f32.powf((p - 69.0) / 12.0)
}

const VOICE_GAIN: f32 = 0.18;

#[derive(Clone, Copy, Debug)]
struct Voice {
    pitch: f32,       // current, smoothed
    target: f32,
    on: bool,
    env: f32,         // 0..1
    stage: u8,        // 0=idle, 1=attack, 2=decay, 3=sustain, 4=release
    phase: [f32; 3],  // oscillator phases 0..1
    lp: f32,          // one-pole filter state
    lp2: f32,         // second one-pole: series muffle stage or other chorus leg
    f1: f32,          // formant resonator: previous output
    f2: f32,          // formant resonator: output before the previous
    ks: [f32; KS_LEN], // Karplus–Strong delay line
    ks_pos: u16,      // read position in the delay line
    ks_len: u16,      // delay line length in samples; 0 = not primed
    ks_damp: f32,     // per-pass amplitude decay, set at prime time
    prime: bool,      // set on trigger; the Karplus–Strong arm consumes it
    rng: u32,         // noise LCG state for priming the delay line
}

impl Default for Voice {
    fn default() -> Voice {
        Voice {
            pitch: 0.0,
            target: 0.0,
            on: false,
            env: 0.0,
            stage: 0,
            phase: [0.0; 3],
            lp: 0.0,
            lp2: 0.0,
            f1: 0.0,
            f2: 0.0,
            ks: [0.0; KS_LEN],
            ks_pos: 0,
            ks_len: 0,
            ks_damp: 0.0,
            prime: false,
            rng: 0,
        }
    }
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
                if self.patch.is_ks() {
                    v.prime = true;
                }
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
            if self.patch.is_ks() {
                v.prime = true;
            }
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
        let ks = patch.is_ks();

        for i in 0..left.len() {
            let mut sum: f32 = 0.0;

            for v in &mut self.voices {
                if v.stage == 0 {
                    continue;
                }
                if v.prime && !ks {
                    v.prime = false; // a prime request is only meaningful to the plucks
                }

                // Pitch smoothing
                v.pitch += (v.target - v.pitch) * self.pitch_k;
                let freq = midi_to_hz(v.pitch);
                let dt = freq / sr;

                // Advance oscillator phases (per-patch harmonic and detune layout)
                let mult: [f32; 3] = match patch {
                    Patch::Organ | Patch::FeltPiano => [1.0, 2.0, 3.0],
                    Patch::Kalimba => [1.0, 2.76, 5.4],
                    Patch::MusicBox => [1.0, 1.006, 2.76],
                    Patch::Vibraphone => [1.0, 2.4, 1.0],
                    Patch::Handpan => [1.0, 2.0, 1.283],
                    Patch::TwelveString => [1.0, 1.004, 1.0],
                    Patch::SaxSection
                    | Patch::ClarinetChoir
                    | Patch::StringSection
                    | Patch::Choir => [0.998, 1.0, 1.003],
                    Patch::FrenchHorns => [0.9985, 1.0, 1.0035],
                    Patch::Theremin => [1.0, VIB_RATE / sr, 2.0],
                    _ => [1.0, 1.0, 1.0],
                };
                for (pj, m) in v.phase.iter_mut().zip(mult.iter()) {
                    *pj = (*pj + dt * *m).rem_euclid(1.0);
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
                    Patch::Koto | Patch::Banjo => {
                        // Karplus–Strong: the delay line is primed with noise at strike time;
                        // its length sets the pitch and its damping sets the decay rate.
                        let decay = if patch == Patch::Banjo { 3.5 } else { 2.0 };
                        if v.prime {
                            v.prime = false;
                            let len = (sr / freq).round().clamp(3.0, KS_LEN as f32) as usize;
                            v.ks_len = len as u16;
                            v.ks_pos = 0;
                            v.ks_damp = (-decay * len as f32 / sr).exp();
                            v.rng = freq.to_bits() ^ 0x9E37_79B9;
                            for s in &mut v.ks[..len] {
                                v.rng =
                                    v.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                                *s = (v.rng as f32 / 4_294_967_296.0 - 0.5) * 0.7;
                            }
                        }
                        if v.ks_len >= 3 {
                            let n = v.ks_len as usize;
                            let i = v.ks_pos as usize;
                            let out = v.ks[i];
                            v.ks[i] = (out + v.ks[(i + 1) % n]) * 0.5 * v.ks_damp;
                            v.ks_pos = ((i + 1) % n) as u16;
                            sum += out * env * VOICE_GAIN;
                        } else {
                            // Unprimed note (should not happen): decaying filtered saw.
                            let filt = 0.05 + 0.4 * env;
                            v.lp += (saw(p0, dt) - v.lp) * filt;
                            sum += v.lp * env * VOICE_GAIN;
                        }
                    }
                    Patch::MusicBox => {
                        sum += (sine(p0) + 0.6 * sine(p1) + 0.2 * sine(p2)) / 1.8 * env * VOICE_GAIN;
                    }
                    Patch::Vibraphone => {
                        let m = sine(p1);
                        sum += (sine(p0 + 0.9 * env * m) + 0.3 * sine(p0)) / 1.3 * env * VOICE_GAIN;
                    }
                    Patch::FeltPiano => {
                        sum += (sine(p0) + 0.2 * sine(p1) + 0.05 * sine(p2)) / 1.25 * env * VOICE_GAIN;
                    }
                    Patch::Handpan => {
                        sum += (0.7 * sine(p0) + 0.2 * sine(p1) + 0.3 * sine(p2)) / 1.2 * env * VOICE_GAIN;
                    }
                    Patch::Kalimba => {
                        sum += (sine(p0) + 0.35 * sine(p1) + 0.15 * sine(p2)) / 1.5 * env * VOICE_GAIN;
                    }
                    Patch::SaxSection => {
                        let x = (saw(p0, dt * 0.998) + saw(p1, dt) + saw(p2, dt * 1.003)) / 3.0;
                        v.lp += (x - v.lp) * 0.25;
                        sum += (v.lp + 0.15 * (x - v.lp)) * env * VOICE_GAIN;
                    }
                    Patch::ClarinetChoir => {
                        let x = (square(p0) + square(p1) + square(p2)) / 3.0;
                        v.lp += (x - v.lp) * 0.3;
                        sum += v.lp * env * VOICE_GAIN;
                    }
                    Patch::PizzicatoStrings => {
                        let f = 0.03 + 0.18 * env;
                        v.lp += (saw(p0, dt) - v.lp) * f;
                        v.lp2 += (v.lp - v.lp2) * 0.35;
                        sum += v.lp2 * env * VOICE_GAIN;
                    }
                    Patch::TwelveString => {
                        let f = 0.05 + 0.4 * env;
                        v.lp += (saw(p0, dt) - v.lp) * f;
                        v.lp2 += (saw(p1, dt * 1.004) - v.lp2) * f;
                        sum += (v.lp + v.lp2) * 0.5 * env * VOICE_GAIN;
                    }
                    Patch::Harp => {
                        let f = 0.05 + 0.3 * env;
                        v.lp += (saw(p0, dt) - v.lp) * f;
                        sum += v.lp * env * VOICE_GAIN;
                    }
                    Patch::Choir => {
                        let x = (saw(p0, dt * 0.998) + saw(p1, dt) + saw(p2, dt * 1.003)) / 3.0;
                        v.lp += (x - v.lp) * 0.2;
                        // Resonator near 1 kHz (radius 0.99); the scale tames its peak.
                        let w = TAU * 1100.0 / sr;
                        let y = x + 1.98 * w.cos() * v.f1 - 0.9801 * v.f2;
                        v.f2 = v.f1;
                        v.f1 = y;
                        sum += (0.7 * v.lp + 0.5 * y * (0.02 * w.sin())) * env * VOICE_GAIN;
                    }
                    Patch::StringSection => {
                        let x = (saw(p0, dt * 0.998) + saw(p1, dt) + saw(p2, dt * 1.003)) / 3.0;
                        v.lp += (x - v.lp) * 0.15;
                        v.lp2 += (v.lp - v.lp2) * 0.4;
                        sum += v.lp2 * env * VOICE_GAIN;
                    }
                    Patch::FrenchHorns => {
                        let x = (saw(p0, dt * 0.9985) + saw(p1, dt) + saw(p2, dt * 1.0035)) / 3.0;
                        v.lp += (x - v.lp) * 0.2;
                        v.lp2 += (v.lp - v.lp2) * 0.25;
                        sum += v.lp2 * env * VOICE_GAIN;
                    }
                    Patch::Theremin => {
                        let vib = (v.phase[1] * TAU).cos();
                        let carrier = p0 - (VIB_DEPTH / (TAU * VIB_RATE)) * vib;
                        sum += (0.9 * sine(carrier) + 0.1 * sine(p2)) * env * VOICE_GAIN;
                    }
                    Patch::NeoSoulGuitar => {
                        let f = 0.04 + 0.3 * env;
                        v.lp += (saw(p0, dt) - v.lp) * f;
                        v.lp2 += (v.lp - v.lp2) * 0.5;
                        sum += v.lp2 * env * VOICE_GAIN;
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

/// Unit square wave (odd harmonics only) from phase t ∈ [0,1).
fn square(t: f32) -> f32 {
    if t < 0.5 {
        1.0
    } else {
        -1.0
    }
}
