// ============================================================================
// state.rs — Shared application state between engine and GUI
// ============================================================================
//
// The engine thread publishes its state into a `SharedState` struct via
// atomic writes. The GUI thread reads this state at ~60 FPS for display.
//
// Communication is lock-free: the engine writes to `EngineState` through
// a crossbeam channel, and the GUI drains the channel each frame.
// ============================================================================

use crossbeam_channel::{Receiver, Sender};
use std::collections::VecDeque;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Engine → GUI state snapshot (sent each analysis frame)
// ---------------------------------------------------------------------------

/// A single snapshot of engine state, sent from the engine to the GUI.
#[derive(Debug, Clone)]
pub struct EngineSnapshot {
    /// Current MIDI note name (e.g. "A4") or "---" if silent.
    pub note_name: String,
    /// Current MIDI note number (0–127), or None if silent.
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
    pub timestamp: Instant,
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
        }
    }
}

// ---------------------------------------------------------------------------
// MIDI log entry
// ---------------------------------------------------------------------------

/// A log entry for the MIDI event log in advanced mode.
#[derive(Debug, Clone)]
pub struct MidiLogEntry {
    pub timestamp: Instant,
    pub message: String,
}

// ---------------------------------------------------------------------------
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
    pub start_time: Instant,

    /// MIDI activity flash timer.
    pub midi_flash_until: Instant,
}

// History sizes
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
    ) -> Self {
        GuiState {
            current: EngineSnapshot::default(),
            rx,
            rms_history: VecDeque::with_capacity(RMS_HISTORY_SIZE),
            pitch_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
            velocity_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
            centroid_history: VecDeque::with_capacity(GRAPH_HISTORY_SIZE),
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
            }

            self.current = snapshot;
        }

        // Drain MIDI log entries.
        while let Ok(entry) = self.midi_log_rx.try_recv() {
            self.midi_log.push_back(entry);
            if self.midi_log.len() > MIDI_LOG_SIZE {
                self.midi_log.pop_front();
            }
        }

        self.frame_count += 1;
    }
}

// ---------------------------------------------------------------------------
// Channel creation helpers
// ---------------------------------------------------------------------------

/// Create the engine → GUI snapshot channel (bounded, non-blocking).
pub fn create_snapshot_channel() -> (Sender<EngineSnapshot>, Receiver<EngineSnapshot>) {
    // Bounded to 256 — if GUI falls behind, old snapshots are dropped.
    crossbeam_channel::bounded(256)
}

/// Create the MIDI log channel.
pub fn create_midi_log_channel() -> (Sender<MidiLogEntry>, Receiver<MidiLogEntry>) {
    crossbeam_channel::bounded(512)
}
