// ============================================================================
<<<<<<< HEAD
// state.rs — Shared application state between engine and GUI
// ============================================================================
//
// The engine thread publishes its state into a `SharedState` struct via
// atomic writes. The GUI thread reads this state at ~60 FPS for display.
//
// Communication is lock-free: the engine writes to `EngineState` through
// a crossbeam channel, and the GUI drains the channel each frame.
=======
// state.rs — Phase 5: Shared state with user-adjustable parameters
// ============================================================================
//
// Communication model:
//   Engine → GUI:  EngineSnapshot via crossbeam channel (lock-free)
//   GUI → Engine:  EngineParams via Arc<Mutex<EngineParams>> (low contention)
//   MIDI log:      MidiLogEntry via crossbeam channel
//
// EngineParams holds all user-adjustable tuning parameters. The engine clones
// a snapshot of the params once per analysis frame (cheap) so the GUI can
// update them freely via sliders without blocking the audio path.
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
// ============================================================================

use crossbeam_channel::{Receiver, Sender};
use std::collections::VecDeque;
<<<<<<< HEAD
use std::time::Instant;

// ---------------------------------------------------------------------------
// Engine → GUI state snapshot (sent each analysis frame)
=======
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Pitch detection mode
// ---------------------------------------------------------------------------

/// Which pitch detection algorithm to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PitchMode {
    /// CREPE neural network only (most accurate, ~64 ms latency).
    Crepe,
    /// YIN autocorrelation only (fast ~12 ms, less accurate).
    Yin,
    /// Hybrid: YIN for fast onset, CREPE refines pitch on sustained notes.
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
// User-adjustable engine parameters (GUI → Engine)
// ---------------------------------------------------------------------------

/// Parameters the user can tweak in real time via the GUI.
/// The engine reads a clone of this struct each analysis frame.
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
    pub pitch_bend_enabled: bool,
    pub pitch_bend_range: f32,
    pub cc_brightness_enabled: bool,

    // -- Frequency range --
    pub min_freq_hz: f32,
    pub max_freq_hz: f32,
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

            midi_channel: 0, // 0-indexed, displayed as 1-16
            pitch_bend_enabled: true,
            pitch_bend_range: 2.0,
            cc_brightness_enabled: true,

            min_freq_hz: 80.0,
            max_freq_hz: 1000.0,
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
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
// ---------------------------------------------------------------------------

/// A single snapshot of engine state, sent from the engine to the GUI.
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
<<<<<<< HEAD
    /// Current MIDI note name (e.g. "A4") or "---" if silent.
    pub note_name: String,
    /// Current MIDI note number (0–127), or None if silent.
    #[allow(dead_code)]
    pub midi_note: Option<u8>,
    /// Frequency in Hz, or 0.0 if silent.
    pub frequency: f32,
    /// Smoothed RMS amplitude (0.0–1.0).
    pub rms: f32,
    /// MIDI velocity (0–127).
    pub velocity: u8,
    /// Pitch bend raw value (0–16383, center=8192).
    pub pitch_bend: u16,
    /// CC 74 brightness value (0–127).
    pub cc_brightness: u8,
    /// Pitch detection confidence (0.0–1.0).
    pub confidence: f32,
    /// Spectral centroid in Hz.
    pub centroid_hz: f32,
    /// Whether a note is currently active.
    pub note_active: bool,
    /// Timestamp of this snapshot.
    #[allow(dead_code)]
    pub timestamp: Instant,
=======
    pub note_name: String,
    #[allow(dead_code)]
    pub midi_note: Option<u8>,
    pub frequency: f32,
    pub rms: f32,
    pub velocity: u8,
    pub pitch_bend: u16,
    pub cc_brightness: u8,
    pub confidence: f32,
    pub centroid_hz: f32,
    pub note_active: bool,
    #[allow(dead_code)]
    pub timestamp: Instant,

    /// Which detector produced this pitch (for display).
    pub pitch_source: PitchSource,
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
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
}

impl Default for EngineSnapshot {
    fn default() -> Self {
        EngineSnapshot {
            note_name: "---".to_string(),
            midi_note: None,
            frequency: 0.0,
            rms: 0.0,
            velocity: 0,
            pitch_bend: 8192,
            cc_brightness: 0,
            confidence: 0.0,
            centroid_hz: 0.0,
            note_active: false,
            timestamp: Instant::now(),
<<<<<<< HEAD
=======
            pitch_source: PitchSource::None,
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
        }
    }
}

// ---------------------------------------------------------------------------
// MIDI log entry
// ---------------------------------------------------------------------------

<<<<<<< HEAD
/// A log entry for the MIDI event log in advanced mode.
=======
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
#[derive(Debug, Clone)]
pub struct MidiLogEntry {
    pub timestamp: Instant,
    pub message: String,
}

// ---------------------------------------------------------------------------
<<<<<<< HEAD
// GUI-side state (accumulated from snapshots)
// ---------------------------------------------------------------------------

/// State held by the GUI, updated each frame from engine snapshots.
pub struct GuiState {
    /// Latest engine snapshot.
    pub current: EngineSnapshot,

    /// Receiver for engine snapshots.
    pub rx: Receiver<EngineSnapshot>,

    /// Waveform ring buffer (last ~2048 RMS values for waveform display).
    pub rms_history: VecDeque<f32>,

    /// Pitch history (last ~300 values for pitch graph).
    pub pitch_history: VecDeque<f32>,

    /// Velocity history (last ~300 values).
    pub velocity_history: VecDeque<f32>,

    /// Centroid history (last ~300 values).
    pub centroid_history: VecDeque<f32>,

    /// MIDI event log (last 100 entries).
    pub midi_log: VecDeque<MidiLogEntry>,

    /// Receiver for MIDI log entries.
    pub midi_log_rx: Receiver<MidiLogEntry>,

    /// Whether the engine is currently running.
    #[allow(dead_code)]
    pub engine_running: bool,

    /// MIDI port name (connected).
    pub midi_port_name: String,

    /// Whether MIDI is connected.
    pub midi_connected: bool,

    /// Sample rate.
    pub sample_rate: u32,

    /// Advanced mode toggle.
    pub advanced_mode: bool,

    /// Frame counter for FPS display.
    pub frame_count: u64,

    /// Start time for FPS computation.
    #[allow(dead_code)]
    pub start_time: Instant,

    /// MIDI activity flash timer.
    pub midi_flash_until: Instant,
}

// History sizes
=======
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
    pub show_settings: bool,
    pub show_midi_log: bool,

    // Shared params handle (GUI writes, engine reads)
    pub params: SharedParams,

    // MIDI activity flash
    pub midi_flash_until: Instant,

    pub frame_count: u64,
}

>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
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
<<<<<<< HEAD
=======
        params: SharedParams,
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    ) -> Self {
        GuiState {
            current: EngineSnapshot::default(),
            rx,
            rms_history: VecDeque::with_capacity(RMS_HISTORY_SIZE),
            pitch_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
            velocity_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
            centroid_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
<<<<<<< HEAD
            midi_log: VecDeque::with_capacity(MIDI_LOG_SIZE),
            midi_log_rx,
            engine_running: true,
            midi_port_name,
            midi_connected,
            sample_rate,
            advanced_mode: false,
            frame_count: 0,
            start_time: Instant::now(),
            midi_flash_until: Instant::now(),
        }
    }

    /// Drain all pending snapshots from the engine, keeping only the latest.
    pub fn update_from_engine(&mut self) {
        // Drain all pending snapshots, keep the latest.
        while let Ok(snapshot) = self.rx.try_recv() {
            // Push to histories.
            self.rms_history.push_back(snapshot.rms);
            if self.rms_history.len() > RMS_HISTORY_SIZE {
                self.rms_history.pop_front();
            }

            self.pitch_history.push_back(snapshot.frequency);
            if self.pitch_history.len() > GRAPH_HISTORY_SIZE {
                self.pitch_history.pop_front();
            }

            self.velocity_history.push_back(snapshot.velocity as f32);
            if self.velocity_history.len() > GRAPH_HISTORY_SIZE {
                self.velocity_history.pop_front();
            }

            self.centroid_history.push_back(snapshot.centroid_hz);
            if self.centroid_history.len() > GRAPH_HISTORY_SIZE {
                self.centroid_history.pop_front();
            }

            // Flash MIDI indicator on note activity.
            if snapshot.note_active {
                self.midi_flash_until =
                    Instant::now() + std::time::Duration::from_millis(120);
=======
            confidence_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
            midi_log: VecDeque::with_capacity(MIDI_LOG_SIZE),
            midi_log_rx,
            midi_port_name,
            midi_connected,
            sample_rate,
            show_settings: true,
            show_midi_log: false,
            params,
            midi_flash_until: Instant::now(),
            frame_count: 0,
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
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
            }

            self.current = snapshot;
        }

<<<<<<< HEAD
        // Drain MIDI log entries.
=======
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
        while let Ok(entry) = self.midi_log_rx.try_recv() {
            self.midi_log.push_back(entry);
            if self.midi_log.len() > MIDI_LOG_SIZE {
                self.midi_log.pop_front();
            }
        }

        self.frame_count += 1;
    }
}

<<<<<<< HEAD
=======
fn push_bounded(buf: &mut VecDeque<f32>, val: f32, max: usize) {
    buf.push_back(val);
    if buf.len() > max {
        buf.pop_front();
    }
}

>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
// ---------------------------------------------------------------------------
// Channel creation helpers
// ---------------------------------------------------------------------------

<<<<<<< HEAD
/// Create the engine → GUI snapshot channel (bounded, non-blocking).
pub fn create_snapshot_channel() -> (Sender<EngineSnapshot>, Receiver<EngineSnapshot>) {
    // Bounded to 256 — if GUI falls behind, old snapshots are dropped.
    crossbeam_channel::bounded(256)
}

/// Create the MIDI log channel.
=======
pub fn create_snapshot_channel() -> (Sender<EngineSnapshot>, Receiver<EngineSnapshot>) {
    crossbeam_channel::bounded(256)
}

>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
pub fn create_midi_log_channel() -> (Sender<MidiLogEntry>, Receiver<MidiLogEntry>) {
    crossbeam_channel::bounded(512)
}
