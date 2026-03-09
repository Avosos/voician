// ============================================================================
// pitch.rs — YIN pitch detection with FFT-based autocorrelation
// ============================================================================
//
// Implements the YIN fundamental frequency estimator (de Cheveigné & Kawahara,
// 2002) using FFT-based autocorrelation for O(N log N) performance.
//
// Pipeline:
//   1. FFT-based autocorrelation (zero-padded to avoid circular artifacts)
//   2. YIN difference function computed from autocorrelation + energy sums
//   3. Cumulative mean normalized difference (CMND)
//   4. Absolute threshold search for first minimum below threshold
//   5. Parabolic interpolation for sub-sample period accuracy
//   6. Frequency → MIDI note conversion
//
// All buffers are pre-allocated at construction time. No heap allocations
// occur during `detect()`, making it suitable for real-time use.
// ============================================================================

use rustfft::num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Result of a successful pitch detection.
#[derive(Debug, Clone, Copy)]
pub struct PitchResult {
    /// Detected fundamental frequency in Hz.
    pub frequency: f32,
    /// Confidence in the detection (0.0 = no confidence, 1.0 = perfect).
    pub confidence: f32,
    /// Exact MIDI note as a float (e.g. 69.3 = A4 + 30 cents).
    /// Used for pitch bend calculation.
<<<<<<< HEAD
=======
    #[allow(dead_code)]
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    pub midi_float: f32,
    /// Nearest MIDI note number (0–127).
    #[allow(dead_code)]
    pub midi_note: u8,
    /// Deviation from the nearest MIDI note in semitones (−0.5 to +0.5).
    #[allow(dead_code)]
    pub midi_deviation: f32,
}

/// Pre-allocated YIN pitch detector.
pub struct PitchDetector {
    // -- Configuration --
    window_size: usize,
    fft_size: usize,
    sample_rate: f32,
    min_period: usize, // sample_rate / max_freq
    max_period: usize, // sample_rate / min_freq
    threshold: f32,    // YIN aperiodicity threshold (lower = stricter)

    // -- FFT plans (thread-safe, shareable) --
    fft_forward: Arc<dyn Fft<f32>>,
    fft_inverse: Arc<dyn Fft<f32>>,

    // -- Pre-allocated work buffers --
    fft_buffer: Vec<Complex<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    autocorrelation: Vec<f32>,
    cumulative_energy: Vec<f32>,
    yin_buffer: Vec<f32>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl PitchDetector {
    /// Create a new pitch detector.
    ///
    /// # Arguments
    /// * `window_size` – Analysis window in samples (e.g. 2048).
    /// * `sample_rate` – Audio sample rate in Hz (e.g. 44100.0).
    /// * `min_freq`    – Lowest detectable frequency in Hz (e.g. 80.0).
    /// * `max_freq`    – Highest detectable frequency in Hz (e.g. 1000.0).
    /// * `threshold`   – YIN threshold (0.05–0.25 typical; lower = stricter).
    pub fn new(
        window_size: usize,
        sample_rate: f32,
        min_freq: f32,
        max_freq: f32,
        threshold: f32,
    ) -> Self {
        assert!(window_size >= 64, "window_size too small");
        assert!(min_freq > 0.0 && max_freq > min_freq);

        // FFT size: next power of two ≥ 2 × window_size (for zero-padded
        // linear autocorrelation via circular convolution).
        let fft_size = (2 * window_size).next_power_of_two();

        let mut planner = FftPlanner::<f32>::new();
        let fft_forward = planner.plan_fft_forward(fft_size);
        let fft_inverse = planner.plan_fft_inverse(fft_size);

        let scratch_len = fft_forward
            .get_inplace_scratch_len()
            .max(fft_inverse.get_inplace_scratch_len());

        let min_period = (sample_rate / max_freq).floor() as usize;
        let max_period = (sample_rate / min_freq).ceil() as usize;

        PitchDetector {
            window_size,
            fft_size,
            sample_rate,
            min_period: min_period.max(2),
            max_period: max_period.min(window_size / 2),
            threshold,

            fft_forward,
            fft_inverse,

            fft_buffer: vec![Complex::new(0.0, 0.0); fft_size],
            fft_scratch: vec![Complex::new(0.0, 0.0); scratch_len],
            autocorrelation: vec![0.0; window_size],
            cumulative_energy: vec![0.0; window_size],
            yin_buffer: vec![0.0; window_size / 2],
        }
    }

    /// Detect the fundamental frequency in `signal`.
    ///
    /// `signal` must contain at least `window_size` samples.
    /// Returns `None` if no reliable pitch is found.
    pub fn detect(&mut self, signal: &[f32]) -> Option<PitchResult> {
        debug_assert!(signal.len() >= self.window_size);
        let n = self.window_size;
        let half_n = n / 2;

        // ---- Step 1: FFT-based autocorrelation --------------------------------
        self.compute_autocorrelation(signal);

        // ---- Step 2: Cumulative energy (prefix sum of squares) ----------------
        self.cumulative_energy[0] = signal[0] * signal[0];
        for i in 1..n {
            self.cumulative_energy[i] =
                self.cumulative_energy[i - 1] + signal[i] * signal[i];
        }

        // ---- Step 3: YIN difference function ----------------------------------
        // d(τ) = Σ_j (x[j] − x[j+τ])²
        //      = cumE[N−1−τ] + (cumE[N−1] − cumE[τ−1]) − 2·r(τ)
        self.yin_buffer[0] = 0.0;
        let total_energy = self.cumulative_energy[n - 1];
        for tau in 1..half_n {
            let left_energy = self.cumulative_energy[n - 1 - tau];
            let right_energy = total_energy - self.cumulative_energy[tau - 1];
            let diff = left_energy + right_energy - 2.0 * self.autocorrelation[tau];
            // Clamp: small numerical errors can produce tiny negatives.
            self.yin_buffer[tau] = diff.max(0.0);
        }

        // ---- Step 4: Cumulative Mean Normalized Difference (CMND) -------------
        // d'(0) = 1 by convention.
        // d'(τ) = d(τ) · τ / Σ_{j=1}^{τ} d(j)
        self.yin_buffer[0] = 1.0;
        let mut running_sum: f32 = 0.0;
        for tau in 1..half_n {
            running_sum += self.yin_buffer[tau];
            if running_sum.abs() > 1e-10 {
                self.yin_buffer[tau] =
                    self.yin_buffer[tau] * (tau as f32) / running_sum;
            } else {
                self.yin_buffer[tau] = 1.0;
            }
        }

        // ---- Step 5: Absolute threshold search --------------------------------
        // Find the first τ ∈ [min_period, max_period) where d'(τ) < threshold,
        // then walk to the local minimum.
        let best_tau = self.find_best_period(half_n)?;

        // ---- Step 6: Parabolic interpolation for sub-sample accuracy ----------
        let tau_refined = parabolic_interpolation(&self.yin_buffer, best_tau, half_n);

        // ---- Step 7: Convert to frequency and MIDI note -----------------------
        if tau_refined <= 0.0 {
            return None;
        }
        let frequency = self.sample_rate / tau_refined;
        let confidence = (1.0 - self.yin_buffer[best_tau]).clamp(0.0, 1.0);

        // Reject out-of-range
        if frequency < 70.0 || frequency > 1100.0 {
            return None;
        }

        let midi_float = 69.0 + 12.0 * (frequency / 440.0).log2();
        if midi_float < 0.0 || midi_float > 127.0 {
            return None;
        }

        let midi_note = midi_float.round() as u8;
        let midi_deviation = midi_float - midi_note as f32; // in semitones (−0.5..+0.5)

        Some(PitchResult {
            frequency,
            confidence,
            midi_float,
            midi_note,
            midi_deviation,
        })
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Compute the (un-normalized) autocorrelation via FFT.
    ///
    /// r(τ) = Σ_{j=0}^{N−1−τ} x[j] · x[j+τ]
    ///
    /// Uses the Wiener-Khinchin approach:
    ///   1. Zero-pad signal → 2N
    ///   2. FFT
    ///   3. |X[k]|²
    ///   4. IFFT
    fn compute_autocorrelation(&mut self, signal: &[f32]) {
        let n = self.window_size;

        // Fill FFT buffer: signal + zero-padding.
        for i in 0..n {
            self.fft_buffer[i] = Complex::new(signal[i], 0.0);
        }
        for i in n..self.fft_size {
            self.fft_buffer[i] = Complex::new(0.0, 0.0);
        }

        // Forward FFT.
        self.fft_forward
            .process_with_scratch(&mut self.fft_buffer, &mut self.fft_scratch);

        // Power spectrum: |X[k]|².
        for c in self.fft_buffer.iter_mut() {
            let mag_sq = c.re * c.re + c.im * c.im;
            *c = Complex::new(mag_sq, 0.0);
        }

        // Inverse FFT.
        self.fft_inverse
            .process_with_scratch(&mut self.fft_buffer, &mut self.fft_scratch);

        // Extract real part and normalize by FFT size.
        let norm = 1.0 / self.fft_size as f32;
        for i in 0..n {
            self.autocorrelation[i] = self.fft_buffer[i].re * norm;
        }
    }

    /// Search for the best period (τ) using the YIN threshold method.
    fn find_best_period(&self, half_n: usize) -> Option<usize> {
        let search_max = self.max_period.min(half_n - 1);
        let mut tau = self.min_period;

        while tau < search_max {
            if self.yin_buffer[tau] < self.threshold {
                // Walk downhill to the local minimum.
                while tau + 1 < search_max
                    && self.yin_buffer[tau + 1] < self.yin_buffer[tau]
                {
                    tau += 1;
                }
                return Some(tau);
            }
            tau += 1;
        }

        // Fallback: if nothing was below threshold, find the global minimum
        // in the search range and accept it if reasonably low.
        let mut best_tau = self.min_period;
        let mut best_val = f32::MAX;
        for t in self.min_period..search_max {
            if self.yin_buffer[t] < best_val {
                best_val = self.yin_buffer[t];
                best_tau = t;
            }
        }
        if best_val < 0.5 {
            Some(best_tau)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Parabolic interpolation around index `tau` for sub-sample accuracy.
///
/// Fits a parabola through (τ−1, τ, τ+1) and returns the refined τ
/// at the minimum.
fn parabolic_interpolation(buffer: &[f32], tau: usize, len: usize) -> f32 {
    if tau < 1 || tau >= len - 1 {
        return tau as f32;
    }
    let s0 = buffer[tau - 1]; // d'(τ-1)
    let s1 = buffer[tau];     // d'(τ)
    let s2 = buffer[tau + 1]; // d'(τ+1)

    // Denominator of the parabolic vertex formula.
    let denom = 2.0 * s0 - 4.0 * s1 + 2.0 * s2;
    if denom.abs() < 1e-10 {
        return tau as f32;
    }

    tau as f32 + (s0 - s2) / denom
}

// ---------------------------------------------------------------------------
// Utility: MIDI ↔ frequency helpers (public for use by other modules)
// ---------------------------------------------------------------------------

/// Convert frequency in Hz to a floating-point MIDI note number.
/// A4 = 440 Hz = MIDI note 69.
#[allow(dead_code)]
pub fn freq_to_midi_float(freq: f32) -> f32 {
    69.0 + 12.0 * (freq / 440.0).log2()
}

/// Convert a MIDI note number to frequency in Hz.
#[allow(dead_code)]
pub fn midi_to_freq(note: u8) -> f32 {
    440.0 * 2.0f32.powf((note as f32 - 69.0) / 12.0)
}
