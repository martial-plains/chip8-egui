use std::{
    f64::consts::{PI, TAU},
    sync::{atomic::AtomicU8, Arc},
};

use anyhow::Context;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, Sample, Stream, StreamConfig,
};

/// Manages the audio on the current system, and plays a single
/// frequency whenever the `Chip8` sound timer is above `0`.
pub struct System {
    stream: Stream,
}

impl System {
    /// Create a new [`System`] associated with the given sound timer.
    ///
    /// Whenver the sound timer is above `0`, a frequency will play (assuming
    /// `System::play` has been called beforehand).
    ///
    /// # Errors
    ///
    /// * This function may return an error if the default output device cannot
    ///   be obtained by the host system. In such a case, it will panic with
    ///   the message "failed to get output device". To handle this error
    ///   gracefully, the caller should catch the panic using `catch_unwind`
    ///   or a similar mechanism.
    ///
    /// * This function may also return an error if the `get_stream` method fails
    ///   to create an audio stream. The specific error type returned by
    ///   `get_stream` is not documented, but it is likely to be an `anyhow::Error`.
    ///   The caller should handle this error appropriately, for example by
    ///   returning it to the caller of the function or logging it.
    pub fn new(timer: Arc<AtomicU8>) -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("failed to get output device");

        Self::get_stream(&device, timer).map(|stream| Self { stream })
    }

    /// Create and retrieve a [`Stream`] depending on the sample format of the given [`Device`].
    fn get_stream(device: &Device, timer: Arc<AtomicU8>) -> anyhow::Result<Stream> {
        let config = device.default_output_config()?;
        match config.sample_format() {
            cpal::SampleFormat::I16 => Self::create_stream::<i16>(device, &config.into(), timer),
            cpal::SampleFormat::U16 => Self::create_stream::<u16>(device, &config.into(), timer),
            cpal::SampleFormat::F32 => Self::create_stream::<f32>(device, &config.into(), timer),
            _ => unimplemented!(),
        }
    }

    /// Create a new [`Stream`].
    fn create_stream<T>(
        device: &Device,
        config: &StreamConfig,
        timer: Arc<AtomicU8>,
    ) -> anyhow::Result<Stream>
    where
        T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f64>,
    {
        let sample_rate = f64::from(config.sample_rate.0);
        let channels = usize::from(config.channels);

        let mut sample_clock = 0f64;
        let mut next_sample = move || {
            sample_clock = (sample_clock + 1.0) % sample_rate;
            if timer.load(std::sync::atomic::Ordering::SeqCst) > 0 {
                (440.0 * TAU * sample_clock / sample_rate).sin().asin() * 2.0 / PI
            } else {
                0.0
            }
        };

        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let value: T = next_sample().to_sample();
                    for sample in frame.iter_mut() {
                        *sample = value;
                    }
                }
            },
            |err| log::error!("An error occurred on the audio stream: {err}"),
            None,
        )?;
        Ok(stream)
    }

    /// Play the audio stream.
    ///
    /// # Errors
    ///
    /// Returns an error if the audio stream fails to play.
    pub fn play(&self) -> anyhow::Result<()> {
        self.stream.play().context("Failed to play audio stream.")
    }
}
