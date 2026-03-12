// ============================================================================
// gui.rs — Voician v1.0: Dubler-style tabbed GUI
// ============================================================================
//
//   ┌────────────────────────────────────────────────────────┐
//   │  VOICIAN v1.0  │ source │ MIDI │ key │    ⚙  Log  🎵 │
//   ├────────────────────────────────────────────────────────┤
//   │  [ Pitch ] [ Triggers ] [ Controls ] [ Monitor ]       │
//   ├────────────────────────────────────────────────────────┤
//   │                    Tab Content                         │
//   └────────────────────────────────────────────────────────┘
// ============================================================================

use crate::cc_map::{CcSource, NUM_CC_SLOTS};
use crate::chords::ChordType;
use crate::scale::{RootNote, ScaleType};
use crate::state::{
    EngineParams, GuiState, GuiTab, PitchBendMode, PitchMode, PitchSource, SharedParams,
};
use eframe::egui;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Color palette (dark theme)
// ---------------------------------------------------------------------------

const BG_DARK: egui::Color32 = egui::Color32::from_rgb(14, 14, 20);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(24, 24, 34);
const SIDEBAR_BG: egui::Color32 = egui::Color32::from_rgb(20, 20, 28);
const TAB_BG: egui::Color32 = egui::Color32::from_rgb(30, 30, 42);
const TAB_ACTIVE: egui::Color32 = egui::Color32::from_rgb(60, 60, 90);

const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(80, 140, 255);
const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(60, 210, 120);
const ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 160, 50);
const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(255, 70, 70);
const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(170, 100, 255);
const ACCENT_CYAN: egui::Color32 = egui::Color32::from_rgb(80, 210, 230);
const ACCENT_YELLOW: egui::Color32 = egui::Color32::from_rgb(255, 220, 60);

const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(110, 110, 130);
const TEXT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(220, 220, 235);

const TRIGGER_COLORS: [egui::Color32; 4] = [
    ACCENT_RED,
    ACCENT_ORANGE,
    ACCENT_YELLOW,
    ACCENT_CYAN,
];

// ---------------------------------------------------------------------------
// App struct
// ---------------------------------------------------------------------------

pub struct VoicianApp {
    pub gui_state: GuiState,
    /// Local copy of params that we edit via sliders, then push to engine.
    local_params: EngineParams,
    params_handle: SharedParams,
}

impl VoicianApp {
    pub fn new(gui_state: GuiState) -> Self {
        let params_handle = gui_state.params.clone();
        let local_params = params_handle.lock().unwrap().clone();
        VoicianApp {
            gui_state,
            local_params,
            params_handle,
        }
    }

    /// Push local params to the engine's shared state.
    fn push_params(&self) {
        if let Ok(mut guard) = self.params_handle.try_lock() {
            *guard = self.local_params.clone();
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App
// ---------------------------------------------------------------------------

impl eframe::App for VoicianApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gui_state.update_from_engine();
        apply_dark_theme(ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // == Top bar ==
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(6.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("VOICIAN")
                            .color(ACCENT_BLUE)
                            .size(18.0),
                    );
                    ui.label(
                        egui::RichText::new("v1.0").color(TEXT_DIM).size(11.0),
                    );
                    ui.separator();

                    // Pitch source badge.
                    let src = self.gui_state.current.pitch_source;
                    let src_color = match src {
                        PitchSource::Crepe => ACCENT_PURPLE,
                        PitchSource::Yin => ACCENT_CYAN,
                        PitchSource::None => TEXT_DIM,
                    };
                    ui.label(
                        egui::RichText::new(src.label()).color(src_color).size(12.0).strong(),
                    );
                    ui.separator();

                    // MIDI status.
                    let (midi_text, midi_color) = if self.gui_state.midi_connected {
                        (format!("MIDI: {}", self.gui_state.midi_port_name), ACCENT_GREEN)
                    } else {
                        ("MIDI: ---".to_string(), ACCENT_RED)
                    };
                    ui.label(egui::RichText::new(midi_text).color(midi_color).size(11.0));

                    // Detected key.
                    if !self.gui_state.current.detected_key.is_empty() {
                        ui.separator();
                        ui.label(
                            egui::RichText::new(format!(
                                "Key: {}", self.gui_state.current.detected_key
                            ))
                            .color(ACCENT_YELLOW).size(11.0),
                        );
                    }

                    // MIDI activity dot.
                    let midi_active = Instant::now() < self.gui_state.midi_flash_until;
                    let dot_color = if midi_active {
                        ACCENT_GREEN
                    } else {
                        egui::Color32::from_rgb(40, 40, 50)
                    };
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 5.0, dot_color);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Strudel.
                        let strudel_label = if self.gui_state.strudel_open {
                            "Strudel \u{25CF}"
                        } else {
                            "Strudel"
                        };
                        if ui.button(egui::RichText::new(strudel_label).size(11.0)).clicked() {
                            self.gui_state.strudel_open = true;
                            crate::strudel::open_browser();
                        }

                        // MIDI log toggle.
                        let log_label = if self.gui_state.show_midi_log { "Log \u{25BE}" } else { "Log \u{25B8}" };
                        if ui.button(egui::RichText::new(log_label).size(11.0)).clicked() {
                            self.gui_state.show_midi_log = !self.gui_state.show_midi_log;
                        }

                        // Settings toggle.
                        let settings_label = if self.gui_state.show_settings { "\u{2699} \u{25BE}" } else { "\u{2699} \u{25B8}" };
                        if ui.button(egui::RichText::new(settings_label).size(11.0)).clicked() {
                            self.gui_state.show_settings = !self.gui_state.show_settings;
                        }

                        ui.label(
                            egui::RichText::new(format!("{} Hz", self.gui_state.sample_rate))
                                .color(TEXT_DIM).size(10.0),
                        );
                    });
                });
            });

        // == Tab bar ==
        egui::TopBottomPanel::top("tab_bar")
            .frame(egui::Frame::new().fill(TAB_BG).inner_margin(4.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for tab in GuiTab::ALL {
                        let active = self.gui_state.active_tab == *tab;
                        let text = egui::RichText::new(tab.label())
                            .size(13.0)
                            .color(if active { ACCENT_BLUE } else { TEXT_BRIGHT });
                        let btn = egui::Button::new(text)
                            .fill(if active { TAB_ACTIVE } else { TAB_BG });
                        if ui.add(btn).clicked() {
                            self.gui_state.active_tab = *tab;
                        }
                    }
                });
            });

        // == Bottom: MIDI log ==
        if self.gui_state.show_midi_log {
            egui::TopBottomPanel::bottom("midi_log")
                .resizable(true)
                .default_height(90.0)
                .max_height(200.0)
                .frame(egui::Frame::new().fill(SIDEBAR_BG).inner_margin(6.0))
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("MIDI Log").color(TEXT_DIM).size(10.0));
                    egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                        for entry in self.gui_state.midi_log.iter() {
                            let elapsed = entry.timestamp.elapsed();
                            ui.label(
                                egui::RichText::new(format!(
                                    "[{:.1}s] {}", elapsed.as_secs_f32(), entry.message
                                ))
                                .color(TEXT_DIM).size(10.0)
                                .family(egui::FontFamily::Monospace),
                            );
                        }
                    });
                });
        }

        // == Side panel: advanced settings ==
        if self.gui_state.show_settings {
            egui::SidePanel::left("settings_panel")
                .default_width(200.0)
                .min_width(170.0)
                .max_width(280.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(SIDEBAR_BG).inner_margin(8.0))
                .show(ctx, |ui| {
                    self.draw_advanced_settings(ui);
                });
        }

        // == Central panel: tab content ==
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG_DARK).inner_margin(12.0))
            .show(ctx, |ui| {
                match self.gui_state.active_tab {
                    GuiTab::Pitch => self.draw_pitch_tab(ui),
                    GuiTab::Triggers => self.draw_triggers_tab(ui),
                    GuiTab::Controls => self.draw_controls_tab(ui),
                    GuiTab::Monitor => self.draw_monitor_tab(ui),
                }
            });
    }
}

// ===========================================================================
// Pitch Tab
// ===========================================================================

impl VoicianApp {
    fn draw_pitch_tab(&mut self, ui: &mut egui::Ui) {
        let snap = &self.gui_state.current;
        let mut changed = false;

        // -- Big note display --
        ui.vertical_centered(|ui| {
            let note_color = if snap.note_active { ACCENT_GREEN } else { TEXT_DIM };
            ui.label(
                egui::RichText::new(&snap.note_name)
                    .color(note_color)
                    .size(72.0)
                    .strong(),
            );

            if snap.frequency > 0.0 {
                ui.label(
                    egui::RichText::new(format!("{:.1} Hz", snap.frequency))
                        .color(TEXT_DIM).size(14.0),
                );
            }

            // Quantized note.
            if self.local_params.scale_lock_enabled && !snap.quantized_note_name.is_empty() {
                ui.label(
                    egui::RichText::new(format!("\u{2192} {}", snap.quantized_note_name))
                        .color(ACCENT_YELLOW).size(18.0),
                );
            }

            // Chord display.
            if !snap.chord_notes.is_empty() {
                let chord_str: Vec<String> = snap.chord_notes.iter().map(|n| note_name_util(*n)).collect();
                ui.label(
                    egui::RichText::new(format!("Chord: {}", chord_str.join(" ")))
                        .color(ACCENT_PURPLE).size(13.0),
                );
            }
        });

        ui.add_space(10.0);

        // -- Meter row --
        ui.horizontal(|ui| {
            let w = ((ui.available_width() - 30.0) / 4.0).max(70.0);
            draw_meter(ui, "Volume", snap.rms, 0.5, ACCENT_BLUE, w);
            ui.add_space(6.0);
            draw_meter(ui, "Velocity", snap.velocity as f32 / 127.0, 1.0, ACCENT_ORANGE, w);
            ui.add_space(6.0);
            draw_meter(ui, "Confidence", snap.confidence, 1.0, ACCENT_PURPLE, w);
            ui.add_space(6.0);
            draw_meter(ui, "Pitch Bend", pitch_bend_norm(snap.pitch_bend), 1.0, ACCENT_CYAN, w);
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        // -- Scale lock + Pitch bend + Pitch mode --
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                section_label(ui, "SCALE LOCK", ACCENT_YELLOW);
                if ui.checkbox(
                    &mut self.local_params.scale_lock_enabled,
                    egui::RichText::new("Enable").color(TEXT_BRIGHT).size(12.0),
                ).changed() {
                    changed = true;
                }
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Root:").color(TEXT_DIM).size(11.0));
                    egui::ComboBox::from_id_salt("root_note")
                        .selected_text(self.local_params.root_note.label())
                        .width(50.0)
                        .show_ui(ui, |ui| {
                            for root in RootNote::ALL {
                                if ui.selectable_value(&mut self.local_params.root_note, *root, root.label()).changed() {
                                    changed = true;
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Scale:").color(TEXT_DIM).size(11.0));
                    egui::ComboBox::from_id_salt("scale_type")
                        .selected_text(self.local_params.scale_type.label())
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            for scale in ScaleType::ALL {
                                if ui.selectable_value(&mut self.local_params.scale_type, *scale, scale.label()).changed() {
                                    changed = true;
                                }
                            }
                        });
                });
                if ui.checkbox(
                    &mut self.local_params.auto_key_detect,
                    egui::RichText::new("Auto-Detect Key").color(TEXT_BRIGHT).size(11.0),
                ).changed() {
                    changed = true;
                }
            });

            ui.add_space(30.0);

            ui.vertical(|ui| {
                section_label(ui, "PITCH BEND", ACCENT_CYAN);
                for mode in PitchBendMode::ALL {
                    let sel = self.local_params.pitch_bend_mode == *mode;
                    let label = egui::RichText::new(mode.label()).size(12.0)
                        .color(if sel { ACCENT_CYAN } else { TEXT_BRIGHT });
                    if ui.selectable_label(sel, label).clicked() {
                        self.local_params.pitch_bend_mode = *mode;
                        changed = true;
                    }
                }
                ui.add_space(4.0);
                changed |= labeled_slider(ui, "Bend Range (st)", &mut self.local_params.pitch_bend_range, 0.5..=12.0);
            });

            ui.add_space(30.0);

            ui.vertical(|ui| {
                section_label(ui, "PITCH MODE", ACCENT_BLUE);
                for mode in &[PitchMode::Hybrid, PitchMode::Crepe, PitchMode::Yin] {
                    let sel = self.local_params.pitch_mode == *mode;
                    let label = egui::RichText::new(mode.label()).size(12.0)
                        .color(if sel { ACCENT_BLUE } else { TEXT_BRIGHT });
                    if ui.selectable_label(sel, label).clicked() {
                        self.local_params.pitch_mode = *mode;
                        changed = true;
                    }
                }
            });
        });

        if !self.gui_state.midi_connected {
            ui.add_space(12.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("\u{26A0} No MIDI port. Install loopMIDI and restart.")
                        .color(ACCENT_ORANGE).size(12.0),
                );
            });
        }

        if changed { self.push_params(); }
    }
}

// ===========================================================================
// Triggers Tab
// ===========================================================================

impl VoicianApp {
    fn draw_triggers_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        ui.horizontal(|ui| {
            section_label(ui, "PERCUSSION TRIGGERS", ACCENT_RED);
            ui.add_space(20.0);
            if ui.checkbox(
                &mut self.local_params.triggers_enabled,
                egui::RichText::new("Enable").color(TEXT_BRIGHT).size(12.0),
            ).changed() {
                changed = true;
            }
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("MIDI Channel:").color(TEXT_DIM).size(11.0));
            let mut ch = self.local_params.trigger_channel as f32 + 1.0;
            let slider = egui::Slider::new(&mut ch, 1.0..=16.0).step_by(1.0).max_decimals(0);
            if ui.add(slider).changed() {
                self.local_params.trigger_channel = (ch - 1.0).round() as u8;
                changed = true;
            }
        });
        changed |= labeled_slider(ui, "Onset Sensitivity", &mut self.local_params.trigger_onset_threshold, 0.01..=0.3);

        ui.add_space(10.0);

        let slot_names = ["Kick", "Snare", "Hi-Hat", "Perc"];
        let slot_notes: [u8; 4] = [36, 38, 42, 39];
        let now = Instant::now();

        ui.columns(4, |cols| {
            for i in 0..4 {
                let color = TRIGGER_COLORS[i];
                let is_hit = now < self.gui_state.trigger_flash_until[i];

                egui::Frame::new()
                    .fill(if is_hit {
                        egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 40)
                    } else { PANEL_BG })
                    .corner_radius(6.0)
                    .inner_margin(10.0)
                    .show(&mut cols[i], |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new(slot_names[i]).color(color).size(16.0).strong());

                            let dot_color = if is_hit { color } else { egui::Color32::from_rgb(40, 40, 50) };
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                            ui.painter().circle_filled(rect.center(), 10.0, dot_color);

                            ui.add_space(6.0);
                            ui.label(egui::RichText::new(format!("Note: {}", slot_notes[i])).color(TEXT_DIM).size(11.0));
                            ui.label(egui::RichText::new(gm_drum_name(slot_notes[i])).color(TEXT_DIM).size(10.0));
                        });
                    });
            }
        });

        ui.add_space(12.0);
        ui.label(
            egui::RichText::new("Beatbox into your mic. Percussive sounds trigger MIDI drum notes.")
                .color(TEXT_DIM).size(11.0),
        );

        if changed { self.push_params(); }
    }
}

// ===========================================================================
// Controls Tab
// ===========================================================================

impl VoicianApp {
    fn draw_controls_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        // == Chords ==
        section_label(ui, "CHORD GENERATION", ACCENT_PURPLE);
        ui.horizontal(|ui| {
            if ui.checkbox(
                &mut self.local_params.chord_enabled,
                egui::RichText::new("Enable Chords").color(TEXT_BRIGHT).size(12.0),
            ).changed() { changed = true; }
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Type:").color(TEXT_DIM).size(11.0));
            egui::ComboBox::from_id_salt("chord_type")
                .selected_text(self.local_params.chord_type.label())
                .width(100.0)
                .show_ui(ui, |ui| {
                    for ct in ChordType::ALL {
                        if ui.selectable_value(&mut self.local_params.chord_type, *ct, ct.label()).changed() {
                            changed = true;
                        }
                    }
                });
            ui.add_space(16.0);
            ui.label(egui::RichText::new("Voicing:").color(TEXT_DIM).size(11.0));
            egui::ComboBox::from_id_salt("chord_voicing")
                .selected_text(format!("{:?}", self.local_params.chord_voicing))
                .width(110.0)
                .show_ui(ui, |ui| {
                    use crate::chords::Voicing;
                    for v in &[Voicing::RootPosition, Voicing::FirstInversion, Voicing::SecondInversion, Voicing::Spread] {
                        if ui.selectable_value(&mut self.local_params.chord_voicing, *v, format!("{:?}", v)).changed() {
                            changed = true;
                        }
                    }
                });
        });

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // == CC Mapping ==
        section_label(ui, "CC MAPPING", ACCENT_ORANGE);
        ui.horizontal(|ui| {
            if ui.checkbox(
                &mut self.local_params.cc_mapping_enabled,
                egui::RichText::new("Enable CC Mapping").color(TEXT_BRIGHT).size(12.0),
            ).changed() { changed = true; }
        });
        ui.add_space(4.0);

        let cc_labels = ["Slot 1", "Slot 2", "Slot 3", "Slot 4"];
        let snap_cc = self.gui_state.current.cc_values;

        egui::Grid::new("cc_grid").num_columns(4).spacing([10.0, 6.0]).striped(true).show(ui, |ui| {
            ui.label(egui::RichText::new("Slot").color(TEXT_DIM).size(10.0));
            ui.label(egui::RichText::new("Source").color(TEXT_DIM).size(10.0));
            ui.label(egui::RichText::new("CC#").color(TEXT_DIM).size(10.0));
            ui.label(egui::RichText::new("Value").color(TEXT_DIM).size(10.0));
            ui.end_row();

            for i in 0..NUM_CC_SLOTS {
                ui.label(egui::RichText::new(cc_labels[i]).color(TEXT_BRIGHT).size(11.0));

                egui::ComboBox::from_id_salt(format!("cc_src_{}", i))
                    .selected_text(self.local_params.cc_sources[i].label())
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        for src in CcSource::ALL {
                            if ui.selectable_value(&mut self.local_params.cc_sources[i], *src, src.label()).changed() {
                                changed = true;
                            }
                        }
                    });

                let mut cc_f = self.local_params.cc_numbers[i] as f32;
                let slider = egui::Slider::new(&mut cc_f, 0.0..=127.0).step_by(1.0).max_decimals(0);
                if ui.add(slider).changed() {
                    self.local_params.cc_numbers[i] = cc_f as u8;
                    changed = true;
                }

                let val = snap_cc[i];
                let norm = val as f32 / 127.0;
                let (rect, _) = ui.allocate_exact_size(egui::vec2(50.0, 14.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 3.0, egui::Color32::from_rgb(35, 35, 45));
                let fill = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * norm, rect.height()));
                ui.painter().rect_filled(fill, 3.0, ACCENT_ORANGE);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, format!("{}", val), egui::FontId::proportional(9.0), TEXT_BRIGHT);

                ui.end_row();
            }
        });

        ui.add_space(12.0);
        ui.label(egui::RichText::new("Map voice features to MIDI CC controllers.").color(TEXT_DIM).size(11.0));

        if changed { self.push_params(); }
    }
}

// ===========================================================================
// Monitor Tab
// ===========================================================================

impl VoicianApp {
    fn draw_monitor_tab(&self, ui: &mut egui::Ui) {
        let graph_h = ((ui.available_height() - 40.0) / 2.0).max(60.0);

        ui.columns(2, |cols| {
            cols[0].label(egui::RichText::new("Volume (RMS)").color(TEXT_DIM).size(10.0));
            draw_graph(&mut cols[0], &self.gui_state.rms_history, 0.0, 0.5, ACCENT_BLUE, graph_h);
            cols[0].add_space(4.0);
            cols[0].label(egui::RichText::new("Pitch (Hz)").color(TEXT_DIM).size(10.0));
            draw_graph(&mut cols[0], &self.gui_state.pitch_history, 0.0, 800.0, ACCENT_GREEN, graph_h);

            cols[1].label(egui::RichText::new("Confidence").color(TEXT_DIM).size(10.0));
            draw_graph(&mut cols[1], &self.gui_state.confidence_history, 0.0, 1.0, ACCENT_PURPLE, graph_h);
            cols[1].add_space(4.0);
            cols[1].label(egui::RichText::new("Centroid (Hz)").color(TEXT_DIM).size(10.0));
            draw_graph(&mut cols[1], &self.gui_state.centroid_history, 0.0, 4000.0, ACCENT_CYAN, graph_h);
        });
    }
}

// ===========================================================================
// Advanced Settings sidebar
// ===========================================================================

impl VoicianApp {
    fn draw_advanced_settings(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            section_label(ui, "DETECTION", ACCENT_GREEN);
            changed |= labeled_slider(ui, "CREPE Confidence", &mut self.local_params.confidence_threshold, 0.1..=0.95);
            changed |= labeled_slider(ui, "YIN Threshold", &mut self.local_params.yin_threshold, 0.01..=0.5);
            changed |= labeled_slider(ui, "Silence Gate", &mut self.local_params.silence_threshold, 0.001..=0.1);

            ui.add_space(8.0); ui.separator(); ui.add_space(4.0);

            section_label(ui, "NOTE STABILITY", ACCENT_ORANGE);
            {
                let mut f = self.local_params.stability_frames as f32;
                if labeled_slider(ui, "Stability Frames", &mut f, 1.0..=8.0) {
                    self.local_params.stability_frames = f.round() as usize;
                    changed = true;
                }
            }
            changed |= labeled_slider(ui, "Stability Tolerance", &mut self.local_params.stability_tolerance, 0.05..=1.0);
            changed |= labeled_slider(ui, "Note Change Thresh", &mut self.local_params.note_change_threshold, 0.2..=1.5);

            ui.add_space(8.0); ui.separator(); ui.add_space(4.0);

            section_label(ui, "SMOOTHING", ACCENT_PURPLE);
            changed |= labeled_slider(ui, "Pitch", &mut self.local_params.pitch_smoothing, 0.0..=0.95);
            changed |= labeled_slider(ui, "Amplitude", &mut self.local_params.amplitude_smoothing, 0.0..=0.95);
            changed |= labeled_slider(ui, "Centroid", &mut self.local_params.centroid_smoothing, 0.0..=0.95);

            ui.add_space(8.0); ui.separator(); ui.add_space(4.0);

            section_label(ui, "MIDI OUTPUT", ACCENT_CYAN);
            {
                let mut ch = self.local_params.midi_channel as f32 + 1.0;
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Channel").color(TEXT_DIM).size(11.0));
                    let s = egui::Slider::new(&mut ch, 1.0..=16.0).step_by(1.0).max_decimals(0);
                    if ui.add(s).changed() {
                        self.local_params.midi_channel = (ch - 1.0).round() as u8;
                        changed = true;
                    }
                });
            }

            ui.add_space(8.0); ui.separator(); ui.add_space(4.0);

            section_label(ui, "FREQ RANGE", ACCENT_RED);
            changed |= labeled_slider_hz(ui, "Min Freq", &mut self.local_params.min_freq_hz, 30.0..=500.0);
            changed |= labeled_slider_hz(ui, "Max Freq", &mut self.local_params.max_freq_hz, 200.0..=2000.0);

            ui.add_space(10.0); ui.separator(); ui.add_space(4.0);

            if ui.button(egui::RichText::new("\u{21BA} Reset to Defaults").color(TEXT_BRIGHT).size(12.0)).clicked() {
                self.local_params = EngineParams::default();
                changed = true;
            }
        });

        if changed { self.push_params(); }
    }
}

// ---------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------

fn section_label(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    ui.label(egui::RichText::new(text).color(color).size(11.0).strong());
    ui.add_space(2.0);
}

fn pitch_bend_norm(value: u16) -> f32 {
    ((value as f32 - 8192.0) / 8192.0).abs()
}

fn note_name_util(midi: u8) -> String {
    const NAMES: [&str; 12] = ["C","C#","D","D#","E","F","F#","G","G#","A","A#","B"];
    let name = NAMES[(midi % 12) as usize];
    let oct = (midi as i32 / 12) - 1;
    format!("{}{}", name, oct)
}

fn gm_drum_name(note: u8) -> &'static str {
    match note {
        35 => "Acoustic Bass Drum",
        36 => "Bass Drum 1",
        37 => "Side Stick",
        38 => "Acoustic Snare",
        39 => "Hand Clap",
        40 => "Electric Snare",
        42 => "Closed Hi-Hat",
        44 => "Pedal Hi-Hat",
        46 => "Open Hi-Hat",
        49 => "Crash Cymbal 1",
        51 => "Ride Cymbal 1",
        _ => "Percussion",
    }
}

fn draw_meter(
    ui: &mut egui::Ui,
    label: &str,
    value: f32,
    max_val: f32,
    color: egui::Color32,
    width: f32,
) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).color(TEXT_DIM).size(10.0));
        let normalized = (value / max_val).clamp(0.0, 1.0);
        let height = 12.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(35, 35, 45));
        let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * normalized, rect.height()));
        ui.painter().rect_filled(fill_rect, 4.0, color);
        let text = format!("{:.0}%", normalized * 100.0);
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, egui::FontId::proportional(9.0), TEXT_BRIGHT);
    });
}

fn draw_graph(
    ui: &mut egui::Ui,
    data: &std::collections::VecDeque<f32>,
    min_val: f32,
    max_val: f32,
    color: egui::Color32,
    height: f32,
) {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(22, 22, 30));

    if data.len() < 2 { return; }

    let range = (max_val - min_val).max(0.001);
    let n = data.len();
    let step = rect.width() / (n - 1) as f32;

    let points: Vec<egui::Pos2> = data.iter().enumerate().map(|(i, &val)| {
        let x = rect.min.x + i as f32 * step;
        let normalized = ((val - min_val) / range).clamp(0.0, 1.0);
        let y = rect.max.y - normalized * rect.height();
        egui::pos2(x, y)
    }).collect();

    let stroke = egui::Stroke::new(1.5, color);
    for pair in points.windows(2) {
        ui.painter().line_segment([pair[0], pair[1]], stroke);
    }
}

fn labeled_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(TEXT_DIM).size(11.0));
    });
    let slider = egui::Slider::new(value, range).max_decimals(3).step_by(0.005);
    ui.add(slider).changed()
}

fn labeled_slider_hz(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(format!("{} ({:.0} Hz)", label, *value)).color(TEXT_DIM).size(11.0));
    });
    let slider = egui::Slider::new(value, range).max_decimals(0).step_by(10.0).suffix(" Hz");
    ui.add(slider).changed()
}

// ---------------------------------------------------------------------------
// Dark theme
// ---------------------------------------------------------------------------

fn apply_dark_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let visuals = &mut style.visuals;

    visuals.dark_mode = true;
    visuals.panel_fill = BG_DARK;
    visuals.window_fill = PANEL_BG;
    visuals.faint_bg_color = PANEL_BG;
    visuals.extreme_bg_color = egui::Color32::from_rgb(12, 12, 18);
    visuals.override_text_color = Some(TEXT_BRIGHT);

    visuals.widgets.noninteractive.bg_fill = PANEL_BG;
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(40, 40, 55);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(50, 50, 70);
    visuals.widgets.active.bg_fill = ACCENT_BLUE;

    visuals.selection.bg_fill = ACCENT_BLUE;
    visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT_BLUE);

    ctx.set_style(style);
}

// ---------------------------------------------------------------------------
// eframe launcher
// ---------------------------------------------------------------------------

pub fn run_gui(gui_state: GuiState) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 650.0])
            .with_min_inner_size([700.0, 480.0])
            .with_title("Voician v1.0 — Voice to MIDI"),
        ..Default::default()
    };

    eframe::run_native(
        "Voician",
        options,
        Box::new(|_cc| Ok(Box::new(VoicianApp::new(gui_state)))),
    )
}
