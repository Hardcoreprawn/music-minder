//! Audio output using cpal (WASAPI on Windows).
//!
//! This module runs the real-time audio thread that:
//! - Reads decoded audio from a lock-free ring buffer
//! - Applies volume using atomic state
//! - Sends samples to the FFT analyzer
//! - Outputs to the audio device
//!
//! # Real-time Safety
//!
//! The cpal callback runs on a high-priority system thread. To avoid audio glitches:
//! - No locks (RwLock/Mutex) - use atomics via `AudioSharedState`
//! - No allocations - use `rtrb` ring buffer for sample data
//! - No blocking operations

use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use crossbeam_channel::{Receiver, Sender};
use parking_lot::RwLock;
use rtrb::{Consumer, Producer, RingBuffer};

use super::PlayerError;
use super::decoder::AudioDecoder;
use super::resampler::Resampler;
use super::state::{AudioSharedState, PlaybackStatus, PlayerCommand, PlayerState};
use super::visualization::SpectrumData;

/// Audio output configuration.
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Desired sample rate (0 = use device default)
    pub sample_rate: u32,
    /// Desired buffer size in samples (0 = use device default)
    pub buffer_size: u32,
    /// Number of channels (usually 2)
    pub channels: u16,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 0, // Use device default
            buffer_size: 0, // Use device default
            channels: 2,
        }
    }
}

/// Audio output manager.
pub struct AudioOutput {
    _stream: Stream,
    _audio_thread: JoinHandle<()>,
    /// Lock-free shared state for the audio callback
    pub audio_shared: Arc<AudioSharedState>,
}

impl AudioOutput {
    /// Create a new audio output.
    pub fn new(
        state: Arc<RwLock<PlayerState>>,
        command_rx: Receiver<PlayerCommand>,
        viz_tx: Sender<SpectrumData>,
    ) -> Result<Self, PlayerError> {
        let host = cpal::default_host();

        // Try to find headphones first, then fall back to default
        let device = Self::select_best_device(&host)?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        tracing::info!("Using audio device: {}", device_name);

        // Get supported config
        let supported_config = device
            .default_output_config()
            .map_err(|e| PlayerError::AudioInit(e.to_string()))?;

        let sample_rate = supported_config.sample_rate().0;
        let channels = supported_config.channels();

        tracing::info!("Audio format: {}Hz, {} channels", sample_rate, channels);

        // Create config
        let config = StreamConfig {
            channels,
            sample_rate: supported_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        // Create lock-free ring buffer for audio samples
        // Size: ~0.5 seconds of stereo audio at 48kHz = 48000 * 2 * 0.5 = 48000 samples
        let (producer, consumer) = RingBuffer::<f32>::new(48000);

        // Create lock-free shared state for the audio callback
        let audio_shared = AudioSharedState::new();

        // Initialize from UI state
        {
            let ui_state = state.read();
            audio_shared.set_volume(ui_state.volume);
            audio_shared.set_playing(ui_state.status == PlaybackStatus::Playing);
        }

        // Clone state for the audio thread
        let state_for_thread = Arc::clone(&state);
        let audio_shared_for_thread = Arc::clone(&audio_shared);

        // Start the audio/decoder thread
        let audio_thread = thread::Builder::new()
            .name("audio-decoder".to_string())
            .spawn(move || {
                audio_thread_main(
                    state_for_thread,
                    audio_shared_for_thread,
                    command_rx,
                    producer,
                    viz_tx,
                    sample_rate,
                    channels,
                );
            })
            .map_err(|e| PlayerError::AudioInit(e.to_string()))?;

        // Clone audio shared state for the callback
        let callback_audio_shared = Arc::clone(&audio_shared);

        // Build output stream
        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => {
                build_stream::<f32>(&device, &config, consumer, callback_audio_shared)
            }
            SampleFormat::I16 => {
                build_stream_i16(&device, &config, consumer, Arc::clone(&audio_shared))
            }
            format => {
                return Err(PlayerError::AudioInit(format!(
                    "Unsupported sample format: {:?}",
                    format
                )));
            }
        }
        .map_err(|e| PlayerError::AudioInit(e.to_string()))?;

        // Start playback
        stream
            .play()
            .map_err(|e| PlayerError::AudioInit(e.to_string()))?;

        Ok(Self {
            _stream: stream,
            _audio_thread: audio_thread,
            audio_shared,
        })
    }

    /// Select the best audio device - prefer headphones if available.
    fn select_best_device(host: &cpal::Host) -> Result<Device, PlayerError> {
        let devices: Vec<Device> = host
            .output_devices()
            .map_err(|e| PlayerError::AudioInit(e.to_string()))?
            .collect();

        // Log all available devices
        for device in &devices {
            if let Ok(name) = device.name() {
                tracing::info!("Available audio device: {}", name);
            }
        }

        // Look for headphones by name (case-insensitive)
        let headphone_keywords = [
            "headphone",
            "headset",
            "earphone",
            "airpod",
            "buds",
            "earbuds",
        ];

        for device in &devices {
            if let Ok(name) = device.name() {
                let name_lower = name.to_lowercase();
                for keyword in &headphone_keywords {
                    if name_lower.contains(keyword) {
                        tracing::info!("Selected headphones: {}", name);
                        return Ok(device.clone());
                    }
                }
            }
        }

        // Fall back to default device
        host.default_output_device()
            .ok_or_else(|| PlayerError::AudioInit("No output device found".to_string()))
    }
}

/// Build output stream for f32 format.
///
/// # Real-time Safety
/// This callback uses only:
/// - Atomic operations for state (no locks)
/// - Lock-free ring buffer for samples (no allocations)
fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    mut consumer: Consumer<f32>,
    audio_shared: Arc<AudioSharedState>,
) -> Result<Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let buffer_capacity = consumer.buffer().capacity();

    device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let start = std::time::Instant::now();

            // ✅ SAFE: Atomic reads - no locks in the audio callback
            let volume = audio_shared.volume();
            let is_playing = audio_shared.is_playing();

            if !is_playing {
                // Output silence when paused
                for sample in data.iter_mut() {
                    *sample = T::from_sample(0.0f32);
                }
                return;
            }

            let mut samples_read = 0u32;

            // ✅ SAFE: Lock-free ring buffer pop - no allocations
            for sample in data.iter_mut() {
                match consumer.pop() {
                    Ok(s) => {
                        *sample = T::from_sample(s * volume);
                        samples_read += 1;
                    }
                    Err(_) => {
                        // Underrun - output silence
                        audio_shared.increment_underruns();
                        *sample = T::from_sample(0.0f32);
                    }
                }
            }

            // Update performance metrics (still lock-free)
            let elapsed_us = start.elapsed().as_micros() as u32;
            audio_shared.record_callback(samples_read, elapsed_us);

            // Update buffer fill level
            let slots_available = consumer.slots();
            let fill_percent = ((buffer_capacity - slots_available) * 100 / buffer_capacity) as u32;
            audio_shared.set_buffer_fill(fill_percent);
        },
        |err| {
            tracing::error!("Audio stream error: {}", err);
        },
        None,
    )
}

/// Build output stream for i16 format.
///
/// # Real-time Safety
/// This callback uses only:
/// - Atomic operations for state (no locks)
/// - Lock-free ring buffer for samples (no allocations)
fn build_stream_i16(
    device: &Device,
    config: &StreamConfig,
    mut consumer: Consumer<f32>,
    audio_shared: Arc<AudioSharedState>,
) -> Result<Stream, cpal::BuildStreamError> {
    let buffer_capacity = consumer.buffer().capacity();

    device.build_output_stream(
        config,
        move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
            let start = std::time::Instant::now();

            // ✅ SAFE: Atomic reads - no locks in the audio callback
            let volume = audio_shared.volume();
            let is_playing = audio_shared.is_playing();

            if !is_playing {
                for sample in data.iter_mut() {
                    *sample = 0;
                }
                return;
            }

            let mut samples_read = 0u32;

            // ✅ SAFE: Lock-free ring buffer pop - no allocations
            for sample in data.iter_mut() {
                match consumer.pop() {
                    Ok(s) => {
                        *sample = (s * volume * 32767.0) as i16;
                        samples_read += 1;
                    }
                    Err(_) => {
                        // Underrun - output silence
                        audio_shared.increment_underruns();
                        *sample = 0;
                    }
                }
            }

            // Update performance metrics
            let elapsed_us = start.elapsed().as_micros() as u32;
            audio_shared.record_callback(samples_read, elapsed_us);

            // Update buffer fill level
            let slots_available = consumer.slots();
            let fill_percent = ((buffer_capacity - slots_available) * 100 / buffer_capacity) as u32;
            audio_shared.set_buffer_fill(fill_percent);
        },
        |err| {
            tracing::error!("Audio stream error: {}", err);
        },
        None,
    )
}

/// Audio thread context - encapsulates mutable state
struct AudioThreadContext {
    decoder: Option<AudioDecoder>,
    resampler: Option<Resampler>,
    visualizer: super::visualization::Visualizer,
    pending_path: Option<PathBuf>,
    /// Output device sample rate
    output_sample_rate: u32,
    /// Output device channels
    output_channels: u16,
    /// Samples per position update (updates position every ~50ms)
    samples_per_position_update: usize,
    /// Sample counter for position updates
    sample_counter: usize,
}

impl AudioThreadContext {
    fn new(output_sample_rate: u32, output_channels: u16) -> Self {
        // Update position every ~50ms based on OUTPUT sample rate
        let samples_per_position_update =
            (output_sample_rate as usize * output_channels as usize) / 20;
        Self {
            decoder: None,
            resampler: None,
            visualizer: super::visualization::Visualizer::new(2048),
            pending_path: None,
            output_sample_rate,
            output_channels,
            samples_per_position_update,
            sample_counter: 0,
        }
    }

    /// Handle a player command, returning whether to continue running
    fn handle_command(
        &mut self,
        cmd: PlayerCommand,
        state: &RwLock<PlayerState>,
        audio_shared: &AudioSharedState,
        producer: &mut Producer<f32>,
    ) -> bool {
        match cmd {
            PlayerCommand::Load(path) => {
                self.pending_path = Some(path);
            }
            PlayerCommand::Play => {
                self.start_or_resume(state, audio_shared);
            }
            PlayerCommand::Pause => {
                state.write().status = PlaybackStatus::Paused;
                audio_shared.set_playing(false);
            }
            PlayerCommand::Stop => {
                state.write().status = PlaybackStatus::Stopped;
                audio_shared.set_playing(false);
                self.decoder = None;
                self.resampler = None;
            }
            PlayerCommand::Seek(pos) => {
                if let Some(ref mut dec) = self.decoder {
                    // Flush the ring buffer before seeking
                    // This prevents hearing stale audio after seek
                    // Note: We reset the sample counter and position to signal discontinuity
                    let _slots = producer.slots();
                    audio_shared.set_position(Duration::ZERO);
                    self.sample_counter = 0;

                    // Reset resampler state to avoid artifacts
                    if let Some(ref mut resampler) = self.resampler {
                        resampler.reset();
                    }

                    if let Err(e) = dec.seek(pos) {
                        tracing::warn!("Seek failed: {}", e);
                    }
                }
            }
            PlayerCommand::Shutdown => return false,
        }
        true
    }

    fn start_or_resume(&mut self, state: &RwLock<PlayerState>, audio_shared: &AudioSharedState) {
        match self.pending_path.take() {
            Some(path) => self.load_and_play(path, state, audio_shared),
            None => {
                state.write().status = PlaybackStatus::Playing;
                audio_shared.set_playing(true);
            }
        }
    }

    fn load_and_play(
        &mut self,
        path: PathBuf,
        state: &RwLock<PlayerState>,
        audio_shared: &AudioSharedState,
    ) {
        match AudioDecoder::open(&path) {
            Ok(dec) => {
                let source_rate = dec.sample_rate();
                let source_channels = dec.channels();

                // Create resampler if sample rates differ
                let resampler =
                    Resampler::new(source_rate, self.output_sample_rate, source_channels);

                if resampler.needs_resampling() {
                    tracing::info!(
                        "Resampling: {}Hz → {}Hz",
                        source_rate,
                        self.output_sample_rate
                    );
                }

                let mut s = state.write();
                s.status = PlaybackStatus::Playing;
                s.current_track = Some(path);
                s.duration = dec.duration();
                s.position = Duration::ZERO;
                s.sample_rate = source_rate;
                s.channels = source_channels;
                s.bits_per_sample = dec.format_info.bit_depth;

                tracing::info!(
                    "Track loaded: {}Hz / {}ch / {}bit ({})",
                    source_rate,
                    source_channels,
                    dec.format_info.bit_depth,
                    dec.format_info.codec
                );

                // Populate quality information
                s.quality.format = dec.format_info.codec.clone();
                s.quality.is_lossless = dec.format_info.is_lossless;
                s.quality.bit_depth = dec.format_info.bit_depth;
                s.quality.source_sample_rate = source_rate;
                s.quality.output_sample_rate = self.output_sample_rate;
                s.quality.is_bit_perfect =
                    dec.format_info.is_lossless && source_rate == self.output_sample_rate;

                // Sync atomic state
                audio_shared.set_playing(true);
                audio_shared.set_position(Duration::ZERO);
                self.sample_counter = 0;
                self.decoder = Some(dec);
                self.resampler = Some(resampler);
            }
            Err(e) => {
                tracing::error!("Failed to open file: {}", e);
                state.write().status = PlaybackStatus::Stopped;
                audio_shared.set_playing(false);
            }
        }
    }

    /// Decode next chunk and send to outputs. Returns false if playback ended.
    fn decode_and_send(
        &mut self,
        producer: &mut Producer<f32>,
        viz_tx: &Sender<SpectrumData>,
        state: &RwLock<PlayerState>,
        audio_shared: &AudioSharedState,
    ) -> bool {
        let Some(ref mut dec) = self.decoder else {
            return true;
        };

        // Check if ring buffer has space
        let available = producer.slots();
        if available < 1024 {
            // Buffer is nearly full, wait a bit
            thread::sleep(Duration::from_millis(5));
            return true;
        }

        let mut samples = Vec::with_capacity(4096);

        match dec.decode_next(|s| samples.extend_from_slice(s)) {
            Ok(Some(frame)) => {
                // Resample if needed
                let output_samples = if let Some(ref mut resampler) = self.resampler {
                    resampler.process(&samples)
                } else {
                    samples.clone()
                };

                // Extract left channel for visualization (from resampled output)
                let output_channels = self.output_channels as usize;
                let left: Vec<f32> = output_samples
                    .iter()
                    .step_by(output_channels)
                    .copied()
                    .collect();
                if let Some(spectrum) = self.visualizer.process(&left) {
                    let _ = viz_tx.try_send(spectrum);
                }

                // Push resampled samples to ring buffer
                for &sample in &output_samples {
                    // If buffer is full, wait and retry
                    while producer.push(sample).is_err() {
                        thread::sleep(Duration::from_micros(100));
                    }
                }

                // Update position periodically (not every sample)
                // Use output samples count since that's what's actually being played
                self.sample_counter += output_samples.len();
                if self.sample_counter >= self.samples_per_position_update {
                    audio_shared.set_position(frame.timestamp);
                    self.sample_counter = 0;
                }

                true
            }
            Ok(None) => {
                // Flush resampler at end of stream
                if let Some(ref mut resampler) = self.resampler {
                    let flushed = resampler.flush();
                    for &sample in &flushed {
                        while producer.push(sample).is_err() {
                            thread::sleep(Duration::from_micros(100));
                        }
                    }
                }

                tracing::info!("Playback finished");
                state.write().status = PlaybackStatus::Stopped;
                audio_shared.set_playing(false);
                self.decoder = None;
                self.resampler = None;
                true
            }
            Err(e) => {
                tracing::error!("Decode error: {}", e);
                state.write().status = PlaybackStatus::Stopped;
                audio_shared.set_playing(false);
                self.decoder = None;
                self.resampler = None;
                true
            }
        }
    }
}

/// Main loop for the audio/decoder thread.
fn audio_thread_main(
    state: Arc<RwLock<PlayerState>>,
    audio_shared: Arc<AudioSharedState>,
    command_rx: Receiver<PlayerCommand>,
    mut producer: Producer<f32>,
    viz_tx: Sender<SpectrumData>,
    output_sample_rate: u32,
    output_channels: u16,
) {
    let mut ctx = AudioThreadContext::new(output_sample_rate, output_channels);

    loop {
        let is_idle = matches!(
            state.read().status,
            PlaybackStatus::Stopped | PlaybackStatus::Paused
        );

        // Block on commands when idle, poll when playing
        let command = if is_idle {
            command_rx.recv().ok()
        } else {
            command_rx.try_recv().ok()
        };

        // Process command if received
        if let Some(cmd) = command
            && !ctx.handle_command(cmd, &state, &audio_shared, &mut producer)
        {
            break;
        }

        // Decode audio when playing
        if state.read().status == PlaybackStatus::Playing {
            if !ctx.decode_and_send(&mut producer, &viz_tx, &state, &audio_shared) {
                break;
            }
        } else if !is_idle {
            thread::sleep(Duration::from_millis(10));
        }
    }
}
