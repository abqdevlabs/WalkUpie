use std::num::{NonZeroU16, NonZeroU32};

use rodio::buffer::SamplesBuffer;
use rodio::mixer::Mixer;
use rodio::DeviceSinkBuilder;

const SAMPLE_RATE: u32 = 44100;

pub struct Sound {
    _sink: rodio::MixerDeviceSink,
    mixer: Mixer,
}

impl Sound {
    pub fn new() -> Option<Self> {
        let sink = DeviceSinkBuilder::open_default_sink().ok()?;
        let mixer = sink.mixer().clone();
        Some(Self { _sink: sink, mixer })
    }

    pub fn chime(&self) {
        let mut samples: Vec<f32> = Vec::with_capacity(SAMPLE_RATE as usize * 2);
        // A pleasant two-part chime: E5 -> A5, then a longer resolve (E6).
        for (freq, dur) in [(659.25, 0.12), (880.0, 0.12), (1318.51, 0.30)] {
            for i in 0..(SAMPLE_RATE as f32 * dur) as usize {
                let t = i as f32 / SAMPLE_RATE as f32;
                let envelope = (1.0 - t / dur).max(0.0);
                samples.push((freq * std::f32::consts::TAU * t).sin() * envelope * 0.4);
            }
        }
        let source = SamplesBuffer::new(
            NonZeroU16::new(1).unwrap(),
            NonZeroU32::new(SAMPLE_RATE).unwrap(),
            samples,
        );
        self.mixer.add(source);
    }
}