// ============================================================================
// chords.rs — Chord generation from single notes
// ============================================================================
//
// Dubler 2-style chord system:
//   • Map a single detected note to a full chord (multiple MIDI notes)
//   • Library of chord presets (major, minor, 7th, sus, etc.)
//   • Custom chord builder
//   • Voicing options (root position, inversions, spread)
// ============================================================================

// ---------------------------------------------------------------------------
// Chord type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordType {
    Off,
    Major,
    Minor,
    Diminished,
    Augmented,
    Sus2,
    Sus4,
    Major7,
    Minor7,
    Dominant7,
    Add9,
    Power,
    Octave,
}

impl ChordType {
    pub fn label(&self) -> &'static str {
        match self {
            ChordType::Off => "Off",
            ChordType::Major => "Major",
            ChordType::Minor => "Minor",
            ChordType::Diminished => "Dim",
            ChordType::Augmented => "Aug",
            ChordType::Sus2 => "Sus2",
            ChordType::Sus4 => "Sus4",
            ChordType::Major7 => "Maj7",
            ChordType::Minor7 => "Min7",
            ChordType::Dominant7 => "Dom7",
            ChordType::Add9 => "Add9",
            ChordType::Power => "5th",
            ChordType::Octave => "Oct",
        }
    }

    /// Semitone intervals above root.
    pub fn intervals(&self) -> &'static [u8] {
        match self {
            ChordType::Off => &[0],
            ChordType::Major => &[0, 4, 7],
            ChordType::Minor => &[0, 3, 7],
            ChordType::Diminished => &[0, 3, 6],
            ChordType::Augmented => &[0, 4, 8],
            ChordType::Sus2 => &[0, 2, 7],
            ChordType::Sus4 => &[0, 5, 7],
            ChordType::Major7 => &[0, 4, 7, 11],
            ChordType::Minor7 => &[0, 3, 7, 10],
            ChordType::Dominant7 => &[0, 4, 7, 10],
            ChordType::Add9 => &[0, 4, 7, 14],
            ChordType::Power => &[0, 7],
            ChordType::Octave => &[0, 12],
        }
    }

    pub const ALL: &'static [ChordType] = &[
        ChordType::Off,
        ChordType::Major,
        ChordType::Minor,
        ChordType::Diminished,
        ChordType::Augmented,
        ChordType::Sus2,
        ChordType::Sus4,
        ChordType::Major7,
        ChordType::Minor7,
        ChordType::Dominant7,
        ChordType::Add9,
        ChordType::Power,
        ChordType::Octave,
    ];
}

// ---------------------------------------------------------------------------
// Voicing
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Voicing {
    RootPosition,
    FirstInversion,
    SecondInversion,
    Spread,
}

impl Voicing {
    pub fn label(&self) -> &'static str {
        match self {
            Voicing::RootPosition => "Root",
            Voicing::FirstInversion => "1st Inv",
            Voicing::SecondInversion => "2nd Inv",
            Voicing::Spread => "Spread",
        }
    }

    pub const ALL: &'static [Voicing] = &[
        Voicing::RootPosition,
        Voicing::FirstInversion,
        Voicing::SecondInversion,
        Voicing::Spread,
    ];
}

// ---------------------------------------------------------------------------
// Chord engine
// ---------------------------------------------------------------------------

pub struct ChordEngine {
    pub chord_type: ChordType,
    pub voicing: Voicing,
    pub enabled: bool,
    /// Currently sounding chord notes (for tracking Note Offs).
    pub active_notes: Vec<u8>,
}

impl ChordEngine {
    pub fn new() -> Self {
        ChordEngine {
            chord_type: ChordType::Off,
            voicing: Voicing::RootPosition,
            enabled: false,
            active_notes: Vec::new(),
        }
    }

    /// Given a root MIDI note, generate all chord notes.
    pub fn generate(&self, root_note: u8) -> Vec<u8> {
        if self.chord_type == ChordType::Off || !self.enabled {
            return vec![root_note];
        }

        let intervals = self.chord_type.intervals();
        let mut notes: Vec<u8> = intervals
            .iter()
            .map(|&iv| root_note.saturating_add(iv))
            .filter(|&n| n <= 127)
            .collect();

        // Apply voicing.
        match self.voicing {
            Voicing::RootPosition => {} // Default.
            Voicing::FirstInversion => {
                if notes.len() >= 2 {
                    // Move root up an octave.
                    notes[0] = (notes[0] + 12).min(127);
                    notes.sort();
                }
            }
            Voicing::SecondInversion => {
                if notes.len() >= 3 {
                    // Move root and third up an octave.
                    notes[0] = (notes[0] + 12).min(127);
                    notes[1] = (notes[1] + 12).min(127);
                    notes.sort();
                }
            }
            Voicing::Spread => {
                // Alternate octave displacement for wider voicing.
                for (i, note) in notes.iter_mut().enumerate() {
                    if i % 2 == 1 && *note + 12 <= 127 {
                        *note += 12;
                    }
                }
                notes.sort();
            }
        }

        notes
    }
}
