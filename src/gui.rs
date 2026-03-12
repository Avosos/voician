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

// ---------------------------------------------------------------------------
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
        });

        ui.add_space(8.0);

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
                &self.gui_state.rms_history,
                0.0,
                0.5,
                ACCENT_BLUE,
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
        );
    });
}

fn draw_info_box(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).color(TEXT_DIM).size(9.0));
        ui.label(
            egui::RichText::new(value)
                .color(TEXT_BRIGHT)
                .size(12.0)
                .strong(),
        );
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

    let points: Vec<egui::Pos2> = data
        .iter()
        .enumerate()
        .map(|(i, &val)| {
            let x = rect.min.x + i as f32 * step;
            let normalized = ((val - min_val) / range).clamp(0.0, 1.0);
            let y = rect.max.y - normalized * rect.height();
            egui::pos2(x, y)
        })
        .collect();

    let stroke = egui::Stroke::new(1.5, color);
    for pair in points.windows(2) {
        ui.painter().line_segment([pair[0], pair[1]], stroke);
    }
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

    ctx.set_style(style);
}

// ---------------------------------------------------------------------------
// eframe launcher
// ---------------------------------------------------------------------------

pub fn run_gui(gui_state: GuiState) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 620.0])
            .with_min_inner_size([650.0, 450.0])
            .with_title("Voician — Voice to MIDI (Phase 5)"),
        ..Default::default()
    };

    eframe::run_native(
        "Voician",
        options,
        Box::new(|_cc| Ok(Box::new(VoicianApp::new(gui_state)))),
    )
}
