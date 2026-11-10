//! Built-in sound engine: up to 7 voices, one patch at a time.
//!
//! The caller drives each voice's pitch (fractional MIDI) every control block; the synth
//! renders stereo audio. No allocation in `render`.

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
        todo!()
    }
}

/// 440 · 2^((p − 69) / 12).
pub fn midi_to_hz(p: f32) -> f32 {
    todo!()
}

#[derive(Clone, Debug)]
pub struct Synth {
    sample_rate: f32,
    // Private state is yours to design.
}

impl Synth {
    pub fn new(sample_rate: f32) -> Synth {
        todo!()
    }

    pub fn set_patch(&mut self, patch: Patch) {
        todo!()
    }

    /// Master gain in dB (0 = unity).
    pub fn set_volume_db(&mut self, db: f32) {
        todo!()
    }

    /// `Some(pitch)`: the voice sounds at `pitch`; going from `None` to `Some` triggers its
    /// envelope. `None`: the voice releases. Pitch changes while sounding do not retrigger;
    /// they are smoothed over ~2 ms so per-block updates do not zipper.
    pub fn set_voice(&mut self, voice: usize, pitch: Option<f32>) {
        todo!()
    }

    /// Retrigger a sounding voice's envelope without changing its pitch.
    pub fn strike(&mut self, voice: usize) {
        todo!()
    }

    /// Render into both buffers (same length), overwriting them. Output stays within
    /// −1..=1 (soft-clip the mix) and is never NaN.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        todo!()
    }
}
