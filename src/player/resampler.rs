//! Audio resampler using rubato for high-quality sample rate conversion.
//!
//! This module handles converting audio from the source sample rate (e.g., 44.1kHz)
//! to the output device sample rate (e.g., 48kHz) to prevent pitch/speed issues.

use rubato::{FftFixedIn, Resampler as RubatoResampler};

/// Audio resampler wrapper.
pub struct Resampler {
    resampler: Option<FftFixedIn<f32>>,
    input_rate: u32,
    output_rate: u32,
    channels: usize,
    /// Input buffer for accumulating samples before resampling
    input_buffer: Vec<Vec<f32>>,
    /// Minimum samples needed for resampling
    chunk_size: usize,
}

impl Resampler {
    /// Create a new resampler.
    ///
    /// If input and output rates match, no resampling is performed.
    pub fn new(input_rate: u32, output_rate: u32, channels: u16) -> Self {
        let channels = channels as usize;
        
        if input_rate == output_rate {
            // No resampling needed
            return Self {
                resampler: None,
                input_rate,
                output_rate,
                channels,
                input_buffer: vec![Vec::new(); channels],
                chunk_size: 0,
            };
        }

        // Use a reasonable chunk size for resampling
        // Larger = more efficient but more latency
        let chunk_size = 1024;
        
        let resampler = FftFixedIn::<f32>::new(
            input_rate as usize,
            output_rate as usize,
            chunk_size,
            2, // Sub-chunks for async processing
            channels,
        )
        .expect("Failed to create resampler");

        tracing::info!(
            "Resampler: {}Hz → {}Hz ({} channels)",
            input_rate,
            output_rate,
            channels
        );

        Self {
            resampler: Some(resampler),
            input_rate,
            output_rate,
            channels,
            input_buffer: vec![Vec::new(); channels],
            chunk_size,
        }
    }

    /// Check if resampling is needed.
    pub fn needs_resampling(&self) -> bool {
        self.resampler.is_some()
    }

    /// Get the resampling ratio.
    pub fn ratio(&self) -> f64 {
        self.output_rate as f64 / self.input_rate as f64
    }

    /// Process interleaved samples, returning resampled interleaved output.
    ///
    /// Input: interleaved samples [L, R, L, R, ...]
    /// Output: resampled interleaved samples
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        // If no resampling needed, return input as-is
        let Some(ref mut resampler) = self.resampler else {
            return input.to_vec();
        };

        // Deinterleave input into per-channel buffers
        for (i, sample) in input.iter().enumerate() {
            let channel = i % self.channels;
            self.input_buffer[channel].push(*sample);
        }

        let mut output = Vec::new();

        // Process complete chunks
        while self.input_buffer[0].len() >= self.chunk_size {
            // Extract chunks from each channel
            let mut input_chunks: Vec<Vec<f32>> = Vec::with_capacity(self.channels);
            for ch_buf in &mut self.input_buffer {
                let chunk: Vec<f32> = ch_buf.drain(..self.chunk_size).collect();
                input_chunks.push(chunk);
            }

            // Resample
            match resampler.process(&input_chunks, None) {
                Ok(resampled) => {
                    // Interleave output
                    if !resampled.is_empty() && !resampled[0].is_empty() {
                        let frames = resampled[0].len();
                        for frame in 0..frames {
                            for ch in &resampled {
                                output.push(ch[frame]);
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Resampling error: {}", e);
                }
            }
        }

        output
    }

    /// Flush any remaining samples in the buffer.
    /// Call this at end of stream.
    pub fn flush(&mut self) -> Vec<f32> {
        let Some(ref mut resampler) = self.resampler else {
            return Vec::new();
        };

        // Pad remaining samples to chunk size
        let remaining = self.input_buffer[0].len();
        if remaining == 0 {
            return Vec::new();
        }

        let pad_needed = self.chunk_size - remaining;
        for ch_buf in &mut self.input_buffer {
            ch_buf.extend(std::iter::repeat_n(0.0, pad_needed));
        }

        // Process final chunk
        let input_chunks: Vec<Vec<f32>> = self.input_buffer.drain(..).collect();
        self.input_buffer = vec![Vec::new(); self.channels];

        let mut output = Vec::new();
        match resampler.process(&input_chunks, None) {
            Ok(resampled) => {
                if !resampled.is_empty() && !resampled[0].is_empty() {
                    // Only take the non-padded portion
                    let expected_frames = (remaining as f64 * self.ratio()).ceil() as usize;
                    let frames = resampled[0].len().min(expected_frames);
                    for frame in 0..frames {
                        for ch in &resampled {
                            output.push(ch[frame]);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Resampling flush error: {}", e);
            }
        }

        output
    }

    /// Reset the resampler state (call after seek).
    pub fn reset(&mut self) {
        for ch_buf in &mut self.input_buffer {
            ch_buf.clear();
        }
        if let Some(ref mut resampler) = self.resampler {
            resampler.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_resampling_when_rates_match() {
        let resampler = Resampler::new(48000, 48000, 2);
        assert!(!resampler.needs_resampling());
    }

    #[test]
    fn test_resampling_needed_when_rates_differ() {
        let resampler = Resampler::new(44100, 48000, 2);
        assert!(resampler.needs_resampling());
    }

    #[test]
    fn test_ratio_calculation() {
        let resampler = Resampler::new(44100, 48000, 2);
        let ratio = resampler.ratio();
        assert!((ratio - 48000.0 / 44100.0).abs() < 0.0001);
    }

    #[test]
    fn test_passthrough_when_no_resampling() {
        let mut resampler = Resampler::new(48000, 48000, 2);
        let input = vec![0.1, 0.2, 0.3, 0.4];
        let output = resampler.process(&input);
        assert_eq!(input, output);
    }
}
