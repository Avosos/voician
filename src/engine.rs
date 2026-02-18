// ============================================================================
// engine.rs — Phase 3: Expressive voice-to-MIDI engine with GUI state output
// ============================================================================
//
// Consumes mono audio samples, runs pitch detection (YIN), spectral analysis,
// and drives a state machine that generates expressive MIDI output.
//
// Phase 3 addition: publishes EngineSnapshot to the GUI via crossbeam channel
// after each analysis frame, enabling real-time visualization.
//
// State machine:
//   Silent → Pending → Active (with continuous pitch bend + CC 74)
//
// Design: no heap allocations in the hot path; all buffers pre-allocated.
// ============================================================================

use crate::analysis::{compute_rms, Smoother, SpectralAnalyzer};
use crate::midi::MidiController;
use crate::pitch::{PitchDetector, PitchResult};
use crate::state::EngineSnapshot;
use crossbeam_channel::Sender;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Tuning constants
// ---------------------------------------------------------------------------

pub const WINDOW_SIZE: usize = 2048;
pub const HOP_SIZE: usize = 512;

const SILENCE_RMS_THRESHOLD: f32 = 0.012;
const STABILITY_FRAMES: usize = 2;
const STABILITY_TOLERANCE_SEMITONES: f32 = 0.3;
const NOTE_CHANGE_THRESHOLD_SEMITONES: f32 = 0.5;
const YIN_THRESHOLD: f32 = 0.15;
const MIN_FREQ_HZ: f32 = 80.0;
const MAX_FREQ_HZ: f32 = 1000.0;
const PITCH_BEND_RANGE_SEMITONES: f32 = 2.0;
const PITCH_BEND_DEADZONE: u16 = 32;
const CC_BRIGHTNESS: u8 = 74;
const CC_DEADZONE: u8 = 1;
const CENTROID_MIN_HZ: f32 = 300.0;
const CENTROID_MAX_HZ: f32 = 4000.0;
const SMOOTH_ALPHA_PITCH: f32 = 0.25;
const SMOOTH_ALPHA_AMPLITUDE: f32 = 0.15;
const SMOOTH_ALPHA_CENTROID: f32 = 0.20;

// ---------------------------------------------------------------------------
// Note name lookup
// ---------------------------------------------------------------------------

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn note_name(note: u8) -> String {
    let name = NOTE_NAMES[(note % 12) as usize];
    let octave = (note as i32 / 12) - 1;
    format!("{}{}", name, octave)
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

enum NoteState {
    Silent,
    Pending {
        candidate_note: u8,
        candidate_midi_float: f32,
        stable_count: usize,
    },
    Active {
        note: u8,
        velocity: u8,
    },
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

pub struct Engine {
    pitch_detector: PitchDetector,
    spectral_analyzer: SpectralAnalyzer,
    midi: MidiController,

    analysis_buffer: Vec<f32>,
    buffer_write_pos: usize,

    state: NoteState,

    pitch_smoother: Smoother,
    amplitude_smoother: Smoother,
    centroid_smoother: Smoother,

    last_pitch_bend: u16,
    last_cc_brightness: u8,

    // -- GUI state channel --
    snapshot_tx: Sender<EngineSnapshot>,

    // -- Cached snapshot values (updated each frame) --
    last_frequency: f32,
    last_confidence: f32,
    last_centroid_hz: f32,
    last_rms: f32,

    #[allow(dead_code)]
    frame_count: u64,
}

impl Engine {
    pub fn new(
        midi: MidiController,
        sample_rate: f32,
        snapshot_tx: Sender<EngineSnapshot>,
    ) -> Self {
        let pitch_detector = PitchDetector::new(
            WINDOW_SIZE,
            sample_rate,
            MIN_FREQ_HZ,
            MAX_FREQ_HZ,
            YIN_THRESHOLD,
        );
        let spectral_analyzer = SpectralAnalyzer::new(WINDOW_SIZE, sample_rate);

        Engine {
            pitch_detector,
            spectral_analyzer,
            midi,

            analysis_buffer: vec![0.0; WINDOW_SIZE],
            buffer_write_pos: 0,

            state: NoteState::Silent,

            pitch_smoother: Smoother::new(SMOOTH_ALPHA_PITCH),
            amplitude_smoother: Smoother::new(SMOOTH_ALPHA_AMPLITUDE),
            centroid_smoother: Smoother::new(SMOOTH_ALPHA_CENTROID),

            last_pitch_bend: 8192,
            last_cc_brightness: 0,

            snapshot_tx,

            last_frequency: 0.0,
            last_confidence: 0.0,
            last_centroid_hz: 0.0,
            last_rms: 0.0,

            frame_count: 0,
        }
    }

    pub fn process_samples(&mut self, samples: &[f32]) {
        let mut offset = 0;

        while offset < samples.len() {
            let remaining = WINDOW_SIZE - self.buffer_write_pos;
            let to_copy = remaining.min(samples.len() - offset);

            self.analysis_buffer[self.buffer_write_pos..self.buffer_write_pos + to_copy]
                .copy_from_slice(&samples[offset..offset + to_copy]);

            self.buffer_write_pos += to_copy;
            offset += to_copy;

            if self.buffer_write_pos >= WINDOW_SIZE {
                self.analyze_frame();

                let keep = WINDOW_SIZE - HOP_SIZE;
                self.analysis_buffer.copy_within(HOP_SIZE.., 0);
                self.buffer_write_pos = keep;
            }
        }
    }

    // =======================================================================
    // Frame analysis
    // =======================================================================

    fn analyze_frame(&mut self) {
        self.frame_count += 1;

        // -- 1. RMS amplitude ------------------------------------------------
        let raw_rms = compute_rms(&self.analysis_buffer);
        let smoothed_rms = self.amplitude_smoother.update(raw_rms);
        self.last_rms = smoothed_rms;

        // -- 2. Silence gate -------------------------------------------------
        if smoothed_rms < SILENCE_RMS_THRESHOLD {
            self.handle_silence();
            self.publish_snapshot();
            return;
        }

        // -- 3. Pitch detection ----------------------------------------------
        let detection = self.pitch_detector.detect(&self.analysis_buffer);

        // -- 4. Spectral centroid --------------------------------------------
        let raw_centroid = self
            .spectral_analyzer
            .compute_centroid(&self.analysis_buffer);
        let smoothed_centroid = self.centroid_smoother.update(raw_centroid);
        self.last_centroid_hz = smoothed_centroid;

        // -- 5. State machine ------------------------------------------------
        match detection {
            Some(result) => {
                self.last_frequency = result.frequency;
                self.last_confidence = result.confidence;
                let smoothed_midi = self.pitch_smoother.update(result.midi_float);
                self.handle_pitch(result, smoothed_midi, smoothed_rms, smoothed_centroid);
            }
            None => {
                self.handle_no_pitch();
            }
        }

        self.publish_snapshot();
    }

    // =======================================================================
    // Publish snapshot to GUI
    // =======================================================================

    fn publish_snapshot(&self) {
        let (note_name_str, midi_note, note_active, velocity, pitch_bend, cc_brightness) =
            match &self.state {
                NoteState::Silent => {
                    ("---".to_string(), None, false, 0u8, 8192u16, 0u8)
                }
                NoteState::Pending {
                    candidate_note, ..
                } => {
                    let name = format!("({})", note_name(*candidate_note));
                    (name, Some(*candidate_note), false, 0, 8192, 0)
                }
                NoteState::Active { note, velocity } => {
                    let name = note_name(*note);
                    (
                        name,
                        Some(*note),
                        true,
                        *velocity,
                        self.last_pitch_bend,
                        self.last_cc_brightness,
                    )
                }
            };

        let snapshot = EngineSnapshot {
            note_name: note_name_str,
            midi_note,
            frequency: self.last_frequency,
            rms: self.last_rms,
            velocity,
            pitch_bend,
            cc_brightness,
            confidence: self.last_confidence,
            centroid_hz: self.last_centroid_hz,
            note_active,
            timestamp: Instant::now(),
        };

        // Non-blocking send — if GUI is behind, old snapshots are dropped.
        let _ = self.snapshot_tx.try_send(snapshot);
    }

    // =======================================================================
    // State machine transitions
    // =======================================================================

    fn handle_silence(&mut self) {
        if let NoteState::Active { note, .. } = self.state {
            self.midi.send_note_off(note);
            self.midi.reset_pitch_bend();
            self.last_pitch_bend = 8192;
        }
        self.state = NoteState::Silent;
        self.pitch_smoother.reset();
        self.centroid_smoother.reset();
        self.last_frequency = 0.0;
        self.last_confidence = 0.0;
    }

    fn handle_pitch(
        &mut self,
        result: PitchResult,
        smoothed_midi: f32,
        smoothed_rms: f32,
        smoothed_centroid: f32,
    ) {
        let detected_note = smoothed_midi.round() as u8;

        match self.state {
            NoteState::Silent => {
                self.state = NoteState::Pending {
                    candidate_note: detected_note,
                    candidate_midi_float: smoothed_midi,
                    stable_count: 1,
                };
            }

            NoteState::Pending {
                candidate_note,
                candidate_midi_float,
                stable_count,
            } => {
                let deviation = (smoothed_midi - candidate_midi_float).abs();

                if deviation < STABILITY_TOLERANCE_SEMITONES
                    && detected_note == candidate_note
                {
                    let new_count = stable_count + 1;

                    if new_count >= STABILITY_FRAMES {
                        let velocity = amplitude_to_velocity(smoothed_rms);

                        self.midi.send_note_on(candidate_note, velocity);

                        let pb =
                            deviation_to_pitch_bend(smoothed_midi - candidate_note as f32);
                        self.midi.send_pitch_bend(pb);
                        self.last_pitch_bend = pb;

                        let cc = centroid_to_cc(smoothed_centroid);
                        self.midi.send_cc(CC_BRIGHTNESS, cc);
                        self.last_cc_brightness = cc;

                        println!(
                            "  ● NOTE ON   {:>3} ({})  vel={:>3}  freq={:>7.1} Hz  conf={:.2}",
                            candidate_note,
                            note_name(candidate_note),
                            velocity,
                            result.frequency,
                            result.confidence,
                        );

                        self.state = NoteState::Active {
                            note: candidate_note,
                            velocity,
                        };
                    } else {
                        self.state = NoteState::Pending {
                            candidate_note,
                            candidate_midi_float: smoothed_midi,
                            stable_count: new_count,
                        };
                    }
                } else {
                    self.state = NoteState::Pending {
                        candidate_note: detected_note,
                        candidate_midi_float: smoothed_midi,
                        stable_count: 1,
                    };
                }
            }

            NoteState::Active { note, velocity } => {
                let deviation_from_active = smoothed_midi - note as f32;

                if deviation_from_active.abs() > NOTE_CHANGE_THRESHOLD_SEMITONES
                    && detected_note != note
                {
                    self.midi.send_note_off(note);
                    self.midi.reset_pitch_bend();
                    self.last_pitch_bend = 8192;

                    println!(
                        "  ○ NOTE OFF  {:>3} ({})  → {}",
                        note,
                        note_name(note),
                        note_name(detected_note),
                    );

                    self.state = NoteState::Pending {
                        candidate_note: detected_note,
                        candidate_midi_float: smoothed_midi,
                        stable_count: 1,
                    };
                } else {
                    // Same note — continuous expressive data.
                    let pb = deviation_to_pitch_bend(deviation_from_active);
                    let pb_diff = if pb > self.last_pitch_bend {
                        pb - self.last_pitch_bend
                    } else {
                        self.last_pitch_bend - pb
                    };
                    if pb_diff >= PITCH_BEND_DEADZONE {
                        self.midi.send_pitch_bend(pb);
                        self.last_pitch_bend = pb;
                    }

                    let cc = centroid_to_cc(smoothed_centroid);
                    let cc_diff = if cc > self.last_cc_brightness {
                        cc - self.last_cc_brightness
                    } else {
                        self.last_cc_brightness - cc
                    };
                    if cc_diff >= CC_DEADZONE {
                        self.midi.send_cc(CC_BRIGHTNESS, cc);
                        self.last_cc_brightness = cc;
                    }

                    self.state = NoteState::Active { note, velocity };
                }
            }
        }
    }

    fn handle_no_pitch(&mut self) {
        if let NoteState::Active { note, .. } = self.state {
            self.midi.send_note_off(note);
            self.midi.reset_pitch_bend();
            self.last_pitch_bend = 8192;
            println!(
                "  ○ NOTE OFF  {:>3} ({})  — pitch lost",
                note,
                note_name(note),
            );
        }
        self.state = NoteState::Silent;
        self.pitch_smoother.reset();
        self.last_frequency = 0.0;
        self.last_confidence = 0.0;
    }
}

// ---------------------------------------------------------------------------
// Drop
// ---------------------------------------------------------------------------

impl Drop for Engine {
    fn drop(&mut self) {
        if let NoteState::Active { note, .. } = self.state {
            self.midi.send_note_off(note);
            println!(
                "  ○ NOTE OFF  {:>3} ({})  — shutdown",
                note,
                note_name(note)
            );
        }
        self.midi.reset_pitch_bend();
        self.midi.all_notes_off();
    }
}

// ===========================================================================
// Utility / mapping functions
// ===========================================================================

fn amplitude_to_velocity(rms: f32) -> u8 {
    const MIN_RMS: f32 = 0.01;
    const MAX_RMS: f32 = 0.40;
    const MIN_VEL: f32 = 20.0;
    const MAX_VEL: f32 = 127.0;

    let normalized = ((rms - MIN_RMS) / (MAX_RMS - MIN_RMS)).clamp(0.0, 1.0);
    let curved = normalized.sqrt();
    let velocity = MIN_VEL + curved * (MAX_VEL - MIN_VEL);
    (velocity as u8).clamp(20, 127)
}

fn deviation_to_pitch_bend(deviation_semitones: f32) -> u16 {
    let clamped = deviation_semitones.clamp(
        -PITCH_BEND_RANGE_SEMITONES,
        PITCH_BEND_RANGE_SEMITONES,
    );
    let normalized =
        (clamped + PITCH_BEND_RANGE_SEMITONES) / (2.0 * PITCH_BEND_RANGE_SEMITONES);
    let value = (normalized * 16383.0).round() as u16;
    value.min(16383)
}

fn centroid_to_cc(centroid_hz: f32) -> u8 {
    let normalized = ((centroid_hz - CENTROID_MIN_HZ)
        / (CENTROID_MAX_HZ - CENTROID_MIN_HZ))
        .clamp(0.0, 1.0);
    (normalized * 127.0).round() as u8
}
