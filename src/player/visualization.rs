//! Audio visualization using FFT analysis.
//!
//! Provides real-time spectrum analysis for visualizations like:
//! - Spectrum analyzer (frequency bars)
//! - VU meters
//! - Waveform display

use realfft::{RealFftPlanner, RealToComplex};
use std::sync::Arc;

/// Visualization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualizationMode {
    /// No visualization
    #[default]
    Off,
    /// Spectrum analyzer bars
    Spectrum,
    /// VU meter (peak levels)
    VuMeter,
    /// Waveform oscilloscope
    Waveform,
}

/// Spectrum data for visualization.
#[derive(Debug, Clone)]
pub struct SpectrumData {
    /// Frequency magnitudes (0.0 - 1.0), logarithmically scaled
    pub spectrum: Vec<f32>,
    /// Number of frequency bands
    pub bands: usize,
    /// Peak level (0.0 - 1.0)
    pub peak_level: f32,
    /// RMS level (0.0 - 1.0)
    pub rms_level: f32,
    /// Raw waveform samples for oscilloscope
    pub waveform: Vec<f32>,
}

impl Default for SpectrumData {
    fn default() -> Self {
        Self {
            spectrum: vec![0.0; 32],
            bands: 32,
            peak_level: 0.0,
            rms_level: 0.0,
            waveform: Vec::new(),
        }
    }
}

/// FFT-based audio analyzer.
pub struct Visualizer {
    /// FFT planner
    fft: Arc<dyn RealToComplex<f32>>,
    /// FFT size
    fft_size: usize,
    /// Input buffer for FFT
    input_buffer: Vec<f32>,
    /// Position in input buffer
    buffer_pos: usize,
    /// Output buffer for FFT (complex)
    output_buffer: Vec<rustfft::num_complex::Complex<f32>>,
    /// Scratch space for FFT
    scratch: Vec<rustfft::num_complex::Complex<f32>>,
    /// Window function (Hann)
    window: Vec<f32>,
    /// Number of output bands
    num_bands: usize,
    /// Previous spectrum for smoothing
    prev_spectrum: Vec<f32>,
    /// Smoothing factor (0.0 = no smoothing, 1.0 = frozen)
    smoothing: f32,
}

impl Visualizer {
    /// Create a new visualizer with the given FFT size.
    ///
    /// FFT size should be a power of 2 (e.g., 1024, 2048, 4096).
    pub fn new(fft_size: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        
        // Hann window for reduced spectral leakage
        let window: Vec<f32> = (0..fft_size)
            .map(|i| {
                let x = std::f32::consts::PI * 2.0 * i as f32 / (fft_size - 1) as f32;
                0.5 * (1.0 - x.cos())
            })
            .collect();
        
        let num_bands = 32; // Classic 32-band spectrum
        
        Self {
            fft,
            fft_size,
            input_buffer: vec![0.0; fft_size],
            buffer_pos: 0,
            output_buffer: vec![rustfft::num_complex::Complex::new(0.0, 0.0); fft_size / 2 + 1],
            scratch: vec![rustfft::num_complex::Complex::new(0.0, 0.0); fft_size],
            window,
            num_bands,
            prev_spectrum: vec![0.0; num_bands],
            smoothing: 0.7,
        }
    }

    /// Process audio samples and return spectrum data if ready.
    ///
    /// Call this with chunks of audio samples. Returns `Some` when enough
    /// samples have been collected for an FFT frame.
    pub fn process(&mut self, samples: &[f32]) -> Option<SpectrumData> {
        // Calculate levels from input
        let mut peak = 0.0f32;
        let mut sum_sq = 0.0f32;
        
        for &sample in samples {
            peak = peak.max(sample.abs());
            sum_sq += sample * sample;
        }
        
        let rms = if !samples.is_empty() {
            (sum_sq / samples.len() as f32).sqrt()
        } else {
            0.0
        };
        
        // Store waveform samples (downsampled for display)
        let waveform: Vec<f32> = if samples.len() > 256 {
            samples.iter().step_by(samples.len() / 256).copied().collect()
        } else {
            samples.to_vec()
        };
        
        // Add samples to FFT buffer
        for &sample in samples {
            self.input_buffer[self.buffer_pos] = sample;
            self.buffer_pos += 1;
            
            if self.buffer_pos >= self.fft_size {
                self.buffer_pos = 0;
                
                // Apply window function
                let mut windowed: Vec<f32> = self.input_buffer
                    .iter()
                    .zip(&self.window)
                    .map(|(s, w)| s * w)
                    .collect();
                
                // Perform FFT
                self.fft.process_with_scratch(
                    &mut windowed,
                    &mut self.output_buffer,
                    &mut self.scratch,
                ).ok()?;
                
                // Convert to magnitude spectrum with logarithmic frequency bands
                let spectrum = self.compute_bands();
                
                return Some(SpectrumData {
                    spectrum,
                    bands: self.num_bands,
                    peak_level: peak.min(1.0),
                    rms_level: rms.min(1.0),
                    waveform,
                });
            }
        }
        
        None
    }

    /// Compute logarithmically-spaced frequency bands.
    fn compute_bands(&mut self) -> Vec<f32> {
        let nyquist = self.output_buffer.len();
        let mut bands = vec![0.0f32; self.num_bands];
        
        // Logarithmic frequency mapping
        // Band 0 covers lowest frequencies, band N-1 covers highest
        for (band_idx, band) in bands.iter_mut().enumerate() {
            // Calculate frequency range for this band (logarithmic)
            let low_ratio = (band_idx as f32 / self.num_bands as f32).powf(2.0);
            let high_ratio = ((band_idx + 1) as f32 / self.num_bands as f32).powf(2.0);
            
            let low_bin = (low_ratio * nyquist as f32) as usize;
            let high_bin = (high_ratio * nyquist as f32).ceil() as usize;
            let high_bin = high_bin.min(nyquist);
            
            if low_bin >= high_bin {
                continue;
            }
            
            // Average magnitude in this band
            let mut sum = 0.0f32;
            for bin in low_bin..high_bin {
                let mag = self.output_buffer[bin].norm();
                sum += mag;
            }
            let avg = sum / (high_bin - low_bin) as f32;
            
            // Convert to dB scale (with floor at -60dB)
            let db = if avg > 0.0 {
                20.0 * avg.log10()
            } else {
                -60.0
            };
            
            // Normalize to 0.0 - 1.0 range (-60dB to 0dB)
            *band = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
        }
        
        // Apply smoothing (exponential moving average)
        for (i, band) in bands.iter_mut().enumerate() {
            *band = self.prev_spectrum[i] * self.smoothing + *band * (1.0 - self.smoothing);
            self.prev_spectrum[i] = *band;
        }
        
        bands
    }

    /// Set the number of frequency bands.
    pub fn set_bands(&mut self, bands: usize) {
        self.num_bands = bands.clamp(8, 128);
        self.prev_spectrum = vec![0.0; self.num_bands];
    }

    /// Set smoothing factor (0.0 = instant, 0.9 = very smooth).
    pub fn set_smoothing(&mut self, smoothing: f32) {
        self.smoothing = smoothing.clamp(0.0, 0.99);
    }

    /// Reset the visualizer state.
    pub fn reset(&mut self) {
        self.buffer_pos = 0;
        self.input_buffer.fill(0.0);
        self.prev_spectrum.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualizer_creation() {
        let viz = Visualizer::new(1024);
        assert_eq!(viz.fft_size, 1024);
        assert_eq!(viz.num_bands, 32);
    }

    #[test]
    fn test_visualizer_process() {
        let mut viz = Visualizer::new(256);
        
        // Generate a sine wave
        let samples: Vec<f32> = (0..512)
            .map(|i| (i as f32 * 0.1).sin())
            .collect();
        
        // First call might not produce output
        let result1 = viz.process(&samples[..256]);
        // Second call should have enough data
        let result2 = viz.process(&samples[256..]);
        
        // At least one should produce spectrum data
        assert!(result1.is_some() || result2.is_some());
    }

    #[test]
    fn test_spectrum_data_default() {
        let data = SpectrumData::default();
        assert_eq!(data.bands, 32);
        assert_eq!(data.spectrum.len(), 32);
        assert_eq!(data.peak_level, 0.0);
    }
}
