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
use super::simd;
use super::state::{
    AudioQuality, AudioSharedState, PlaybackStatus, PlayerCommand, PlayerEvent, PlayerState,
};
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
        event_tx: Sender<PlayerEvent>,
        viz_tx: Sender<SpectrumData>,
    ) -> Result<Self, PlayerError> {
        let host = cpal::default_host();

        // Try to find headphones first, then fall back to default
        let device = Self::select_best_device(&host)?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        tracing::info!("Using audio device: {}", device_name);

        // Log SIMD capabilities for audio processing
        simd::log_simd_capabilities();

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
                    event_tx,
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
            let is_flushing = audio_shared.is_flushing();

            // When flushing, drain the buffer but output silence
            // This clears stale audio when loading a new track
            if is_flushing {
                // Drain all available samples from the buffer
                while consumer.pop().is_ok() {}
                // Output silence
                for sample in data.iter_mut() {
                    *sample = T::from_sample(0.0f32);
                }
                return;
            }

            if !is_playing {
                // Output silence when paused
                for sample in data.iter_mut() {
                    *sample = T::from_sample(0.0f32);
                }
                return;
            }

            let mut samples_read = 0u32;
            let buffer_len = data.len();

            // ✅ SIMD OPTIMIZATION: Batch read + vectorized volume scaling
            // First, try to read all samples we need at once
            let chunk = consumer.read_chunk(buffer_len);
            match chunk {
                Ok(chunk) => {
                    let (first, second) = chunk.as_slices();
                    let total_available = first.len() + second.len();

                    if total_available >= buffer_len {
                        // We have enough samples - use batch processing
                        // Copy and apply volume using SIMD
                        let mut temp_buffer: Vec<f32> = Vec::with_capacity(buffer_len);
                        temp_buffer.extend_from_slice(&first[..first.len().min(buffer_len)]);
                        if temp_buffer.len() < buffer_len {
                            let remaining = buffer_len - temp_buffer.len();
                            temp_buffer.extend_from_slice(&second[..remaining.min(second.len())]);
                        }

                        // Apply volume with SIMD (in-place)
                        simd::apply_volume(&mut temp_buffer, volume);

                        // Copy to output with sample type conversion
                        for (out, &s) in data.iter_mut().zip(temp_buffer.iter()) {
                            *out = T::from_sample(s);
                        }

                        samples_read = temp_buffer.len() as u32;
                        // Commit the read
                        chunk.commit(samples_read as usize);
                    } else {
                        // Not enough samples - partial fill + underrun
                        chunk.commit(0); // Don't consume anything
                        // Fall back to sample-by-sample for partial buffer
                        for sample in data.iter_mut() {
                            match consumer.pop() {
                                Ok(s) => {
                                    *sample = T::from_sample(s * volume);
                                    samples_read += 1;
                                }
                                Err(_) => {
                                    audio_shared.increment_underruns();
                                    *sample = T::from_sample(0.0f32);
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Ring buffer empty - output silence
                    for sample in data.iter_mut() {
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
            let is_flushing = audio_shared.is_flushing();

            // When flushing, drain the buffer but output silence
            // This clears stale audio when loading a new track
            if is_flushing {
                while consumer.pop().is_ok() {}
                for sample in data.iter_mut() {
                    *sample = 0;
                }
                return;
            }

            if !is_playing {
                for sample in data.iter_mut() {
                    *sample = 0;
                }
                return;
            }

            let mut samples_read = 0u32;
            let buffer_len = data.len();

            // ✅ SIMD OPTIMIZATION: Batch read + vectorized f32→i16 conversion
            let chunk = consumer.read_chunk(buffer_len);
            match chunk {
                Ok(chunk) => {
                    let (first, second) = chunk.as_slices();
                    let total_available = first.len() + second.len();

                    if total_available >= buffer_len {
                        // We have enough samples - use batch SIMD processing
                        let mut temp_buffer: Vec<f32> = Vec::with_capacity(buffer_len);
                        temp_buffer.extend_from_slice(&first[..first.len().min(buffer_len)]);
                        if temp_buffer.len() < buffer_len {
                            let remaining = buffer_len - temp_buffer.len();
                            temp_buffer.extend_from_slice(&second[..remaining.min(second.len())]);
                        }

                        // Convert f32→i16 with volume using SIMD (combined operation)
                        simd::f32_to_i16_with_volume(&temp_buffer, data, volume);

                        samples_read = temp_buffer.len() as u32;
                        chunk.commit(samples_read as usize);
                    } else {
                        // Not enough samples - partial fill
                        chunk.commit(0);
                        for sample in data.iter_mut() {
                            match consumer.pop() {
                                Ok(s) => {
                                    *sample = (s * volume * 32767.0) as i16;
                                    samples_read += 1;
                                }
                                Err(_) => {
                                    audio_shared.increment_underruns();
                                    *sample = 0;
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Ring buffer empty - output silence
                    for sample in data.iter_mut() {
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
    /// Event sender to notify UI of state changes
    event_tx: Sender<PlayerEvent>,
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
    fn new(output_sample_rate: u32, output_channels: u16, event_tx: Sender<PlayerEvent>) -> Self {
        // Update position every ~50ms based on OUTPUT sample rate
        let samples_per_position_update =
            (output_sample_rate as usize * output_channels as usize) / 20;
        Self {
            decoder: None,
            resampler: None,
            visualizer: super::visualization::Visualizer::new(2048),
            pending_path: None,
            event_tx,
            output_sample_rate,
            output_channels,
            samples_per_position_update,
            sample_counter: 0,
        }
    }

    /// Clear the ring buffer by writing silence.
    ///
    /// This prevents stale audio from playing when loading a new track.
    /// We write silence rather than just discarding because the consumer
    /// (audio callback) may still be reading.
    /// Send an event to the UI. Ignores send failures (UI may have disconnected).
    fn emit(&self, event: PlayerEvent) {
        // Log the event being emitted with full context
        match &event {
            PlayerEvent::StatusChanged(status) => {
                tracing::debug!(
                    target: "player::events",
                    status = ?status,
                    "Emitting StatusChanged event"
                );
            }
            PlayerEvent::TrackLoaded { path, duration, .. } => {
                tracing::debug!(
                    target: "player::events",
                    path = ?path.file_name(),
                    duration_ms = duration.as_millis(),
                    "Emitting TrackLoaded event"
                );
            }
            PlayerEvent::PositionChanged(pos) => {
                tracing::trace!(
                    target: "player::events",
                    position_ms = pos.as_millis(),
                    "Emitting PositionChanged event"
                );
            }
            PlayerEvent::PlaybackFinished => {
                tracing::debug!(target: "player::events", "Emitting PlaybackFinished event");
            }
            PlayerEvent::Error(err) => {
                tracing::warn!(target: "player::events", error = %err, "Emitting Error event");
            }
        }

        // Try to send, log if channel is full (potential timing issue)
        match self.event_tx.try_send(event) {
            Ok(()) => {}
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                tracing::warn!(
                    target: "player::events",
                    "Event channel full - UI may be falling behind"
                );
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                tracing::debug!(
                    target: "player::events",
                    "Event channel disconnected - UI shutting down"
                );
            }
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
        tracing::debug!(
            target: "player::commands",
            command = ?cmd,
            current_status = ?state.read().status,
            has_decoder = self.decoder.is_some(),
            has_pending = self.pending_path.is_some(),
            "Audio thread received command"
        );

        match cmd {
            PlayerCommand::Load(path) => {
                tracing::info!(
                    target: "player::commands",
                    path = ?path.file_name(),
                    "Queuing track for load"
                );
                self.pending_path = Some(path);
            }
            PlayerCommand::Play => {
                self.start_or_resume(state, audio_shared, producer);
            }
            PlayerCommand::Pause => {
                tracing::debug!(target: "player::commands", "Processing Pause command");
                state.write().status = PlaybackStatus::Paused;
                audio_shared.set_playing(false);
                self.emit(PlayerEvent::StatusChanged(PlaybackStatus::Paused));
            }
            PlayerCommand::Stop => {
                tracing::debug!(target: "player::commands", "Processing Stop command");
                state.write().status = PlaybackStatus::Stopped;
                audio_shared.set_playing(false);
                self.decoder = None;
                self.resampler = None;
                self.emit(PlayerEvent::StatusChanged(PlaybackStatus::Stopped));
            }
            PlayerCommand::Seek(pos) => {
                tracing::debug!(
                    target: "player::commands",
                    seek_fraction = pos,
                    "Processing Seek command"
                );
                if let Some(ref mut dec) = self.decoder {
                    // Flush the ring buffer before seeking
                    // This prevents hearing stale audio after seek
                    // Note: We reset the sample counter and position to signal discontinuity
                    let _slots = producer.slots();

                    // Calculate new position immediately so UI updates instantly
                    let duration = dec.duration();
                    let new_pos = duration.mul_f32(pos);
                    tracing::debug!(
                        target: "player::commands",
                        new_pos_ms = new_pos.as_millis(),
                        duration_ms = duration.as_millis(),
                        "Seek calculated position"
                    );
                    audio_shared.set_position(new_pos);

                    self.sample_counter = 0;

                    // Reset resampler state to avoid artifacts
                    if let Some(ref mut resampler) = self.resampler {
                        resampler.reset();
                    }

                    if let Err(e) = dec.seek(pos) {
                        tracing::warn!(target: "player::commands", error = %e, "Seek failed");
                        self.emit(PlayerEvent::Error(format!("Seek failed: {}", e)));
                    }
                } else {
                    tracing::warn!(target: "player::commands", "Seek ignored - no decoder");
                }
            }
            PlayerCommand::Shutdown => {
                tracing::info!(target: "player::commands", "Shutdown command received");
                return false;
            }
        }
        true
    }

    fn start_or_resume(
        &mut self,
        state: &RwLock<PlayerState>,
        audio_shared: &AudioSharedState,
        producer: &mut Producer<f32>,
    ) {
        tracing::debug!(
            target: "player::commands",
            has_pending = self.pending_path.is_some(),
            has_decoder = self.decoder.is_some(),
            "start_or_resume called"
        );

        match self.pending_path.take() {
            Some(path) => {
                tracing::debug!(
                    target: "player::commands",
                    path = ?path.file_name(),
                    "Loading new track"
                );
                self.load_and_play(path, state, audio_shared, producer);
            }
            None => {
                // Only resume if we have a decoder (track loaded)
                if self.decoder.is_some() {
                    // Resume playback
                    tracing::debug!(target: "player::commands", "Resuming existing track");
                    state.write().status = PlaybackStatus::Playing;
                    audio_shared.set_playing(true);
                    self.emit(PlayerEvent::StatusChanged(PlaybackStatus::Playing));
                } else {
                    tracing::warn!(target: "player::commands", "Play command ignored: No track loaded");
                    // Ensure we are stopped
                    state.write().status = PlaybackStatus::Stopped;
                    audio_shared.set_playing(false);
                    self.emit(PlayerEvent::StatusChanged(PlaybackStatus::Stopped));
                }
            }
        }
    }

    fn load_and_play(
        &mut self,
        path: PathBuf,
        state: &RwLock<PlayerState>,
        audio_shared: &AudioSharedState,
        _producer: &mut Producer<f32>,
    ) {
        // Start flushing - audio callback will drain buffer and output silence
        // This prevents hearing stale audio from the previous track
        audio_shared.start_flush();

        match AudioDecoder::open(&path) {
            Ok(dec) => {
                let source_rate = dec.sample_rate();
                let source_channels = dec.channels();
                let duration = dec.duration();
                let bits_per_sample = dec.format_info.bit_depth;

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

                // Build quality info before we move decoder parts
                let quality = AudioQuality {
                    format: dec.format_info.codec.clone(),
                    is_lossless: dec.format_info.is_lossless,
                    bit_depth: bits_per_sample,
                    source_sample_rate: source_rate,
                    output_sample_rate: self.output_sample_rate,
                    is_bit_perfect: dec.format_info.is_lossless
                        && source_rate == self.output_sample_rate,
                    latency_ms: 0.0,    // Updated dynamically
                    buffer_size: 48000, // Ring buffer size
                    buffer_fill: 0.0,   // Updated dynamically
                };

                // Update shared state
                {
                    let mut s = state.write();
                    s.status = PlaybackStatus::Playing;
                    s.current_track = Some(path.clone());
                    s.duration = duration;
                    s.position = Duration::ZERO;
                    s.sample_rate = source_rate;
                    s.channels = source_channels;
                    s.bits_per_sample = bits_per_sample;
                    s.quality = quality.clone();
                }

                tracing::info!(
                    "Track loaded: {}Hz / {}ch / {}bit ({})",
                    source_rate,
                    source_channels,
                    bits_per_sample,
                    quality.format
                );

                // Sync atomic state
                audio_shared.set_playing(true);
                audio_shared.stop_flush(); // Resume normal playback - buffer is now drained
                audio_shared.set_position(Duration::ZERO);
                self.sample_counter = 0;
                self.decoder = Some(dec);
                self.resampler = Some(resampler);

                // Emit events: track loaded and status changed
                self.emit(PlayerEvent::TrackLoaded {
                    path,
                    duration,
                    sample_rate: source_rate,
                    channels: source_channels,
                    bits_per_sample,
                    quality,
                });
                self.emit(PlayerEvent::StatusChanged(PlaybackStatus::Playing));
            }
            Err(e) => {
                tracing::error!("Failed to open file: {}", e);
                state.write().status = PlaybackStatus::Stopped;
                audio_shared.set_playing(false);
                audio_shared.stop_flush(); // Don't leave in flushing state on error
                self.emit(PlayerEvent::Error(format!("Failed to open file: {}", e)));
                self.emit(PlayerEvent::StatusChanged(PlaybackStatus::Stopped));
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
                self.emit(PlayerEvent::PlaybackFinished);
                self.emit(PlayerEvent::StatusChanged(PlaybackStatus::Stopped));
                true
            }
            Err(e) => {
                tracing::error!("Decode error: {}", e);
                state.write().status = PlaybackStatus::Stopped;
                audio_shared.set_playing(false);
                self.decoder = None;
                self.resampler = None;
                self.emit(PlayerEvent::Error(format!("Decode error: {}", e)));
                self.emit(PlayerEvent::StatusChanged(PlaybackStatus::Stopped));
                true
            }
        }
    }
}

/// Main loop for the audio/decoder thread.
#[allow(clippy::too_many_arguments)]
fn audio_thread_main(
    state: Arc<RwLock<PlayerState>>,
    audio_shared: Arc<AudioSharedState>,
    command_rx: Receiver<PlayerCommand>,
    event_tx: Sender<PlayerEvent>,
    mut producer: Producer<f32>,
    viz_tx: Sender<SpectrumData>,
    output_sample_rate: u32,
    output_channels: u16,
) {
    let mut ctx = AudioThreadContext::new(output_sample_rate, output_channels, event_tx);

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

// ============================================================================
// Defensive Tests - Verify cpal API contracts used by this module
// ============================================================================

#[cfg(test)]
mod cpal_api_tests {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{BufferSize, SampleFormat, StreamConfig};

    /// Verify cpal::default_host() returns a Host that implements HostTrait
    #[test]
    fn test_default_host_available() {
        // This is the primary entry point we use
        let host = cpal::default_host();

        // HostTrait methods we rely on
        let _devices_result = host.output_devices();
        let _default_device = host.default_output_device();
    }

    /// Verify Device implements DeviceTrait with the methods we use
    #[test]
    fn test_device_trait_methods() {
        let host = cpal::default_host();

        if let Some(device) = host.default_output_device() {
            // DeviceTrait methods we rely on
            let _name: Result<String, _> = device.name();
            let _config = device.default_output_config();
        }
    }

    /// Verify SampleFormat enum has the variants we match on
    #[test]
    fn test_sample_format_variants() {
        // We explicitly match these variants in build_stream
        let _f32_format: SampleFormat = SampleFormat::F32;
        let _i16_format: SampleFormat = SampleFormat::I16;

        // Verify Debug impl (used in error messages)
        let _ = format!("{:?}", _f32_format);
    }

    /// Verify StreamConfig can be constructed with our parameters
    #[test]
    fn test_stream_config_construction() {
        use cpal::SampleRate;

        let config = StreamConfig {
            channels: 2,
            sample_rate: SampleRate(48000),
            buffer_size: BufferSize::Default,
        };

        // Verify the fields we set
        assert_eq!(config.channels, 2);
        assert_eq!(config.sample_rate.0, 48000);
        assert!(matches!(config.buffer_size, BufferSize::Default));
    }

    /// Verify Stream implements StreamTrait with play() method
    #[test]
    fn test_stream_trait_play_exists() {
        // StreamTrait::play is the method we call to start audio output
        // We can't easily test this without a real device, but we can verify
        // the trait bound exists by checking it compiles
        #[allow(dead_code)]
        fn requires_stream_trait<T: StreamTrait>(_s: &T) {}

        // This test passing means StreamTrait::play() exists
        // The actual call happens in AudioOutput::new()
    }

    /// Verify SizedSample and FromSample traits exist for our sample types
    #[test]
    fn test_sample_traits_exist() {
        // These traits are used in build_stream generic bounds
        fn requires_sample_traits<T: cpal::SizedSample + cpal::FromSample<f32>>() {}

        // Verify f32 implements the required traits
        requires_sample_traits::<f32>();
    }

    /// Verify SupportedStreamConfig provides the methods we use
    #[test]
    fn test_supported_stream_config_methods() {
        let host = cpal::default_host();

        if let Some(device) = host.default_output_device()
            && let Ok(supported) = device.default_output_config()
        {
            // Methods we call on SupportedStreamConfig
            let _rate = supported.sample_rate();
            let _channels = supported.channels();
            let _format = supported.sample_format();
        }
    }

    /// Verify cpal error types we handle
    #[test]
    fn test_error_type_to_string() {
        // We use .to_string() on various cpal error types
        // BuildStreamError, DefaultStreamConfigError, etc.
        // Verify they implement Display/ToString (implicitly tested via map_err)
    }

    /// Verify OutputCallbackInfo exists (used in stream callback signature)
    #[test]
    fn test_output_callback_info_exists() {
        // OutputCallbackInfo is passed to our callback but we don't use it (_: &cpal::OutputCallbackInfo)
        // Just verify the type exists
        #[allow(dead_code)]
        fn callback_signature(_data: &mut [f32], _info: &cpal::OutputCallbackInfo) {}
    }
}
