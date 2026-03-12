// ============================================================================
// engine.rs — Voician v1.0: Dubler-style hybrid engine
// ============================================================================
//
// Dual pitch detection pipeline (YIN + CREPE) with:
//   • Percussive trigger detection (beatbox → drums)
//   • Scale quantization & auto key detection
//   • Chord generation
//   • Multi-CC mapping from voice features
//   • IntelliBend / TruBend pitch bend modes
// ============================================================================

use crate::analysis::{compute_rms, Smoother, SpectralAnalyzer};
use crate::cc_map::{CcMapEngine, VoiceFeatures, NUM_CC_SLOTS};
use crate::chords::ChordEngine;
use crate::crepe::{self, CrepeDetector, Resampler, CREPE_FRAME_SIZE};
use crate::midi::MidiController;
use crate::pitch::PitchDetector;
use crate::scale::{KeyDetector, ScaleQuantizer};
use crate::state::{
    EngineParams, EngineSnapshot, PitchBendMode, PitchMode, PitchSource, SharedParams,
};
use crate::strudel::StrudelMessage;
use crate::triggers::TriggerEngine;
use crossbeam_channel::Sender;
use std::time::Instant;

pub const WINDOW_SIZE: usize = 2048;
pub const HOP_SIZE: usize = 512;

const PITCH_BEND_DEADZONE: u16 = 32;

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
        /// Extra notes from chord engine (not including root).
        chord_notes: Vec<u8>,
    },
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

pub struct Engine {
    // -- Detectors --
    crepe: CrepeDetector,
    resampler: Resampler,
    yin: PitchDetector,
    spectral_analyzer: SpectralAnalyzer,

    // -- MIDI --
    midi: MidiController,

    // -- Dubler subsystems --
    trigger_engine: TriggerEngine,
    scale_quantizer: ScaleQuantizer,
    key_detector: KeyDetector,
    chord_engine: ChordEngine,
    cc_map: CcMapEngine,

    // -- Analysis buffer --
    analysis_buffer: Vec<f32>,
    buffer_write_pos: usize,

    // -- CREPE accumulator (16 kHz) --
    crepe_buffer: Vec<f32>,

    // -- Pitch results --
    yin_freq: f32,
    yin_confidence: f32,
    crepe_freq: f32,
    crepe_confidence: f32,
    crepe_has_result: bool,

    // -- State machine --
    state: NoteState,

    // -- Smoothers --
    pitch_smoother: Smoother,
    amplitude_smoother: Smoother,
    centroid_smoother: Smoother,
    current_pitch_alpha: f32,
    current_amp_alpha: f32,
    current_cent_alpha: f32,

    // -- Last sent MIDI --
    last_pitch_bend: u16,

    // -- Params --
    params: SharedParams,
    p: EngineParams,

    // -- Channels --
    snapshot_tx: Sender<EngineSnapshot>,
    strudel_tx: Sender<StrudelMessage>,

    // -- Cached display values --
    last_frequency: f32,
    last_confidence: f32,
    last_centroid_hz: f32,
    last_rms: f32,
    last_pitch_source: PitchSource,
    last_trigger_hits: [bool; 4],
    last_cc_values: [u8; NUM_CC_SLOTS],
    last_quantized_name: String,
    last_detected_key: String,
    last_chord_notes: Vec<u8>,

    sample_rate: f32,
    frame_count: u64,
}

impl Engine {
    pub fn new(
        crepe: CrepeDetector,
        midi: MidiController,
        sample_rate: f32,
        snapshot_tx: Sender<EngineSnapshot>,
        strudel_tx: Sender<StrudelMessage>,
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

        let scale_quantizer = ScaleQuantizer::new(p.scale_type, p.root_note);
        let chord_engine = ChordEngine::new(p.chord_type, p.chord_voicing);

        Engine {
            crepe,
            resampler,
            yin,
            spectral_analyzer,
            midi,

            trigger_engine: TriggerEngine::new(sample_rate, HOP_SIZE),
            scale_quantizer,
            key_detector: KeyDetector::new(),
            chord_engine,
            cc_map: CcMapEngine::new(),

            analysis_buffer: vec![0.0; WINDOW_SIZE],
            buffer_write_pos: 0,

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

            last_pitch_bend: 8192,

            params,
            p,

            snapshot_tx,
            strudel_tx,

            last_frequency: 0.0,
            last_confidence: 0.0,
            last_centroid_hz: 0.0,
            last_rms: 0.0,
            last_pitch_source: PitchSource::None,
            last_trigger_hits: [false; 4],
            last_cc_values: [0; NUM_CC_SLOTS],
            last_quantized_name: String::new(),
            last_detected_key: String::new(),
            last_chord_notes: Vec::new(),

            sample_rate,
            frame_count: 0,
        }
    }

    // =======================================================================
    // Refresh params
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

        // Update subsystem params.
        self.scale_quantizer.set_scale(self.p.scale_type, self.p.root_note);
        self.chord_engine.chord_type = self.p.chord_type;
        self.chord_engine.voicing = self.p.chord_voicing;
        self.trigger_engine.onset_threshold = self.p.trigger_onset_threshold;
        self.cc_map.enabled = self.p.cc_mapping_enabled;

        // Sync CC slot sources/numbers from params.
        for i in 0..NUM_CC_SLOTS {
            self.cc_map.slots[i].source = self.p.cc_sources[i];
            self.cc_map.slots[i].cc_number = self.p.cc_numbers[i];
        }
    }

    // =======================================================================
    // Main entry point
    // =======================================================================

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
                self.refresh_params();
                self.run_native_analysis();

                let keep = WINDOW_SIZE - HOP_SIZE;
                self.analysis_buffer.copy_within(HOP_SIZE.., 0);
                self.buffer_write_pos = keep;
            }
        }

        // Resample to 16 kHz for CREPE.
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
    // Native-rate analysis
    // =======================================================================

    fn run_native_analysis(&mut self) {
        let raw_rms = compute_rms(&self.analysis_buffer);
        self.last_rms = self.amplitude_smoother.update(raw_rms);

        let raw_centroid = self.spectral_analyzer.compute_centroid(&self.analysis_buffer);
        self.last_centroid_hz = self.centroid_smoother.update(raw_centroid);

        // --- Trigger detection (percussive sounds) ---
        self.last_trigger_hits = [false; 4];
        if self.p.triggers_enabled {
            let hits = self.trigger_engine.process(&self.analysis_buffer, raw_rms);
            for (slot_idx, velocity) in hits {
                if slot_idx < 4 {
                    self.last_trigger_hits[slot_idx] = true;
                    let midi_note = self.trigger_engine.slots[slot_idx].midi_note;
                    // Send trigger on dedicated channel.
                    self.midi.send_note_on_channel(
                        self.p.trigger_channel,
                        midi_note,
                        velocity,
                    );
                    // Schedule a short note-off (will be sent next frame).
                    // For now, just send immediately since triggers are percussive.
                    self.midi.send_note_off_channel(
                        self.p.trigger_channel,
                        midi_note,
                    );
                }
            }
        }

        // --- YIN pitch detection ---
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
            self.crepe_confidence = conf;
        }

        if self.p.pitch_mode == PitchMode::Crepe {
            self.drive_state_machine();
        }

        // In Hybrid mode, CREPE refines pitch on active notes.
        if self.p.pitch_mode == PitchMode::Hybrid {
            if let NoteState::Active { note, velocity, ref chord_notes } = self.state {
                if self.crepe_freq > 0.0 {
                    self.last_frequency = self.crepe_freq;
                    self.last_confidence = self.crepe_confidence;
                    self.last_pitch_source = PitchSource::Crepe;

                    let midi_float = crepe::freq_to_midi(self.crepe_freq);
                    let smoothed_midi = self.pitch_smoother.update(midi_float);
                    let deviation = smoothed_midi - note as f32;

                    // Update pitch bend.
                    self.apply_pitch_bend(note, smoothed_midi, deviation);

                    // Check note change.
                    let detected_note = smoothed_midi.round() as u8;
                    if detected_note != note
                        && (smoothed_midi - note as f32).abs() > self.p.note_change_threshold
                    {
                        self.send_all_notes_off(note, chord_notes);
                        self.state = NoteState::Pending {
                            candidate_note: detected_note,
                            candidate_midi_float: smoothed_midi,
                            stable_count: 1,
                        };
                    } else {
                        let cn = chord_notes.clone();
                        self.state = NoteState::Active { note, velocity, chord_notes: cn };
                    }

                    self.publish_snapshot();
                }
            }
        }
    }

    // =======================================================================
    // Unified state machine
    // =======================================================================

    fn drive_state_machine(&mut self) {
        self.frame_count += 1;

        let smoothed_rms = self.last_rms;

        // Silence gate.
        if smoothed_rms < self.p.silence_threshold {
            self.handle_silence();
            self.process_cc_mapping(0.0);
            self.publish_snapshot();
            return;
        }

        // Select pitch.
        let (frequency, confidence, source) = self.select_pitch();

        if frequency <= 0.0
            || frequency < self.p.min_freq_hz
            || frequency > self.p.max_freq_hz
        {
            self.handle_no_pitch();
            self.process_cc_mapping(0.0);
            self.publish_snapshot();
            return;
        }

        self.last_frequency = frequency;
        self.last_confidence = confidence;
        self.last_pitch_source = source;

        let midi_float = crepe::freq_to_midi(frequency);
        let smoothed_midi = self.pitch_smoother.update(midi_float);
        let smoothed_centroid = self.last_centroid_hz;

        // CC mapping (runs continuously while voice is active).
        self.process_cc_mapping(smoothed_midi);

        // Auto key detection.
        if self.p.auto_key_detect {
            self.key_detector.feed(smoothed_midi.round() as u8, confidence);
            let (root, scale, _conf) = self.key_detector.detect();
            self.last_detected_key = format!("{} {}", root.label(), scale.label());

            // Optionally update the quantizer from detected key.
            self.scale_quantizer.set_scale(scale, root);
        }

        self.handle_pitch(smoothed_midi, smoothed_rms, smoothed_centroid);
        self.publish_snapshot();
    }

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

    // =======================================================================
    // CC mapping
    // =======================================================================

    fn process_cc_mapping(&mut self, midi_float: f32) {
        if !self.p.cc_mapping_enabled {
            return;
        }

        // Compute ZCR for noisiness.
        let zcr = compute_zcr(&self.analysis_buffer, self.sample_rate);

        let features = VoiceFeatures {
            rms: self.last_rms,
            centroid_hz: self.last_centroid_hz,
            midi_float,
            zcr,
        };

        let cc_msgs = self.cc_map.process(&features);
        for (cc_num, cc_val) in cc_msgs {
            self.midi.send_cc(cc_num, cc_val);
        }

        // Update display values.
        for i in 0..NUM_CC_SLOTS {
            self.last_cc_values[i] = self.cc_map.slots[i].last_sent;
        }
    }

    // =======================================================================
    // Pitch bend modes
    // =======================================================================

    fn apply_pitch_bend(&mut self, note: u8, smoothed_midi: f32, deviation: f32) {
        match self.p.pitch_bend_mode {
            PitchBendMode::Off => {}
            PitchBendMode::TruBend => {
                let pb = deviation_to_pitch_bend(deviation, self.p.pitch_bend_range);
                let pb_diff = pb.abs_diff(self.last_pitch_bend);
                if pb_diff >= PITCH_BEND_DEADZONE {
                    self.midi.send_pitch_bend(pb);
                    self.last_pitch_bend = pb;
                }
            }
            PitchBendMode::IntelliBend => {
                // IntelliBend: only the fractional micro-pitch within the
                // current note — snaps to note center quickly.
                let micro = smoothed_midi - note as f32;
                // Dampen small deviations.
                let dampened = if micro.abs() < 0.15 { 0.0 } else { micro * 0.5 };
                let pb = deviation_to_pitch_bend(dampened, self.p.pitch_bend_range);
                let pb_diff = pb.abs_diff(self.last_pitch_bend);
                if pb_diff >= PITCH_BEND_DEADZONE {
                    self.midi.send_pitch_bend(pb);
                    self.last_pitch_bend = pb;
                }
            }
        }
    }

    // =======================================================================
    // Publish snapshot
    // =======================================================================

    fn publish_snapshot(&self) {
        let (note_name_str, midi_note, note_active, velocity, chord_display) = match &self.state {
            NoteState::Silent => ("---".to_string(), None, false, 0u8, Vec::new()),
            NoteState::Pending { candidate_note, .. } => {
                let name = format!("({})", note_name(*candidate_note));
                (name, Some(*candidate_note), false, 0, Vec::new())
            }
            NoteState::Active { note, velocity, chord_notes } => {
                let name = note_name(*note);
                let mut all = vec![*note];
                all.extend_from_slice(chord_notes);
                (name, Some(*note), true, *velocity, all)
            }
        };

        let snapshot = EngineSnapshot {
            note_name: note_name_str.clone(),
            midi_note,
            frequency: self.last_frequency,
            confidence: self.last_confidence,
            pitch_source: self.last_pitch_source,
            note_active,
            pitch_bend: self.last_pitch_bend,

            rms: self.last_rms,
            velocity,
            centroid_hz: self.last_centroid_hz,

            quantized_note_name: self.last_quantized_name.clone(),
            detected_key: self.last_detected_key.clone(),

            chord_notes: chord_display,

            trigger_hits: self.last_trigger_hits,

            cc_values: self.last_cc_values,

            timestamp: Instant::now(),
        };

        let _ = self.snapshot_tx.try_send(snapshot);

        // Strudel bridge.
        let strudel_msg = StrudelMessage {
            note_name: note_name_str,
            midi_note,
            frequency: self.last_frequency,
            velocity,
            note_active,
            rms: self.last_rms,
            confidence: self.last_confidence,
            centroid_hz: self.last_centroid_hz,
            pitch_bend: self.last_pitch_bend,
            cc_brightness: self.last_cc_values.first().copied().unwrap_or(0),
        };
        let _ = self.strudel_tx.try_send(strudel_msg);
    }

    // =======================================================================
    // State machine transitions
    // =======================================================================

    fn handle_silence(&mut self) {
        if let NoteState::Active { note, ref chord_notes, .. } = self.state {
            self.send_all_notes_off(note, chord_notes);
        }
        self.state = NoteState::Silent;
        self.pitch_smoother.reset();
        self.centroid_smoother.reset();
        self.cc_map.reset();
        self.last_frequency = 0.0;
        self.last_confidence = 0.0;
        self.last_pitch_source = PitchSource::None;
        self.last_quantized_name.clear();
        self.last_chord_notes.clear();
        self.crepe_has_result = false;
    }

    fn handle_pitch(
        &mut self,
        smoothed_midi: f32,
        smoothed_rms: f32,
        _smoothed_centroid: f32,
    ) {
        // Apply scale quantization.
        let (final_note, bend_offset) = if self.p.scale_lock_enabled {
            let (q_note, offset) = self.scale_quantizer.quantize_float(smoothed_midi);
            self.last_quantized_name = note_name(q_note);
            (q_note, offset)
        } else {
            let n = smoothed_midi.round() as u8;
            self.last_quantized_name = note_name(n);
            (n, smoothed_midi - smoothed_midi.round())
        };

        match &self.state {
            NoteState::Silent => {
                self.state = NoteState::Pending {
                    candidate_note: final_note,
                    candidate_midi_float: smoothed_midi,
                    stable_count: 1,
                };
            }

            NoteState::Pending {
                candidate_note,
                candidate_midi_float: _,
                stable_count,
            } => {
                let candidate_note = *candidate_note;
                let stable_count = *stable_count;

                if final_note == candidate_note {
                    let new_count = stable_count + 1;

                    if new_count >= self.p.stability_frames {
                        let velocity = amplitude_to_velocity(smoothed_rms);

                        // Generate chord notes.
                        let chord_notes = if self.p.chord_enabled {
                            let all = self.chord_engine.generate(candidate_note);
                            // All notes except root.
                            all.into_iter().filter(|&n| n != candidate_note).collect::<Vec<_>>()
                        } else {
                            Vec::new()
                        };

                        // Send MIDI: root note + chord notes.
                        self.midi.send_note_on(candidate_note, velocity);
                        for &cn in &chord_notes {
                            self.midi.send_note_on(cn, velocity);
                        }

                        // Pitch bend.
                        let deviation = smoothed_midi - candidate_note as f32 + bend_offset;
                        self.apply_pitch_bend(candidate_note, smoothed_midi, deviation);

                        self.last_chord_notes = chord_notes.clone();

                        println!(
                            "  NOTE ON   {:>3} ({})  vel={:>3}  freq={:>7.1} Hz  conf={:.2}  [{}]{}",
                            candidate_note,
                            note_name(candidate_note),
                            velocity,
                            self.last_frequency,
                            self.last_confidence,
                            self.last_pitch_source.label(),
                            if !chord_notes.is_empty() {
                                format!("  chord: {:?}", chord_notes)
                            } else {
                                String::new()
                            },
                        );

                        self.state = NoteState::Active {
                            note: candidate_note,
                            velocity,
                            chord_notes,
                        };
                    } else {
                        self.state = NoteState::Pending {
                            candidate_note: final_note,
                            candidate_midi_float: smoothed_midi,
                            stable_count: new_count,
                        };
                    }
                } else {
                    self.state = NoteState::Pending {
                        candidate_note: final_note,
                        candidate_midi_float: smoothed_midi,
                        stable_count: 1,
                    };
                }
            }

            NoteState::Active { note, velocity, chord_notes } => {
                let note = *note;
                let velocity = *velocity;
                let chord_notes_clone = chord_notes.clone();
                let deviation_from_active = smoothed_midi - note as f32;

                if deviation_from_active.abs() > self.p.note_change_threshold
                    && final_note != note
                {
                    // Note change.
                    self.send_all_notes_off(note, &chord_notes_clone);

                    println!(
                        "  NOTE OFF  {:>3} ({})  -> {}",
                        note, note_name(note), note_name(final_note),
                    );

                    self.state = NoteState::Pending {
                        candidate_note: final_note,
                        candidate_midi_float: smoothed_midi,
                        stable_count: 1,
                    };
                } else {
                    // Update pitch bend.
                    self.apply_pitch_bend(note, smoothed_midi, deviation_from_active);

                    self.state = NoteState::Active {
                        note,
                        velocity,
                        chord_notes: chord_notes_clone,
                    };
                }
            }
        }
    }

    fn handle_no_pitch(&mut self) {
        if let NoteState::Active { note, ref chord_notes, .. } = self.state {
            self.send_all_notes_off(note, chord_notes);
            println!(
                "  NOTE OFF  {:>3} ({})  -- pitch lost",
                note, note_name(note),
            );
        }
        self.state = NoteState::Silent;
        self.pitch_smoother.reset();
        self.last_frequency = 0.0;
        self.last_confidence = 0.0;
        self.last_pitch_source = PitchSource::None;
        self.last_quantized_name.clear();
        self.last_chord_notes.clear();
        self.crepe_has_result = false;
    }

    /// Turn off root note + all chord notes, reset pitch bend.
    fn send_all_notes_off(&mut self, root: u8, chord_notes: &[u8]) {
        self.midi.send_note_off(root);
        for &cn in chord_notes {
            self.midi.send_note_off(cn);
        }
        if self.p.pitch_bend_mode != PitchBendMode::Off {
            self.midi.reset_pitch_bend();
        }
        self.last_pitch_bend = 8192;
        self.crepe_has_result = false;
    }
}

// ---------------------------------------------------------------------------
// Drop
// ---------------------------------------------------------------------------

impl Drop for Engine {
    fn drop(&mut self) {
        if let NoteState::Active { note, ref chord_notes, .. } = self.state {
            self.midi.send_note_off(note);
            for &cn in chord_notes {
                self.midi.send_note_off(cn);
            }
        }
        self.midi.reset_pitch_bend();
        self.midi.all_notes_off();
    }
}

// ===========================================================================
// Utilities
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

fn deviation_to_pitch_bend(deviation_semitones: f32, bend_range: f32) -> u16 {
    let clamped = deviation_semitones.clamp(-bend_range, bend_range);
    let normalized = (clamped + bend_range) / (2.0 * bend_range);
    let value = (normalized * 16383.0).round() as u16;
    value.min(16383)
}

fn compute_zcr(samples: &[f32], sample_rate: f32) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let mut crossings = 0u32;
    for i in 1..samples.len() {
        if (samples[i] >= 0.0) != (samples[i - 1] >= 0.0) {
            crossings += 1;
        }
    }
    let zcr_raw = crossings as f32 / samples.len() as f32;
    // Normalize: typical speech ZCR is 0.01-0.10.
    (zcr_raw * sample_rate / 8000.0).clamp(0.0, 1.0)
}
