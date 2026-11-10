//! Device glue: a cpal output stream running the `Engine`, and a MIDI thread that forwards
//! what the engine emits to a virtual port named "keychords" (loopMIDI on Windows).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use midi::mpe::Msg;
use rtrb::{Consumer, Producer, RingBuffer};

use crate::engine::{Command, Engine};

pub struct Audio {
    _stream: cpal::Stream,
    pub commands: Producer<Command>,
    /// The engine's sample clock, published once per buffer.
    pub now: Arc<AtomicU64>,
    pub sample_rate: f64,
    /// Name of the MIDI port in use, if one opened.
    pub midi_port: Option<String>,
}

pub fn start() -> Result<Audio, String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("no audio output device")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    if supported.sample_format() != cpal::SampleFormat::F32 {
        return Err(format!("unsupported sample format {:?}", supported.sample_format()));
    }
    let config: cpal::StreamConfig = supported.config();
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate as f64;

    let (commands, mut cmd_rx) = RingBuffer::<Command>::new(256);
    let (mut midi_tx, midi_rx) = RingBuffer::<Msg>::new(4096);
    let now = Arc::new(AtomicU64::new(0));
    let clock = now.clone();

    let mut engine = Engine::new(sample_rate as f32);
    let (mut left, mut right) = (vec![0.0f32; 8192], vec![0.0f32; 8192]);
    let mut midi_buf: Vec<Msg> = Vec::with_capacity(2048);

    let stream = device
        .build_output_stream(
            config.clone(),
            move |data: &mut [f32], _| {
                while let Ok(cmd) = cmd_rx.pop() {
                    engine.handle(cmd, &mut midi_buf);
                }
                let frames = data.len() / channels;
                if left.len() < frames {
                    left.resize(frames, 0.0);
                    right.resize(frames, 0.0);
                }
                engine.process(&mut left[..frames], &mut right[..frames], &mut midi_buf);
                for (i, frame) in data.chunks_mut(channels).enumerate() {
                    for (c, s) in frame.iter_mut().enumerate() {
                        *s = if c % 2 == 0 { left[i] } else { right[i] };
                    }
                }
                for m in midi_buf.drain(..) {
                    let _ = midi_tx.push(m);
                }
                clock.store(engine.now(), Ordering::Relaxed);
            },
            |e| eprintln!("audio stream error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;

    let midi_port = start_midi(midi_rx);
    Ok(Audio { _stream: stream, commands, now, sample_rate, midi_port })
}

fn start_midi(mut rx: Consumer<Msg>) -> Option<String> {
    let out = midir::MidiOutput::new("keychords").ok()?;
    #[cfg(unix)]
    let (mut conn, name) = {
        use midir::os::unix::VirtualOutput;
        (out.create_virtual("keychords").ok()?, "keychords (virtual)".to_string())
    };
    #[cfg(not(unix))]
    let (mut conn, name) = {
        let port = out.ports().into_iter().find(|p| {
            out.port_name(p).is_ok_and(|n| n.to_lowercase().contains("loopmidi"))
        })?;
        let name = out.port_name(&port).ok()?;
        (out.connect(&port, "keychords").ok()?, name)
    };
    thread::spawn(move || loop {
        match rx.pop() {
            Ok(m) => {
                let _ = conn.send(&m);
            }
            Err(_) => thread::sleep(Duration::from_millis(1)),
        }
    });
    Some(name)
}
