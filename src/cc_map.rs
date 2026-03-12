// ============================================================================
// cc_map.rs — Multi-CC mapping from voice features
// ============================================================================
//
// Dubler 2-style CC control system:
//   • 4 assignable CC slots
//   • Each slot maps a voice feature → MIDI CC number
//   • Sources: envelope (RMS), brightness (centroid), pitch deviation, attack
//   • Configurable min/max scaling per slot
//   • Envelope follower with attack/release
// ============================================================================

// ---------------------------------------------------------------------------
// CC source
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CcSource {
    Off,
    /// RMS envelope (volume).
    Envelope,
    /// Spectral centroid (brightness/timbre).
    Brightness,
    /// Pitch deviation from nearest note (vibrato depth).
    PitchDeviation,
    /// Rate of pitch change (vibrato speed).
    PitchRate,
    /// Zero crossing rate (noisiness).
    Noisiness,
}

impl CcSource {
    pub fn label(&self) -> &'static str {
        match self {
            CcSource::Off => "Off",
            CcSource::Envelope => "Envelope",
            CcSource::Brightness => "Brightness",
            CcSource::PitchDeviation => "Vibrato Depth",
            CcSource::PitchRate => "Vibrato Speed",
            CcSource::Noisiness => "Noisiness",
        }
    }

    pub const ALL: &'static [CcSource] = &[
        CcSource::Off,
        CcSource::Envelope,
        CcSource::Brightness,
        CcSource::PitchDeviation,
        CcSource::PitchRate,
        CcSource::Noisiness,
    ];
}

// ---------------------------------------------------------------------------
// CC slot
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CcSlot {
    pub source: CcSource,
    /// MIDI CC number (0-127).
    pub cc_number: u8,
    /// Minimum output value (0-127).
    pub min_value: u8,
    /// Maximum output value (0-127).
    pub max_value: u8,
    /// Smoothing factor (0 = no smoothing, 0.99 = very smooth).
    pub smoothing: f32,
    /// Invert the mapping.
    pub inverted: bool,
    /// Current smoothed value.
    current: f32,
    /// Last sent CC value (for deadzone).
    last_sent: u8,
}

impl CcSlot {
    pub fn new(cc_number: u8) -> Self {
        CcSlot {
            source: CcSource::Off,
            cc_number,
            min_value: 0,
            max_value: 127,
            smoothing: 0.3,
            inverted: false,
            current: 0.0,
            last_sent: 0,
        }
    }

    /// Process a raw source value (0.0-1.0) and return the CC value to send,
    /// or None if unchanged (deadzone filtering).
    pub fn process(&mut self, raw_value: f32) -> Option<u8> {
        if self.source == CcSource::Off {
            return None;
        }

        let mut value = raw_value.clamp(0.0, 1.0);
        if self.inverted {
            value = 1.0 - value;
        }

        // Apply smoothing.
        self.current = self.smoothing * self.current + (1.0 - self.smoothing) * value;

        // Map to CC range.
        let range = self.max_value as f32 - self.min_value as f32;
        let cc = self.min_value as f32 + self.current * range;
        let cc_u8 = cc.round().clamp(0.0, 127.0) as u8;

        // Deadzone: only send if changed by at least 1.
        if cc_u8 != self.last_sent {
            self.last_sent = cc_u8;
            Some(cc_u8)
        } else {
            None
        }
    }

    pub fn reset(&mut self) {
        self.current = 0.0;
        self.last_sent = 0;
    }
}

// ---------------------------------------------------------------------------
// CC mapping engine
// ---------------------------------------------------------------------------

/// Number of CC mapping slots.
pub const NUM_CC_SLOTS: usize = 4;

pub struct CcMapEngine {
    pub slots: [CcSlot; NUM_CC_SLOTS],
    pub enabled: bool,

    // For pitch rate detection.
    prev_pitch: f32,
    pitch_rate_smoother: f32,
}

impl CcMapEngine {
    pub fn new() -> Self {
        CcMapEngine {
            slots: [
                CcSlot::new(1),   // CC 1 = Mod Wheel.
                CcSlot::new(74),  // CC 74 = Brightness.
                CcSlot::new(71),  // CC 71 = Resonance.
                CcSlot::new(11),  // CC 11 = Expression.
            ],
            enabled: true,

            prev_pitch: 0.0,
            pitch_rate_smoother: 0.0,
        }
    }

    /// Process all voice features and return CC messages to send.
    /// Returns: Vec of (cc_number, cc_value).
    pub fn process(&mut self, features: &VoiceFeatures) -> Vec<(u8, u8)> {
        if !self.enabled {
            return Vec::new();
        }

        // Compute pitch rate.
        let pitch_delta = (features.midi_float - self.prev_pitch).abs();
        self.prev_pitch = features.midi_float;
        self.pitch_rate_smoother = 0.8 * self.pitch_rate_smoother + 0.2 * pitch_delta;

        let mut results = Vec::new();

        for slot in self.slots.iter_mut() {
            let raw = match slot.source {
                CcSource::Off => continue,
                CcSource::Envelope => features.rms / 0.4, // Normalize: 0.4 = loud.
                CcSource::Brightness => {
                    (features.centroid_hz - 300.0) / 3700.0 // 300-4000 Hz → 0-1.
                }
                CcSource::PitchDeviation => {
                    (features.midi_float - features.midi_float.round()).abs() * 4.0
                }
                CcSource::PitchRate => {
                    (self.pitch_rate_smoother * 2.0).min(1.0)
                }
                CcSource::Noisiness => features.zcr,
            };

            if let Some(cc_val) = slot.process(raw) {
                results.push((slot.cc_number, cc_val));
            }
        }

        results
    }

    pub fn reset(&mut self) {
        for slot in self.slots.iter_mut() {
            slot.reset();
        }
        self.prev_pitch = 0.0;
        self.pitch_rate_smoother = 0.0;
    }

    /// Get the last sent value for a slot (for display).
    pub fn get_last_sent(&self, index: usize) -> u8 {
        self.slots[index].last_sent
    }
}

// ---------------------------------------------------------------------------
// Voice features (input to CC mapping)
// ---------------------------------------------------------------------------

/// Aggregated voice features for CC mapping.
pub struct VoiceFeatures {
    pub rms: f32,
    pub centroid_hz: f32,
    pub midi_float: f32,
    pub zcr: f32,
}
