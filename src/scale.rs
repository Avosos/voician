// ============================================================================
// scale.rs — Musical scale quantization and key detection
// ============================================================================
//
// Provides:
//   • Scale definitions (major, minor, modes, pentatonic, blues, chromatic)
//   • Note quantization to nearest scale degree
//   • Auto key detection from recent note histogram
// ============================================================================

// ---------------------------------------------------------------------------
// Scale type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleType {
    Chromatic,
    Major,
    NaturalMinor,
    HarmonicMinor,
    MelodicMinor,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Pentatonic,
    MinorPentatonic,
    Blues,
}

impl ScaleType {
    pub fn label(&self) -> &'static str {
        match self {
            ScaleType::Chromatic => "Chromatic",
            ScaleType::Major => "Major",
            ScaleType::NaturalMinor => "Natural Minor",
            ScaleType::HarmonicMinor => "Harmonic Minor",
            ScaleType::MelodicMinor => "Melodic Minor",
            ScaleType::Dorian => "Dorian",
            ScaleType::Phrygian => "Phrygian",
            ScaleType::Lydian => "Lydian",
            ScaleType::Mixolydian => "Mixolydian",
            ScaleType::Pentatonic => "Pentatonic",
            ScaleType::MinorPentatonic => "Minor Pentatonic",
            ScaleType::Blues => "Blues",
        }
    }

    /// Semitone intervals from root that belong to this scale.
    pub fn intervals(&self) -> &'static [u8] {
        match self {
            ScaleType::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            ScaleType::Major => &[0, 2, 4, 5, 7, 9, 11],
            ScaleType::NaturalMinor => &[0, 2, 3, 5, 7, 8, 10],
            ScaleType::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            ScaleType::MelodicMinor => &[0, 2, 3, 5, 7, 9, 11],
            ScaleType::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            ScaleType::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            ScaleType::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            ScaleType::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            ScaleType::Pentatonic => &[0, 2, 4, 7, 9],
            ScaleType::MinorPentatonic => &[0, 3, 5, 7, 10],
            ScaleType::Blues => &[0, 3, 5, 6, 7, 10],
        }
    }

    pub const ALL: &'static [ScaleType] = &[
        ScaleType::Chromatic,
        ScaleType::Major,
        ScaleType::NaturalMinor,
        ScaleType::HarmonicMinor,
        ScaleType::MelodicMinor,
        ScaleType::Dorian,
        ScaleType::Phrygian,
        ScaleType::Lydian,
        ScaleType::Mixolydian,
        ScaleType::Pentatonic,
        ScaleType::MinorPentatonic,
        ScaleType::Blues,
    ];
}

// ---------------------------------------------------------------------------
// Root note
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootNote {
    C, Cs, D, Ds, E, F, Fs, G, Gs, A, As, B,
}

impl RootNote {
    pub fn semitone(&self) -> u8 {
        match self {
            RootNote::C => 0, RootNote::Cs => 1, RootNote::D => 2, RootNote::Ds => 3,
            RootNote::E => 4, RootNote::F => 5, RootNote::Fs => 6, RootNote::G => 7,
            RootNote::Gs => 8, RootNote::A => 9, RootNote::As => 10, RootNote::B => 11,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            RootNote::C => "C", RootNote::Cs => "C#", RootNote::D => "D",
            RootNote::Ds => "D#", RootNote::E => "E", RootNote::F => "F",
            RootNote::Fs => "F#", RootNote::G => "G", RootNote::Gs => "G#",
            RootNote::A => "A", RootNote::As => "A#", RootNote::B => "B",
        }
    }

    pub const ALL: &'static [RootNote] = &[
        RootNote::C, RootNote::Cs, RootNote::D, RootNote::Ds,
        RootNote::E, RootNote::F, RootNote::Fs, RootNote::G,
        RootNote::Gs, RootNote::A, RootNote::As, RootNote::B,
    ];

    pub fn from_semitone(s: u8) -> Self {
        RootNote::ALL[(s % 12) as usize]
    }
}

// ---------------------------------------------------------------------------
// Scale quantizer
// ---------------------------------------------------------------------------

pub struct ScaleQuantizer {
    /// Lookup table: for each pitch class (0-11), the nearest in-scale pitch
    /// class. Stored as signed offset in semitones (-6..+6).
    offset_table: [i8; 12],
}

impl ScaleQuantizer {
    /// Build a quantizer for the given root + scale.
    pub fn new(root: RootNote, scale: ScaleType) -> Self {
        let intervals = scale.intervals();
        let root_st = root.semitone();

        // Build set of in-scale pitch classes.
        let mut in_scale = [false; 12];
        for &iv in intervals {
            in_scale[((root_st + iv) % 12) as usize] = true;
        }

        // For each pitch class, find the nearest in-scale pitch class.
        let mut offset_table = [0i8; 12];
        for pc in 0..12u8 {
            if in_scale[pc as usize] {
                offset_table[pc as usize] = 0;
            } else {
                // Search outward: +1, -1, +2, -2, ...
                for dist in 1..=6i8 {
                    let up = ((pc as i8 + dist) % 12 + 12) % 12;
                    let down = ((pc as i8 - dist) % 12 + 12) % 12;
                    if in_scale[up as usize] {
                        offset_table[pc as usize] = dist;
                        break;
                    }
                    if in_scale[down as usize] {
                        offset_table[pc as usize] = -dist;
                        break;
                    }
                }
            }
        }

        ScaleQuantizer { offset_table }
    }

    /// Quantize a MIDI note to the nearest in-scale note.
    pub fn quantize(&self, midi_note: u8) -> u8 {
        let pc = midi_note % 12;
        let offset = self.offset_table[pc as usize];
        (midi_note as i8 + offset).clamp(0, 127) as u8
    }

    /// Quantize a floating-point MIDI value. Returns quantized note + residual
    /// deviation (for pitch bend).
    pub fn quantize_float(&self, midi_float: f32) -> (u8, f32) {
        let base_note = midi_float.round() as u8;
        let quantized = self.quantize(base_note);
        // The residual is how far the original float was from the quantized note.
        let residual = midi_float - quantized as f32;
        (quantized, residual)
    }
}

// ---------------------------------------------------------------------------
// Auto key detector
// ---------------------------------------------------------------------------

/// Detects the most likely key from a running histogram of played notes.
pub struct KeyDetector {
    /// Histogram of pitch classes (0–11), weighted by duration/confidence.
    histogram: [f32; 12],
    /// Exponential decay factor per update (keeps it recent).
    decay: f32,
}

impl KeyDetector {
    pub fn new() -> Self {
        KeyDetector {
            histogram: [0.0; 12],
            decay: 0.995,
        }
    }

    /// Feed a detected MIDI note (weighted by confidence).
    pub fn feed(&mut self, midi_note: u8, confidence: f32) {
        // Decay all bins.
        for h in self.histogram.iter_mut() {
            *h *= self.decay;
        }
        // Accumulate.
        let pc = (midi_note % 12) as usize;
        self.histogram[pc] += confidence;
    }

    /// Detect the most likely key (root note) for the major scale.
    /// Uses the Krumhansl-Schmuckler key-finding algorithm (simplified).
    pub fn detect(&self) -> (RootNote, ScaleType, f32) {
        // Major and minor profiles (Krumhansl).
        const MAJOR_PROFILE: [f32; 12] = [
            6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
        ];
        const MINOR_PROFILE: [f32; 12] = [
            6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
        ];

        let mut best_root = RootNote::C;
        let mut best_scale = ScaleType::Major;
        let mut best_score = f32::NEG_INFINITY;

        for root in 0..12u8 {
            // Correlate histogram (rotated by root) with profiles.
            let major_score = correlate(&self.histogram, &MAJOR_PROFILE, root);
            let minor_score = correlate(&self.histogram, &MINOR_PROFILE, root);

            if major_score > best_score {
                best_score = major_score;
                best_root = RootNote::from_semitone(root);
                best_scale = ScaleType::Major;
            }
            if minor_score > best_score {
                best_score = minor_score;
                best_root = RootNote::from_semitone(root);
                best_scale = ScaleType::NaturalMinor;
            }
        }

        let confidence = if best_score > 0.0 { (best_score / 10.0).min(1.0) } else { 0.0 };
        (best_root, best_scale, confidence)
    }

    pub fn reset(&mut self) {
        self.histogram = [0.0; 12];
    }
}

/// Pearson correlation between histogram (rotated by offset) and profile.
fn correlate(histogram: &[f32; 12], profile: &[f32; 12], offset: u8) -> f32 {
    let n = 12.0f32;
    let mut sum_hp = 0.0;
    let mut sum_h = 0.0;
    let mut sum_p = 0.0;
    let mut sum_h2 = 0.0;
    let mut sum_p2 = 0.0;

    for i in 0..12 {
        let h = histogram[((i as u8 + offset) % 12) as usize];
        let p = profile[i];
        sum_hp += h * p;
        sum_h += h;
        sum_p += p;
        sum_h2 += h * h;
        sum_p2 += p * p;
    }

    let num = n * sum_hp - sum_h * sum_p;
    let den = ((n * sum_h2 - sum_h * sum_h) * (n * sum_p2 - sum_p * sum_p)).sqrt();
    if den < 1e-10 { 0.0 } else { num / den }
}
