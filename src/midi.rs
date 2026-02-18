// ============================================================================
// midi.rs — MIDI output via midir (Phase 3: GUI-compatible, non-interactive)
// ============================================================================
//
// Manages a connection to a MIDI output port (loopMIDI on Windows).
// Auto-detects loopMIDI ports without interactive prompts.
// Sends MIDI log entries to the GUI via crossbeam channel.
//
// If no MIDI port is available, the controller enters a "disconnected" mode
// where all send operations silently succeed (no panics or errors).
// ============================================================================

use crate::state::MidiLogEntry;
use crossbeam_channel::Sender;
use midir::{MidiOutput, MidiOutputConnection};
use std::time::Instant;

// ---------------------------------------------------------------------------
// MIDI status bytes
// ---------------------------------------------------------------------------

const STATUS_NOTE_ON: u8 = 0x90;
const STATUS_NOTE_OFF: u8 = 0x80;
const STATUS_CC: u8 = 0xB0;
const STATUS_PITCH_BEND: u8 = 0xE0;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Wrapper around a midir MIDI output connection with optional GUI logging.
pub struct MidiController {
    connection: Option<MidiOutputConnection>,
    channel: u8,
    port_name: String,
    log_tx: Option<Sender<MidiLogEntry>>,
}

/// Result of attempting to connect to a MIDI port.
pub struct MidiConnectResult {
    pub controller: MidiController,
    pub port_name: String,
    pub connected: bool,
    pub available_ports: Vec<String>,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl MidiController {
    /// Create a new MIDI controller with automatic port detection.
    /// Non-interactive: never prompts for input. Safe for GUI mode.
    ///
    /// Priority:
    /// 1. Port containing "loopmidi" (case-insensitive)
    /// 2. Port containing "voician" or "virtual"
    /// 3. First available port
    /// 4. Disconnected mode (no MIDI output)
    pub fn connect(log_tx: Option<Sender<MidiLogEntry>>) -> MidiConnectResult {
        Self::connect_with_channel(0, log_tx)
    }

    /// Create a new MIDI controller on a specific channel with auto-detection.
    pub fn connect_with_channel(
        channel: u8,
        log_tx: Option<Sender<MidiLogEntry>>,
    ) -> MidiConnectResult {
        assert!(channel < 16, "MIDI channel must be 0–15");

        let midi_out = match MidiOutput::new("voician") {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[midi] Failed to create MIDI output: {}", e);
                return MidiConnectResult {
                    controller: MidiController {
                        connection: None,
                        channel,
                        port_name: "None".to_string(),
                        log_tx,
                    },
                    port_name: "None (MIDI init failed)".to_string(),
                    connected: false,
                    available_ports: vec![],
                };
            }
        };

        let ports = midi_out.ports();
        let mut available_ports: Vec<String> = Vec::new();

        for port in ports.iter() {
            if let Ok(name) = midi_out.port_name(port) {
                available_ports.push(name);
            }
        }

        if ports.is_empty() {
            eprintln!("[midi] No MIDI output ports found.");
            return MidiConnectResult {
                controller: MidiController {
                    connection: None,
                    channel,
                    port_name: "None".to_string(),
                    log_tx,
                },
                port_name: "None (no ports)".to_string(),
                connected: false,
                available_ports,
            };
        }

        // --- Auto-detect best port ---
        let mut best_idx: Option<usize> = None;

        // Priority 1: loopMIDI
        for (i, name) in available_ports.iter().enumerate() {
            if name.to_lowercase().contains("loopmidi") {
                best_idx = Some(i);
                break;
            }
        }

        // Priority 2: voician or virtual
        if best_idx.is_none() {
            for (i, name) in available_ports.iter().enumerate() {
                let lower = name.to_lowercase();
                if lower.contains("voician") || lower.contains("virtual") {
                    best_idx = Some(i);
                    break;
                }
            }
        }

        // Priority 3: first available
        if best_idx.is_none() {
            best_idx = Some(0);
        }

        let idx = best_idx.unwrap();
        let port_name = available_ports[idx].clone();

        println!("[midi] Connecting to: {}", port_name);

        match midi_out.connect(&ports[idx], "voician-out") {
            Ok(connection) => {
                println!("[midi] Connected to: {} (channel {})", port_name, channel + 1);
                MidiConnectResult {
                    controller: MidiController {
                        connection: Some(connection),
                        channel,
                        port_name: port_name.clone(),
                        log_tx,
                    },
                    port_name,
                    connected: true,
                    available_ports,
                }
            }
            Err(e) => {
                eprintln!("[midi] Failed to connect to '{}': {:?}", port_name, e.kind());
                MidiConnectResult {
                    controller: MidiController {
                        connection: None,
                        channel,
                        port_name: "None".to_string(),
                        log_tx,
                    },
                    port_name: format!("Failed: {}", port_name),
                    connected: false,
                    available_ports,
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Logging helper
    // -----------------------------------------------------------------------

    fn log(&self, msg: String) {
        if let Some(ref tx) = self.log_tx {
            let _ = tx.try_send(MidiLogEntry {
                timestamp: Instant::now(),
                message: msg,
            });
        }
    }

    // -----------------------------------------------------------------------
    // MIDI message senders (gracefully handle disconnected state)
    // -----------------------------------------------------------------------

    /// Send a MIDI Note On message.
    pub fn send_note_on(&mut self, note: u8, velocity: u8) {
        if let Some(ref mut conn) = self.connection {
            let msg = [STATUS_NOTE_ON | self.channel, note, velocity];
            if let Err(e) = conn.send(&msg) {
                eprintln!("[midi] NOTE_ON send error: {}", e);
            }
        }
        self.log(format!("NOTE ON  {:>3} vel={}", note, velocity));
    }

    /// Send a MIDI Note Off message.
    pub fn send_note_off(&mut self, note: u8) {
        if let Some(ref mut conn) = self.connection {
            let msg = [STATUS_NOTE_OFF | self.channel, note, 0];
            if let Err(e) = conn.send(&msg) {
                eprintln!("[midi] NOTE_OFF send error: {}", e);
            }
        }
        self.log(format!("NOTE OFF {:>3}", note));
    }

    /// Send a MIDI Control Change message.
    pub fn send_cc(&mut self, controller: u8, value: u8) {
        if let Some(ref mut conn) = self.connection {
            let msg = [STATUS_CC | self.channel, controller, value];
            if let Err(e) = conn.send(&msg) {
                eprintln!("[midi] CC send error: {}", e);
            }
        }
        // Don't log CC — too noisy for the log.
    }

    /// Send a MIDI Pitch Bend message (14-bit value).
    pub fn send_pitch_bend(&mut self, value: u16) {
        if let Some(ref mut conn) = self.connection {
            let clamped = value.min(16383);
            let lsb = (clamped & 0x7F) as u8;
            let msb = ((clamped >> 7) & 0x7F) as u8;
            let msg = [STATUS_PITCH_BEND | self.channel, lsb, msb];
            if let Err(e) = conn.send(&msg) {
                eprintln!("[midi] PITCH_BEND send error: {}", e);
            }
        }
    }

    /// Send pitch bend reset (center = 8192).
    pub fn reset_pitch_bend(&mut self) {
        self.send_pitch_bend(8192);
    }

    /// Send "All Notes Off" (CC 123, value 0).
    pub fn all_notes_off(&mut self) {
        if let Some(ref mut conn) = self.connection {
            let msg = [STATUS_CC | self.channel, 123, 0];
            if let Err(e) = conn.send(&msg) {
                eprintln!("[midi] ALL_NOTES_OFF send error: {}", e);
            }
        }
    }

    /// Get the connected port name.
    pub fn port_name(&self) -> &str {
        &self.port_name
    }
}
