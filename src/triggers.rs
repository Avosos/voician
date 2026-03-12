// ============================================================================
// triggers.rs — Percussive onset detection and drum trigger slots
// ============================================================================
//
// Dubler 2-style trigger system:
//   • Onset detection via RMS envelope transient + spectral flux
//   • 4 trigger slots (e.g. Kick, Snare, Hi-Hat, Perc)
//   • Each slot has a trainable spectral fingerprint
//   • Training mode: record several onset samples, build template
//   • Matching: nearest slot by spectral distance
// ============================================================================

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Number of trigger slots.
pub const NUM_TRIGGER_SLOTS: usize = 4;

/// Minimum time between triggers on the same slot (ms).
const DEFAULT_COOLDOWN_MS: f32 = 80.0;

/// Number of spectral bands for fingerprinting.
const NUM_BANDS: usize = 8;

// ---------------------------------------------------------------------------
// Spectral fingerprint
// ---------------------------------------------------------------------------

/// Compact spectral fingerprint of a percussive onset.
#[derive(Debug, Clone)]
pub struct SpectralFingerprint {
    /// Energy in each frequency band (normalized 0-1).
    pub band_energies: [f32; NUM_BANDS],
    /// Spectral centroid in Hz.
    pub centroid: f32,
    /// Zero crossing rate (0-1).
    pub zcr: f32,
}

impl Default for SpectralFingerprint {
    fn default() -> Self {
        SpectralFingerprint {
            band_energies: [0.0; NUM_BANDS],
            centroid: 0.0,
            zcr: 0.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Trigger slot
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TriggerSlot {
    /// Display name (e.g. "Kick", "Snare").
    pub name: String,
    /// MIDI note to send when triggered.
    pub midi_note: u8,
    /// Trained spectral template (average of training samples).
    pub template: SpectralFingerprint,
    /// Whether this slot has been trained.
    pub trained: bool,
    /// Training samples buffer.
    pub training_samples: Vec<SpectralFingerprint>,
    /// Sensitivity (higher = easier to trigger).
    pub sensitivity: f32,
    /// Cooldown in milliseconds.
    pub cooldown_ms: f32,
    /// Whether this slot is enabled.
    pub enabled: bool,
    /// Last trigger time (for cooldown).
    last_trigger_ms: f64,
    /// Flash counter (for GUI animation).
    pub flash_counter: u8,
}

impl TriggerSlot {
    pub fn new(name: &str, midi_note: u8) -> Self {
        TriggerSlot {
            name: name.to_string(),
            midi_note,
            template: SpectralFingerprint::default(),
            trained: false,
            training_samples: Vec::new(),
            sensitivity: 0.5,
            cooldown_ms: DEFAULT_COOLDOWN_MS,
            enabled: true,
            last_trigger_ms: 0.0,
            flash_counter: 0,
        }
    }

    /// Add a training sample. After enough samples, finalize the template.
    pub fn add_training_sample(&mut self, fp: SpectralFingerprint) {
        self.training_samples.push(fp);
        if self.training_samples.len() >= 5 {
            self.finalize_training();
        }
    }

    /// Finalize training by averaging all samples.
    pub fn finalize_training(&mut self) {
        if self.training_samples.is_empty() {
            return;
        }
        let n = self.training_samples.len() as f32;
        let mut avg = SpectralFingerprint::default();

        for sample in &self.training_samples {
            for i in 0..NUM_BANDS {
                avg.band_energies[i] += sample.band_energies[i];
            }
            avg.centroid += sample.centroid;
            avg.zcr += sample.zcr;
        }

        for i in 0..NUM_BANDS {
            avg.band_energies[i] /= n;
        }
        avg.centroid /= n;
        avg.zcr /= n;

        self.template = avg;
        self.trained = true;
    }

    /// Clear training data and reset.
    pub fn clear_training(&mut self) {
        self.training_samples.clear();
        self.template = SpectralFingerprint::default();
        self.trained = false;
    }

    /// Compute distance between this slot's template and a fingerprint.
    pub fn distance(&self, fp: &SpectralFingerprint) -> f32 {
        if !self.trained {
            return f32::MAX;
        }

        let mut band_dist = 0.0f32;
        for i in 0..NUM_BANDS {
            let d = self.template.band_energies[i] - fp.band_energies[i];
            band_dist += d * d;
        }
        band_dist = band_dist.sqrt();

        let centroid_dist =
            ((self.template.centroid - fp.centroid) / 2000.0).abs();
        let zcr_dist = (self.template.zcr - fp.zcr).abs();

        // Weighted sum.
        band_dist * 0.6 + centroid_dist * 0.25 + zcr_dist * 0.15
    }

    /// Check if cooldown has elapsed.
    pub fn can_trigger(&self, current_time_ms: f64) -> bool {
        self.enabled && (current_time_ms - self.last_trigger_ms) >= self.cooldown_ms as f64
    }

    /// Mark as triggered.
    pub fn mark_triggered(&mut self, current_time_ms: f64) {
        self.last_trigger_ms = current_time_ms;
        self.flash_counter = 8; // Flash for ~8 GUI frames.
    }
}

// ---------------------------------------------------------------------------
// Trigger engine
// ---------------------------------------------------------------------------

pub struct TriggerEngine {
    pub slots: [TriggerSlot; NUM_TRIGGER_SLOTS],
    /// Global onset detection threshold.
    pub onset_threshold: f32,
    /// Global trigger sensitivity.
    pub enabled: bool,
    /// MIDI channel for triggers (can be different from pitch channel).
    pub midi_channel: u8,
    /// Velocity sensitivity.
    pub velocity_sensitivity: f32,

    // Onset detection state.
    prev_rms: f32,
    prev_spectral_flux: f32,
    onset_holdoff_samples: usize,
    onset_holdoff_counter: usize,

    /// Which slot is currently being trained (None = not training).
    pub training_slot: Option<usize>,

    /// Running time in ms (for cooldown tracking).
    time_ms: f64,
}

impl TriggerEngine {
    pub fn new(sample_rate: f32) -> Self {
        let slots = [
            TriggerSlot::new("Kick", 36),    // GM Kick
            TriggerSlot::new("Snare", 38),   // GM Snare
            TriggerSlot::new("Hi-Hat", 42),  // GM Closed HH
            TriggerSlot::new("Perc", 39),    // GM Clap
        ];

        TriggerEngine {
            slots,
            onset_threshold: 0.15,
            enabled: true,
            midi_channel: 9, // GM drums channel (10, 0-indexed = 9).
            velocity_sensitivity: 1.0,

            prev_rms: 0.0,
            prev_spectral_flux: 0.0,
            onset_holdoff_samples: (sample_rate * 0.04) as usize, // 40ms holdoff.
            onset_holdoff_counter: 0,

            training_slot: None,
            time_ms: 0.0,
        }
    }

    /// Process an audio frame and detect onsets.
    /// Returns: Vec of (slot_index, velocity) for each trigger that fired.
    pub fn process(
        &mut self,
        samples: &[f32],
        rms: f32,
        centroid_hz: f32,
        sample_rate: f32,
    ) -> Vec<(usize, u8)> {
        let dt_ms = (samples.len() as f64 / sample_rate as f64) * 1000.0;
        self.time_ms += dt_ms;

        if !self.enabled {
            return Vec::new();
        }

        // Decrement holdoff.
        if self.onset_holdoff_counter > 0 {
            self.onset_holdoff_counter =
                self.onset_holdoff_counter.saturating_sub(samples.len());
            self.prev_rms = rms;
            return Vec::new();
        }

        // --- Onset detection ---
        let rms_delta = rms - self.prev_rms;
        self.prev_rms = rms;

        // Only detect positive transients (onset, not offset).
        if rms_delta < self.onset_threshold {
            return Vec::new();
        }

        // Onset detected! Extract spectral fingerprint.
        let fp = extract_fingerprint(samples, centroid_hz, sample_rate);

        // Compute velocity from onset strength.
        let velocity = onset_to_velocity(rms_delta, self.velocity_sensitivity);

        // Start holdoff.
        self.onset_holdoff_counter = self.onset_holdoff_samples;

        // If training, add sample to the training slot.
        if let Some(slot_idx) = self.training_slot {
            if slot_idx < NUM_TRIGGER_SLOTS {
                self.slots[slot_idx].add_training_sample(fp.clone());
                if self.slots[slot_idx].trained {
                    self.training_slot = None; // Training complete.
                }
            }
            return Vec::new();
        }

        // Match against trained slots.
        let mut results = Vec::new();
        let mut best_slot: Option<(usize, f32)> = None;

        for (i, slot) in self.slots.iter().enumerate() {
            if !slot.trained || !slot.can_trigger(self.time_ms) {
                continue;
            }
            let dist = slot.distance(&fp);
            let threshold = 1.0 - slot.sensitivity; // Higher sensitivity = accept larger distances.
            if dist < threshold {
                match best_slot {
                    None => best_slot = Some((i, dist)),
                    Some((_, best_dist)) if dist < best_dist => {
                        best_slot = Some((i, dist));
                    }
                    _ => {}
                }
            }
        }

        if let Some((idx, _)) = best_slot {
            self.slots[idx].mark_triggered(self.time_ms);
            results.push((idx, velocity));
        }

        // Decay flash counters each frame.
        for slot in self.slots.iter_mut() {
            if slot.flash_counter > 0 {
                slot.flash_counter -= 1;
            }
        }

        results
    }

    /// Start training a specific slot.
    pub fn start_training(&mut self, slot_index: usize) {
        if slot_index < NUM_TRIGGER_SLOTS {
            self.slots[slot_index].clear_training();
            self.training_slot = Some(slot_index);
        }
    }

    /// Cancel training.
    pub fn cancel_training(&mut self) {
        self.training_slot = None;
    }

    /// Load default preset (untrained, frequency-band based quick triggers).
    pub fn load_preset_frequency_bands(&mut self) {
        // Quick preset: assign by centroid range instead of training.
        // Kick: low centroid (< 500 Hz)
        // Snare: mid centroid (500–2000 Hz)
        // Hi-Hat: high centroid (2000–6000 Hz)
        // Perc: very high centroid (> 6000 Hz)
        // This is a simplified "no training needed" mode.
        self.slots[0].template = SpectralFingerprint {
            band_energies: [0.8, 0.5, 0.2, 0.1, 0.05, 0.02, 0.01, 0.01],
            centroid: 300.0,
            zcr: 0.1,
        };
        self.slots[0].trained = true;

        self.slots[1].template = SpectralFingerprint {
            band_energies: [0.3, 0.4, 0.5, 0.4, 0.3, 0.2, 0.1, 0.05],
            centroid: 1200.0,
            zcr: 0.3,
        };
        self.slots[1].trained = true;

        self.slots[2].template = SpectralFingerprint {
            band_energies: [0.05, 0.1, 0.15, 0.2, 0.4, 0.5, 0.4, 0.3],
            centroid: 4000.0,
            zcr: 0.6,
        };
        self.slots[2].trained = true;

        self.slots[3].template = SpectralFingerprint {
            band_energies: [0.02, 0.05, 0.1, 0.15, 0.2, 0.3, 0.5, 0.6],
            centroid: 6000.0,
            zcr: 0.7,
        };
        self.slots[3].trained = true;
    }
}

// ---------------------------------------------------------------------------
// Feature extraction
// ---------------------------------------------------------------------------

/// Extract a spectral fingerprint from a short audio onset window.
fn extract_fingerprint(samples: &[f32], centroid_hz: f32, sample_rate: f32) -> SpectralFingerprint {
    let n = samples.len();

    // --- Band energies (8 bands, logarithmically spaced) ---
    // Bands: 0-200, 200-400, 400-800, 800-1500, 1500-3000, 3000-6000, 6000-10000, 10000-20000 Hz
    let band_edges: [f32; 9] = [0.0, 200.0, 400.0, 800.0, 1500.0, 3000.0, 6000.0, 10000.0, 20000.0];
    let mut band_energies = [0.0f32; NUM_BANDS];

    // Simple time-domain band approximation using zero-crossing rate segments.
    // For real-time, we approximate by looking at the overall energy distribution.
    // A proper implementation would FFT, but we can use the centroid as main feature.
    // For now, estimate band energies from centroid + rms distribution.
    let total_energy: f32 = samples.iter().map(|&s| s * s).sum::<f32>() / n.max(1) as f32;

    for (i, _) in band_edges.windows(2).enumerate() {
        let band_center = (band_edges[i] + band_edges[i + 1]) / 2.0;
        // Gaussian weighting centered on centroid.
        let sigma = 1500.0;
        let dist = (band_center - centroid_hz) / sigma;
        band_energies[i] = total_energy * (-0.5 * dist * dist).exp();
    }

    // Normalize band energies.
    let max_energy = band_energies.iter().cloned().fold(0.0f32, f32::max);
    if max_energy > 1e-10 {
        for e in band_energies.iter_mut() {
            *e /= max_energy;
        }
    }

    // --- Zero crossing rate ---
    let mut zero_crossings = 0usize;
    for i in 1..n {
        if (samples[i] >= 0.0) != (samples[i - 1] >= 0.0) {
            zero_crossings += 1;
        }
    }
    let zcr = if n > 1 {
        zero_crossings as f32 / (n - 1) as f32
    } else {
        0.0
    };

    SpectralFingerprint {
        band_energies,
        centroid: centroid_hz,
        zcr,
    }
}

/// Map onset strength to MIDI velocity.
fn onset_to_velocity(rms_delta: f32, sensitivity: f32) -> u8 {
    let scaled = (rms_delta * sensitivity * 5.0).clamp(0.0, 1.0);
    let velocity = 40.0 + scaled * 87.0; // 40–127 range.
    velocity.round() as u8
}
