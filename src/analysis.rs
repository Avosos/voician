// ============================================================================
// analysis.rs — Real-time audio feature extraction
// ============================================================================
//
// Provides allocation-free functions for extracting expressive features
// from audio frames:
//
//   • RMS amplitude  — overall loudness
//   • Spectral centroid — "brightness" of the sound
//
// Also provides smoothing filters (exponential moving average) for all
// continuous parameters, preventing MIDI jitter.
//
// All functions operate on pre-allocated buffers and are safe for the
// real-time processing path.
// ============================================================================

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// RMS computation
// ---------------------------------------------------------------------------

/// Compute RMS (Root Mean Square) amplitude of a sample buffer.
///
/// RMS = sqrt( Σ(x²) / N )
///
/// Returns 0.0 for empty buffers.
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

// ---------------------------------------------------------------------------
// Spectral centroid analyzer (pre-allocated, real-time safe)
// ---------------------------------------------------------------------------

/// Pre-allocated spectral centroid analyzer.
///
/// The spectral centroid is the "center of mass" of the spectrum:
///
///   centroid = Σ(f_k · |X_k|) / Σ(|X_k|)
///
/// It correlates with perceived brightness — higher centroid = brighter sound.
/// Mapped to MIDI CC 74 (filter cutoff) for real-time timbre control.
pub struct SpectralAnalyzer {
    fft_size: usize,
    sample_rate: f32,

    fft_forward: Arc<dyn Fft<f32>>,

    // Pre-allocated work buffers (no allocations during process())
    fft_buffer: Vec<Complex<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    window: Vec<f32>, // Hann window for spectral leakage reduction
}

impl SpectralAnalyzer {
    /// Create a new spectral analyzer.
    ///
    /// * `window_size` – Must match the analysis window size (e.g. 2048).
    /// * `sample_rate` – Audio sample rate in Hz.
    pub fn new(window_size: usize, sample_rate: f32) -> Self {
        let fft_size = window_size.next_power_of_two();

        let mut planner = FftPlanner::<f32>::new();
        let fft_forward = planner.plan_fft_forward(fft_size);
        let scratch_len = fft_forward.get_inplace_scratch_len();

        // Pre-compute Hann window coefficients.
        let window: Vec<f32> = (0..window_size)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32
                    / (window_size - 1) as f32)
                    .cos())
            })
            .collect();

        SpectralAnalyzer {
            fft_size,
            sample_rate,
            fft_forward,
            fft_buffer: vec![Complex::new(0.0, 0.0); fft_size],
            fft_scratch: vec![Complex::new(0.0, 0.0); scratch_len],
            window,
        }
    }

    /// Compute the spectral centroid of `signal` in Hz.
    ///
    /// `signal` must have at least `window_size` samples (same size used at
    /// construction). Returns the centroid frequency in Hz, or 0.0 if the
    /// signal is too quiet for meaningful spectral analysis.
    pub fn compute_centroid(&mut self, signal: &[f32]) -> f32 {
        let n = self.window.len().min(signal.len());

        // Apply Hann window and load into FFT buffer.
        for i in 0..n {
            self.fft_buffer[i] = Complex::new(signal[i] * self.window[i], 0.0);
        }
        // Zero-pad remainder (if fft_size > window_size).
        for i in n..self.fft_size {
            self.fft_buffer[i] = Complex::new(0.0, 0.0);
        }

        // Forward FFT.
        self.fft_forward
            .process_with_scratch(&mut self.fft_buffer, &mut self.fft_scratch);

        // Compute spectral centroid from the positive-frequency half.
        // centroid = Σ(f_k · |X_k|) / Σ(|X_k|)
        let bin_count = self.fft_size / 2;
        let freq_resolution = self.sample_rate / self.fft_size as f32;

        let mut weighted_sum: f64 = 0.0;
        let mut magnitude_sum: f64 = 0.0;

        for k in 1..bin_count {
            let mag = (self.fft_buffer[k].re * self.fft_buffer[k].re
                + self.fft_buffer[k].im * self.fft_buffer[k].im)
                .sqrt() as f64;
            let freq = (k as f64) * freq_resolution as f64;
            weighted_sum += freq * mag;
            magnitude_sum += mag;
        }

        if magnitude_sum < 1e-10 {
            return 0.0;
        }

        (weighted_sum / magnitude_sum) as f32
    }
}

// ---------------------------------------------------------------------------
// Exponential Moving Average (EMA) smoother
// ---------------------------------------------------------------------------

/// A simple single-pole exponential moving average filter.
///
///   y[n] = α · x[n] + (1 − α) · y[n−1]
///
/// Used for smoothing pitch, amplitude, and centroid to prevent MIDI jitter.
/// α closer to 1.0 = faster tracking, more noise.
/// α closer to 0.0 = slower tracking, smoother output.
#[derive(Clone)]
pub struct Smoother {
    alpha: f32,
    value: f32,
    initialized: bool,
}

impl Smoother {
    /// Create a new smoother with the given α coefficient.
    pub fn new(alpha: f32) -> Self {
        assert!((0.0..=1.0).contains(&alpha), "alpha must be in [0, 1]");
        Smoother {
            alpha,
            value: 0.0,
            initialized: false,
        }
    }

    /// Feed a new sample and return the smoothed value.
    pub fn update(&mut self, input: f32) -> f32 {
        if !self.initialized {
            self.value = input;
            self.initialized = true;
        } else {
            self.value = self.alpha * input + (1.0 - self.alpha) * self.value;
        }
        self.value
    }

    /// Get the current smoothed value without updating.
    #[allow(dead_code)]
    pub fn current(&self) -> f32 {
        self.value
    }

    /// Reset the smoother to uninitialized state.
    pub fn reset(&mut self) {
        self.initialized = false;
        self.value = 0.0;
    }
}
