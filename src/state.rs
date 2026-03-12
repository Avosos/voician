// ============================================================================
// state.rs — Shared state types for Voician v1.0 (Dubler-style)
// ============================================================================
//
// Communication model:
//   Engine → GUI:  EngineSnapshot via crossbeam channel (lock-free)
//   GUI → Engine:  EngineParams via Arc<Mutex<EngineParams>> (low contention)
//   MIDI log:      MidiLogEntry via crossbeam channel
// ============================================================================

use crossbeam_channel::{Receiver, Sender};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::cc_map::{CcSource, NUM_CC_SLOTS};
use crate::chords::{ChordType, Voicing};
use crate::scale::{RootNote, ScaleType};

// ---------------------------------------------------------------------------
// Pitch detection mode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchMode {
    Crepe,
    Yin,
    Hybrid,
}

impl PitchMode {
    pub fn label(&self) -> &'static str {
        match self {
            PitchMode::Crepe => "CREPE (Neural)",
            PitchMode::Yin => "YIN (Fast)",
            PitchMode::Hybrid => "Hybrid",
        }
    }
}

// ---------------------------------------------------------------------------
// Pitch bend mode (Dubler-style)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchBendMode {
    /// No pitch bend output.
    Off,
    /// IntelliBend: snaps to note, only bends when intentionally sliding between notes.
    /// Applies bend only during the transition to a new note.
    IntelliBend,
    /// TruBend: raw, continuous pitch-to-bend mapping.
    /// Follows exact vocal pitch at all times.
    TruBend,
}

impl PitchBendMode {
    pub fn label(&self) -> &'static str {
        match self {
            PitchBendMode::Off => "Off",
            PitchBendMode::IntelliBend => "IntelliBend",
            PitchBendMode::TruBend => "TruBend",
        }
    }
    pub const ALL: &'static [PitchBendMode] = &[
        PitchBendMode::Off,
        PitchBendMode::IntelliBend,
        PitchBendMode::TruBend,
    ];
}

// ---------------------------------------------------------------------------
// GUI tab
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiTab {
    Play,
    Key,
    Chords,
    Assign,
    Monitor,
}

impl GuiTab {
    pub fn label(&self) -> &'static str {
        match self {
            GuiTab::Play => "Play",
            GuiTab::Key => "Key",
            GuiTab::Chords => "Chords",
            GuiTab::Assign => "Assign",
            GuiTab::Monitor => "Monitor",
        }
    }
    pub const ALL: &'static [GuiTab] = &[
        GuiTab::Play,
        GuiTab::Key,
        GuiTab::Chords,
        GuiTab::Assign,
        GuiTab::Monitor,
    ];
}

// ---------------------------------------------------------------------------
// User-adjustable engine parameters (GUI → Engine)
// ---------------------------------------------------------------------------

/// Parameters the user can tweak in real time via the GUI.
#[derive(Debug, Clone)]
pub struct EngineParams {
    // -- Pitch detection --
    pub pitch_mode: PitchMode,
    pub confidence_threshold: f32,
    pub yin_threshold: f32,

    // -- Noise gate --
    pub silence_threshold: f32,

    // -- Note stability --
    pub stability_frames: usize,
    pub stability_tolerance: f32,
    pub note_change_threshold: f32,

    // -- Smoothing --
    pub pitch_smoothing: f32,
    pub amplitude_smoothing: f32,
    pub centroid_smoothing: f32,

    // -- MIDI output --
    pub midi_channel: u8,
    pub pitch_bend_mode: PitchBendMode,
    pub pitch_bend_range: f32,

    // -- Frequency range --
    pub min_freq_hz: f32,
    pub max_freq_hz: f32,

    // -- Scale / Key lock --
    pub scale_lock_enabled: bool,
    pub scale_type: ScaleType,
    pub root_note: RootNote,
    pub auto_key_detect: bool,

    // -- Chords --
    pub chord_enabled: bool,
    pub chord_type: ChordType,
    pub chord_voicing: Voicing,

    // -- Triggers --
    pub triggers_enabled: bool,
    pub trigger_channel: u8,
    /// RMS delta threshold for onset detection.
    pub trigger_onset_threshold: f32,

    // -- CC mapping --
    pub cc_mapping_enabled: bool,
    pub cc_sources: [CcSource; NUM_CC_SLOTS],
    pub cc_numbers: [u8; NUM_CC_SLOTS],
}

impl Default for EngineParams {
    fn default() -> Self {
        EngineParams {
            pitch_mode: PitchMode::Hybrid,
            confidence_threshold: 0.50,
            yin_threshold: 0.15,

            silence_threshold: 0.012,

            stability_frames: 2,
            stability_tolerance: 0.3,
            note_change_threshold: 0.5,

            pitch_smoothing: 0.25,
            amplitude_smoothing: 0.15,
            centroid_smoothing: 0.20,

            midi_channel: 0,
            pitch_bend_mode: PitchBendMode::TruBend,
            pitch_bend_range: 2.0,

            min_freq_hz: 80.0,
            max_freq_hz: 1000.0,

            scale_lock_enabled: false,
            scale_type: ScaleType::Chromatic,
            root_note: RootNote::C,
            auto_key_detect: false,

            chord_enabled: false,
            chord_type: ChordType::Major,
            chord_voicing: Voicing::RootPosition,

            triggers_enabled: true,
            trigger_channel: 9, // Channel 10 (drums).
            trigger_onset_threshold: 0.08,

            cc_mapping_enabled: true,
            cc_sources: [
                CcSource::Envelope,
                CcSource::Brightness,
                CcSource::Off,
                CcSource::Off,
            ],
            cc_numbers: [1, 74, 71, 11],
        }
    }
}

/// Thread-safe handle for shared engine parameters.
pub type SharedParams = Arc<Mutex<EngineParams>>;

/// Create a new shared params handle with default values.
pub fn create_shared_params() -> SharedParams {
    Arc::new(Mutex::new(EngineParams::default()))
}

// ---------------------------------------------------------------------------
// Engine → GUI state snapshot
// ---------------------------------------------------------------------------

/// A single snapshot of engine state, sent from the engine to the GUI.
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
    // -- Pitch --
    pub note_name: String,
    pub midi_note: Option<u8>,
    pub frequency: f32,
    pub confidence: f32,
    pub pitch_source: PitchSource,
    pub note_active: bool,
    pub pitch_bend: u16,

    // -- Audio analysis --
    pub rms: f32,
    pub velocity: u8,
    pub centroid_hz: f32,

    // -- Scale --
    pub quantized_note_name: String,
    pub detected_key: String,

    // -- Chords --
    pub chord_notes: Vec<u8>,

    // -- Triggers --
    pub trigger_hits: [bool; 4],

    // -- CC --
    pub cc_values: [u8; NUM_CC_SLOTS],

    pub timestamp: Instant,
}

/// Indicates which detector produced the current pitch reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchSource {
    None,
    Yin,
    Crepe,
}

impl PitchSource {
    pub fn label(&self) -> &'static str {
        match self {
            PitchSource::None => "---",
            PitchSource::Yin => "YIN",
            PitchSource::Crepe => "CREPE",
        }
    }
}

impl Default for EngineSnapshot {
    fn default() -> Self {
        EngineSnapshot {
            note_name: "---".to_string(),
            midi_note: None,
            frequency: 0.0,
            confidence: 0.0,
            pitch_source: PitchSource::None,
            note_active: false,
            pitch_bend: 8192,

            rms: 0.0,
            velocity: 0,
            centroid_hz: 0.0,

            quantized_note_name: "---".to_string(),
            detected_key: String::new(),

            chord_notes: Vec::new(),

            trigger_hits: [false; 4],

            cc_values: [0; NUM_CC_SLOTS],

            timestamp: Instant::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// MIDI log entry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MidiLogEntry {
    pub timestamp: Instant,
    pub message: String,
}

// ---------------------------------------------------------------------------
// GUI-side state
// ---------------------------------------------------------------------------

pub struct GuiState {
    pub current: EngineSnapshot,
    pub rx: Receiver<EngineSnapshot>,

    // Histories for graphs
    pub rms_history: VecDeque<f32>,
    pub pitch_history: VecDeque<f32>,
    pub velocity_history: VecDeque<f32>,
    pub centroid_history: VecDeque<f32>,
    pub confidence_history: VecDeque<f32>,

    // MIDI log
    pub midi_log: VecDeque<MidiLogEntry>,
    pub midi_log_rx: Receiver<MidiLogEntry>,

    // Connection info
    pub midi_port_name: String,
    pub midi_connected: bool,
    pub sample_rate: u32,

    // UI state
    pub active_tab: GuiTab,
    pub show_settings: bool,
    pub show_midi_log: bool,

    // Shared params handle (GUI writes, engine reads)
    pub params: SharedParams,

    // MIDI activity flash
    pub midi_flash_until: Instant,
    /// Per-trigger-slot flash timings.
    pub trigger_flash_until: [Instant; 4],

    pub frame_count: u64,

    // Strudel integration
    pub strudel_open: bool,

    // Trigger training UI state.
    pub trigger_training_slot: Option<usize>,
    pub trigger_training_samples: usize,
}

const RMS_HISTORY_SIZE: usize = 512;
const GRAPH_HISTORY_SIZE: usize = 300;
const MIDI_LOG_SIZE: usize = 100;

impl GuiState {
    pub fn new(
        rx: Receiver<EngineSnapshot>,
        midi_log_rx: Receiver<MidiLogEntry>,
        midi_port_name: String,
        midi_connected: bool,
        sample_rate: u32,
        params: SharedParams,
    ) -> Self {
        let now = Instant::now();
        GuiState {
            current: EngineSnapshot::default(),
            rx,
            rms_history: VecDeque::with_capacity(RMS_HISTORY_SIZE),
            pitch_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
            velocity_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
            centroid_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
            confidence_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
            midi_log: VecDeque::with_capacity(MIDI_LOG_SIZE),
            midi_log_rx,
            midi_port_name,
            midi_connected,
            sample_rate,
            active_tab: GuiTab::Play,
            show_settings: false,
            show_midi_log: false,
            params,
            midi_flash_until: now,
            trigger_flash_until: [now; 4],
            frame_count: 0,
            strudel_open: false,
            trigger_training_slot: None,
            trigger_training_samples: 0,
        }
    }

    pub fn update_from_engine(&mut self) {
        while let Ok(snapshot) = self.rx.try_recv() {
            push_bounded(&mut self.rms_history, snapshot.rms, RMS_HISTORY_SIZE);
            push_bounded(&mut self.pitch_history, snapshot.frequency, GRAPH_HISTORY_SIZE);
            push_bounded(&mut self.velocity_history, snapshot.velocity as f32, GRAPH_HISTORY_SIZE);
            push_bounded(&mut self.centroid_history, snapshot.centroid_hz, GRAPH_HISTORY_SIZE);
            push_bounded(&mut self.confidence_history, snapshot.confidence, GRAPH_HISTORY_SIZE);

            if snapshot.note_active {
                self.midi_flash_until = Instant::now() + std::time::Duration::from_millis(120);
            }

            // Trigger flash.
            let now = Instant::now();
            for (i, &hit) in snapshot.trigger_hits.iter().enumerate() {
                if hit {
                    self.trigger_flash_until[i] = now + std::time::Duration::from_millis(150);
                }
            }

            self.current = snapshot;
        }

        while let Ok(entry) = self.midi_log_rx.try_recv() {
            self.midi_log.push_back(entry);
            if self.midi_log.len() > MIDI_LOG_SIZE {
                self.midi_log.pop_front();
            }
        }

        self.frame_count += 1;
    }
}

fn push_bounded(buf: &mut VecDeque<f32>, val: f32, max: usize) {
    buf.push_back(val);
    if buf.len() > max {
        buf.pop_front();
    }
}

// ---------------------------------------------------------------------------
// Channel creation helpers
// ---------------------------------------------------------------------------

pub fn create_snapshot_channel() -> (Sender<EngineSnapshot>, Receiver<EngineSnapshot>) {
    crossbeam_channel::bounded(256)
}

pub fn create_midi_log_channel() -> (Sender<MidiLogEntry>, Receiver<MidiLogEntry>) {
    crossbeam_channel::bounded(512)
}

pub fn create_strudel_channel() -> (Sender<crate::strudel::StrudelMessage>, Receiver<crate::strudel::StrudelMessage>) {
    crossbeam_channel::bounded(256)
}
