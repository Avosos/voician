// ============================================================================
// crepe.rs — CREPE neural network pitch detection via ONNX Runtime
// ============================================================================
//
// Wraps the CREPE (Convolutional Representation for Pitch Estimation) model
// for high-accuracy monophonic pitch detection. CREPE frames the pitch
// estimation problem as a classification task over 360 pitch bins covering
// C1 (32.70 Hz) to B7 (1975.5 Hz) at 20-cent resolution.
//
// Model specification (CREPE "full"):
//   • Input:  [1, 1024] float32 — 1024 mono samples at 16 kHz (64 ms frame)
//   • Output: [1, 360]  float32 — unnormalized logits over 360 pitch bins
//
// Each bin corresponds to a pitch in cents:
//   cents(i) = 1997.3794084376191 + i * 20       (i = 0..359)
//   frequency(cents) = 10.0 * 2^(cents / 1200)
//
// The confidence is the maximum softmax probability across all bins.
// The pitch is the weighted average of frequencies near the peak bin
// (Viterbi-style local refinement).
//
// ONNX Runtime session is created once at startup and reused for every
// inference call. The session is Send + Sync, safe for the DSP thread.
// ============================================================================

use anyhow::{Context, Result};
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::TensorRef;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// CREPE model expects audio at this sample rate.
pub const CREPE_SAMPLE_RATE: u32 = 16_000;

/// Number of samples per CREPE input frame (64 ms at 16 kHz).
pub const CREPE_FRAME_SIZE: usize = 1024;

/// Number of pitch bins in the CREPE output.
const NUM_BINS: usize = 360;

/// Starting pitch in cents for bin 0 (C1 ≈ 32.70 Hz).
const CENTS_OFFSET: f64 = 1997.3794084376191;

/// Cents spacing between adjacent bins.
const CENTS_PER_BIN: f64 = 20.0;

/// Number of neighboring bins to use for weighted-average pitch refinement.
/// 5 bins = ±2 bins = ±40 cents around the peak.
const REFINEMENT_RADIUS: usize = 2;

// ---------------------------------------------------------------------------
// Pre-computed lookup table: bin index → frequency (Hz)
// ---------------------------------------------------------------------------

/// Lazily computed frequency table for all 360 bins.
fn bin_to_frequency(bin: usize) -> f32 {
    let cents = CENTS_OFFSET + bin as f64 * CENTS_PER_BIN;
    // frequency = 10 * 2^(cents / 1200)
    let freq = 10.0_f64 * (2.0_f64).powf(cents / 1200.0);
    freq as f32
}

// ---------------------------------------------------------------------------
// CrepeDetector
// ---------------------------------------------------------------------------

/// CREPE pitch detector backed by ONNX Runtime inference.
///
/// Holds the ONNX session (loaded once) and pre-allocated buffers for
/// zero-allocation inference in the hot path.
pub struct CrepeDetector {
    session: Session,

    /// Pre-computed frequency for each of the 360 bins.
    freq_table: [f32; NUM_BINS],
}

impl CrepeDetector {
    // =======================================================================
    // Initialization
    // =======================================================================

    /// Load the CREPE ONNX model and initialize the detector.
    ///
    /// `model_path` should point to a `crepe_full.onnx` file.
    /// The session is configured for maximum inference speed with graph
    /// optimization level 3 and a single intra-op thread (to avoid
    /// contention with the audio thread).
    pub fn initialize(model_path: &str) -> Result<Self> {
        println!("[crepe] Loading ONNX model from: {}", model_path);

        let session = Session::builder()
            .context("Failed to create ONNX session builder")?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .context("Failed to set optimization level")?
            .with_intra_threads(1)
            .context("Failed to set intra-op threads")?
            .commit_from_file(model_path)
            .context(format!(
                "Failed to load CREPE model from '{}'. \
                 Ensure the file exists and is a valid ONNX model.",
                model_path
            ))?;

        // Log model input/output metadata.
        let inputs = session.inputs();
        let outputs = session.outputs();
        println!("[crepe] Model loaded successfully.");
        println!("[crepe]   Inputs:  {} ({:?})", inputs.len(),
            inputs.iter().map(|o| o.name()).collect::<Vec<_>>());
        println!("[crepe]   Outputs: {} ({:?})", outputs.len(),
            outputs.iter().map(|o| o.name()).collect::<Vec<_>>());

        // Pre-compute frequency lookup table.
        let mut freq_table = [0.0f32; NUM_BINS];
        for (i, f) in freq_table.iter_mut().enumerate() {
            *f = bin_to_frequency(i);
        }

        println!(
            "[crepe] Frequency range: {:.1} Hz (bin 0) — {:.1} Hz (bin {})",
            freq_table[0],
            freq_table[NUM_BINS - 1],
            NUM_BINS - 1,
        );

        Ok(CrepeDetector {
            session,
            freq_table,
        })
    }

    // =======================================================================
    // Inference
    // =======================================================================

    /// Run pitch detection on a single audio frame.
    ///
    /// # Arguments
    /// * `audio_frame` — Exactly [`CREPE_FRAME_SIZE`] (1024) mono f32 samples
    ///   at 16 kHz. The samples should be in the range [-1.0, 1.0].
    ///
    /// # Returns
    /// * `(frequency_hz, confidence)` — The detected pitch in Hz and a
    ///   confidence score in [0.0, 1.0]. If the model cannot determine a
    ///   pitch, returns (0.0, 0.0).
    pub fn detect_pitch(&mut self, audio_frame: &[f32]) -> (f32, f32) {
        debug_assert_eq!(
            audio_frame.len(),
            CREPE_FRAME_SIZE,
            "CREPE expects exactly {} samples, got {}",
            CREPE_FRAME_SIZE,
            audio_frame.len(),
        );

        // -- Normalize the frame (zero-mean, unit-variance) --
        // CREPE training normalizes each frame independently.
        let mut normalized = [0.0f32; CREPE_FRAME_SIZE];
        let mean = audio_frame.iter().sum::<f32>() / CREPE_FRAME_SIZE as f32;
        let variance = audio_frame
            .iter()
            .map(|&s| (s - mean) * (s - mean))
            .sum::<f32>()
            / CREPE_FRAME_SIZE as f32;
        let std_dev = variance.sqrt().max(1e-8); // avoid division by zero

        for (i, &s) in audio_frame.iter().enumerate() {
            normalized[i] = (s - mean) / std_dev;
        }

        // -- Create input tensor [1, 1024] --
        let input_tensor = match TensorRef::from_array_view(
            ([1usize, CREPE_FRAME_SIZE], &normalized[..]),
        ) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("[crepe] Failed to create input tensor: {}", e);
                return (0.0, 0.0);
            }
        };

        // -- Run inference --
        let outputs = match self.session.run(ort::inputs![input_tensor]) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[crepe] Inference failed: {}", e);
                return (0.0, 0.0);
            }
        };

        // -- Extract output logits [1, 360] and copy to owned Vec --
        let logits_owned: Vec<f32> = {
            let (shape, logits) = match outputs[0].try_extract_tensor::<f32>() {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("[crepe] Failed to extract output tensor: {}", e);
                    return (0.0, 0.0);
                }
            };

            // Validate shape.
            let total_elements: usize = shape.iter().map(|&d| d as usize).product();
            if total_elements != NUM_BINS {
                eprintln!(
                    "[crepe] Unexpected output shape {:?} (expected {} elements)",
                    shape, NUM_BINS,
                );
                return (0.0, 0.0);
            }

            logits.to_vec()
        };
        // `outputs` is no longer borrowed after this point.
        drop(outputs);

        // -- Use model output directly as activations --
        // CREPE's final layer uses sigmoid activation, so each output
        // value is already a probability in [0, 1]. No softmax needed.
        let activations = logits_owned;

        // -- Find peak bin --
        let (peak_bin, peak_activation) = activations
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .unwrap_or((0, &0.0));

        let confidence = *peak_activation;

        // -- Weighted-average pitch refinement around peak --
        let frequency = self.refine_pitch(&activations, peak_bin);

        (frequency, confidence)
    }

    // =======================================================================
    // Pitch refinement
    // =======================================================================

    /// Compute a weighted average of frequencies around the peak bin for
    /// sub-bin pitch accuracy.
    fn refine_pitch(&self, probabilities: &[f32], peak_bin: usize) -> f32 {
        let lo = peak_bin.saturating_sub(REFINEMENT_RADIUS);
        let hi = (peak_bin + REFINEMENT_RADIUS).min(NUM_BINS - 1);

        let mut weighted_freq = 0.0f32;
        let mut weight_sum = 0.0f32;

        for bin in lo..=hi {
            let w = probabilities[bin];
            weighted_freq += w * self.freq_table[bin];
            weight_sum += w;
        }

        if weight_sum > 1e-8 {
            weighted_freq / weight_sum
        } else {
            self.freq_table[peak_bin]
        }
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Numerically stable softmax over a flat f32 slice.
#[allow(dead_code)]
fn softmax(logits: &[f32]) -> Vec<f32> {
    let max_logit = logits
        .iter()
        .cloned()
        .fold(f32::NEG_INFINITY, f32::max);

    let mut exp_values: Vec<f32> = logits.iter().map(|&x| (x - max_logit).exp()).collect();
    let sum: f32 = exp_values.iter().sum();

    if sum > 0.0 {
        for v in exp_values.iter_mut() {
            *v /= sum;
        }
    }

    exp_values
}

// ---------------------------------------------------------------------------
// Resampler: convert native sample rate → 16 kHz
// ---------------------------------------------------------------------------

/// Simple linear-interpolation resampler for converting audio from the
/// device's native sample rate down to CREPE's 16 kHz.
///
/// This is intentionally a basic resampler (no anti-aliasing filter) because:
/// - Voice fundamentals are well below 8 kHz (Nyquist at 16 kHz)
/// - The CREPE CNN is robust to minor aliasing artifacts
/// - Simplicity avoids heap allocations and latency
pub struct Resampler {
    /// Source sample rate (e.g. 44100, 48000).
    source_rate: f32,
    /// Target sample rate (always 16000).
    target_rate: f32,
    /// Ratio: source_rate / target_rate.
    ratio: f64,
    /// Fractional position in the source stream.
    position: f64,
    /// Carry-over buffer for samples straddling frame boundaries.
    carry_buffer: Vec<f32>,
}

impl Resampler {
    /// Create a new resampler from `source_rate` Hz to 16 kHz.
    pub fn new(source_rate: u32) -> Self {
        let source = source_rate as f32;
        let target = CREPE_SAMPLE_RATE as f32;
        Resampler {
            source_rate: source,
            target_rate: target,
            ratio: source as f64 / target as f64,
            position: 0.0,
            carry_buffer: Vec::with_capacity(4096),
        }
    }

    /// Returns the number of source samples needed to produce `target_frames`
    /// output samples (approximate upper bound).
    #[allow(dead_code)]
    pub fn source_samples_needed(&self, target_frames: usize) -> usize {
        ((target_frames as f64 * self.ratio).ceil() as usize) + 2
    }

    /// Resample a block of source audio, producing output at 16 kHz.
    ///
    /// Appends the source samples to an internal carry buffer, then generates
    /// as many 16 kHz output samples as possible via linear interpolation.
    /// Leftover source samples are retained for the next call.
    ///
    /// Returns the resampled output (may be empty if not enough source data).
    pub fn process(&mut self, source: &[f32]) -> Vec<f32> {
        self.carry_buffer.extend_from_slice(source);
        let mut output = Vec::new();
        let n = self.carry_buffer.len();

        while self.position < (n - 1) as f64 {
            let idx = self.position as usize;
            let frac = self.position as f32 - idx as f32;
            let sample = self.carry_buffer[idx] * (1.0 - frac)
                + self.carry_buffer[idx + 1] * frac;
            output.push(sample);
            self.position += self.ratio;
        }

        // Remove consumed samples, keep the tail.
        let consumed = self.position as usize;
        if consumed > 0 && consumed < self.carry_buffer.len() {
            self.carry_buffer.drain(0..consumed);
            self.position -= consumed as f64;
        } else if consumed >= self.carry_buffer.len() {
            let leftover = self.carry_buffer.len();
            self.position -= leftover as f64;
            self.carry_buffer.clear();
        }

        output
    }

    /// Reset the resampler state (e.g., after a silence gap).
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.position = 0.0;
        self.carry_buffer.clear();
    }

    /// Source sample rate.
    #[allow(dead_code)]
    pub fn source_rate(&self) -> f32 {
        self.source_rate
    }

    /// Target sample rate (always 16 kHz).
    #[allow(dead_code)]
    pub fn target_rate(&self) -> f32 {
        self.target_rate
    }
}

// ---------------------------------------------------------------------------
// Helper: frequency ↔ MIDI conversion
// ---------------------------------------------------------------------------

/// Convert a frequency in Hz to a MIDI note number (float).
/// A4 = 440 Hz = MIDI 69.0.
pub fn freq_to_midi(freq: f32) -> f32 {
    if freq <= 0.0 {
        return 0.0;
    }
    69.0 + 12.0 * (freq / 440.0).log2()
}
