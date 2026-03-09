// ============================================================================
<<<<<<< HEAD
// gui.rs — Voician GUI v2 — Premium dark-themed interface
// ============================================================================
//
// A complete redesign of the Voician GUI with:
//
//   • Polished header bar with brand identity, connection status, and controls
//   • Minimal mode — Hero note display with radial glow, meters with gradients,
//     MIDI data tiles, connection status
//   • Advanced mode — Compact note display, 4 real-time graphs (RMS, Pitch,
//     Velocity, Centroid), formatted MIDI event log with colored badges
//   • Consistent spacing, rounded panels, and a cohesive blue/teal color scheme
//
// The GUI runs at 60 FPS via eframe's repaint scheduling.
// Engine state is received via crossbeam channels (lock-free).
// ============================================================================

use crate::state::GuiState;
=======
// gui.rs — Phase 5: Pro GUI with settings sidebar and real-time controls
// ============================================================================
//
// Three-panel layout:
//
//   ┌─────────────┬───────────────────────────────────────────────┐
//   │  SETTINGS   │    ┌──────────────────────────────────────┐   │
//   │  (sidebar)  │    │  BIG NOTE  +  meters  +  status      │   │
//   │             │    ├──────────────────────────────────────┤   │
//   │  Pitch mode │    │  Graphs: Volume, Pitch, Confidence   │   │
//   │  Thresholds │    ├──────────────────────────────────────┤   │
//   │  Smoothing  │    │  MIDI log (collapsible)               │   │
//   │  MIDI opts  │    └──────────────────────────────────────┘   │
//   └─────────────┴───────────────────────────────────────────────┘
//
// Settings are live-adjustable via Arc<Mutex<EngineParams>>.
// ============================================================================

use crate::state::{EngineParams, GuiState, PitchMode, PitchSource, SharedParams};
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
use eframe::egui;
use std::time::Instant;

// ---------------------------------------------------------------------------
<<<<<<< HEAD
// Color palette — Premium dark theme with blue/teal accents
// ---------------------------------------------------------------------------

const BG_BASE: egui::Color32 = egui::Color32::from_rgb(10, 10, 16);
const BG_PANEL: egui::Color32 = egui::Color32::from_rgb(18, 18, 28);
const BG_CARD: egui::Color32 = egui::Color32::from_rgb(24, 24, 36);
const BG_ELEVATED: egui::Color32 = egui::Color32::from_rgb(32, 32, 46);
const BORDER_SUBTLE: egui::Color32 = egui::Color32::from_rgb(42, 42, 58);

const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(80, 140, 255);
const ACCENT_TEAL: egui::Color32 = egui::Color32::from_rgb(60, 216, 156);
const ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 170, 50);
const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(255, 80, 80);
const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(160, 110, 255);
const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(60, 210, 120);

const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(230, 230, 242);
const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(155, 155, 175);
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(100, 100, 120);
=======
// Color palette (dark theme)
// ---------------------------------------------------------------------------

const BG_DARK: egui::Color32 = egui::Color32::from_rgb(18, 18, 24);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(28, 28, 38);
const SIDEBAR_BG: egui::Color32 = egui::Color32::from_rgb(22, 22, 32);
const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(80, 140, 255);
const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(60, 210, 120);
const ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 160, 50);
const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(255, 70, 70);
const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(170, 100, 255);
const ACCENT_CYAN: egui::Color32 = egui::Color32::from_rgb(80, 210, 230);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(120, 120, 140);
const TEXT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(220, 220, 235);
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83

// ---------------------------------------------------------------------------
// App struct
// ---------------------------------------------------------------------------

pub struct VoicianApp {
    pub gui_state: GuiState,
<<<<<<< HEAD
=======
    /// Local copy of params that we edit via sliders, then push to engine.
    local_params: EngineParams,
    params_handle: SharedParams,
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
}

impl VoicianApp {
    pub fn new(gui_state: GuiState) -> Self {
<<<<<<< HEAD
        VoicianApp { gui_state }
=======
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
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    }
}

// ---------------------------------------------------------------------------
<<<<<<< HEAD
// eframe::App implementation
=======
// eframe::App
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
// ---------------------------------------------------------------------------

impl eframe::App for VoicianApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gui_state.update_from_engine();
<<<<<<< HEAD
        apply_theme(ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // Header bar
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::new()
                .fill(BG_PANEL)
                .inner_margin(egui::Margin::symmetric(16, 10))
                .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("VOICIAN")
                            .color(ACCENT_BLUE)
                            .size(17.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new("v0.3")
                            .color(TEXT_MUTED)
                            .size(10.0),
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    let (badge_text, badge_color, badge_bg) = if self.gui_state.midi_connected {
                        (
                            format!("\u{25cf} MIDI: {}", self.gui_state.midi_port_name),
                            ACCENT_GREEN,
                            egui::Color32::from_rgba_premultiplied(60, 210, 120, 20),
                        )
                    } else {
                        (
                            "\u{25cb} MIDI: Disconnected".to_string(),
                            ACCENT_RED,
                            egui::Color32::from_rgba_premultiplied(255, 80, 80, 20),
                        )
                    };
                    let badge_resp = ui.allocate_ui(egui::vec2(200.0, 20.0), |ui| {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 20.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 10.0, badge_bg);
                        ui.painter().rect_stroke(
                            rect,
                            10.0,
                            egui::Stroke::new(0.5, badge_color.linear_multiply(0.3)),
                            egui::StrokeKind::Outside,
                        );
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &badge_text,
                            egui::FontId::proportional(10.5),
                            badge_color,
                        );
                    });
                    let _ = badge_resp;

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mode_label = if self.gui_state.advanced_mode {
                            "\u{25c6} Advanced"
                        } else {
                            "\u{25c7} Minimal"
                        };
                        let btn = ui.add(
                            egui::Button::new(
                                egui::RichText::new(mode_label)
                                    .size(11.0)
                                    .color(TEXT_SECONDARY),
                            )
                            .fill(BG_ELEVATED)
                            .stroke(egui::Stroke::new(0.5, BORDER_SUBTLE))
                            .corner_radius(8.0),
                        );
                        if btn.clicked() {
                            self.gui_state.advanced_mode = !self.gui_state.advanced_mode;
                        }

                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(format!("{} Hz", self.gui_state.sample_rate))
                                .color(TEXT_MUTED)
                                .size(10.0),
=======
        apply_dark_theme(ctx);
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // -- Top bar --
        egui::TopBottomPanel::top("top_bar")
            .frame(egui::Frame::new().fill(PANEL_BG).inner_margin(6.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("🎤 VOICIAN")
                            .color(ACCENT_BLUE)
                            .size(18.0),
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
                        egui::RichText::new(src.label())
                            .color(src_color)
                            .size(12.0)
                            .strong(),
                    );

                    ui.separator();

                    // MIDI status.
                    let (status_text, status_color) = if self.gui_state.midi_connected {
                        (
                            format!("MIDI: {}", self.gui_state.midi_port_name),
                            ACCENT_GREEN,
                        )
                    } else {
                        ("MIDI: Disconnected".to_string(), ACCENT_RED)
                    };
                    ui.label(
                        egui::RichText::new(status_text)
                            .color(status_color)
                            .size(12.0),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Toggle buttons.
                        let settings_label = if self.gui_state.show_settings {
                            "⚙ Settings ▾"
                        } else {
                            "⚙ Settings ▸"
                        };
                        if ui
                            .button(egui::RichText::new(settings_label).size(12.0))
                            .clicked()
                        {
                            self.gui_state.show_settings = !self.gui_state.show_settings;
                        }

                        let log_label = if self.gui_state.show_midi_log {
                            "📋 Log ▾"
                        } else {
                            "📋 Log ▸"
                        };
                        if ui
                            .button(egui::RichText::new(log_label).size(12.0))
                            .clicked()
                        {
                            self.gui_state.show_midi_log = !self.gui_state.show_midi_log;
                        }

                        ui.label(
                            egui::RichText::new(format!("{} Hz", self.gui_state.sample_rate))
                                .color(TEXT_DIM)
                                .size(11.0),
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
                        );
                    });
                });
            });

<<<<<<< HEAD
        // Main area
        egui::CentralPanel::default()
            .frame(egui::Frame::new()
                .fill(BG_BASE)
                .inner_margin(egui::Margin::same(16)))
            .show(ctx, |ui| {
                if self.gui_state.advanced_mode {
                    self.draw_advanced(ui);
                } else {
                    self.draw_minimal(ui);
                }
=======
        // -- Bottom panel: collapsible MIDI log --
        if self.gui_state.show_midi_log {
            egui::TopBottomPanel::bottom("midi_log")
                .resizable(true)
                .default_height(100.0)
                .max_height(200.0)
                .frame(egui::Frame::new().fill(SIDEBAR_BG).inner_margin(6.0))
                .show(ctx, |ui| {
                    ui.label(
                        egui::RichText::new("MIDI Log")
                            .color(TEXT_DIM)
                            .size(11.0),
                    );
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for entry in self.gui_state.midi_log.iter() {
                                let elapsed = entry.timestamp.elapsed();
                                let time_str = format!("{:.1}s", elapsed.as_secs_f32());
                                ui.label(
                                    egui::RichText::new(format!(
                                        "[{}] {}",
                                        time_str, entry.message
                                    ))
                                    .color(TEXT_DIM)
                                    .size(10.0)
                                    .family(egui::FontFamily::Monospace),
                                );
                            }
                        });
                });
        }

        // -- Left sidebar: settings --
        if self.gui_state.show_settings {
            egui::SidePanel::left("settings_panel")
                .default_width(220.0)
                .min_width(180.0)
                .max_width(300.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(SIDEBAR_BG).inner_margin(10.0))
                .show(ctx, |ui| {
                    self.draw_settings(ui);
                });
        }

        // -- Central panel: note display + graphs --
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG_DARK).inner_margin(12.0))
            .show(ctx, |ui| {
                self.draw_main_panel(ui);
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
            });
    }
}

// ---------------------------------------------------------------------------
<<<<<<< HEAD
// Minimal mode — Hero layout
// ---------------------------------------------------------------------------

impl VoicianApp {
    fn draw_minimal(&self, ui: &mut egui::Ui) {
        let snap = &self.gui_state.current;
        let available = ui.available_size();

        ui.vertical_centered(|ui| {
            let vert_padding = ((available.y - 320.0) / 3.0).max(8.0);
            ui.add_space(vert_padding);

            // Hero note display with glow
            let note_active = snap.note_active;
            let note_color = if note_active { ACCENT_TEAL } else { TEXT_MUTED };
            let glow_alpha = if note_active { 0.12 } else { 0.03 };

            let circle_size = 160.0;
            let (circle_rect, _) = ui.allocate_exact_size(
                egui::vec2(circle_size, circle_size),
                egui::Sense::hover(),
            );
            let center = circle_rect.center();

            // Outer glow ring
            ui.painter().circle_filled(
                center,
                circle_size * 0.5,
                egui::Color32::from_rgba_premultiplied(
                    note_color.r(),
                    note_color.g(),
                    note_color.b(),
                    (glow_alpha * 255.0) as u8,
                ),
            );
            // Inner filled circle
            ui.painter().circle_filled(center, 62.0, BG_CARD);
            ui.painter().circle_stroke(
                center,
                62.0,
                egui::Stroke::new(
                    if note_active { 2.5 } else { 1.0 },
                    note_color.linear_multiply(if note_active { 0.8 } else { 0.2 }),
                ),
            );

            // Note name text
            ui.painter().text(
                center + egui::vec2(0.0, -4.0),
                egui::Align2::CENTER_CENTER,
                &snap.note_name,
                egui::FontId::proportional(48.0),
                note_color,
            );

            // Frequency underneath
            if snap.frequency > 0.0 {
                ui.painter().text(
                    center + egui::vec2(0.0, 30.0),
                    egui::Align2::CENTER_CENTER,
                    &format!("{:.1} Hz", snap.frequency),
                    egui::FontId::proportional(12.0),
                    TEXT_MUTED,
                );
            }

            ui.add_space(24.0);

            // Meters row
            let meter_width = (available.x - 100.0).min(560.0) / 3.0;
            ui.horizontal(|ui| {
                let h_pad = ((available.x - meter_width * 3.0 - 40.0) / 2.0).max(0.0);
                ui.add_space(h_pad);
                draw_premium_meter(ui, "VOLUME", snap.rms, 0.5, ACCENT_BLUE, meter_width);
                ui.add_space(20.0);
                draw_premium_meter(
                    ui,
                    "VELOCITY",
                    snap.velocity as f32 / 127.0,
                    1.0,
                    ACCENT_ORANGE,
                    meter_width,
                );
                ui.add_space(20.0);
                draw_premium_meter(
                    ui,
                    "CONFIDENCE",
                    snap.confidence,
                    1.0,
                    ACCENT_PURPLE,
                    meter_width,
                );
            });

            ui.add_space(20.0);

            // MIDI data tiles
            let tile_width = (available.x - 100.0).min(560.0) / 4.0;
            ui.horizontal(|ui| {
                let h_pad = ((available.x - tile_width * 4.0 - 48.0) / 2.0).max(0.0);
                ui.add_space(h_pad);
                draw_data_tile(ui, "PITCH BEND", &format_pitch_bend(snap.pitch_bend), ACCENT_BLUE, tile_width);
                ui.add_space(16.0);
                draw_data_tile(ui, "CC 74", &format!("{}", snap.cc_brightness), ACCENT_TEAL, tile_width);
                ui.add_space(16.0);
                draw_data_tile(ui, "CENTROID", &format!("{:.0} Hz", snap.centroid_hz), ACCENT_PURPLE, tile_width);
                ui.add_space(16.0);

                let midi_active = Instant::now() < self.gui_state.midi_flash_until;
                let indicator_label = if midi_active { "\u{25cf} ACTIVE" } else { "\u{25cb} IDLE" };
                let indicator_color = if midi_active { ACCENT_GREEN } else { TEXT_MUTED };
                draw_data_tile(ui, "MIDI", indicator_label, indicator_color, tile_width);
            });

            // No MIDI warning
            if !self.gui_state.midi_connected {
                ui.add_space(20.0);
                draw_warning_banner(
                    ui,
                    "No MIDI port detected — install loopMIDI and restart",
                    "https://www.tobias-erichsen.de/software/loopmidi.html",
                );
            }
        });
    }

    // -----------------------------------------------------------------------
    // Advanced mode — Compact display + graphs + log
    // -----------------------------------------------------------------------

    fn draw_advanced(&self, ui: &mut egui::Ui) {
        let snap = &self.gui_state.current;

        // Top row: compact note + stats
        ui.horizontal(|ui| {
            let note_color = if snap.note_active { ACCENT_TEAL } else { TEXT_MUTED };

            // Mini note circle
            let (circle_rect, _) = ui.allocate_exact_size(
                egui::vec2(52.0, 52.0),
                egui::Sense::hover(),
            );
            let center = circle_rect.center();
            ui.painter().circle_filled(center, 26.0, BG_CARD);
            ui.painter().circle_stroke(
                center,
                26.0,
                egui::Stroke::new(
                    if snap.note_active { 2.0 } else { 0.5 },
                    note_color.linear_multiply(if snap.note_active { 0.7 } else { 0.15 }),
                ),
            );
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                &snap.note_name,
                egui::FontId::proportional(20.0),
                note_color,
            );

            ui.add_space(12.0);

            // Stats columns
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    stat_label(ui, "Freq", &format!("{:.1} Hz", snap.frequency));
                    ui.add_space(16.0);
                    stat_label(ui, "Vel", &format!("{}", snap.velocity));
                    ui.add_space(16.0);
                    stat_label(ui, "PB", &format_pitch_bend(snap.pitch_bend));
                    ui.add_space(16.0);
                    stat_label(ui, "CC74", &format!("{}", snap.cc_brightness));
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    stat_label(
                        ui,
                        "Conf",
                        &format!("{:.0}%", snap.confidence * 100.0),
                    );
                    ui.add_space(16.0);
                    stat_label(
                        ui,
                        "Centroid",
                        &format!("{:.0} Hz", snap.centroid_hz),
                    );
                });
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let midi_active = Instant::now() < self.gui_state.midi_flash_until;
                let dot_color = if midi_active {
                    ACCENT_GREEN
                } else {
                    egui::Color32::from_rgb(45, 45, 60)
                };
                let (dot_rect, _) = ui.allocate_exact_size(
                    egui::vec2(14.0, 14.0),
                    egui::Sense::hover(),
                );
                ui.painter().circle_filled(dot_rect.center(), 7.0, dot_color);
                if midi_active {
                    ui.painter().circle_stroke(
                        dot_rect.center(),
                        10.0,
                        egui::Stroke::new(1.0, ACCENT_GREEN.linear_multiply(0.3)),
                    );
                }
            });
=======
// Settings sidebar
// ---------------------------------------------------------------------------

impl VoicianApp {
    fn draw_settings(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        egui::ScrollArea::vertical().show(ui, |ui| {
            // ---- Pitch Mode ----
            ui.label(
                egui::RichText::new("PITCH MODE")
                    .color(ACCENT_BLUE)
                    .size(11.0)
                    .strong(),
            );
            ui.add_space(2.0);

            let modes = [PitchMode::Hybrid, PitchMode::Crepe, PitchMode::Yin];
            for mode in &modes {
                let selected = self.local_params.pitch_mode == *mode;
                let label = egui::RichText::new(mode.label())
                    .size(12.0)
                    .color(if selected { ACCENT_BLUE } else { TEXT_BRIGHT });
                if ui.selectable_label(selected, label).clicked() {
                    self.local_params.pitch_mode = *mode;
                    changed = true;
                }
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            // ---- Detection Thresholds ----
            ui.label(
                egui::RichText::new("DETECTION")
                    .color(ACCENT_GREEN)
                    .size(11.0)
                    .strong(),
            );
            ui.add_space(2.0);

            changed |= labeled_slider(
                ui,
                "CREPE Confidence",
                &mut self.local_params.confidence_threshold,
                0.1..=0.95,
            );
            changed |= labeled_slider(
                ui,
                "YIN Threshold",
                &mut self.local_params.yin_threshold,
                0.01..=0.5,
            );
            changed |= labeled_slider(
                ui,
                "Silence Gate",
                &mut self.local_params.silence_threshold,
                0.001..=0.1,
            );

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            // ---- Note Stability ----
            ui.label(
                egui::RichText::new("NOTE STABILITY")
                    .color(ACCENT_ORANGE)
                    .size(11.0)
                    .strong(),
            );
            ui.add_space(2.0);

            {
                let mut frames = self.local_params.stability_frames as f32;
                if labeled_slider(ui, "Stability Frames", &mut frames, 1.0..=8.0) {
                    self.local_params.stability_frames = frames.round() as usize;
                    changed = true;
                }
            }
            changed |= labeled_slider(
                ui,
                "Stability Tolerance",
                &mut self.local_params.stability_tolerance,
                0.05..=1.0,
            );
            changed |= labeled_slider(
                ui,
                "Note Change Thresh",
                &mut self.local_params.note_change_threshold,
                0.2..=1.5,
            );

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            // ---- Smoothing ----
            ui.label(
                egui::RichText::new("SMOOTHING")
                    .color(ACCENT_PURPLE)
                    .size(11.0)
                    .strong(),
            );
            ui.add_space(2.0);

            changed |= labeled_slider(
                ui,
                "Pitch",
                &mut self.local_params.pitch_smoothing,
                0.0..=0.95,
            );
            changed |= labeled_slider(
                ui,
                "Amplitude",
                &mut self.local_params.amplitude_smoothing,
                0.0..=0.95,
            );
            changed |= labeled_slider(
                ui,
                "Centroid",
                &mut self.local_params.centroid_smoothing,
                0.0..=0.95,
            );

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            // ---- MIDI Output ----
            ui.label(
                egui::RichText::new("MIDI OUTPUT")
                    .color(ACCENT_CYAN)
                    .size(11.0)
                    .strong(),
            );
            ui.add_space(2.0);

            // MIDI channel (display 1-16, store 0-15).
            {
                let mut ch_display = self.local_params.midi_channel as f32 + 1.0;
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Channel")
                            .color(TEXT_DIM)
                            .size(11.0),
                    );
                    ui.add_space(4.0);
                    let slider = egui::Slider::new(&mut ch_display, 1.0..=16.0)
                        .step_by(1.0)
                        .max_decimals(0);
                    if ui.add(slider).changed() {
                        self.local_params.midi_channel = (ch_display - 1.0).round() as u8;
                        changed = true;
                    }
                });
            }

            changed |= labeled_slider(
                ui,
                "Pitch Bend Range",
                &mut self.local_params.pitch_bend_range,
                0.5..=12.0,
            );

            ui.add_space(4.0);
            if ui
                .checkbox(
                    &mut self.local_params.pitch_bend_enabled,
                    egui::RichText::new("Pitch Bend").color(TEXT_BRIGHT).size(12.0),
                )
                .changed()
            {
                changed = true;
            }
            if ui
                .checkbox(
                    &mut self.local_params.cc_brightness_enabled,
                    egui::RichText::new("CC 74 Brightness").color(TEXT_BRIGHT).size(12.0),
                )
                .changed()
            {
                changed = true;
            }

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            // ---- Frequency Range ----
            ui.label(
                egui::RichText::new("FREQ RANGE")
                    .color(ACCENT_RED)
                    .size(11.0)
                    .strong(),
            );
            ui.add_space(2.0);

            changed |= labeled_slider_hz(
                ui,
                "Min Freq",
                &mut self.local_params.min_freq_hz,
                30.0..=500.0,
            );
            changed |= labeled_slider_hz(
                ui,
                "Max Freq",
                &mut self.local_params.max_freq_hz,
                200.0..=2000.0,
            );

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            // ---- Reset button ----
            if ui
                .button(
                    egui::RichText::new("↺ Reset to Defaults")
                        .color(TEXT_BRIGHT)
                        .size(12.0),
                )
                .clicked()
            {
                self.local_params = EngineParams::default();
                changed = true;
            }
        });

        if changed {
            self.push_params();
        }
    }
}

// ---------------------------------------------------------------------------
// Main display panel
// ---------------------------------------------------------------------------

impl VoicianApp {
    fn draw_main_panel(&self, ui: &mut egui::Ui) {
        let snap = &self.gui_state.current;

        // ---- Big note display ----
        ui.vertical_centered(|ui| {
            let note_color = if snap.note_active {
                ACCENT_GREEN
            } else {
                TEXT_DIM
            };
            ui.label(
                egui::RichText::new(&snap.note_name)
                    .color(note_color)
                    .size(64.0)
                    .strong(),
            );

            // Frequency + source.
            if snap.frequency > 0.0 {
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1} Hz  [{}]",
                        snap.frequency,
                        snap.pitch_source.label(),
                    ))
                    .color(TEXT_DIM)
                    .size(14.0),
                );
            }
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
        });

        ui.add_space(8.0);

<<<<<<< HEAD
        // Graphs — 2x2 grid
        let graph_height = ((ui.available_height() - 120.0) / 2.0).max(60.0);

        ui.columns(2, |cols| {
            draw_graph_panel(
                &mut cols[0],
                "VOLUME (RMS)",
=======
        // ---- Meters row ----
        ui.horizontal(|ui| {
            let meter_w = ((ui.available_width() - 40.0) / 4.0).max(80.0);
            draw_meter(ui, "Volume", snap.rms, 0.5, ACCENT_BLUE, meter_w);
            ui.add_space(8.0);
            draw_meter(
                ui,
                "Velocity",
                snap.velocity as f32 / 127.0,
                1.0,
                ACCENT_ORANGE,
                meter_w,
            );
            ui.add_space(8.0);
            draw_meter(ui, "Confidence", snap.confidence, 1.0, ACCENT_PURPLE, meter_w);
            ui.add_space(8.0);

            // MIDI info column.
            ui.vertical(|ui| {
                draw_info_box(ui, "PB", &format_pitch_bend(snap.pitch_bend));
                draw_info_box(ui, "CC74", &format!("{}", snap.cc_brightness));

                // MIDI activity dot.
                let midi_active = Instant::now() < self.gui_state.midi_flash_until;
                let dot_color = if midi_active {
                    ACCENT_GREEN
                } else {
                    egui::Color32::from_rgb(40, 40, 50)
                };
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 6.0, dot_color);
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // ---- Graphs (2 columns × 2 rows) ----
        let graph_h = ((ui.available_height() - 20.0) / 2.0).max(50.0);

        ui.columns(2, |cols| {
            cols[0].label(egui::RichText::new("Volume (RMS)").color(TEXT_DIM).size(10.0));
            draw_graph(
                &mut cols[0],
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
                &self.gui_state.rms_history,
                0.0,
                0.5,
                ACCENT_BLUE,
<<<<<<< HEAD
                graph_height,
            );
            draw_graph_panel(
                &mut cols[1],
                "PITCH (Hz)",
                &self.gui_state.pitch_history,
                0.0,
                800.0,
                ACCENT_TEAL,
                graph_height,
            );
        });

        ui.add_space(6.0);

        ui.columns(2, |cols| {
            draw_graph_panel(
                &mut cols[0],
                "VELOCITY",
                &self.gui_state.velocity_history,
                0.0,
                127.0,
                ACCENT_ORANGE,
                graph_height,
            );
            draw_graph_panel(
                &mut cols[1],
                "CENTROID (Hz)",
                &self.gui_state.centroid_history,
                0.0,
                4000.0,
                ACCENT_PURPLE,
                graph_height,
            );
        });

        ui.add_space(6.0);

        // MIDI log
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("MIDI LOG")
                    .color(TEXT_MUTED)
                    .size(10.0)
                    .strong(),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(format!("{} events", self.gui_state.midi_log.len()))
                    .color(TEXT_MUTED)
                    .size(9.0),
            );
        });
        ui.add_space(4.0);

        let log_frame = egui::Frame::new()
            .fill(BG_CARD)
            .corner_radius(8.0)
            .inner_margin(egui::Margin::same(8))
            .stroke(egui::Stroke::new(0.5, BORDER_SUBTLE));

        log_frame.show(ui, |ui| {
            let log_height = ui.available_height().max(50.0);
            egui::ScrollArea::vertical()
                .max_height(log_height)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if self.gui_state.midi_log.is_empty() {
                        ui.label(
                            egui::RichText::new("No MIDI events yet — sing or hum into your mic")
                                .color(TEXT_MUTED)
                                .size(10.0)
                                .italics(),
                        );
                    } else {
                        for entry in self.gui_state.midi_log.iter() {
                            let elapsed = entry.timestamp.elapsed();
                            let time_str = format!("{:>5.1}s", elapsed.as_secs_f32());

                            let msg_color = if entry.message.contains("ON") {
                                ACCENT_TEAL
                            } else if entry.message.contains("OFF") {
                                TEXT_MUTED
                            } else if entry.message.contains("Bend") {
                                ACCENT_BLUE
                            } else if entry.message.contains("CC") {
                                ACCENT_PURPLE
                            } else {
                                TEXT_SECONDARY
                            };

                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&time_str)
                                        .color(TEXT_MUTED)
                                        .size(9.5)
                                        .family(egui::FontFamily::Monospace),
                                );
                                ui.label(
                                    egui::RichText::new(&entry.message)
                                        .color(msg_color)
                                        .size(9.5)
                                        .family(egui::FontFamily::Monospace),
                                );
                            });
                        }
                    }
                });
        });
    }
}

// ===========================================================================
// Drawing helpers
// ===========================================================================

/// Premium meter with rounded bar, label, and percentage text.
fn draw_premium_meter(
=======
                graph_h,
            );

            cols[0].add_space(4.0);
            cols[0].label(egui::RichText::new("Pitch (Hz)").color(TEXT_DIM).size(10.0));
            draw_graph(
                &mut cols[0],
                &self.gui_state.pitch_history,
                0.0,
                800.0,
                ACCENT_GREEN,
                graph_h,
            );

            cols[1].label(egui::RichText::new("Confidence").color(TEXT_DIM).size(10.0));
            draw_graph(
                &mut cols[1],
                &self.gui_state.confidence_history,
                0.0,
                1.0,
                ACCENT_PURPLE,
                graph_h,
            );

            cols[1].add_space(4.0);
            cols[1].label(egui::RichText::new("Centroid (Hz)").color(TEXT_DIM).size(10.0));
            draw_graph(
                &mut cols[1],
                &self.gui_state.centroid_history,
                0.0,
                4000.0,
                ACCENT_CYAN,
                graph_h,
            );
        });

        // ---- MIDI disconnected warning ----
        if !self.gui_state.midi_connected {
            ui.add_space(8.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(
                        "⚠ No MIDI port detected. Install loopMIDI and restart.",
                    )
                    .color(ACCENT_ORANGE)
                    .size(12.0),
                );
                ui.hyperlink_to(
                    "Download loopMIDI →",
                    "https://www.tobias-erichsen.de/software/loopmidi.html",
                );
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------

fn draw_meter(
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    ui: &mut egui::Ui,
    label: &str,
    value: f32,
    max_val: f32,
    color: egui::Color32,
    width: f32,
) {
    ui.vertical(|ui| {
<<<<<<< HEAD
        ui.label(
            egui::RichText::new(label)
                .color(TEXT_MUTED)
                .size(9.5)
                .strong(),
        );
        ui.add_space(4.0);

        let normalized = (value / max_val).clamp(0.0, 1.0);
        let bar_height = 10.0;
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, bar_height), egui::Sense::hover());

        // Track background
        ui.painter().rect_filled(rect, 5.0, BG_ELEVATED);

        // Filled bar
        if normalized > 0.01 {
            let fill_width = (rect.width() * normalized).max(4.0);
            let fill_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(fill_width, rect.height()),
            );
            ui.painter().rect_filled(fill_rect, 5.0, color);

            // Subtle highlight on top half for depth
            let highlight_rect = egui::Rect::from_min_size(
                fill_rect.min,
                egui::vec2(fill_rect.width(), fill_rect.height() * 0.45),
            );
            ui.painter().rect_filled(
                highlight_rect,
                egui::CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 },
                color.linear_multiply(1.2).gamma_multiply(0.3),
            );
        }

        // Percentage text
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new(format!("{:.0}%", normalized * 100.0))
                .color(TEXT_SECONDARY)
                .size(10.0),
=======
        ui.label(egui::RichText::new(label).color(TEXT_DIM).size(10.0));
        let normalized = (value / max_val).clamp(0.0, 1.0);
        let height = 12.0;
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

        ui.painter().rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgb(35, 35, 45),
        );

        let fill_rect = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() * normalized, rect.height()),
        );
        ui.painter().rect_filled(fill_rect, 4.0, color);

        let text = format!("{:.0}%", normalized * 100.0);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(9.0),
            TEXT_BRIGHT,
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
        );
    });
}

<<<<<<< HEAD
/// Data tile — small card with label and value.
fn draw_data_tile(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    color: egui::Color32,
    width: f32,
) {
    let tile_height = 48.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, tile_height), egui::Sense::hover());

    ui.painter().rect_filled(rect, 8.0, BG_CARD);
    ui.painter().rect_stroke(rect, 8.0, egui::Stroke::new(0.5, BORDER_SUBTLE), egui::StrokeKind::Outside);

    ui.painter().text(
        rect.min + egui::vec2(10.0, 10.0),
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(9.0),
        TEXT_MUTED,
    );

    ui.painter().text(
        rect.min + egui::vec2(10.0, 28.0),
        egui::Align2::LEFT_TOP,
        value,
        egui::FontId::proportional(14.0),
        color,
    );
}

/// Stat label for the advanced mode header.
fn stat_label(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(label)
                .color(TEXT_MUTED)
                .size(9.0),
        );
        ui.label(
            egui::RichText::new(value)
                .color(TEXT_PRIMARY)
=======
fn draw_info_box(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(TEXT_DIM).size(9.0));
        ui.label(
            egui::RichText::new(value)
                .color(TEXT_BRIGHT)
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
                .size(12.0)
                .strong(),
        );
    });
}

<<<<<<< HEAD
/// Graph panel with title, card background, and line graph.
fn draw_graph_panel(
    ui: &mut egui::Ui,
    title: &str,
=======
fn draw_graph(
    ui: &mut egui::Ui,
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    data: &std::collections::VecDeque<f32>,
    min_val: f32,
    max_val: f32,
    color: egui::Color32,
    height: f32,
) {
<<<<<<< HEAD
    ui.label(
        egui::RichText::new(title)
            .color(TEXT_MUTED)
            .size(9.5)
            .strong(),
    );
    ui.add_space(2.0);

    let panel_height = height - 18.0;
    let width = ui.available_width();
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, panel_height), egui::Sense::hover());

    ui.painter().rect_filled(rect, 6.0, BG_CARD);
    ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(0.5, BORDER_SUBTLE), egui::StrokeKind::Outside);

    if data.len() < 2 {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Waiting for data\u{2026}",
            egui::FontId::proportional(10.0),
            TEXT_MUTED,
        );
        return;
    }

    let inset = 6.0;
    let graph_rect = rect.shrink(inset);

    let range = (max_val - min_val).max(0.001);
    let n = data.len();
    let step = graph_rect.width() / (n - 1) as f32;
=======
    let width = ui.available_width();
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

    ui.painter().rect_filled(
        rect,
        4.0,
        egui::Color32::from_rgb(22, 22, 30),
    );

    if data.len() < 2 {
        return;
    }

    let range = (max_val - min_val).max(0.001);
    let n = data.len();
    let step = rect.width() / (n - 1) as f32;
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83

    let points: Vec<egui::Pos2> = data
        .iter()
        .enumerate()
        .map(|(i, &val)| {
<<<<<<< HEAD
            let x = graph_rect.min.x + i as f32 * step;
            let normalized = ((val - min_val) / range).clamp(0.0, 1.0);
            let y = graph_rect.max.y - normalized * graph_rect.height();
=======
            let x = rect.min.x + i as f32 * step;
            let normalized = ((val - min_val) / range).clamp(0.0, 1.0);
            let y = rect.max.y - normalized * rect.height();
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
            egui::pos2(x, y)
        })
        .collect();

<<<<<<< HEAD
    // Fill area under the curve
    if points.len() >= 2 {
        let fill_color = egui::Color32::from_rgba_premultiplied(
            color.r(),
            color.g(),
            color.b(),
            12,
        );
        for pair in points.windows(2) {
            let mut mesh = egui::Mesh::default();
            let idx_base = mesh.vertices.len() as u32;
            mesh.colored_vertex(pair[0], fill_color);
            mesh.colored_vertex(pair[1], fill_color);
            mesh.colored_vertex(egui::pos2(pair[1].x, graph_rect.max.y), fill_color);
            mesh.colored_vertex(egui::pos2(pair[0].x, graph_rect.max.y), fill_color);
            mesh.indices.extend_from_slice(&[
                idx_base, idx_base + 1, idx_base + 2,
                idx_base, idx_base + 2, idx_base + 3,
            ]);
            ui.painter().add(egui::Shape::mesh(mesh));
        }
    }

    // Line
=======
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
    let stroke = egui::Stroke::new(1.5, color);
    for pair in points.windows(2) {
        ui.painter().line_segment([pair[0], pair[1]], stroke);
    }
<<<<<<< HEAD

    // Current value label
    if let Some(&last_val) = data.back() {
        let val_text = if max_val > 200.0 {
            format!("{:.0}", last_val)
        } else if max_val > 1.5 {
            format!("{:.0}", last_val)
        } else {
            format!("{:.3}", last_val)
        };
        ui.painter().text(
            graph_rect.right_top() + egui::vec2(-4.0, 4.0),
            egui::Align2::RIGHT_TOP,
            val_text,
            egui::FontId::proportional(9.0),
            color.linear_multiply(0.8),
        );
    }
}

/// Warning banner for missing MIDI connection.
fn draw_warning_banner(ui: &mut egui::Ui, message: &str, link: &str) {
    let width = ui.available_width().min(480.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, 44.0), egui::Sense::hover());

    let banner_bg = egui::Color32::from_rgba_premultiplied(255, 170, 50, 12);
    ui.painter().rect_filled(rect, 8.0, banner_bg);
    ui.painter().rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(0.5, ACCENT_ORANGE.linear_multiply(0.2)),
        egui::StrokeKind::Outside,
    );

    ui.painter().text(
        rect.min + egui::vec2(14.0, 10.0),
        egui::Align2::LEFT_TOP,
        format!("\u{26a0} {}", message),
        egui::FontId::proportional(11.0),
        ACCENT_ORANGE,
    );

    ui.painter().text(
        rect.min + egui::vec2(28.0, 26.0),
        egui::Align2::LEFT_TOP,
        format!("\u{2192} {}", link),
        egui::FontId::proportional(9.5),
        ACCENT_BLUE,
    );

    let resp = ui.interact(rect, ui.id().with("warn_link"), egui::Sense::click());
    if resp.clicked() {
        ui.ctx().open_url(egui::OpenUrl::new_tab(link));
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
=======
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
}

fn format_pitch_bend(value: u16) -> String {
    let centered = value as i32 - 8192;
    if centered == 0 {
        "0".to_string()
    } else if centered > 0 {
        format!("+{}", centered)
    } else {
        format!("{}", centered)
    }
}

<<<<<<< HEAD
// ---------------------------------------------------------------------------
// Theme setup
// ---------------------------------------------------------------------------

fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;

    v.dark_mode = true;
    v.panel_fill = BG_BASE;
    v.window_fill = BG_PANEL;
    v.faint_bg_color = BG_PANEL;
    v.extreme_bg_color = BG_BASE;
    v.override_text_color = Some(TEXT_PRIMARY);

    v.widgets.noninteractive.bg_fill = BG_PANEL;
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, TEXT_SECONDARY);
    v.widgets.inactive.bg_fill = BG_ELEVATED;
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, TEXT_SECONDARY);
    v.widgets.hovered.bg_fill = egui::Color32::from_rgb(48, 48, 68);
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);
    v.widgets.active.bg_fill = ACCENT_BLUE;
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, TEXT_PRIMARY);

    v.selection.bg_fill = ACCENT_BLUE.linear_multiply(0.3);
    v.selection.stroke = egui::Stroke::new(1.0, ACCENT_BLUE);

    v.window_corner_radius = egui::CornerRadius::same(12);
    v.menu_corner_radius = egui::CornerRadius::same(8);

    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
=======
/// Labeled slider for floating-point parameter. Returns true if changed.
fn labeled_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(label)
                .color(TEXT_DIM)
                .size(11.0),
        );
    });
    let slider = egui::Slider::new(value, range)
        .max_decimals(3)
        .step_by(0.005);
    if ui.add(slider).changed() {
        changed = true;
    }
    changed
}

/// Labeled slider showing Hz values.
fn labeled_slider_hz(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("{} ({:.0} Hz)", label, *value))
                .color(TEXT_DIM)
                .size(11.0),
        );
    });
    let slider = egui::Slider::new(value, range)
        .max_decimals(0)
        .step_by(10.0)
        .suffix(" Hz");
    if ui.add(slider).changed() {
        changed = true;
    }
    changed
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
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83

    ctx.set_style(style);
}

// ---------------------------------------------------------------------------
// eframe launcher
// ---------------------------------------------------------------------------

<<<<<<< HEAD
/// Launch the Voician GUI window.
pub fn run_gui(gui_state: GuiState) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 560.0])
            .with_min_inner_size([560.0, 420.0])
            .with_title("Voician — Voice to MIDI"),
=======
pub fn run_gui(gui_state: GuiState) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 620.0])
            .with_min_inner_size([650.0, 450.0])
            .with_title("Voician — Voice to MIDI (Phase 5)"),
>>>>>>> f9bf6609f6fe09a87c01bf0a4c5a7ca8d06a1e83
        ..Default::default()
    };

    eframe::run_native(
        "Voician",
        options,
        Box::new(|_cc| Ok(Box::new(VoicianApp::new(gui_state)))),
    )
}
