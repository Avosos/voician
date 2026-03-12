// ============================================================================
// gui.rs — Voician v1.0: Dubler 2 inspired GUI
// ============================================================================
//
//   ┌──────────────────────────────────────────────────────────────┐
//   │  VOICIAN  │  ●Crepe  ●MIDI  ●Key:G    ⚙ Log ♫  48000 Hz   │
//   ├──────────────────────────────────────────────────────────────┤
//   │   Play      Key      Chords     Assign     Monitor          │
//   ├──────────────────────────────────────────────────────────────┤
//   │ ● Triggers     │  ● Pitch              │  VOL  VEL         │
//   │ Sensitivity ── │  [Chromatic Wheel]     │  ◔    ◔           │
//   │ ┌────┐ ┌────┐  │       B3              │  CONF BEND        │
//   │ │Kick│ │Snar│  │                       │  ◔    ◔           │
//   │ ┌────┐ ┌────┐  │  Input Level ── Stk ──│                   │
//   │ │HiH │ │Perc│  │                       │                   │
//   ├──────────────────────────────────────────────────────────────┤
//   │ ●PB [48] │ Key [G▼] [Major▼] │ ●Chord [Triads▼] │ Oct [4] │
//   ├──────────────────────────────────────────────────────────────┤
//   │ ▓▓▓▓░▓▓▓░▓▓▓░▓░▓▓▓░▓▓▓░▓▓▓▓░▓▓▓░▓▓▓░▓░▓▓▓░▓▓▓░   piano │
//   └──────────────────────────────────────────────────────────────┘
// ============================================================================

use crate::cc_map::{CcSource, NUM_CC_SLOTS};
use crate::chords::ChordType;
use crate::scale::{RootNote, ScaleType};
use crate::state::{
    EngineParams, GuiState, GuiTab, PitchBendMode, PitchMode, PitchSource, SharedParams,
};
use eframe::egui;
use std::f32::consts::{FRAC_PI_2, PI, TAU};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Dubler 2 warm-charcoal color palette
// ---------------------------------------------------------------------------

const BG_DARK: egui::Color32 = egui::Color32::from_rgb(44, 44, 52);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(58, 58, 68);
const SIDEBAR_BG: egui::Color32 = egui::Color32::from_rgb(50, 50, 60);
const TAB_BAR_BG: egui::Color32 = egui::Color32::from_rgb(58, 58, 68);
const CARD_BG: egui::Color32 = egui::Color32::from_rgb(62, 62, 74);
const CARD_BORDER: egui::Color32 = egui::Color32::from_rgb(78, 78, 90);
const TRACK_BG: egui::Color32 = egui::Color32::from_rgb(50, 50, 62);

const SEGMENT_BG: egui::Color32 = egui::Color32::from_rgb(52, 52, 64);
const SEGMENT_BORDER: egui::Color32 = egui::Color32::from_rgb(38, 38, 46);
const INNER_CIRCLE: egui::Color32 = egui::Color32::from_rgb(40, 40, 50);
const PAD_BG: egui::Color32 = egui::Color32::from_rgb(66, 66, 78);

const PITCH_PINK: egui::Color32 = egui::Color32::from_rgb(228, 140, 140);
const TRIG_TEAL: egui::Color32 = egui::Color32::from_rgb(0, 212, 170);
const CHORD_BLUE: egui::Color32 = egui::Color32::from_rgb(120, 120, 240);
const ORANGE: egui::Color32 = egui::Color32::from_rgb(251, 146, 60);
const RED: egui::Color32 = egui::Color32::from_rgb(248, 72, 94);
const NEON_GREEN: egui::Color32 = egui::Color32::from_rgb(52, 211, 153);
const GOLD: egui::Color32 = egui::Color32::from_rgb(250, 204, 21);

const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(130, 130, 150);
const TEXT_MID: egui::Color32 = egui::Color32::from_rgb(170, 170, 186);
const TEXT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(235, 235, 245);

const WHITE_KEY_CLR: egui::Color32 = egui::Color32::from_rgb(195, 195, 205);
const BLACK_KEY_CLR: egui::Color32 = egui::Color32::from_rgb(36, 36, 44);

const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

// ---------------------------------------------------------------------------
// App struct
// ---------------------------------------------------------------------------

pub struct VoicianApp {
    pub gui_state: GuiState,
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

    fn push_params(&self) {
        if let Ok(mut guard) = self.params_handle.try_lock() {
            *guard = self.local_params.clone();
        }
    }
}

// ---------------------------------------------------------------------------
// eframe::App — main layout
// ---------------------------------------------------------------------------

impl eframe::App for VoicianApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gui_state.update_from_engine();
        apply_theme(ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // ── Top bar ──────────────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(egui::Margin::symmetric(12, 6)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("VOICIAN").color(TRIG_TEAL).size(18.0).strong());
                    ui.label(egui::RichText::new("v1.0").color(TEXT_DIM).size(9.0));

                    ui.add_space(12.0);

                    let src = self.gui_state.current.pitch_source;
                    let src_color = match src {
                        PitchSource::Crepe => PITCH_PINK,
                        PitchSource::Yin => TRIG_TEAL,
                        PitchSource::None => TEXT_DIM,
                    };
                    draw_pill(ui, src.label(), src_color);
                    ui.add_space(6.0);

                    if self.gui_state.midi_connected {
                        draw_pill(ui, "MIDI", NEON_GREEN);
                    } else {
                        draw_pill(ui, "NO MIDI", RED);
                    }

                    if !self.gui_state.current.detected_key.is_empty() {
                        ui.add_space(6.0);
                        draw_pill(ui, &format!("Key: {}", self.gui_state.current.detected_key), GOLD);
                    }

                    ui.add_space(4.0);
                    let midi_active = Instant::now() < self.gui_state.midi_flash_until;
                    let dot_c = if midi_active { NEON_GREEN } else { egui::Color32::from_rgb(60, 60, 72) };
                    let (r, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 4.0, dot_c);

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{} Hz", self.gui_state.sample_rate)).color(TEXT_DIM).size(9.0));

                        let strudel_lbl = if self.gui_state.strudel_open { "\u{266B} Strudel" } else { "\u{266B}" };
                        if ui.add(egui::Button::new(egui::RichText::new(strudel_lbl).size(11.0).color(TEXT_MID)).fill(egui::Color32::TRANSPARENT)).clicked() {
                            self.gui_state.strudel_open = true;
                            crate::strudel::open_browser();
                        }
                        let log_lbl = if self.gui_state.show_midi_log { "Log \u{25BE}" } else { "Log" };
                        if ui.add(egui::Button::new(egui::RichText::new(log_lbl).size(11.0).color(TEXT_MID)).fill(egui::Color32::TRANSPARENT)).clicked() {
                            self.gui_state.show_midi_log = !self.gui_state.show_midi_log;
                        }
                        let gear_c = if self.gui_state.show_settings { TRIG_TEAL } else { TEXT_MID };
                        if ui.add(egui::Button::new(egui::RichText::new("\u{2699}").size(14.0).color(gear_c)).fill(egui::Color32::TRANSPARENT)).clicked() {
                            self.gui_state.show_settings = !self.gui_state.show_settings;
                        }
                    });
                });
            });

        // ── Tab bar ──────────────────────────────────────────────────────
        egui::TopBottomPanel::top("tab_bar")
            .frame(egui::Frame::new().fill(TAB_BAR_BG).inner_margin(egui::Margin { left: 16, right: 16, top: 6, bottom: 0 }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for tab in GuiTab::ALL {
                        let active = self.gui_state.active_tab == *tab;
                        let label = egui::RichText::new(tab.label())
                            .size(if active { 15.0 } else { 13.0 })
                            .color(if active { TEXT_BRIGHT } else { TEXT_DIM })
                            .strong();
                        let btn = egui::Button::new(label)
                            .fill(egui::Color32::TRANSPARENT)
                            .stroke(egui::Stroke::NONE);
                        let resp = ui.add(btn);
                        if active {
                            let r = resp.rect;
                            ui.painter().line_segment(
                                [egui::pos2(r.min.x, r.max.y + 2.0), egui::pos2(r.max.x, r.max.y + 2.0)],
                                egui::Stroke::new(2.5, TRIG_TEAL),
                            );
                        }
                        if resp.clicked() { self.gui_state.active_tab = *tab; }
                        ui.add_space(10.0);
                    }
                });
                ui.add_space(3.0);
                let rect = ui.available_rect_before_wrap();
                ui.painter().line_segment(
                    [egui::pos2(rect.min.x, rect.min.y), egui::pos2(rect.max.x, rect.min.y)],
                    egui::Stroke::new(1.0, CARD_BORDER),
                );
            });

        // ── Bottom: MIDI log ─────────────────────────────────────────────
        if self.gui_state.show_midi_log {
            egui::TopBottomPanel::bottom("midi_log")
                .resizable(true).default_height(80.0).max_height(180.0)
                .frame(egui::Frame::new().fill(SIDEBAR_BG).inner_margin(6))
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("MIDI Log").color(TEXT_DIM).size(10.0));
                    egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                        for entry in self.gui_state.midi_log.iter() {
                            let t = entry.timestamp.elapsed();
                            ui.label(egui::RichText::new(format!("[{:.1}s] {}", t.as_secs_f32(), entry.message))
                                .color(TEXT_DIM).size(10.0).family(egui::FontFamily::Monospace));
                        }
                    });
                });
        }

        // ── Bottom: Piano keyboard ──────────────────────────────────────
        egui::TopBottomPanel::bottom("piano_kb")
            .frame(egui::Frame::new().fill(BG_DARK).inner_margin(egui::Margin { left: 8, right: 8, top: 4, bottom: 4 }))
            .show(ctx, |ui| {
                let active_midi = if self.gui_state.current.note_active {
                    self.gui_state.current.midi_note
                } else {
                    None
                };
                draw_piano_keyboard(ui, active_midi, &self.gui_state.current.chord_notes);
            });

        // ── Bottom: Controls bar (PB, Key, Chord) ───────────────────────
        {
            let mut changed = false;
            egui::TopBottomPanel::bottom("controls_bar")
                .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(egui::Margin::symmetric(12, 5)))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        // PB section
                        let pb_on = self.local_params.pitch_bend_mode != PitchBendMode::Off;
                        let pb_dot = if pb_on { PITCH_PINK } else { TEXT_DIM };
                        let (dr, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dr.center(), 4.0, pb_dot);
                        ui.label(egui::RichText::new("PB").color(TEXT_MID).size(11.0));
                        ui.label(egui::RichText::new("\u{2014}").color(TEXT_DIM).size(11.0));
                        let mut pbr = self.local_params.pitch_bend_range;
                        if ui.add(egui::DragValue::new(&mut pbr).range(0.5..=48.0).speed(0.5).max_decimals(0).suffix(" st")).changed() {
                            self.local_params.pitch_bend_range = pbr;
                            changed = true;
                        }

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Key section
                        ui.label(egui::RichText::new("Key").color(TEXT_MID).size(11.0));
                        egui::ComboBox::from_id_salt("bottom_root")
                            .selected_text(self.local_params.root_note.label())
                            .width(40.0)
                            .show_ui(ui, |ui| {
                                for root in RootNote::ALL {
                                    if ui.selectable_value(&mut self.local_params.root_note, *root, root.label()).changed() { changed = true; }
                                }
                            });
                        egui::ComboBox::from_id_salt("bottom_scale")
                            .selected_text(self.local_params.scale_type.label())
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                for scale in ScaleType::ALL {
                                    if ui.selectable_value(&mut self.local_params.scale_type, *scale, scale.label()).changed() { changed = true; }
                                }
                            });

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Chord section
                        let chord_dot = if self.local_params.chord_enabled { CHORD_BLUE } else { TEXT_DIM };
                        let (dr2, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dr2.center(), 4.0, chord_dot);
                        ui.label(egui::RichText::new("Chord").color(TEXT_MID).size(11.0));
                        egui::ComboBox::from_id_salt("bottom_chord")
                            .selected_text(self.local_params.chord_type.label())
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                for ct in ChordType::ALL {
                                    if ui.selectable_value(&mut self.local_params.chord_type, *ct, ct.label()).changed() { changed = true; }
                                }
                            });

                        ui.add_space(16.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // MIDI ch
                        ui.label(egui::RichText::new("Ch").color(TEXT_DIM).size(10.0));
                        let mut ch = self.local_params.midi_channel as i32 + 1;
                        if ui.add(egui::DragValue::new(&mut ch).range(1..=16).speed(0.1)).changed() {
                            self.local_params.midi_channel = (ch - 1).clamp(0, 15) as u8;
                            changed = true;
                        }
                    });
                });
            if changed { self.push_params(); }
        }

        // ── Side panel: advanced settings ────────────────────────────────
        if self.gui_state.show_settings {
            egui::SidePanel::left("settings_panel")
                .default_width(200.0).min_width(170.0).max_width(280.0).resizable(true)
                .frame(egui::Frame::new().fill(SIDEBAR_BG).inner_margin(8))
                .show(ctx, |ui| { self.draw_advanced_settings(ui); });
        }

        // ── Central panel: tab content ───────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG_DARK).inner_margin(10))
            .show(ctx, |ui| {
                match self.gui_state.active_tab {
                    GuiTab::Play => self.draw_play_tab(ui),
                    GuiTab::Key => self.draw_key_tab(ui),
                    GuiTab::Chords => self.draw_chords_tab(ui),
                    GuiTab::Assign => self.draw_assign_tab(ui),
                    GuiTab::Monitor => self.draw_monitor_tab(ui),
                }
            });
    }
}

// ===========================================================================
// Play Tab — combined Triggers + Pitch Wheel + Meters
// ===========================================================================

impl VoicianApp {
    fn draw_play_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let snap = &self.gui_state.current;
        let avail = ui.available_size();

        // Three-column layout: Triggers | Pitch Wheel | Meters
        let trigger_w = (avail.x * 0.26).max(170.0).min(250.0);
        let meter_w = (avail.x * 0.24).max(160.0).min(240.0);

        ui.horizontal(|ui| {
            // ── LEFT: Triggers ───────────────────────────────────────
            ui.vertical(|ui| {
                ui.set_width(trigger_w);
                ui.horizontal(|ui| {
                    let (dr, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dr.center(), 4.0, TRIG_TEAL);
                    ui.label(egui::RichText::new("Triggers").color(TRIG_TEAL).size(13.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.checkbox(&mut self.local_params.triggers_enabled,
                            egui::RichText::new("").size(1.0)).changed() { changed = true; }
                    });
                });

                // Sensitivity slider.
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Sensitivity").color(TEXT_DIM).size(10.0));
                    ui.label(egui::RichText::new("\u{2014}").color(TEXT_DIM).size(10.0));
                    let slider = egui::Slider::new(&mut self.local_params.trigger_onset_threshold, 0.01..=0.3)
                        .show_value(false);
                    if ui.add(slider).changed() { changed = true; }
                    let (dr2, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dr2.center(), 4.0, TRIG_TEAL);
                });

                ui.add_space(8.0);

                // 2×3 grid of square trigger pads.
                let pad_names = ["Bass Drum 1", "Acoustic Snare", "Closed Hi Hat", "Hand Clap", "Ride Cymbal 1", "High Tom"];
                let pad_notes: [u8; 6] = [36, 38, 42, 39, 51, 47];
                let now = Instant::now();
                let pad_w = (trigger_w - 12.0) / 2.0;
                let pad_h = 72.0;

                for row in 0..3 {
                    ui.horizontal(|ui| {
                        for col in 0..2 {
                            let idx = row * 2 + col;
                            // Only 4 active trigger slots; slots 4-5 are visual placeholders.
                            let is_hit = if idx < 4 {
                                now < self.gui_state.trigger_flash_until[idx]
                            } else {
                                false
                            };

                            let fill = if is_hit { TRIG_TEAL.gamma_multiply(0.35) } else { PAD_BG };
                            let border_c = if is_hit { TRIG_TEAL } else { CARD_BORDER };
                            let border_w = if is_hit { 2.0 } else { 1.0 };
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(pad_w - 4.0, pad_h), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 8.0, fill);
                            ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(border_w, border_c), egui::epaint::StrokeKind::Outside);

                            // Pad note number in dim.
                            ui.painter().text(
                                egui::pos2(rect.min.x + 8.0, rect.min.y + 10.0),
                                egui::Align2::LEFT_TOP,
                                format!("{}", pad_notes[idx]),
                                egui::FontId::proportional(9.0),
                                TEXT_DIM,
                            );

                            // Pad label in teal.
                            ui.painter().text(
                                egui::pos2(rect.min.x + 8.0, rect.max.y - 10.0),
                                egui::Align2::LEFT_BOTTOM,
                                pad_names[idx],
                                egui::FontId::proportional(10.0),
                                if is_hit { TEXT_BRIGHT } else { TRIG_TEAL },
                            );
                        }
                    });
                    ui.add_space(3.0);
                }

                // "+" placeholder
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("+").color(CHORD_BLUE).size(18.0));
                });
            });

            ui.add_space(8.0);

            // ── CENTER: Pitch Wheel ──────────────────────────────────
            ui.vertical(|ui| {
                let center_w = ui.available_width() - meter_w - 16.0;
                ui.set_width(center_w.max(200.0));

                // Pitch header and sliders.
                ui.horizontal(|ui| {
                    let (dr, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().circle_filled(dr.center(), 4.0, PITCH_PINK);
                    ui.label(egui::RichText::new("Pitch").color(PITCH_PINK).size(13.0).strong());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Stickiness (maps to stability tolerance).
                        let (dr2, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dr2.center(), 4.0, TRIG_TEAL);
                        let stk = egui::Slider::new(&mut self.local_params.stability_tolerance, 0.05..=1.0)
                            .show_value(false);
                        if ui.add(stk).changed() { changed = true; }
                        ui.label(egui::RichText::new("Stickiness").color(TEXT_DIM).size(10.0));

                        ui.add_space(12.0);

                        // Input Level (maps to silence threshold, inverted for UX).
                        let (dr3, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dr3.center(), 4.0, PITCH_PINK);
                        let il = egui::Slider::new(&mut self.local_params.silence_threshold, 0.001..=0.1)
                            .show_value(false);
                        if ui.add(il).changed() { changed = true; }
                        ui.label(egui::RichText::new("Input Level").color(TEXT_DIM).size(10.0));
                    });
                });

                ui.add_space(4.0);

                // Chromatic pitch wheel.
                let wheel_size = (ui.available_width().min(ui.available_height() - 20.0)).min(340.0).max(180.0);
                let note_class: Option<usize> = if snap.note_active {
                    snap.midi_note.map(|n| (n % 12) as usize)
                } else {
                    None
                };
                draw_pitch_wheel(ui, wheel_size, note_class, &snap.note_name, snap.confidence);

                // Quantized + chord display.
                ui.vertical_centered(|ui| {
                    if self.local_params.scale_lock_enabled && !snap.quantized_note_name.is_empty() {
                        ui.label(egui::RichText::new(format!("\u{2192} {}", snap.quantized_note_name))
                            .color(GOLD).size(14.0).strong());
                    }
                    if !snap.chord_notes.is_empty() {
                        let s: Vec<String> = snap.chord_notes.iter().map(|n| note_name_util(*n)).collect();
                        ui.label(egui::RichText::new(s.join("  \u{2022}  ")).color(PITCH_PINK).size(11.0));
                    }
                });
            });

            ui.add_space(8.0);

            // ── RIGHT: Ring meters ───────────────────────────────────
            ui.vertical(|ui| {
                ui.set_width(meter_w);

                let ring_r = (meter_w * 0.22).min(44.0).max(28.0);
                let col_w = meter_w / 2.0;

                let rms_norm = (snap.rms / 0.4).clamp(0.0, 1.0);
                let vel_norm = snap.velocity as f32 / 127.0;
                let conf_norm = snap.confidence.clamp(0.0, 1.0);
                let bend_norm = pitch_bend_norm(snap.pitch_bend);

                // Row 1: VOL, VEL
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let pad = (col_w - ring_r * 2.0 - 2.0).max(0.0) / 2.0;
                    ui.add_space(pad);
                    draw_ring_meter(ui, ring_r, rms_norm, "VOL", TRIG_TEAL);
                    ui.add_space(pad.max(4.0));
                    draw_ring_meter(ui, ring_r, vel_norm, "VEL", ORANGE);
                });

                ui.add_space(12.0);

                // Row 2: CONF, BEND
                ui.horizontal(|ui| {
                    let pad = (col_w - ring_r * 2.0 - 2.0).max(0.0) / 2.0;
                    ui.add_space(pad);
                    draw_ring_meter(ui, ring_r, conf_norm, "CONF", PITCH_PINK);
                    ui.add_space(pad.max(4.0));
                    draw_ring_meter(ui, ring_r, bend_norm, "BEND", CHORD_BLUE);
                });
            });
        });

        if !self.gui_state.midi_connected {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("\u{26A0}  No MIDI port — install loopMIDI and restart")
                    .color(ORANGE).size(11.0));
            });
        }

        if changed { self.push_params(); }
    }
}

// ===========================================================================
// Key Tab — Scale lock, pitch mode, pitch bend
// ===========================================================================

impl VoicianApp {
    fn draw_key_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        ui.columns(2, |cols| {
            // Left column: Scale Lock
            card_frame(&mut cols[0], |ui| {
                section_label(ui, "SCALE LOCK", GOLD);
                if ui.checkbox(&mut self.local_params.scale_lock_enabled,
                    egui::RichText::new("Enable Scale Lock").color(TEXT_BRIGHT).size(12.0)).changed() { changed = true; }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Root").color(TEXT_DIM).size(11.0));
                    egui::ComboBox::from_id_salt("key_root")
                        .selected_text(self.local_params.root_note.label()).width(50.0)
                        .show_ui(ui, |ui| {
                            for root in RootNote::ALL {
                                if ui.selectable_value(&mut self.local_params.root_note, *root, root.label()).changed() { changed = true; }
                            }
                        });
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Scale").color(TEXT_DIM).size(11.0));
                    egui::ComboBox::from_id_salt("key_scale")
                        .selected_text(self.local_params.scale_type.label()).width(110.0)
                        .show_ui(ui, |ui| {
                            for scale in ScaleType::ALL {
                                if ui.selectable_value(&mut self.local_params.scale_type, *scale, scale.label()).changed() { changed = true; }
                            }
                        });
                });
                ui.add_space(4.0);
                if ui.checkbox(&mut self.local_params.auto_key_detect,
                    egui::RichText::new("Auto-Detect Key").color(TEXT_MID).size(11.0)).changed() { changed = true; }
                if !self.gui_state.current.detected_key.is_empty() {
                    ui.label(egui::RichText::new(format!("Detected: {}", self.gui_state.current.detected_key)).color(GOLD).size(11.0));
                }
            });

            cols[0].add_space(10.0);

            // Pitch Mode
            card_frame(&mut cols[0], |ui| {
                section_label(ui, "PITCH MODE", TRIG_TEAL);
                for mode in &[PitchMode::Hybrid, PitchMode::Crepe, PitchMode::Yin] {
                    let sel = self.local_params.pitch_mode == *mode;
                    let c = if sel { TRIG_TEAL } else { TEXT_DIM };
                    if ui.selectable_label(sel, egui::RichText::new(mode.label()).size(12.0).color(c)).clicked() {
                        self.local_params.pitch_mode = *mode; changed = true;
                    }
                }
            });

            // Right column: Pitch Bend
            card_frame(&mut cols[1], |ui| {
                section_label(ui, "PITCH BEND", PITCH_PINK);
                for mode in PitchBendMode::ALL {
                    let sel = self.local_params.pitch_bend_mode == *mode;
                    let c = if sel { PITCH_PINK } else { TEXT_DIM };
                    if ui.selectable_label(sel, egui::RichText::new(mode.label()).size(12.0).color(c)).clicked() {
                        self.local_params.pitch_bend_mode = *mode; changed = true;
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Range (semitones)").color(TEXT_DIM).size(11.0));
                });
                let sl = egui::Slider::new(&mut self.local_params.pitch_bend_range, 0.5..=24.0)
                    .step_by(0.5).max_decimals(1);
                if ui.add(sl).changed() { changed = true; }
            });

            cols[1].add_space(10.0);

            // Freq Range
            card_frame(&mut cols[1], |ui| {
                section_label(ui, "FREQUENCY RANGE", RED);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Min: {:.0} Hz", self.local_params.min_freq_hz)).color(TEXT_DIM).size(11.0));
                });
                if ui.add(egui::Slider::new(&mut self.local_params.min_freq_hz, 30.0..=500.0).step_by(10.0).suffix(" Hz")).changed() { changed = true; }
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("Max: {:.0} Hz", self.local_params.max_freq_hz)).color(TEXT_DIM).size(11.0));
                });
                if ui.add(egui::Slider::new(&mut self.local_params.max_freq_hz, 200.0..=2000.0).step_by(10.0).suffix(" Hz")).changed() { changed = true; }
            });
        });

        if changed { self.push_params(); }
    }
}

// ===========================================================================
// Chords Tab
// ===========================================================================

impl VoicianApp {
    fn draw_chords_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        card_frame(ui, |ui| {
            section_label(ui, "CHORD GENERATION", CHORD_BLUE);
            if ui.checkbox(&mut self.local_params.chord_enabled,
                egui::RichText::new("Enable Chords").color(TEXT_BRIGHT).size(12.0)).changed() { changed = true; }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Type").color(TEXT_DIM).size(11.0));
                egui::ComboBox::from_id_salt("chords_type")
                    .selected_text(self.local_params.chord_type.label()).width(110.0)
                    .show_ui(ui, |ui| {
                        for ct in ChordType::ALL {
                            if ui.selectable_value(&mut self.local_params.chord_type, *ct, ct.label()).changed() { changed = true; }
                        }
                    });
                ui.add_space(16.0);
                ui.label(egui::RichText::new("Voicing").color(TEXT_DIM).size(11.0));
                egui::ComboBox::from_id_salt("chords_voicing")
                    .selected_text(format!("{:?}", self.local_params.chord_voicing)).width(120.0)
                    .show_ui(ui, |ui| {
                        use crate::chords::Voicing;
                        for v in &[Voicing::RootPosition, Voicing::FirstInversion, Voicing::SecondInversion, Voicing::Spread] {
                            if ui.selectable_value(&mut self.local_params.chord_voicing, *v, format!("{:?}", v)).changed() { changed = true; }
                        }
                    });
            });
        });

        ui.add_space(12.0);

        // Live chord display.
        if !self.gui_state.current.chord_notes.is_empty() {
            card_frame(ui, |ui| {
                section_label(ui, "ACTIVE CHORD", PITCH_PINK);
                let notes: Vec<String> = self.gui_state.current.chord_notes.iter().map(|n| note_name_util(*n)).collect();
                ui.label(egui::RichText::new(notes.join("  \u{2022}  ")).color(CHORD_BLUE).size(16.0).strong());
            });
        }

        if changed { self.push_params(); }
    }
}

// ===========================================================================
// Assign Tab — CC Mapping
// ===========================================================================

impl VoicianApp {
    fn draw_assign_tab(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        card_frame(ui, |ui| {
            ui.horizontal(|ui| {
                section_label(ui, "CC MAPPING", ORANGE);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.checkbox(&mut self.local_params.cc_mapping_enabled,
                        egui::RichText::new("Enable").color(TEXT_BRIGHT).size(11.0)).changed() { changed = true; }
                });
            });
            ui.add_space(6.0);

            let snap_cc = self.gui_state.current.cc_values;

            egui::Grid::new("assign_cc_grid").num_columns(4).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label(egui::RichText::new("Slot").color(TEXT_DIM).size(9.0));
                ui.label(egui::RichText::new("Source").color(TEXT_DIM).size(9.0));
                ui.label(egui::RichText::new("CC#").color(TEXT_DIM).size(9.0));
                ui.label(egui::RichText::new("Value").color(TEXT_DIM).size(9.0));
                ui.end_row();

                for i in 0..NUM_CC_SLOTS {
                    ui.label(egui::RichText::new(format!("{}", i + 1)).color(TRIG_TEAL).size(12.0).strong());

                    egui::ComboBox::from_id_salt(format!("assign_src_{}", i))
                        .selected_text(self.local_params.cc_sources[i].label()).width(90.0)
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
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER,
                        format!("{}", val), egui::FontId::proportional(9.0), TEXT_BRIGHT);

                    ui.end_row();
                }
            });
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
                draw_graph(ui, &self.gui_state.rms_history, 0.0, 0.5, TRIG_TEAL, graph_h - 30.0);
            });
            cols[0].add_space(6.0);
            card_frame(&mut cols[0], |ui| {
                ui.label(egui::RichText::new("PITCH (HZ)").color(TEXT_DIM).size(9.0));
                draw_graph(ui, &self.gui_state.pitch_history, 0.0, 800.0, NEON_GREEN, graph_h - 30.0);
            });
            card_frame(&mut cols[1], |ui| {
                ui.label(egui::RichText::new("CONFIDENCE").color(TEXT_DIM).size(9.0));
                draw_graph(ui, &self.gui_state.confidence_history, 0.0, 1.0, PITCH_PINK, graph_h - 30.0);
            });
            cols[1].add_space(6.0);
            card_frame(&mut cols[1], |ui| {
                ui.label(egui::RichText::new("CENTROID (HZ)").color(TEXT_DIM).size(9.0));
                draw_graph(ui, &self.gui_state.centroid_history, 0.0, 4000.0, CHORD_BLUE, graph_h - 30.0);
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

            section_label(ui, "SMOOTHING", PITCH_PINK);
            changed |= labeled_slider(ui, "Pitch", &mut self.local_params.pitch_smoothing, 0.0..=0.95);
            changed |= labeled_slider(ui, "Amplitude", &mut self.local_params.amplitude_smoothing, 0.0..=0.95);
            changed |= labeled_slider(ui, "Centroid", &mut self.local_params.centroid_smoothing, 0.0..=0.95);

            ui.add_space(8.0); ui.separator(); ui.add_space(4.0);

            section_label(ui, "TRIGGERS", TRIG_TEAL);
            {
                let mut tch = self.local_params.trigger_channel as f32 + 1.0;
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Drum Ch").color(TEXT_DIM).size(11.0));
                    if ui.add(egui::Slider::new(&mut tch, 1.0..=16.0).step_by(1.0).max_decimals(0)).changed() {
                        self.local_params.trigger_channel = (tch - 1.0).round() as u8;
                        changed = true;
                    }
                });
            }

            ui.add_space(8.0); ui.separator(); ui.add_space(4.0);

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

fn card_frame(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD_BG)
        .stroke(egui::Stroke::new(1.0, CARD_BORDER))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| { add_contents(ui); });
}

fn draw_pill(ui: &mut egui::Ui, label: &str, color: egui::Color32) {
    let galley = ui.painter().layout_no_wrap(label.to_string(), egui::FontId::proportional(9.0), color);
    let text_size = galley.size();
    let pad = egui::vec2(8.0, 3.0);
    let desired = text_size + pad * 2.0;
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    ui.painter().rect_stroke(rect, rect.height() / 2.0, egui::Stroke::new(1.0, color), egui::epaint::StrokeKind::Outside);
    ui.painter().galley(egui::pos2(rect.min.x + pad.x, rect.min.y + pad.y), galley, color);
}

// ── Chromatic pitch wheel ────────────────────────────────────────────────────

fn draw_pitch_wheel(
    ui: &mut egui::Ui,
    size: f32,
    note_class: Option<usize>,
    note_name: &str,
    confidence: f32,
) {
    let outer_r = size * 0.44;
    let inner_r = size * 0.28;
    let label_r = size * 0.50;
    let segment_angle = TAU / 12.0;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let center = rect.center();

    // Draw 12 chromatic segments as wedges from center, then overlay inner circle.
    for i in 0..12_usize {
        // Center each segment so the note sits at the midpoint.
        let start = i as f32 * segment_angle - FRAC_PI_2 - segment_angle / 2.0;
        let gap = 0.025_f32;
        let a0 = start + gap;
        let a1 = start + segment_angle - gap;

        let is_active = note_class == Some(i);

        // Active segment extends outward proportionally to confidence.
        let seg_outer = if is_active {
            outer_r + confidence * 18.0
        } else {
            outer_r
        };

        // Build wedge polygon: center → outer arc.
        let arc_steps = 10;
        let mut pts = vec![center];
        for j in 0..=arc_steps {
            let a = a0 + (a1 - a0) * j as f32 / arc_steps as f32;
            pts.push(egui::pos2(center.x + seg_outer * a.cos(), center.y + seg_outer * a.sin()));
        }

        let fill = if is_active { PITCH_PINK } else { SEGMENT_BG };
        let border = SEGMENT_BORDER;
        ui.painter().add(egui::Shape::convex_polygon(pts, fill, egui::Stroke::new(1.0, border)));
    }

    // Inner circle overlay to create the donut.
    ui.painter().circle_filled(center, inner_r + 3.0, BG_DARK);
    ui.painter().circle_filled(center, inner_r, INNER_CIRCLE);
    ui.painter().circle_stroke(center, inner_r, egui::Stroke::new(1.5, SEGMENT_BORDER));

    // Decorative mid-ring.
    let mid_r = (outer_r + inner_r) / 2.0;
    ui.painter().circle_stroke(center, mid_r, egui::Stroke::new(0.5, SEGMENT_BORDER.gamma_multiply(0.5)));

    // Note name in center.
    let text_color = if note_class.is_some() { TEXT_BRIGHT } else { TEXT_DIM };
    let center_font = (size * 0.10).max(22.0).min(36.0);
    ui.painter().text(
        center, egui::Align2::CENTER_CENTER,
        note_name, egui::FontId::proportional(center_font), text_color,
    );

    // Note labels around the outside.
    let label_font_big = (size * 0.045).max(12.0).min(16.0);
    let label_font_sm  = (size * 0.032).max(9.0).min(12.0);
    for i in 0..12_usize {
        let angle = i as f32 * segment_angle - FRAC_PI_2;
        let x = center.x + label_r * angle.cos();
        let y = center.y + label_r * angle.sin();
        let is_natural = matches!(i, 0 | 2 | 4 | 5 | 7 | 9 | 11);
        let font_size = if is_natural { label_font_big } else { label_font_sm };
        let color = if Some(i) == note_class {
            PITCH_PINK
        } else if is_natural {
            TEXT_BRIGHT
        } else {
            TEXT_DIM
        };
        ui.painter().text(
            egui::pos2(x, y), egui::Align2::CENTER_CENTER,
            NOTE_NAMES[i], egui::FontId::proportional(font_size), color,
        );
    }
}

// ── Ring meter (circular gauge like Dubler 2 vowel knobs) ────────────────────

fn draw_ring_meter(
    ui: &mut egui::Ui,
    radius: f32,
    value_norm: f32,
    label: &str,
    color: egui::Color32,
) {
    let size = radius * 2.0 + 4.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size + 20.0), egui::Sense::hover());
    let center = egui::pos2(rect.center().x, rect.min.y + radius + 2.0);

    // Dark circle background with subtle border.
    ui.painter().circle_filled(center, radius, INNER_CIRCLE);
    ui.painter().circle_stroke(center, radius, egui::Stroke::new(1.0, CARD_BORDER));

    // Track arc (270°, from 7:30 to 4:30).
    let start_angle = FRAC_PI_2 + PI / 4.0; // 135° in screen coords.
    let total_sweep = 1.5 * PI;              // 270°.
    let arc_r = radius * 0.78;
    let arc_w = (radius * 0.14).max(3.5).min(6.0);

    draw_arc(ui, center, arc_r, start_angle, total_sweep, arc_w, TRACK_BG);

    // Filled arc.
    let filled_sweep = total_sweep * value_norm.clamp(0.0, 1.0);
    if filled_sweep > 0.01 {
        draw_arc(ui, center, arc_r, start_angle, filled_sweep, arc_w + 0.5, color);
    }

    // Label in center of ring.
    let label_size = (radius * 0.36).max(10.0).min(14.0);
    ui.painter().text(
        center, egui::Align2::CENTER_CENTER,
        label, egui::FontId::proportional(label_size), color,
    );

    // Value percentage below ring.
    let pct = format!("{}%", (value_norm * 100.0).round() as u32);
    ui.painter().text(
        egui::pos2(center.x, rect.min.y + radius * 2.0 + 8.0),
        egui::Align2::CENTER_TOP,
        pct, egui::FontId::proportional(9.0), TEXT_DIM,
    );
}

fn draw_arc(
    ui: &mut egui::Ui,
    center: egui::Pos2,
    radius: f32,
    start: f32,
    sweep: f32,
    width: f32,
    color: egui::Color32,
) {
    let steps = ((sweep / 0.06).ceil() as usize).max(3);
    for i in 0..steps {
        let a1 = start + sweep * i as f32 / steps as f32;
        let a2 = start + sweep * (i + 1) as f32 / steps as f32;
        let p1 = egui::pos2(center.x + radius * a1.cos(), center.y + radius * a1.sin());
        let p2 = egui::pos2(center.x + radius * a2.cos(), center.y + radius * a2.sin());
        ui.painter().line_segment([p1, p2], egui::Stroke::new(width, color));
    }
}

// ── Piano keyboard ───────────────────────────────────────────────────────────

fn draw_piano_keyboard(ui: &mut egui::Ui, active_note: Option<u8>, chord_notes: &[u8]) {
    let width = ui.available_width();
    let height = 44.0;
    let start_midi: u8 = 36; // C2
    let end_midi: u8 = 72;   // B4

    let num_white = (start_midi..end_midi).filter(|n| !is_black_key(n % 12)).count();
    let white_w = width / num_white as f32;
    let black_w = white_w * 0.58;
    let black_h = height * 0.60;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

    // White keys.
    let mut x = rect.min.x;
    for midi in start_midi..end_midi {
        if is_black_key(midi % 12) { continue; }
        let active = is_note_active(midi, active_note, chord_notes);
        let fill = if active { PITCH_PINK } else { WHITE_KEY_CLR };
        let kr = egui::Rect::from_min_size(egui::pos2(x, rect.min.y), egui::vec2(white_w - 1.0, height));
        ui.painter().rect_filled(kr, 2.0, fill);
        ui.painter().rect_stroke(kr, 2.0, egui::Stroke::new(0.5, SEGMENT_BORDER), egui::epaint::StrokeKind::Outside);
        x += white_w;
    }

    // Black keys.
    x = rect.min.x;
    for midi in start_midi..end_midi {
        if is_black_key(midi % 12) { continue; }
        let next = midi + 1;
        if next < end_midi && is_black_key(next % 12) {
            let bx = x + white_w - black_w / 2.0;
            let active = is_note_active(next, active_note, chord_notes);
            let fill = if active { PITCH_PINK } else { BLACK_KEY_CLR };
            let kr = egui::Rect::from_min_size(egui::pos2(bx, rect.min.y), egui::vec2(black_w, black_h));
            ui.painter().rect_filled(kr, 2.0, fill);
        }
        x += white_w;
    }
}

fn is_black_key(note_class: u8) -> bool {
    matches!(note_class, 1 | 3 | 6 | 8 | 10)
}

fn is_note_active(midi: u8, active_note: Option<u8>, chord_notes: &[u8]) -> bool {
    active_note == Some(midi) || chord_notes.contains(&midi)
}

// ── Graph ────────────────────────────────────────────────────────────────────

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
        let norm = ((val - min_val) / range).clamp(0.0, 1.0);
        let y = rect.max.y - norm * rect.height();
        egui::pos2(x, y)
    }).collect();

    for pair in points.windows(2) {
        ui.painter().line_segment([pair[0], pair[1]], egui::Stroke::new(1.5, color));
    }
}

// ── Utility helpers ──────────────────────────────────────────────────────────

fn pitch_bend_norm(value: u16) -> f32 {
    ((value as f32 - 8192.0) / 8192.0).abs()
}

fn note_name_util(midi: u8) -> String {
    let name = NOTE_NAMES[(midi % 12) as usize];
    let oct = (midi as i32 / 12) - 1;
    format!("{}{}", name, oct)
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
    ui.add(egui::Slider::new(value, range).max_decimals(3).step_by(0.005)).changed()
}

// ---------------------------------------------------------------------------
// Theme
// ---------------------------------------------------------------------------

fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.dark_mode = true;
    v.panel_fill = BG_DARK;
    v.window_fill = PANEL_BG;
    v.faint_bg_color = PANEL_BG;
    v.extreme_bg_color = egui::Color32::from_rgb(32, 32, 40);
    v.override_text_color = Some(TEXT_BRIGHT);

    v.widgets.noninteractive.bg_fill = PANEL_BG;
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, CARD_BORDER);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(56, 56, 68);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, CARD_BORDER);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(66, 66, 80);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, TRIG_TEAL);
    v.widgets.active.bg_fill = egui::Color32::from_rgb(72, 72, 88);

    v.selection.bg_fill = egui::Color32::from_rgb(60, 60, 100);
    v.selection.stroke = egui::Stroke::new(1.0, TRIG_TEAL);

    let cr = egui::CornerRadius::same(6);
    v.window_corner_radius = egui::CornerRadius::same(10);
    v.menu_corner_radius = cr;
    v.widgets.noninteractive.corner_radius = cr;
    v.widgets.inactive.corner_radius = cr;
    v.widgets.hovered.corner_radius = cr;
    v.widgets.active.corner_radius = cr;

    ctx.set_style(style);
}

// ---------------------------------------------------------------------------
// eframe launcher
// ---------------------------------------------------------------------------

pub fn run_gui(gui_state: GuiState) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1020.0, 680.0])
            .with_min_inner_size([760.0, 520.0])
            .with_title("Voician — Voice to MIDI"),
        ..Default::default()
    };

    eframe::run_native(
        "Voician",
        options,
        Box::new(|_cc| Ok(Box::new(VoicianApp::new(gui_state)))),
    )
}
