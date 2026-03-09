// ============================================================================
<<<<<<< HEAD
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
=======
// engine.rs — Phase 5: Hybrid YIN + CREPE engine with live-tunable params
// ============================================================================
//
// Dual pitch detection pipeline:
//
//   Native rate (44.1/48 kHz):
//     • RMS amplitude + spectral centroid (every HOP_SIZE samples)
//     • YIN pitch detection (every HOP_SIZE samples, ~12 ms response)
//
//   Resampled to 16 kHz:
//     • CREPE neural pitch detection (every 1024 samples = 64 ms)
//
// Three modes (selectable at runtime):
//   • Hybrid — YIN handles fast onsets, CREPE refines pitch on sustained notes
//   • CREPE  — CREPE only (most accurate, higher latency)
//   • YIN    — YIN only (lowest latency, less accurate)
//
// All tuning constants are read from SharedParams each frame, so the GUI
// can adjust them in real time without restarting.
// ============================================================================

use crate::analysis::{compute_rms, Smoother, SpectralAnalyzer};
use crate::crepe::{self, CrepeDetector, Resampler, CREPE_FRAME_SIZE};
use crate::midi::MidiController;
use crate::pitch::PitchDetector;
use crate::state::{
    EngineParams, EngineSnapshot, PitchMode, PitchSource, SharedParams,
};
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
use crossbeam_channel::Sender;
use std::time::Instant;

// ---------------------------------------------------------------------------
<<<<<<< HEAD
// Tuning constants
=======
// Fixed constants (not user-adjustable)
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
// ---------------------------------------------------------------------------

pub const WINDOW_SIZE: usize = 2048;
pub const HOP_SIZE: usize = 512;

<<<<<<< HEAD
const SILENCE_RMS_THRESHOLD: f32 = 0.012;
const STABILITY_FRAMES: usize = 2;
const STABILITY_TOLERANCE_SEMITONES: f32 = 0.3;
const NOTE_CHANGE_THRESHOLD_SEMITONES: f32 = 0.5;
const YIN_THRESHOLD: f32 = 0.15;
const MIN_FREQ_HZ: f32 = 80.0;
const MAX_FREQ_HZ: f32 = 1000.0;
const PITCH_BEND_RANGE_SEMITONES: f32 = 2.0;
=======
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
const PITCH_BEND_DEADZONE: u16 = 32;
const CC_BRIGHTNESS: u8 = 74;
const CC_DEADZONE: u8 = 1;
const CENTROID_MIN_HZ: f32 = 300.0;
const CENTROID_MAX_HZ: f32 = 4000.0;
<<<<<<< HEAD
const SMOOTH_ALPHA_PITCH: f32 = 0.25;
const SMOOTH_ALPHA_AMPLITUDE: f32 = 0.15;
const SMOOTH_ALPHA_CENTROID: f32 = 0.20;
=======
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83

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
<<<<<<< HEAD
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
=======
    // -- Detectors --
    crepe: CrepeDetector,
    resampler: Resampler,
    yin: PitchDetector,
    spectral_analyzer: SpectralAnalyzer,

    // -- MIDI output --
    midi: MidiController,

    // -- Native-rate analysis buffer --
    analysis_buffer: Vec<f32>,
    buffer_write_pos: usize,

    // -- CREPE frame accumulator (16 kHz) --
    crepe_buffer: Vec<f32>,

    // -- Latest pitch results from each detector --
    yin_freq: f32,
    yin_confidence: f32,
    crepe_freq: f32,
    crepe_confidence: f32,
    /// True once CREPE has produced at least one result for the current note.
    crepe_has_result: bool,

    // -- State machine --
    state: NoteState,

    // -- Smoothers (recreated when alpha changes) --
    pitch_smoother: Smoother,
    amplitude_smoother: Smoother,
    centroid_smoother: Smoother,
    current_pitch_alpha: f32,
    current_amp_alpha: f32,
    current_cent_alpha: f32,

    // -- Last sent MIDI values --
    last_pitch_bend: u16,
    last_cc_brightness: u8,

    // -- Shared parameters (GUI → Engine) --
    params: SharedParams,
    /// Cached copy of params, refreshed each analysis frame.
    p: EngineParams,

    // -- GUI snapshot channel --
    snapshot_tx: Sender<EngineSnapshot>,

    // -- Cached display values --
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    last_frequency: f32,
    last_confidence: f32,
    last_centroid_hz: f32,
    last_rms: f32,
<<<<<<< HEAD
=======
    last_pitch_source: PitchSource,
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83

    #[allow(dead_code)]
    frame_count: u64,
}

impl Engine {
    pub fn new(
<<<<<<< HEAD
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
=======
        crepe: CrepeDetector,
        midi: MidiController,
        sample_rate: f32,
        snapshot_tx: Sender<EngineSnapshot>,
        params: SharedParams,
    ) -> Self {
        let p = params.lock().unwrap().clone();
        let resampler = Resampler::new(sample_rate as u32);
        let spectral_analyzer = SpectralAnalyzer::new(WINDOW_SIZE, sample_rate);
        let yin = PitchDetector::new(
            WINDOW_SIZE,
            sample_rate,
            p.min_freq_hz,
            p.max_freq_hz,
            p.yin_threshold,
        );

        Engine {
            crepe,
            resampler,
            yin,
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
            spectral_analyzer,
            midi,

            analysis_buffer: vec![0.0; WINDOW_SIZE],
            buffer_write_pos: 0,

<<<<<<< HEAD
            state: NoteState::Silent,

            pitch_smoother: Smoother::new(SMOOTH_ALPHA_PITCH),
            amplitude_smoother: Smoother::new(SMOOTH_ALPHA_AMPLITUDE),
            centroid_smoother: Smoother::new(SMOOTH_ALPHA_CENTROID),
=======
            crepe_buffer: Vec::with_capacity(CREPE_FRAME_SIZE * 2),

            yin_freq: 0.0,
            yin_confidence: 0.0,
            crepe_freq: 0.0,
            crepe_confidence: 0.0,
            crepe_has_result: false,

            state: NoteState::Silent,

            pitch_smoother: Smoother::new(p.pitch_smoothing),
            amplitude_smoother: Smoother::new(p.amplitude_smoothing),
            centroid_smoother: Smoother::new(p.centroid_smoothing),
            current_pitch_alpha: p.pitch_smoothing,
            current_amp_alpha: p.amplitude_smoothing,
            current_cent_alpha: p.centroid_smoothing,
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83

            last_pitch_bend: 8192,
            last_cc_brightness: 0,

<<<<<<< HEAD
=======
            params,
            p,

>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
            snapshot_tx,

            last_frequency: 0.0,
            last_confidence: 0.0,
            last_centroid_hz: 0.0,
            last_rms: 0.0,
<<<<<<< HEAD
=======
            last_pitch_source: PitchSource::None,
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83

            frame_count: 0,
        }
    }

<<<<<<< HEAD
    pub fn process_samples(&mut self, samples: &[f32]) {
        let mut offset = 0;

=======
    // =======================================================================
    // Refresh params from GUI
    // =======================================================================

    fn refresh_params(&mut self) {
        if let Ok(guard) = self.params.try_lock() {
            self.p = guard.clone();
        }
        // Rebuild smoothers if alpha changed.
        if (self.p.pitch_smoothing - self.current_pitch_alpha).abs() > 0.001 {
            self.pitch_smoother = Smoother::new(self.p.pitch_smoothing);
            self.current_pitch_alpha = self.p.pitch_smoothing;
        }
        if (self.p.amplitude_smoothing - self.current_amp_alpha).abs() > 0.001 {
            self.amplitude_smoother = Smoother::new(self.p.amplitude_smoothing);
            self.current_amp_alpha = self.p.amplitude_smoothing;
        }
        if (self.p.centroid_smoothing - self.current_cent_alpha).abs() > 0.001 {
            self.centroid_smoother = Smoother::new(self.p.centroid_smoothing);
            self.current_cent_alpha = self.p.centroid_smoothing;
        }
    }

    // =======================================================================
    // Main entry point
    // =======================================================================

    pub fn process_samples(&mut self, samples: &[f32]) {
        // -- 1. Native-rate analysis buffer (RMS + centroid + YIN) --
        let mut offset = 0;
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
        while offset < samples.len() {
            let remaining = WINDOW_SIZE - self.buffer_write_pos;
            let to_copy = remaining.min(samples.len() - offset);

            self.analysis_buffer[self.buffer_write_pos..self.buffer_write_pos + to_copy]
                .copy_from_slice(&samples[offset..offset + to_copy]);

            self.buffer_write_pos += to_copy;
            offset += to_copy;

            if self.buffer_write_pos >= WINDOW_SIZE {
<<<<<<< HEAD
                self.analyze_frame();
=======
                self.refresh_params();
                self.run_native_analysis();
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83

                let keep = WINDOW_SIZE - HOP_SIZE;
                self.analysis_buffer.copy_within(HOP_SIZE.., 0);
                self.buffer_write_pos = keep;
            }
        }
<<<<<<< HEAD
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
=======

        // -- 2. Resample to 16 kHz and accumulate CREPE frames --
        if self.p.pitch_mode != PitchMode::Yin {
            let resampled = self.resampler.process(samples);
            self.crepe_buffer.extend_from_slice(&resampled);

            while self.crepe_buffer.len() >= CREPE_FRAME_SIZE {
                let frame: Vec<f32> = self.crepe_buffer.drain(..CREPE_FRAME_SIZE).collect();
                self.run_crepe_analysis(&frame);
            }
        }
    }

    // =======================================================================
    // Native-rate analysis: RMS + centroid + YIN
    // =======================================================================

    fn run_native_analysis(&mut self) {
        let raw_rms = compute_rms(&self.analysis_buffer);
        self.last_rms = self.amplitude_smoother.update(raw_rms);

        let raw_centroid = self
            .spectral_analyzer
            .compute_centroid(&self.analysis_buffer);
        self.last_centroid_hz = self.centroid_smoother.update(raw_centroid);

        // YIN pitch detection (if mode uses it).
        if self.p.pitch_mode != PitchMode::Crepe {
            if let Some(result) = self.yin.detect(&self.analysis_buffer) {
                if result.confidence >= (1.0 - self.p.yin_threshold) {
                    self.yin_freq = result.frequency;
                    self.yin_confidence = result.confidence;
                } else {
                    self.yin_freq = 0.0;
                    self.yin_confidence = 0.0;
                }
            } else {
                self.yin_freq = 0.0;
                self.yin_confidence = 0.0;
            }
        }

        // Drive the state machine from the best available pitch.
        self.drive_state_machine();
    }

    // =======================================================================
    // CREPE analysis (16 kHz)
    // =======================================================================

    fn run_crepe_analysis(&mut self, frame: &[f32]) {
        let (freq, conf) = self.crepe.detect_pitch(frame);

        if conf >= self.p.confidence_threshold
            && freq >= self.p.min_freq_hz
            && freq <= self.p.max_freq_hz
        {
            self.crepe_freq = freq;
            self.crepe_confidence = conf;
            self.crepe_has_result = true;
        } else {
            self.crepe_freq = 0.0;
            self.crepe_confidence = conf; // still report for display
        }

        // In CREPE-only mode, drive state machine from CREPE.
        if self.p.pitch_mode == PitchMode::Crepe {
            self.drive_state_machine();
        }
        // In Hybrid mode, CREPE refines but YIN already drove the onset.
        // We still want to update the active note's pitch bend from CREPE.
        if self.p.pitch_mode == PitchMode::Hybrid {
            if let NoteState::Active { note, velocity } = self.state {
                if self.crepe_freq > 0.0 {
                    self.last_frequency = self.crepe_freq;
                    self.last_confidence = self.crepe_confidence;
                    self.last_pitch_source = PitchSource::Crepe;

                    let midi_float = crepe::freq_to_midi(self.crepe_freq);
                    let smoothed_midi = self.pitch_smoother.update(midi_float);
                    let deviation = smoothed_midi - note as f32;

                    // Update pitch bend from CREPE's more accurate reading.
                    if self.p.pitch_bend_enabled {
                        let pb = deviation_to_pitch_bend(deviation, self.p.pitch_bend_range);
                        let pb_diff = pb.abs_diff(self.last_pitch_bend);
                        if pb_diff >= PITCH_BEND_DEADZONE {
                            self.midi.send_pitch_bend(pb);
                            self.last_pitch_bend = pb;
                        }
                    }

                    // Check if CREPE says we should be on a different note.
                    let detected_note = smoothed_midi.round() as u8;
                    if detected_note != note
                        && (smoothed_midi - note as f32).abs() > self.p.note_change_threshold
                    {
                        self.midi.send_note_off(note);
                        if self.p.pitch_bend_enabled {
                            self.midi.reset_pitch_bend();
                            self.last_pitch_bend = 8192;
                        }
                        self.state = NoteState::Pending {
                            candidate_note: detected_note,
                            candidate_midi_float: smoothed_midi,
                            stable_count: 1,
                        };
                    } else {
                        self.state = NoteState::Active { note, velocity };
                    }

                    self.publish_snapshot();
                }
            }
        }
    }

    // =======================================================================
    // Unified state machine driver
    // =======================================================================

    fn drive_state_machine(&mut self) {
        self.frame_count += 1;

        let smoothed_rms = self.last_rms;

        // -- Silence gate --
        if smoothed_rms < self.p.silence_threshold {
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
            self.handle_silence();
            self.publish_snapshot();
            return;
        }

<<<<<<< HEAD
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

=======
        // -- Select best pitch based on mode --
        let (frequency, confidence, source) = self.select_pitch();

        if frequency <= 0.0
            || frequency < self.p.min_freq_hz
            || frequency > self.p.max_freq_hz
        {
            self.handle_no_pitch();
            self.publish_snapshot();
            return;
        }

        self.last_frequency = frequency;
        self.last_confidence = confidence;
        self.last_pitch_source = source;

        // -- Smooth and drive state machine --
        let midi_float = crepe::freq_to_midi(frequency);
        let smoothed_midi = self.pitch_smoother.update(midi_float);
        let smoothed_centroid = self.last_centroid_hz;

        self.handle_pitch(smoothed_midi, smoothed_rms, smoothed_centroid);
        self.publish_snapshot();
    }

    /// Select the best pitch reading given the current mode.
    fn select_pitch(&self) -> (f32, f32, PitchSource) {
        match self.p.pitch_mode {
            PitchMode::Crepe => {
                if self.crepe_freq > 0.0 {
                    (self.crepe_freq, self.crepe_confidence, PitchSource::Crepe)
                } else {
                    (0.0, 0.0, PitchSource::None)
                }
            }
            PitchMode::Yin => {
                if self.yin_freq > 0.0 {
                    (self.yin_freq, self.yin_confidence, PitchSource::Yin)
                } else {
                    (0.0, 0.0, PitchSource::None)
                }
            }
            PitchMode::Hybrid => {
                // Prefer CREPE when available, fall back to YIN for fast onset.
                if self.crepe_has_result && self.crepe_freq > 0.0 {
                    (self.crepe_freq, self.crepe_confidence, PitchSource::Crepe)
                } else if self.yin_freq > 0.0 {
                    (self.yin_freq, self.yin_confidence, PitchSource::Yin)
                } else {
                    (0.0, 0.0, PitchSource::None)
                }
            }
        }
    }

>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    // =======================================================================
    // Publish snapshot to GUI
    // =======================================================================

    fn publish_snapshot(&self) {
        let (note_name_str, midi_note, note_active, velocity, pitch_bend, cc_brightness) =
            match &self.state {
                NoteState::Silent => {
                    ("---".to_string(), None, false, 0u8, 8192u16, 0u8)
                }
<<<<<<< HEAD
                NoteState::Pending {
                    candidate_note, ..
                } => {
=======
                NoteState::Pending { candidate_note, .. } => {
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
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
<<<<<<< HEAD
        };

        // Non-blocking send — if GUI is behind, old snapshots are dropped.
=======
            pitch_source: self.last_pitch_source,
        };

>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
        let _ = self.snapshot_tx.try_send(snapshot);
    }

    // =======================================================================
    // State machine transitions
    // =======================================================================

    fn handle_silence(&mut self) {
        if let NoteState::Active { note, .. } = self.state {
            self.midi.send_note_off(note);
<<<<<<< HEAD
            self.midi.reset_pitch_bend();
=======
            if self.p.pitch_bend_enabled {
                self.midi.reset_pitch_bend();
            }
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
            self.last_pitch_bend = 8192;
        }
        self.state = NoteState::Silent;
        self.pitch_smoother.reset();
        self.centroid_smoother.reset();
        self.last_frequency = 0.0;
        self.last_confidence = 0.0;
<<<<<<< HEAD
=======
        self.last_pitch_source = PitchSource::None;
        self.crepe_has_result = false;
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    }

    fn handle_pitch(
        &mut self,
<<<<<<< HEAD
        result: PitchResult,
=======
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
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

<<<<<<< HEAD
                if deviation < STABILITY_TOLERANCE_SEMITONES
=======
                if deviation < self.p.stability_tolerance
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
                    && detected_note == candidate_note
                {
                    let new_count = stable_count + 1;

<<<<<<< HEAD
                    if new_count >= STABILITY_FRAMES {
=======
                    if new_count >= self.p.stability_frames {
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
                        let velocity = amplitude_to_velocity(smoothed_rms);

                        self.midi.send_note_on(candidate_note, velocity);

<<<<<<< HEAD
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
=======
                        if self.p.pitch_bend_enabled {
                            let pb = deviation_to_pitch_bend(
                                smoothed_midi - candidate_note as f32,
                                self.p.pitch_bend_range,
                            );
                            self.midi.send_pitch_bend(pb);
                            self.last_pitch_bend = pb;
                        }

                        if self.p.cc_brightness_enabled {
                            let cc = centroid_to_cc(smoothed_centroid);
                            self.midi.send_cc(CC_BRIGHTNESS, cc);
                            self.last_cc_brightness = cc;
                        }

                        println!(
                            "  NOTE ON   {:>3} ({})  vel={:>3}  freq={:>7.1} Hz  conf={:.2}  [{}]",
                            candidate_note,
                            note_name(candidate_note),
                            velocity,
                            self.last_frequency,
                            self.last_confidence,
                            self.last_pitch_source.label(),
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
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

<<<<<<< HEAD
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
=======
                if deviation_from_active.abs() > self.p.note_change_threshold
                    && detected_note != note
                {
                    self.midi.send_note_off(note);
                    if self.p.pitch_bend_enabled {
                        self.midi.reset_pitch_bend();
                    }
                    self.last_pitch_bend = 8192;
                    self.crepe_has_result = false;

                    println!(
                        "  NOTE OFF  {:>3} ({})  -> {}",
                        note, note_name(note), note_name(detected_note),
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
                    );

                    self.state = NoteState::Pending {
                        candidate_note: detected_note,
                        candidate_midi_float: smoothed_midi,
                        stable_count: 1,
                    };
                } else {
<<<<<<< HEAD
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
=======
                    // Update pitch bend.
                    if self.p.pitch_bend_enabled {
                        let pb = deviation_to_pitch_bend(
                            deviation_from_active,
                            self.p.pitch_bend_range,
                        );
                        let pb_diff = pb.abs_diff(self.last_pitch_bend);
                        if pb_diff >= PITCH_BEND_DEADZONE {
                            self.midi.send_pitch_bend(pb);
                            self.last_pitch_bend = pb;
                        }
                    }

                    // Update CC 74 brightness.
                    if self.p.cc_brightness_enabled {
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
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
                    }

                    self.state = NoteState::Active { note, velocity };
                }
            }
        }
    }

    fn handle_no_pitch(&mut self) {
        if let NoteState::Active { note, .. } = self.state {
            self.midi.send_note_off(note);
<<<<<<< HEAD
            self.midi.reset_pitch_bend();
            self.last_pitch_bend = 8192;
            println!(
                "  ○ NOTE OFF  {:>3} ({})  — pitch lost",
                note,
                note_name(note),
=======
            if self.p.pitch_bend_enabled {
                self.midi.reset_pitch_bend();
            }
            self.last_pitch_bend = 8192;
            println!(
                "  NOTE OFF  {:>3} ({})  -- pitch lost",
                note, note_name(note),
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
            );
        }
        self.state = NoteState::Silent;
        self.pitch_smoother.reset();
        self.last_frequency = 0.0;
        self.last_confidence = 0.0;
<<<<<<< HEAD
=======
        self.last_pitch_source = PitchSource::None;
        self.crepe_has_result = false;
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
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
<<<<<<< HEAD
                "  ○ NOTE OFF  {:>3} ({})  — shutdown",
                note,
                note_name(note)
=======
                "  NOTE OFF  {:>3} ({})  -- shutdown",
                note, note_name(note),
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
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

<<<<<<< HEAD
fn deviation_to_pitch_bend(deviation_semitones: f32) -> u16 {
    let clamped = deviation_semitones.clamp(
        -PITCH_BEND_RANGE_SEMITONES,
        PITCH_BEND_RANGE_SEMITONES,
    );
    let normalized =
        (clamped + PITCH_BEND_RANGE_SEMITONES) / (2.0 * PITCH_BEND_RANGE_SEMITONES);
=======
fn deviation_to_pitch_bend(deviation_semitones: f32, bend_range: f32) -> u16 {
    let clamped = deviation_semitones.clamp(-bend_range, bend_range);
    let normalized = (clamped + bend_range) / (2.0 * bend_range);
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    let value = (normalized * 16383.0).round() as u16;
    value.min(16383)
}

fn centroid_to_cc(centroid_hz: f32) -> u8 {
<<<<<<< HEAD
    let normalized = ((centroid_hz - CENTROID_MIN_HZ)
        / (CENTROID_MAX_HZ - CENTROID_MIN_HZ))
=======
    let normalized = ((centroid_hz - CENTROID_MIN_HZ) / (CENTROID_MAX_HZ - CENTROID_MIN_HZ))
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
        .clamp(0.0, 1.0);
    (normalized * 127.0).round() as u8
}
