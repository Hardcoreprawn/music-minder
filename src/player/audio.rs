//! Audio output using cpal (WASAPI on Windows).
//!
//! This module runs the real-time audio thread that:
//! - Reads decoded audio from a ring buffer
//! - Applies volume
//! - Sends samples to the FFT analyzer
//! - Outputs to the audio device

use std::sync::Arc;
use std::path::PathBuf;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig, SampleFormat};
use crossbeam_channel::{Receiver, Sender, bounded};
use parking_lot::RwLock;

use super::decoder::AudioDecoder;
use super::state::{PlayerState, PlaybackStatus, PlayerCommand};
use super::visualization::SpectrumData;
use super::PlayerError;

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
        let supported_config = device.default_output_config()
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
        
        // Create audio buffer channel
        // This carries decoded audio from the decoder thread to the output callback
        let (audio_tx, audio_rx) = bounded::<AudioChunk>(8);
        
        // Clone state for the audio thread
        let state_for_thread = Arc::clone(&state);
        
        // Start the audio/decoder thread
        let audio_thread = thread::Builder::new()
            .name("audio-decoder".to_string())
            .spawn(move || {
                audio_thread_main(state_for_thread, command_rx, audio_tx, viz_tx, sample_rate);
            })
            .map_err(|e| PlayerError::AudioInit(e.to_string()))?;
        
        // State for the callback
        let callback_state = Arc::clone(&state);
        
        // Build output stream
        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(
                &device, &config, audio_rx, callback_state
            ),
            SampleFormat::I16 => build_stream_i16(
                &device, &config, audio_rx, callback_state
            ),
            format => {
                return Err(PlayerError::AudioInit(format!("Unsupported sample format: {:?}", format)));
            }
        }.map_err(|e| PlayerError::AudioInit(e.to_string()))?;
        
        // Start playback
        stream.play().map_err(|e| PlayerError::AudioInit(e.to_string()))?;
        
        Ok(Self {
            _stream: stream,
            _audio_thread: audio_thread,
        })
    }
    
    /// Select the best audio device - prefer headphones if available.
    fn select_best_device(host: &cpal::Host) -> Result<Device, PlayerError> {
        let devices: Vec<Device> = host.output_devices()
            .map_err(|e| PlayerError::AudioInit(e.to_string()))?
            .collect();
        
        // Log all available devices
        for device in &devices {
            if let Ok(name) = device.name() {
                tracing::info!("Available audio device: {}", name);
            }
        }
        
        // Look for headphones by name (case-insensitive)
        let headphone_keywords = ["headphone", "headset", "earphone", "airpod", "buds", "earbuds"];
        
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

/// A chunk of audio samples.
#[derive(Clone)]
struct AudioChunk {
    samples: Vec<f32>,
    timestamp: Duration,
}

/// Build output stream for f32 format.
fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    audio_rx: Receiver<AudioChunk>,
    state: Arc<RwLock<PlayerState>>,
) -> Result<Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    // Buffer state captured by the closure
    let mut chunk_buffer: Option<(AudioChunk, usize)> = None;

    device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            let volume = state.read().volume;
            let is_playing = state.read().status == PlaybackStatus::Playing;
            
            if !is_playing {
                // Output silence when paused
                for sample in data.iter_mut() {
                    *sample = T::from_sample(0.0f32);
                }
                return;
            }
            
            let mut output_pos = 0;
            
            while output_pos < data.len() {
                // Get or fetch a chunk
                if chunk_buffer.is_none() {
                    match audio_rx.try_recv() {
                        Ok(chunk) => {
                            // Update position from chunk timestamp
                            state.write().position = chunk.timestamp;
                            chunk_buffer = Some((chunk, 0));
                        }
                        Err(_) => {
                            // Underrun - fill with silence
                            state.write().underruns += 1;
                            for sample in &mut data[output_pos..] {
                                *sample = T::from_sample(0.0f32);
                            }
                            return;
                        }
                    }
                }
                
                if let Some((ref chunk, ref mut chunk_pos)) = chunk_buffer {
                    let remaining_in_chunk = chunk.samples.len() - *chunk_pos;
                    let remaining_in_output = data.len() - output_pos;
                    let to_copy = remaining_in_chunk.min(remaining_in_output);
                    
                    for i in 0..to_copy {
                        let sample = chunk.samples[*chunk_pos + i] * volume;
                        data[output_pos + i] = T::from_sample(sample);
                    }
                    
                    *chunk_pos += to_copy;
                    output_pos += to_copy;
                    
                    if *chunk_pos >= chunk.samples.len() {
                        chunk_buffer = None;
                    }
                }
            }
        },
        |err| {
            tracing::error!("Audio stream error: {}", err);
        },
        None,
    )
}

/// Build output stream for i16 format.
fn build_stream_i16(
    device: &Device,
    config: &StreamConfig,
    audio_rx: Receiver<AudioChunk>,
    state: Arc<RwLock<PlayerState>>,
) -> Result<Stream, cpal::BuildStreamError> {
    let mut chunk_buffer: Option<(AudioChunk, usize)> = None;
    
    device.build_output_stream(
        config,
        move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
            let volume = state.read().volume;
            let is_playing = state.read().status == PlaybackStatus::Playing;
            
            if !is_playing {
                for sample in data.iter_mut() {
                    *sample = 0;
                }
                return;
            }
            
            let mut output_pos = 0;
            
            while output_pos < data.len() {
                if chunk_buffer.is_none() {
                    match audio_rx.try_recv() {
                        Ok(chunk) => {
                            state.write().position = chunk.timestamp;
                            chunk_buffer = Some((chunk, 0));
                        }
                        Err(_) => {
                            state.write().underruns += 1;
                            for sample in &mut data[output_pos..] {
                                *sample = 0;
                            }
                            return;
                        }
                    }
                }
                
                if let Some((ref chunk, ref mut chunk_pos)) = chunk_buffer {
                    let remaining_in_chunk = chunk.samples.len() - *chunk_pos;
                    let remaining_in_output = data.len() - output_pos;
                    let to_copy = remaining_in_chunk.min(remaining_in_output);
                    
                    for i in 0..to_copy {
                        let sample = (chunk.samples[*chunk_pos + i] * volume * 32767.0) as i16;
                        data[output_pos + i] = sample;
                    }
                    
                    *chunk_pos += to_copy;
                    output_pos += to_copy;
                    
                    if *chunk_pos >= chunk.samples.len() {
                        chunk_buffer = None;
                    }
                }
            }
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
    visualizer: super::visualization::Visualizer,
    pending_path: Option<PathBuf>,
}

impl AudioThreadContext {
    fn new() -> Self {
        Self {
            decoder: None,
            visualizer: super::visualization::Visualizer::new(2048),
            pending_path: None,
        }
    }

    /// Handle a player command, returning whether to continue running
    fn handle_command(
        &mut self,
        cmd: PlayerCommand,
        state: &RwLock<PlayerState>,
    ) -> bool {
        match cmd {
            PlayerCommand::Load(path) => {
                self.pending_path = Some(path);
            }
            PlayerCommand::Play => {
                self.start_or_resume(state);
            }
            PlayerCommand::Pause => {
                state.write().status = PlaybackStatus::Paused;
            }
            PlayerCommand::Stop => {
                state.write().status = PlaybackStatus::Stopped;
                self.decoder = None;
            }
            PlayerCommand::Seek(pos) => {
                if let Some(ref mut dec) = self.decoder
                    && let Err(e) = dec.seek(pos) {
                        tracing::warn!("Seek failed: {}", e);
                    }
            }
            PlayerCommand::Shutdown => return false,
        }
        true
    }

    fn start_or_resume(&mut self, state: &RwLock<PlayerState>) {
        match self.pending_path.take() {
            Some(path) => self.load_and_play(path, state),
            None => state.write().status = PlaybackStatus::Playing,
        }
    }

    fn load_and_play(&mut self, path: PathBuf, state: &RwLock<PlayerState>) {
        match AudioDecoder::open(&path) {
            Ok(dec) => {
                let mut s = state.write();
                s.status = PlaybackStatus::Playing;
                s.current_track = Some(path);
                s.duration = dec.duration();
                s.position = Duration::ZERO;
                s.sample_rate = dec.sample_rate();
                s.channels = dec.channels();
                self.decoder = Some(dec);
            }
            Err(e) => {
                tracing::error!("Failed to open file: {}", e);
                state.write().status = PlaybackStatus::Stopped;
            }
        }
    }

    /// Decode next chunk and send to outputs. Returns false if playback ended.
    fn decode_and_send(
        &mut self,
        audio_tx: &Sender<AudioChunk>,
        viz_tx: &Sender<SpectrumData>,
        state: &RwLock<PlayerState>,
    ) -> bool {
        let Some(ref mut dec) = self.decoder else {
            return true;
        };

        let channels = dec.channels() as usize;
        let mut samples = Vec::with_capacity(4096);

        match dec.decode_next(|s| samples.extend_from_slice(s)) {
            Ok(Some(frame)) => {
                // Extract left channel for visualization
                let left: Vec<f32> = samples.iter().step_by(channels).copied().collect();
                if let Some(spectrum) = self.visualizer.process(&left) {
                    let _ = viz_tx.try_send(spectrum);
                }

                let chunk = AudioChunk {
                    samples,
                    timestamp: frame.timestamp,
                };

                // Channel closed = shutdown
                audio_tx.send(chunk).is_ok()
            }
            Ok(None) => {
                tracing::info!("Playback finished");
                state.write().status = PlaybackStatus::Stopped;
                self.decoder = None;
                true
            }
            Err(e) => {
                tracing::error!("Decode error: {}", e);
                state.write().status = PlaybackStatus::Stopped;
                self.decoder = None;
                true
            }
        }
    }
}

/// Main loop for the audio/decoder thread.
fn audio_thread_main(
    state: Arc<RwLock<PlayerState>>,
    command_rx: Receiver<PlayerCommand>,
    audio_tx: Sender<AudioChunk>,
    viz_tx: Sender<SpectrumData>,
    _output_sample_rate: u32,
) {
    let mut ctx = AudioThreadContext::new();

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
            && !ctx.handle_command(cmd, &state) {
                break;
            }

        // Decode audio when playing
        if state.read().status == PlaybackStatus::Playing {
            if !ctx.decode_and_send(&audio_tx, &viz_tx, &state) {
                break;
            }
        } else if !is_idle {
            thread::sleep(Duration::from_millis(10));
        }
    }
}
