//! Abstract audio visualizations inspired by Winamp/Milkdrop.
//!
//! Features:
//! - Fractal/Mandelbrot-inspired spiral patterns that pulse with the beat
//! - Particle explosions and swirling galaxy effects
//! - 3D wave simulations like an ocean
//! - All reactive to audio frequency and amplitude

use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::mouse::Cursor;
use iced::{Color, Element, Length, Point, Rectangle, Size, Theme};

use crate::player::SpectrumData;
use super::state::VisualizationMode;
use super::messages::Message;

/// A visualization canvas widget with abstract generative art.
pub struct VisualizationCanvas {
    mode: VisualizationMode,
    data: SpectrumData,
    time: f32,
}

impl VisualizationCanvas {
    pub fn new(mode: VisualizationMode, data: SpectrumData, time: f32) -> Self {
        Self { mode, data, time }
    }
}

/// Animation state for beat detection
#[derive(Default)]
pub struct AnimationState {
    beat_energy: f32,
    prev_beat: f32,
}

impl canvas::Program<Message> for VisualizationCanvas {
    type State = AnimationState;

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        
        // Deep dark background
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb(0.02, 0.02, 0.04),
        );
        
        match self.mode {
            VisualizationMode::Off => {}
            VisualizationMode::Spectrum => {
                self.draw_fractal_spectrum(&mut frame, bounds.size(), state);
            }
            VisualizationMode::Waveform => {
                self.draw_ocean_waves(&mut frame, bounds.size());
            }
            VisualizationMode::VuMeter => {
                self.draw_particle_explosion(&mut frame, bounds.size(), state);
            }
        }
        
        vec![frame.into_geometry()]
    }
}

impl VisualizationCanvas {
    /// Fractal-inspired spectrum with Mandelbrot-like spiral patterns
    fn draw_fractal_spectrum(&self, frame: &mut Frame, size: Size, _state: &AnimationState) {
        let center_x = size.width / 2.0;
        let center_y = size.height / 2.0;
        let max_radius = size.width.min(size.height) * 0.45;
        
        // Get overall energy for pulsing
        let energy: f32 = self.data.spectrum.iter().sum::<f32>() / self.data.spectrum.len().max(1) as f32;
        let bass: f32 = self.data.spectrum.iter().take(4).sum::<f32>() / 4.0;
        
        // Draw multiple rotating spiral arms
        let num_arms = 6;
        let num_segments = 64;
        
        for arm in 0..num_arms {
            let arm_offset = (arm as f32 / num_arms as f32) * std::f32::consts::TAU;
            
            let mut builder = canvas::path::Builder::new();
            let mut first = true;
            
            for i in 0..num_segments {
                let t = i as f32 / num_segments as f32;
                let freq_idx = ((t * self.data.spectrum.len() as f32) as usize).min(self.data.spectrum.len().saturating_sub(1));
                let freq_val = self.data.spectrum.get(freq_idx).copied().unwrap_or(0.0);
                
                // Spiral with fractal distortion
                let base_angle = arm_offset + t * std::f32::consts::TAU * 2.0 + self.time * 0.5;
                let radius = t * max_radius * (0.3 + energy * 0.7);
                
                // Add fractal wobble based on frequency
                let wobble = (t * 8.0 + self.time * 2.0).sin() * freq_val * 30.0;
                let fractal_distort = (base_angle * 3.0 + self.time).cos() * freq_val * 20.0;
                
                let r = radius + wobble + fractal_distort;
                let x = center_x + base_angle.cos() * r;
                let y = center_y + base_angle.sin() * r;
                
                if first {
                    builder.move_to(Point::new(x, y));
                    first = false;
                } else {
                    builder.line_to(Point::new(x, y));
                }
            }
            
            let path = builder.build();
            
            // Color based on arm position and energy
            let hue = (arm as f32 / num_arms as f32 + self.time * 0.1) % 1.0;
            let saturation = 0.7 + bass * 0.3;
            let brightness = 0.5 + energy * 0.5;
            let color = hsv_to_rgb(hue, saturation, brightness);
            
            // Glow effect
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(Color::from_rgba(color.r, color.g, color.b, 0.3))
                    .with_width(8.0),
            );
            
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(color)
                    .with_width(2.0),
            );
        }
        
        // Central pulsing orb
        let orb_radius = 20.0 + bass * 60.0;
        let orb_color = hsv_to_rgb((self.time * 0.2) % 1.0, 0.8, 0.9);
        
        // Orb glow layers
        for i in 0..5 {
            let r = orb_radius + i as f32 * 8.0;
            let alpha = 0.3 - i as f32 * 0.05;
            frame.fill(
                &Path::circle(Point::new(center_x, center_y), r),
                Color::from_rgba(orb_color.r, orb_color.g, orb_color.b, alpha),
            );
        }
        
        // Inner bright core
        frame.fill(
            &Path::circle(Point::new(center_x, center_y), orb_radius * 0.5),
            Color::from_rgb(1.0, 1.0, 1.0),
        );
        
        // Frequency ring around center
        self.draw_frequency_ring(frame, center_x, center_y, orb_radius + 40.0);
    }
    
    /// Draw a ring of frequency bars around a center point
    fn draw_frequency_ring(&self, frame: &mut Frame, cx: f32, cy: f32, radius: f32) {
        let num_bars = self.data.spectrum.len().max(1);
        
        for (i, &level) in self.data.spectrum.iter().enumerate() {
            let angle = (i as f32 / num_bars as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            let bar_length = level * 80.0;
            
            let x1 = cx + angle.cos() * radius;
            let y1 = cy + angle.sin() * radius;
            let x2 = cx + angle.cos() * (radius + bar_length);
            let y2 = cy + angle.sin() * (radius + bar_length);
            
            let hue = i as f32 / num_bars as f32;
            let color = hsv_to_rgb(hue, 0.9, 0.5 + level * 0.5);
            
            frame.stroke(
                &Path::line(Point::new(x1, y1), Point::new(x2, y2)),
                Stroke::default()
                    .with_color(color)
                    .with_width(3.0),
            );
        }
    }
    
    /// Ocean waves - 3D perspective wave simulation
    fn draw_ocean_waves(&self, frame: &mut Frame, size: Size) {
        let num_waves = 20;
        let wave_spacing = size.height / (num_waves as f32 + 2.0);
        
        // Get audio data for wave modulation
        let bass: f32 = self.data.spectrum.iter().take(4).sum::<f32>() / 4.0;
        let mid: f32 = self.data.spectrum.iter().skip(8).take(8).sum::<f32>() / 8.0;
        let treble: f32 = self.data.spectrum.iter().skip(20).sum::<f32>() / 12.0;
        
        // Draw waves from back to front (3D perspective)
        for wave_idx in 0..num_waves {
            let depth = wave_idx as f32 / num_waves as f32; // 0 = far, 1 = near
            let y_base = size.height * 0.2 + wave_idx as f32 * wave_spacing;
            
            // Perspective scaling
            let amplitude = 15.0 + depth * 40.0 + bass * 50.0 * depth;
            let frequency = 0.02 + depth * 0.01;
            let speed = 2.0 + depth * 1.0;
            
            let mut builder = canvas::path::Builder::new();
            let segments = 100;
            
            for i in 0..=segments {
                let x = (i as f32 / segments as f32) * size.width;
                
                // Complex wave combining multiple frequencies
                let wave1 = (x * frequency + self.time * speed).sin();
                let wave2 = (x * frequency * 2.3 + self.time * speed * 1.5 + 1.0).sin() * 0.5;
                let wave3 = (x * frequency * 0.7 + self.time * speed * 0.7 + 2.0).sin() * 0.3;
                
                // Modulate with audio
                let freq_mod = if i < segments / 3 {
                    bass
                } else if i < segments * 2 / 3 {
                    mid
                } else {
                    treble
                };
                
                let y = y_base + (wave1 + wave2 + wave3) * amplitude * (0.5 + freq_mod);
                
                if i == 0 {
                    builder.move_to(Point::new(x, y));
                } else {
                    builder.line_to(Point::new(x, y));
                }
            }
            
            let path = builder.build();
            
            // Color gradient from deep blue to cyan based on depth
            let hue = 0.55 + depth * 0.1 + (self.time * 0.05).sin() * 0.05;
            let saturation = 0.8 - depth * 0.3;
            let brightness = 0.3 + depth * 0.5 + bass * 0.2;
            let alpha = 0.3 + depth * 0.5;
            let color = hsv_to_rgb(hue, saturation, brightness);
            
            // Glow for front waves
            if depth > 0.5 {
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(Color::from_rgba(color.r, color.g, color.b, alpha * 0.3))
                        .with_width(6.0),
                );
            }
            
            frame.stroke(
                &path,
                Stroke::default()
                    .with_color(Color::from_rgba(color.r, color.g, color.b, alpha))
                    .with_width(2.0 + depth * 2.0),
            );
        }
        
        // Add some foam/sparkle effects on peaks
        self.draw_foam_particles(frame, size, bass);
        
        // Horizon glow
        let gradient_height = size.height * 0.3;
        for i in 0..20 {
            let y = i as f32 / 20.0 * gradient_height;
            let alpha = 0.1 * (1.0 - i as f32 / 20.0);
            frame.fill_rectangle(
                Point::new(0.0, y),
                Size::new(size.width, gradient_height / 20.0),
                Color::from_rgba(0.1, 0.2, 0.4, alpha),
            );
        }
    }
    
    fn draw_foam_particles(&self, frame: &mut Frame, size: Size, intensity: f32) {
        use std::f32::consts::{E, PI};
        let num_particles = (20.0 + intensity * 50.0) as usize;
        
        for i in 0..num_particles {
            // Pseudo-random positions based on time and index
            // Using golden ratio approximation for pseudo-random distribution
            let seed = i as f32 * 1.618_034;
            let x = ((seed * 7.919 + self.time * 0.3).sin() * 0.5 + 0.5) * size.width;
            let y = ((seed * PI + self.time * 0.5).cos() * 0.5 + 0.5) * size.height * 0.6 + size.height * 0.3;
            let particle_size = 1.0 + (seed * E).sin().abs() * 3.0 * intensity;
            let alpha = 0.3 + (seed + self.time).sin().abs() * 0.5;
            
            frame.fill(
                &Path::circle(Point::new(x, y), particle_size),
                Color::from_rgba(0.9, 0.95, 1.0, alpha),
            );
        }
    }
    
    /// Particle explosion / supernova effect
    fn draw_particle_explosion(&self, frame: &mut Frame, size: Size, state: &AnimationState) {
        let center_x = size.width / 2.0;
        let center_y = size.height / 2.0;
        
        // Overall energy drives explosion intensity
        let energy: f32 = self.data.spectrum.iter().sum::<f32>() / self.data.spectrum.len().max(1) as f32;
        let bass: f32 = self.data.spectrum.iter().take(4).sum::<f32>() / 4.0;
        
        // Beat detection for bursts
        let is_beat = state.beat_energy > state.prev_beat * 1.2 && bass > 0.3;
        let burst_intensity = if is_beat { 2.0 } else { 1.0 };
        
        // Draw multiple particle rings
        let num_rings = 8;
        
        for ring in 0..num_rings {
            let ring_t = ring as f32 / num_rings as f32;
            let base_radius = 30.0 + ring_t * size.width.min(size.height) * 0.4;
            
            // Each ring has particles
            let particles_per_ring = 32 + ring * 8;
            
            for p in 0..particles_per_ring {
                let angle = (p as f32 / particles_per_ring as f32) * std::f32::consts::TAU;
                let freq_idx = p % self.data.spectrum.len().max(1);
                let freq_val = self.data.spectrum.get(freq_idx).copied().unwrap_or(0.0);
                
                // Particle position with audio-reactive radius
                let r = base_radius + freq_val * 100.0 * burst_intensity;
                let wobble = (angle * 5.0 + self.time * 3.0).sin() * freq_val * 20.0;
                let final_r = r + wobble;
                
                // Spiral motion
                let spiral_angle = angle + self.time * (0.5 + ring_t) + ring_t * std::f32::consts::PI;
                
                let x = center_x + spiral_angle.cos() * final_r;
                let y = center_y + spiral_angle.sin() * final_r;
                
                // Size based on frequency
                let particle_size = 2.0 + freq_val * 8.0 * burst_intensity;
                
                // Color: outer rings are cooler colors, inner are warmer
                let hue = (1.0 - ring_t) * 0.3 + (self.time * 0.1) % 1.0;
                let saturation = 0.8 + freq_val * 0.2;
                let brightness = 0.4 + freq_val * 0.6;
                let color = hsv_to_rgb(hue, saturation, brightness);
                
                // Glow effect
                frame.fill(
                    &Path::circle(Point::new(x, y), particle_size * 2.0),
                    Color::from_rgba(color.r, color.g, color.b, 0.2),
                );
                
                // Core particle
                frame.fill(
                    &Path::circle(Point::new(x, y), particle_size),
                    color,
                );
            }
        }
        
        // Central energy core
        let core_radius = 20.0 + energy * 40.0 + bass * 30.0;
        let core_hue = (self.time * 0.15) % 1.0;
        
        // Pulsing core with multiple layers
        for i in 0..6 {
            let r = core_radius * (1.0 + i as f32 * 0.3);
            let alpha = 0.4 - i as f32 * 0.06;
            let color = hsv_to_rgb(core_hue + i as f32 * 0.05, 0.9, 0.9);
            
            frame.fill(
                &Path::circle(Point::new(center_x, center_y), r),
                Color::from_rgba(color.r, color.g, color.b, alpha),
            );
        }
        
        // Bright white center
        frame.fill(
            &Path::circle(Point::new(center_x, center_y), core_radius * 0.3),
            Color::from_rgb(1.0, 1.0, 1.0),
        );
        
        // Energy rays shooting outward
        self.draw_energy_rays(frame, center_x, center_y, core_radius * 1.5, energy);
    }
    
    fn draw_energy_rays(&self, frame: &mut Frame, cx: f32, cy: f32, start_r: f32, energy: f32) {
        let num_rays = 12;
        
        for i in 0..num_rays {
            let base_angle = (i as f32 / num_rays as f32) * std::f32::consts::TAU;
            let angle = base_angle + self.time * 0.3;
            
            let freq_idx = i * 2 % self.data.spectrum.len().max(1);
            let freq_val = self.data.spectrum.get(freq_idx).copied().unwrap_or(0.0);
            
            let ray_length = 50.0 + freq_val * 150.0 + energy * 100.0;
            
            let x1 = cx + angle.cos() * start_r;
            let y1 = cy + angle.sin() * start_r;
            let x2 = cx + angle.cos() * (start_r + ray_length);
            let y2 = cy + angle.sin() * (start_r + ray_length);
            
            let hue = (i as f32 / num_rays as f32 + self.time * 0.1) % 1.0;
            let color = hsv_to_rgb(hue, 0.8, 0.9);
            
            // Ray glow
            frame.stroke(
                &Path::line(Point::new(x1, y1), Point::new(x2, y2)),
                Stroke::default()
                    .with_color(Color::from_rgba(color.r, color.g, color.b, 0.3))
                    .with_width(6.0),
            );
            
            // Ray core
            frame.stroke(
                &Path::line(Point::new(x1, y1), Point::new(x2, y2)),
                Stroke::default()
                    .with_color(color)
                    .with_width(2.0),
            );
        }
    }
}

/// Convert HSV to RGB color.
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let h = h.fract();
    if h < 0.0 { return hsv_to_rgb(h + 1.0, s, v); }
    
    let s = s.clamp(0.0, 1.0);
    let v = v.clamp(0.0, 1.0);
    
    let c = v * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = v - c;
    
    let (r, g, b) = match (h * 6.0) as i32 % 6 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    
    Color::from_rgb(r + m, g + m, b + m)
}

/// Create a visualization canvas element.
pub fn visualization_view<'a>(
    mode: VisualizationMode,
    data: &SpectrumData,
    height: f32,
) -> Element<'a, Message> {
    // Use a hash of the data to create unique cache key
    // This forces redraw when data changes
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    
    // Hash the spectrum data to detect changes
    for &v in &data.spectrum {
        v.to_bits().hash(&mut hasher);
    }
    data.peak_level.to_bits().hash(&mut hasher);
    let _data_hash = hasher.finish();
    
    // Time counter for animation
    static TIME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let frame = TIME.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let time = frame as f32 * 0.016; // ~60fps timing
    
    // Use cache key to force redraw
    Canvas::new(VisualizationCanvas::new(mode, data.clone(), time))
        .width(Length::Fill)
        .height(Length::Fixed(height))
        .into()
}
