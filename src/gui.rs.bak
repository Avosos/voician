// ============================================================================
// gui.rs — Voician v1.0: Dubler-inspired premium dark GUI
// ============================================================================
//
//   ┌────────────────────────────────────────────────────────┐
//   │  VOICIAN  │ source │ MIDI │ key              ⚙ Log 🎵 │
//   ├────────────────────────────────────────────────────────┤
//   │   Pitch    Triggers    Controls    Monitor             │
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
// Dubler-inspired color palette
// ---------------------------------------------------------------------------

/// Near-black background with a subtle cool tint.
const BG_DARK: egui::Color32 = egui::Color32::from_rgb(10, 10, 18);
/// Card/panel surfaces — slightly lifted from background.
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(18, 18, 32);
/// Sidebar / secondary panels.
const SIDEBAR_BG: egui::Color32 = egui::Color32::from_rgb(14, 14, 26);
/// Tab bar background — seamless with header.
const TAB_BAR_BG: egui::Color32 = egui::Color32::from_rgb(14, 14, 26);
/// Card surface (raised container).
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(22, 22, 40);
/// Subtle card/section border.
const CARD_BORDER: egui::Color32 = egui::Color32::from_rgb(38, 38, 62);
/// Meter / graph track (empty).
const TRACK_BG: egui::Color32 = egui::Color32::from_rgb(26, 26, 44);

/// **Primary accent** — Dubler-style teal / turquoise.
const TEAL: egui::Color32 = egui::Color32::from_rgb(0, 212, 170);
/// Muted teal for dimmed states.
const TEAL_DIM: egui::Color32 = egui::Color32::from_rgb(0, 140, 112);
/// Pitch / melody accent — purple.
const PURPLE: egui::Color32 = egui::Color32::from_rgb(168, 85, 247);
/// Trigger / percussion accent — warm orange.
const ORANGE: egui::Color32 = egui::Color32::from_rgb(251, 146, 60);
/// Alert / stop — red.
const RED: egui::Color32 = egui::Color32::from_rgb(248, 72, 94);
/// Confidence / secondary — sky blue.
const SKY: egui::Color32 = egui::Color32::from_rgb(56, 189, 248);
/// Scale lock / key — gold.
const GOLD: egui::Color32 = egui::Color32::from_rgb(250, 204, 21);
/// Active note — bright green.
const NEON_GREEN: egui::Color32 = egui::Color32::from_rgb(52, 211, 153);
/// Chord / harmony accent.
const PINK: egui::Color32 = egui::Color32::from_rgb(236, 72, 153);

/// Dimmed text (labels, captions).
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(100, 100, 130);
/// Secondary text (descriptions).
const TEXT_MID: egui::Color32 = egui::Color32::from_rgb(150, 150, 175);
/// Primary text (values, headings).
const TEXT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(230, 230, 245);

/// Trigger pad colors (one per slot).
const TRIGGER_COLORS: [egui::Color32; 4] = [RED, ORANGE, GOLD, SKY];

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
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(egui::Margin::symmetric(12, 8)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Brand.
                    ui.label(
                        egui::RichText::new("VOICIAN")
                            .color(TEAL)
                            .size(20.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("v1.0").color(TEXT_DIM).size(10.0),
                    );

                    ui.add_space(16.0);

                    // Pitch source pill.
                    let src = self.gui_state.current.pitch_source;
                    let src_color = match src {
                        PitchSource::Crepe => PURPLE,
                        PitchSource::Yin => SKY,
                        PitchSource::None => TEXT_DIM,
                    };
                    draw_pill(ui, src.label(), src_color, src_color);

                    ui.add_space(8.0);

                    // MIDI status pill.
                    if self.gui_state.midi_connected {
                        draw_pill(ui, "MIDI", NEON_GREEN, NEON_GREEN);
                    } else {
                        draw_pill(ui, "NO MIDI", RED, RED);
                    }

                    // Detected key pill.
                    if !self.gui_state.current.detected_key.is_empty() {
                        ui.add_space(8.0);
                        draw_pill(ui, &format!("Key: {}", self.gui_state.current.detected_key), GOLD, GOLD);
                    }

                    // MIDI activity dot.
                    ui.add_space(6.0);
                    let midi_active = Instant::now() < self.gui_state.midi_flash_until;
                    let dot_color = if midi_active { NEON_GREEN } else { egui::Color32::from_rgb(30, 30, 50) };
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(rect.center(), 4.0, dot_color);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Strudel.
                        let strudel_label = if self.gui_state.strudel_open { "\u{266B} Strudel" } else { "\u{266B}" };
                        let strudel_btn = egui::Button::new(
                            egui::RichText::new(strudel_label).size(11.0).color(TEXT_MID)
                        ).fill(egui::Color32::TRANSPARENT);
                        if ui.add(strudel_btn).clicked() {
                            self.gui_state.strudel_open = true;
                            crate::strudel::open_browser();
                        }

                        // Log toggle.
                        let log_label = if self.gui_state.show_midi_log { "Log \u{25BE}" } else { "Log" };
                        let log_btn = egui::Button::new(
                            egui::RichText::new(log_label).size(11.0).color(TEXT_MID)
                        ).fill(egui::Color32::TRANSPARENT);
                        if ui.add(log_btn).clicked() {
                            self.gui_state.show_midi_log = !self.gui_state.show_midi_log;
                        }

                        // Settings toggle.
                        let gear_label = if self.gui_state.show_settings { "\u{2699}" } else { "\u{2699}" };
                        let gear_btn = egui::Button::new(
                            egui::RichText::new(gear_label).size(14.0)
                                .color(if self.gui_state.show_settings { TEAL } else { TEXT_MID })
                        ).fill(egui::Color32::TRANSPARENT);
                        if ui.add(gear_btn).clicked() {
                            self.gui_state.show_settings = !self.gui_state.show_settings;
                        }

                        ui.label(
                            egui::RichText::new(format!("{} Hz", self.gui_state.sample_rate))
                                .color(egui::Color32::from_rgb(60, 60, 80)).size(9.0),
                        );
                    });
                });
            });

        // == Tab bar (underline style) ==
        egui::TopBottomPanel::top("tab_bar")
            .frame(egui::Frame::new().fill(TAB_BAR_BG).inner_margin(egui::Margin { left: 12, right: 12, top: 6, bottom: 0 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for tab in GuiTab::ALL {
                        let active = self.gui_state.active_tab == *tab;
                        let text_color = if active { TEAL } else { TEXT_DIM };

                        let label = egui::RichText::new(tab.label())
                            .size(13.0)
                            .color(text_color);

                        let btn = egui::Button::new(label)
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE);
                        let resp = ui.add(btn);

                        // Draw underline for active tab.
                        if active {
                            let rect = resp.rect;
                            let y = rect.max.y + 2.0;
                            ui.painter().line_segment(
                                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                                egui::Stroke::new(2.5, TEAL),
                            );
                        }

                        if resp.clicked() {
                            self.gui_state.active_tab = *tab;
                        }

                        ui.add_space(8.0);
                    }
                });
                // Thin separator line at bottom.
                ui.add_space(4.0);
                let rect = ui.available_rect_before_wrap();
                ui.painter().line_segment(
                    [egui::pos2(rect.min.x, rect.min.y), egui::pos2(rect.max.x, rect.min.y)],
                    egui::Stroke::new(1.0, CARD_BORDER),
                );
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

        // ── Central note display with circular glow ──────────────────────
        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            let avail_w = ui.available_width();
            let circle_r = 80.0_f32.min(avail_w * 0.12);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(circle_r * 2.0 + 20.0, circle_r * 2.0 + 20.0),
                egui::Sense::hover(),
            );
            let center = rect.center();

            // Outer glow rings (when note is active).
            if snap.note_active {
                for i in 0..3 {
                    let r = circle_r + 10.0 + i as f32 * 8.0;
                    let alpha = (30 - i * 10) as u8;
                    ui.painter().circle_stroke(
                        center, r,
                        egui::Stroke::new(1.5, egui::Color32::from_rgba_premultiplied(
                            TEAL.r(), TEAL.g(), TEAL.b(), alpha,
                        )),
                    );
                }
            }

            // Main circle.
            let circle_fill = if snap.note_active { CARD_BG } else { TRACK_BG };
            ui.painter().circle_filled(center, circle_r, circle_fill);
            ui.painter().circle_stroke(
                center, circle_r,
                egui::Stroke::new(2.0, if snap.note_active { TEAL } else { CARD_BORDER }),
            );

            // Note name inside circle.
            let note_color = if snap.note_active { TEXT_BRIGHT } else { TEXT_DIM };
            ui.painter().text(
                center + egui::vec2(0.0, -6.0),
                egui::Align2::CENTER_CENTER,
                &snap.note_name,
                egui::FontId::proportional(48.0),
                note_color,
            );

            // Frequency below note name.
            if snap.frequency > 0.0 {
                ui.painter().text(
                    center + egui::vec2(0.0, 28.0),
                    egui::Align2::CENTER_CENTER,
                    format!("{:.1} Hz", snap.frequency),
                    egui::FontId::proportional(11.0),
                    TEXT_DIM,
                );
            }
        });

        // Quantized note + chord display below circle.
        ui.vertical_centered(|ui| {
            if self.local_params.scale_lock_enabled && !snap.quantized_note_name.is_empty() {
                ui.label(
                    egui::RichText::new(format!("\u{2192} {}", snap.quantized_note_name))
                        .color(GOLD).size(16.0).strong(),
                );
            }
            if !snap.chord_notes.is_empty() {
                let chord_str: Vec<String> = snap.chord_notes.iter().map(|n| note_name_util(*n)).collect();
                ui.label(
                    egui::RichText::new(chord_str.join("  \u{2022}  "))
                        .color(PINK).size(12.0),
                );
            }
        });

        ui.add_space(8.0);

        // ── Meter row (thin bars inside a card) ──────────────────────────
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                let w = ((ui.available_width() - 36.0) / 4.0).max(60.0);
                draw_meter(ui, "VOL", snap.rms, 0.5, TEAL, w);
                ui.add_space(8.0);
                draw_meter(ui, "VEL", snap.velocity as f32 / 127.0, 1.0, ORANGE, w);
                ui.add_space(8.0);
                draw_meter(ui, "CONF", snap.confidence, 1.0, PURPLE, w);
                ui.add_space(8.0);
                draw_meter(ui, "BEND", pitch_bend_norm(snap.pitch_bend), 1.0, SKY, w);
            });
        });

        ui.add_space(8.0);

        // ── Three-column controls in cards ───────────────────────────────
        ui.columns(3, |cols| {
            // Column 1: Scale Lock.
            card_frame(&mut cols[0], |ui| {
                section_label(ui, "SCALE LOCK", GOLD);
                if ui.checkbox(
                    &mut self.local_params.scale_lock_enabled,
                    egui::RichText::new("Enable").color(TEXT_BRIGHT).size(11.0),
                ).changed() { changed = true; }
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Root").color(TEXT_DIM).size(10.0));
                    egui::ComboBox::from_id_salt("root_note")
                        .selected_text(self.local_params.root_note.label())
                        .width(46.0)
                        .show_ui(ui, |ui| {
                            for root in RootNote::ALL {
                                if ui.selectable_value(&mut self.local_params.root_note, *root, root.label()).changed() { changed = true; }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Scale").color(TEXT_DIM).size(10.0));
                    egui::ComboBox::from_id_salt("scale_type")
                        .selected_text(self.local_params.scale_type.label())
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            for scale in ScaleType::ALL {
                                if ui.selectable_value(&mut self.local_params.scale_type, *scale, scale.label()).changed() { changed = true; }
                            }
                        });
                });
                if ui.checkbox(
                    &mut self.local_params.auto_key_detect,
                    egui::RichText::new("Auto-Detect").color(TEXT_MID).size(10.0),
                ).changed() { changed = true; }
            });

            // Column 2: Pitch Bend.
            card_frame(&mut cols[1], |ui| {
                section_label(ui, "PITCH BEND", SKY);
                for mode in PitchBendMode::ALL {
                    let sel = self.local_params.pitch_bend_mode == *mode;
                    let color = if sel { SKY } else { TEXT_DIM };
                    let resp = ui.selectable_label(sel, egui::RichText::new(mode.label()).size(11.0).color(color));
                    if resp.clicked() { self.local_params.pitch_bend_mode = *mode; changed = true; }
                }
                ui.add_space(4.0);
                changed |= labeled_slider(ui, "Range (st)", &mut self.local_params.pitch_bend_range, 0.5..=12.0);
            });

            // Column 3: Pitch Mode.
            card_frame(&mut cols[2], |ui| {
                section_label(ui, "PITCH MODE", TEAL);
                for mode in &[PitchMode::Hybrid, PitchMode::Crepe, PitchMode::Yin] {
                    let sel = self.local_params.pitch_mode == *mode;
                    let color = if sel { TEAL } else { TEXT_DIM };
                    let resp = ui.selectable_label(sel, egui::RichText::new(mode.label()).size(11.0).color(color));
                    if resp.clicked() { self.local_params.pitch_mode = *mode; changed = true; }
                }
            });
        });

        if !self.gui_state.midi_connected {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("\u{26A0}  No MIDI port — install loopMIDI and restart")
                        .color(ORANGE).size(11.0),
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

        // Header row.
        ui.horizontal(|ui| {
            section_label(ui, "PERCUSSION TRIGGERS", ORANGE);
            ui.add_space(12.0);
            if ui.checkbox(
                &mut self.local_params.triggers_enabled,
                egui::RichText::new("Enable").color(TEXT_BRIGHT).size(11.0),
            ).changed() { changed = true; }
        });

        // Settings row in a card.
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("MIDI Ch").color(TEXT_DIM).size(10.0));
                let mut ch = self.local_params.trigger_channel as f32 + 1.0;
                let slider = egui::Slider::new(&mut ch, 1.0..=16.0).step_by(1.0).max_decimals(0);
                if ui.add(slider).changed() {
                    self.local_params.trigger_channel = (ch - 1.0).round() as u8;
                    changed = true;
                }
                ui.add_space(16.0);
                ui.label(egui::RichText::new("Sensitivity").color(TEXT_DIM).size(10.0));
                if ui.add(
                    egui::Slider::new(&mut self.local_params.trigger_onset_threshold, 0.01..=0.3)
                        .max_decimals(3).step_by(0.005)
                ).changed() { changed = true; }
            });
        });

        ui.add_space(12.0);

        // ── Large circular drum pads ─────────────────────────────────────
        let slot_names = ["Kick", "Snare", "Hi-Hat", "Perc"];
        let slot_notes: [u8; 4] = [36, 38, 42, 39];
        let now = Instant::now();

        let pad_area = ui.available_width();
        let pad_size = ((pad_area - 60.0) / 4.0).min(140.0).max(60.0);
        let circle_r = pad_size * 0.42;

        ui.horizontal(|ui| {
            ui.add_space((pad_area - (pad_size * 4.0 + 36.0)).max(0.0) / 2.0);

            for i in 0..4 {
                let color = TRIGGER_COLORS[i];
                let is_hit = now < self.gui_state.trigger_flash_until[i];

                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(pad_size, pad_size + 40.0),
                    egui::Sense::hover(),
                );
                let pad_center = egui::pos2(rect.center().x, rect.min.y + pad_size * 0.5);

                // Outer glow when hit.
                if is_hit {
                    for ring in 0..4 {
                        let r = circle_r + 6.0 + ring as f32 * 6.0;
                        let a = (60 - ring * 15).max(0) as u8;
                        ui.painter().circle_stroke(
                            pad_center, r,
                            egui::Stroke::new(2.0, egui::Color32::from_rgba_premultiplied(
                                color.r(), color.g(), color.b(), a,
                            )),
                        );
                    }
                }

                // Pad circle.
                let fill = if is_hit {
                    egui::Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 50)
                } else {
                    CARD_BG
                };
                ui.painter().circle_filled(pad_center, circle_r, fill);
                ui.painter().circle_stroke(
                    pad_center, circle_r,
                    egui::Stroke::new(if is_hit { 2.5 } else { 1.5 }, color),
                );

                // Inner dot.
                let dot_color = if is_hit { color } else { egui::Color32::from_rgb(40, 40, 56) };
                ui.painter().circle_filled(pad_center, 6.0, dot_color);

                // Label below pad.
                ui.painter().text(
                    egui::pos2(pad_center.x, rect.min.y + pad_size + 6.0),
                    egui::Align2::CENTER_TOP,
                    slot_names[i],
                    egui::FontId::proportional(13.0),
                    if is_hit { color } else { TEXT_BRIGHT },
                );
                ui.painter().text(
                    egui::pos2(pad_center.x, rect.min.y + pad_size + 22.0),
                    egui::Align2::CENTER_TOP,
                    format!("{} · {}", slot_notes[i], gm_drum_name(slot_notes[i])),
                    egui::FontId::proportional(9.0),
                    TEXT_DIM,
                );

                if i < 3 { ui.add_space(12.0); }
            }
        });

        ui.add_space(16.0);
        ui.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("Beatbox into your mic — percussive sounds trigger drum MIDI notes")
                    .color(TEXT_DIM).size(10.0),
            );
        });

        if changed { self.push_params(); }
    }
}

// ===========================================================================
// Controls Tab
// ===========================================================================

impl VoicianApp {
    fn draw_controls_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        // ── Chords card ──────────────────────────────────────────────────
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, "CHORD GENERATION", PINK);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.checkbox(
                        &mut self.local_params.chord_enabled,
                        egui::RichText::new("Enable").color(TEXT_BRIGHT).size(11.0),
                    ).changed() { changed = true; }
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Type").color(TEXT_DIM).size(10.0));
                egui::ComboBox::from_id_salt("chord_type")
                    .selected_text(self.local_params.chord_type.label())
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        for ct in ChordType::ALL {
                            if ui.selectable_value(&mut self.local_params.chord_type, *ct, ct.label()).changed() { changed = true; }
                        }
                    });
                ui.add_space(16.0);
                ui.label(egui::RichText::new("Voicing").color(TEXT_DIM).size(10.0));
                egui::ComboBox::from_id_salt("chord_voicing")
                    .selected_text(format!("{:?}", self.local_params.chord_voicing))
                    .width(110.0)
                    .show_ui(ui, |ui| {
                        use crate::chords::Voicing;
                        for v in &[Voicing::RootPosition, Voicing::FirstInversion, Voicing::SecondInversion, Voicing::Spread] {
                            if ui.selectable_value(&mut self.local_params.chord_voicing, *v, format!("{:?}", v)).changed() { changed = true; }
                        }
                    });
            });
        });

        ui.add_space(8.0);

        // ── CC Mapping card ──────────────────────────────────────────────
        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, "CC MAPPING", ORANGE);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.checkbox(
                        &mut self.local_params.cc_mapping_enabled,
                        egui::RichText::new("Enable").color(TEXT_BRIGHT).size(11.0),
                    ).changed() { changed = true; }
                });
            });
            ui.add_space(4.0);

            let snap_cc = self.gui_state.current.cc_values;
            let slot_labels = ["1", "2", "3", "4"];

            egui::Grid::new("cc_grid")
                .num_columns(4)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    // Header.
                    ui.label(egui::RichText::new("").size(1.0));
                    ui.label(egui::RichText::new("Source").color(TEXT_DIM).size(9.0));
                    ui.label(egui::RichText::new("CC#").color(TEXT_DIM).size(9.0));
                    ui.label(egui::RichText::new("Value").color(TEXT_DIM).size(9.0));
                    ui.end_row();

                    for i in 0..NUM_CC_SLOTS {
                        // Slot badge.
                        let badge_rect = ui.label(
                            egui::RichText::new(slot_labels[i]).color(TEAL).size(12.0).strong()
                        ).rect;
                        let _ = badge_rect;

                        egui::ComboBox::from_id_salt(format!("cc_src_{}", i))
                            .selected_text(self.local_params.cc_sources[i].label())
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for src in CcSource::ALL {
                                    if ui.selectable_value(&mut self.local_params.cc_sources[i], *src, src.label()).changed() { changed = true; }
                                }
                            });

                        let mut cc_f = self.local_params.cc_numbers[i] as f32;
                        if ui.add(egui::Slider::new(&mut cc_f, 0.0..=127.0).step_by(1.0).max_decimals(0)).changed() {
                            self.local_params.cc_numbers[i] = cc_f as u8;
                            changed = true;
                        }

                        // Value bar.
                        let val = snap_cc[i];
                        let norm = val as f32 / 127.0;
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(60.0, 16.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 4.0, TRACK_BG);
                        if norm > 0.0 {
                            let fill = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * norm, rect.height()));
                            ui.painter().rect_filled(fill, 4.0, ORANGE);
                        }
                        ui.painter().text(
                            rect.center(), egui::Align2::CENTER_CENTER,
                            format!("{}", val), egui::FontId::proportional(9.0), TEXT_BRIGHT,
                        );

                        ui.end_row();
                    }
                });
        });

        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("Map voice features to MIDI CC controllers").color(TEXT_DIM).size(10.0));
        });

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
            card_frame(&mut cols[0], |ui| {
                ui.label(egui::RichText::new("VOLUME (RMS)").color(TEXT_DIM).size(9.0));
                draw_graph(ui, &self.gui_state.rms_history, 0.0, 0.5, TEAL, graph_h - 30.0);
            });
            cols[0].add_space(6.0);
            card_frame(&mut cols[0], |ui| {
                ui.label(egui::RichText::new("PITCH (HZ)").color(TEXT_DIM).size(9.0));
                draw_graph(ui, &self.gui_state.pitch_history, 0.0, 800.0, NEON_GREEN, graph_h - 30.0);
            });

            card_frame(&mut cols[1], |ui| {
                ui.label(egui::RichText::new("CONFIDENCE").color(TEXT_DIM).size(9.0));
                draw_graph(ui, &self.gui_state.confidence_history, 0.0, 1.0, PURPLE, graph_h - 30.0);
            });
            cols[1].add_space(6.0);
            card_frame(&mut cols[1], |ui| {
                ui.label(egui::RichText::new("CENTROID (HZ)").color(TEXT_DIM).size(9.0));
                draw_graph(ui, &self.gui_state.centroid_history, 0.0, 4000.0, SKY, graph_h - 30.0);
            });
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
            section_label(ui, "DETECTION", NEON_GREEN);
            changed |= labeled_slider(ui, "CREPE Confidence", &mut self.local_params.confidence_threshold, 0.1..=0.95);
            changed |= labeled_slider(ui, "YIN Threshold", &mut self.local_params.yin_threshold, 0.01..=0.5);
            changed |= labeled_slider(ui, "Silence Gate", &mut self.local_params.silence_threshold, 0.001..=0.1);

            ui.add_space(8.0); ui.separator(); ui.add_space(4.0);

            section_label(ui, "NOTE STABILITY", ORANGE);
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

            section_label(ui, "SMOOTHING", PURPLE);
            changed |= labeled_slider(ui, "Pitch", &mut self.local_params.pitch_smoothing, 0.0..=0.95);
            changed |= labeled_slider(ui, "Amplitude", &mut self.local_params.amplitude_smoothing, 0.0..=0.95);
            changed |= labeled_slider(ui, "Centroid", &mut self.local_params.centroid_smoothing, 0.0..=0.95);

            ui.add_space(8.0); ui.separator(); ui.add_space(4.0);

            section_label(ui, "MIDI OUTPUT", SKY);
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

            section_label(ui, "FREQ RANGE", RED);
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

/// Rounded card container with CARD_BG fill and CARD_BORDER stroke.
fn card_frame(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, CARD_BORDER))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            add_contents(ui);
        });
}

/// Small rounded pill badge (used in top-bar status indicators).
fn draw_pill(ui: &mut egui::Ui, label: &str, fg: egui::Color32, border: egui::Color32) {
    let galley = ui.painter().layout_no_wrap(
        label.to_string(),
        egui::FontId::proportional(9.0),
        fg,
    );
    let text_size = galley.size();
    let pad = egui::vec2(8.0, 3.0);
    let desired = text_size + pad * 2.0;
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    ui.painter().rect_stroke(rect, rect.height() / 2.0, egui::Stroke::new(1.0, border), egui::epaint::StrokeKind::Outside);
    ui.painter().galley(
        egui::pos2(rect.min.x + pad.x, rect.min.y + pad.y),
        galley,
        fg,
    );
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
        ui.label(egui::RichText::new(label).color(TEXT_DIM).size(9.0));
        let normalized = (value / max_val).clamp(0.0, 1.0);
        let height = 10.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        ui.painter().rect_filled(rect, 5.0, TRACK_BG);
        if normalized > 0.0 {
            let fill_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * normalized, rect.height()));
            ui.painter().rect_filled(fill_rect, 5.0, color);
        }
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
    ui.painter().rect_filled(rect, 6.0, TRACK_BG);

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
    visuals.extreme_bg_color = egui::Color32::from_rgb(8, 8, 14);
    visuals.override_text_color = Some(TEXT_BRIGHT);

    visuals.widgets.noninteractive.bg_fill = PANEL_BG;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, CARD_BORDER);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 30, 48);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, CARD_BORDER);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(38, 38, 58);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, TEAL_DIM);
    visuals.widgets.active.bg_fill = TEAL_DIM;

    visuals.selection.bg_fill = TEAL_DIM;
    visuals.selection.stroke = egui::Stroke::new(1.0, TEAL);

    // Rounded everything for a premium feel.
    style.visuals.window_corner_radius = egui::CornerRadius::same(10);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);

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
