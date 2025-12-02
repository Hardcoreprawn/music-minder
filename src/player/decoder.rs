//! Audio decoder using symphonia for multi-format support.
//!
//! Supported formats:
//! - MP3
//! - FLAC
//! - OGG Vorbis
//! - WAV/PCM
//! - AAC (in MP4 container)

use std::fs::File;
use std::path::Path;
use std::time::Duration;

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{CODEC_TYPE_NULL, Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

use super::PlayerError;
use super::state::TrackInfo;

/// Audio decoder wrapper for symphonia.
pub struct AudioDecoder {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    duration: Duration,
    time_base: Option<symphonia::core::units::TimeBase>,
}

impl AudioDecoder {
    /// Open a file for decoding.
    pub fn open(path: &Path) -> Result<Self, PlayerError> {
        // Create a media source stream
        let file = File::open(path)
            .map_err(|e| PlayerError::FileNotFound(format!("{}: {}", path.display(), e)))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Probe the format
        let mut hint = Hint::new();
        if let Some(ext) = path.extension() {
            hint.with_extension(&ext.to_string_lossy());
        }

        let format_opts = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let metadata_opts = MetadataOptions::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| PlayerError::UnsupportedFormat(e.to_string()))?;

        let reader = probed.format;

        // Find the first audio track
        let track = reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| PlayerError::UnsupportedFormat("No audio track found".to_string()))?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        // Get audio parameters
        let sample_rate = codec_params
            .sample_rate
            .ok_or_else(|| PlayerError::Decode("Unknown sample rate".to_string()))?;
        let channels = codec_params.channels.map(|c| c.count() as u16).unwrap_or(2);

        // Calculate duration
        let time_base = codec_params.time_base;
        let duration = if let Some(n_frames) = codec_params.n_frames {
            if let Some(tb) = time_base {
                let time = tb.calc_time(n_frames);
                Duration::from_secs_f64(time.seconds as f64 + time.frac)
            } else {
                // Estimate from sample rate
                Duration::from_secs_f64(n_frames as f64 / sample_rate as f64)
            }
        } else {
            Duration::ZERO
        };

        // Create decoder
        let decoder_opts = DecoderOptions::default();
        let decoder = symphonia::default::get_codecs()
            .make(&codec_params, &decoder_opts)
            .map_err(|e| PlayerError::Decode(e.to_string()))?;

        Ok(Self {
            reader,
            decoder,
            track_id,
            sample_rate,
            channels,
            duration,
            time_base,
        })
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the number of channels.
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Get the total duration.
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Get track metadata.
    pub fn metadata(&mut self) -> TrackInfo {
        // Extract metadata from the format reader
        let mut info = TrackInfo::default();

        if let Some(metadata) = self.reader.metadata().current() {
            for tag in metadata.tags() {
                match tag.std_key {
                    Some(symphonia::core::meta::StandardTagKey::TrackTitle) => {
                        info.title = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Artist) => {
                        info.artist = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::Album) => {
                        info.album = Some(tag.value.to_string());
                    }
                    Some(symphonia::core::meta::StandardTagKey::TrackNumber) => {
                        if let Ok(n) = tag.value.to_string().parse() {
                            info.track_number = Some(n);
                        }
                    }
                    Some(symphonia::core::meta::StandardTagKey::Date) => {
                        // Try to parse year from date
                        let s = tag.value.to_string();
                        if let Some(year_str) = s.split('-').next()
                            && let Ok(y) = year_str.parse()
                        {
                            info.year = Some(y);
                        }
                    }
                    Some(symphonia::core::meta::StandardTagKey::Genre) => {
                        info.genre = Some(tag.value.to_string());
                    }
                    _ => {}
                }
            }
        }

        info
    }

    /// Seek to a position (0.0 - 1.0).
    pub fn seek(&mut self, position: f32) -> Result<(), PlayerError> {
        if self.duration.is_zero() {
            return Ok(());
        }

        let target_time = self.duration.as_secs_f64() * position as f64;
        let seek_to = SeekTo::Time {
            time: Time::from(target_time),
            track_id: Some(self.track_id),
        };

        self.reader
            .seek(SeekMode::Accurate, seek_to)
            .map_err(|e| PlayerError::Decode(format!("Seek failed: {}", e)))?;

        // Reset decoder state after seeking
        self.decoder.reset();

        Ok(())
    }

    /// Decode the next chunk of audio samples.
    ///
    /// Returns `Ok(None)` at end of stream.
    /// The callback receives interleaved f32 samples.
    pub fn decode_next<F>(&mut self, mut callback: F) -> Result<Option<DecodedFrame>, PlayerError>
    where
        F: FnMut(&[f32]),
    {
        loop {
            // Read next packet
            let packet = match self.reader.next_packet() {
                Ok(p) => p,
                Err(SymphoniaError::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(None); // End of stream
                }
                Err(SymphoniaError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(e) => return Err(PlayerError::Decode(e.to_string())),
            };

            // Skip packets from other tracks
            if packet.track_id() != self.track_id {
                continue;
            }

            // Calculate timestamp
            let timestamp = if let Some(tb) = self.time_base {
                let time = tb.calc_time(packet.ts());
                Duration::from_secs_f64(time.seconds as f64 + time.frac)
            } else {
                Duration::ZERO
            };

            // Decode packet
            let decoded = match self.decoder.decode(&packet) {
                Ok(d) => d,
                Err(SymphoniaError::DecodeError(_)) => continue, // Skip bad frame
                Err(e) => return Err(PlayerError::Decode(e.to_string())),
            };

            // Convert to f32 samples
            let channels = self.channels;
            let samples = Self::convert_to_f32(&decoded, channels);
            let frame = DecodedFrame {
                samples: samples.len() / channels as usize,
                timestamp,
            };

            callback(&samples);

            return Ok(Some(frame));
        }
    }

    /// Convert audio buffer to interleaved f32 samples.
    fn convert_to_f32(buffer: &AudioBufferRef, _channels: u16) -> Vec<f32> {
        match buffer {
            AudioBufferRef::F32(buf) => {
                let planes = buf.planes();
                let plane_slice = planes.planes();
                if plane_slice.is_empty() {
                    return Vec::new();
                }

                let frames = plane_slice[0].len();
                let num_channels = plane_slice.len();
                let mut output = Vec::with_capacity(frames * num_channels);

                for frame in 0..frames {
                    for plane in plane_slice {
                        output.push(plane[frame]);
                    }
                }

                output
            }
            AudioBufferRef::S16(buf) => {
                let planes = buf.planes();
                let mut output = Vec::with_capacity(buf.frames() * planes.planes().len());
                for frame in 0..buf.frames() {
                    for plane in planes.planes() {
                        output.push(plane[frame] as f32 / 32768.0);
                    }
                }
                output
            }
            AudioBufferRef::S24(buf) => {
                let planes = buf.planes();
                let mut output = Vec::with_capacity(buf.frames() * planes.planes().len());
                for frame in 0..buf.frames() {
                    for plane in planes.planes() {
                        output.push(plane[frame].0 as f32 / 8388608.0);
                    }
                }
                output
            }
            AudioBufferRef::S32(buf) => {
                let planes = buf.planes();
                let mut output = Vec::with_capacity(buf.frames() * planes.planes().len());
                for frame in 0..buf.frames() {
                    for plane in planes.planes() {
                        output.push(plane[frame] as f32 / 2147483648.0);
                    }
                }
                output
            }
            AudioBufferRef::U8(buf) => {
                let planes = buf.planes();
                let mut output = Vec::with_capacity(buf.frames() * planes.planes().len());
                for frame in 0..buf.frames() {
                    for plane in planes.planes() {
                        output.push((plane[frame] as f32 - 128.0) / 128.0);
                    }
                }
                output
            }
            _ => Vec::new(),
        }
    }
}

/// Information about a decoded frame.
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    /// Number of samples (per channel) decoded
    pub samples: usize,
    /// Timestamp of this frame
    pub timestamp: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_nonexistent_file() {
        let result = AudioDecoder::open(Path::new("/nonexistent/file.mp3"));
        assert!(result.is_err());
    }
}
