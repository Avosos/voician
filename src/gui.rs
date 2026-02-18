// ============================================================================
// gui.rs — egui/eframe GUI for Voician (Phase 3)
// ============================================================================
//
// Provides a modern dark-themed GUI with two modes:
//
//   • Minimal mode  — Large note display, volume & velocity meters,
//                     MIDI activity indicator, connection status.
//   • Advanced mode — Adds waveform, pitch/velocity/centroid graphs,
//                     MIDI event log, detailed stats.
//
// The GUI runs at 60 FPS via eframe's repaint scheduling.
// Engine state is received via crossbeam channels (lock-free).
// ============================================================================

use crate::state::GuiState;
use eframe::egui;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Color palette (dark theme)
// ---------------------------------------------------------------------------

const BG_DARK: egui::Color32 = egui::Color32::from_rgb(18, 18, 24);
const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(28, 28, 38);
const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(80, 140, 255);
const ACCENT_GREEN: egui::Color32 = egui::Color32::from_rgb(60, 210, 120);
const ACCENT_ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 160, 50);
const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(255, 70, 70);
const ACCENT_PURPLE: egui::Color32 = egui::Color32::from_rgb(170, 100, 255);
const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(120, 120, 140);
const TEXT_BRIGHT: egui::Color32 = egui::Color32::from_rgb(220, 220, 235);

// ---------------------------------------------------------------------------
// App struct (holds GUI state)
// ---------------------------------------------------------------------------

pub struct VoicianApp {
    pub gui_state: GuiState,
}

impl VoicianApp {
    pub fn new(gui_state: GuiState) -> Self {
        VoicianApp { gui_state }
    }
}

// ---------------------------------------------------------------------------
// eframe::App implementation
// ---------------------------------------------------------------------------

impl eframe::App for VoicianApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain engine snapshots.
        self.gui_state.update_from_engine();

        // Apply dark theme.
        apply_dark_theme(ctx);

        // Request continuous repaint at ~60 FPS.
        ctx.request_repaint_after(std::time::Duration::from_millis(16));

        // Top bar.
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new("🎤 VOICIAN")
                        .color(ACCENT_BLUE)
                        .size(18.0),
                );
                ui.separator();

                // MIDI connection status.
                let (status_text, status_color) = if self.gui_state.midi_connected {
                    (
                        format!("MIDI: {}", self.gui_state.midi_port_name),
                        ACCENT_GREEN,
                    )
                } else {
                    ("MIDI: Disconnected".to_string(), ACCENT_RED)
                };
                ui.label(egui::RichText::new(status_text).color(status_color).size(12.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Mode toggle.
                    let mode_text = if self.gui_state.advanced_mode {
                        "▼ Advanced"
                    } else {
                        "▶ Minimal"
                    };
                    if ui
                        .button(egui::RichText::new(mode_text).size(12.0))
                        .clicked()
                    {
                        self.gui_state.advanced_mode = !self.gui_state.advanced_mode;
                    }

                    // Sample rate.
                    ui.label(
                        egui::RichText::new(format!("{} Hz", self.gui_state.sample_rate))
                            .color(TEXT_DIM)
                            .size(11.0),
                    );
                });
            });
        });

        // Main content.
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.gui_state.advanced_mode {
                self.draw_advanced(ui);
            } else {
                self.draw_minimal(ui);
            }
        });
    }
}

// ---------------------------------------------------------------------------
// Minimal mode
// ---------------------------------------------------------------------------

impl VoicianApp {
    fn draw_minimal(&self, ui: &mut egui::Ui) {
        let snap = &self.gui_state.current;

        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            // ---- Big note display ----
            let note_color = if snap.note_active {
                ACCENT_GREEN
            } else {
                TEXT_DIM
            };
            ui.label(
                egui::RichText::new(&snap.note_name)
                    .color(note_color)
                    .size(72.0)
                    .strong(),
            );

            // Frequency.
            if snap.frequency > 0.0 {
                ui.label(
                    egui::RichText::new(format!("{:.1} Hz", snap.frequency))
                        .color(TEXT_DIM)
                        .size(16.0),
                );
            }

            ui.add_space(16.0);

            // ---- Meters in a horizontal layout ----
            ui.horizontal(|ui| {
                ui.add_space(40.0);
                // Volume meter.
                draw_meter(ui, "Volume", snap.rms, 0.5, ACCENT_BLUE, 180.0);
                ui.add_space(20.0);
                // Velocity meter.
                draw_meter(
                    ui,
                    "Velocity",
                    snap.velocity as f32 / 127.0,
                    1.0,
                    ACCENT_ORANGE,
                    180.0,
                );
                ui.add_space(20.0);
                // Confidence meter.
                draw_meter(
                    ui,
                    "Confidence",
                    snap.confidence,
                    1.0,
                    ACCENT_PURPLE,
                    180.0,
                );
            });

            ui.add_space(16.0);

            // ---- MIDI data row ----
            ui.horizontal(|ui| {
                ui.add_space(40.0);
                draw_info_box(ui, "Pitch Bend", &format_pitch_bend(snap.pitch_bend));
                ui.add_space(10.0);
                draw_info_box(ui, "CC 74", &format!("{}", snap.cc_brightness));
                ui.add_space(10.0);
                draw_info_box(
                    ui,
                    "Centroid",
                    &format!("{:.0} Hz", snap.centroid_hz),
                );
                ui.add_space(10.0);

                // MIDI activity indicator.
                let midi_active = Instant::now() < self.gui_state.midi_flash_until;
                let indicator_color = if midi_active {
                    ACCENT_GREEN
                } else {
                    egui::Color32::from_rgb(40, 40, 50)
                };
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(16.0, 16.0),
                    egui::Sense::hover(),
                );
                ui.painter().circle_filled(rect.center(), 8.0, indicator_color);
                ui.label(egui::RichText::new("MIDI").color(TEXT_DIM).size(11.0));
            });

            if !self.gui_state.midi_connected {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(
                        "⚠ No MIDI port detected. Install loopMIDI and restart.",
                    )
                    .color(ACCENT_ORANGE)
                    .size(13.0),
                );
                if ui
                    .hyperlink_to(
                        "Download loopMIDI →",
                        "https://www.tobias-erichsen.de/software/loopmidi.html",
                    )
                    .clicked()
                {
                    // Link opens in browser automatically.
                }
            }
        });
    }

    // -----------------------------------------------------------------------
    // Advanced mode
    // -----------------------------------------------------------------------

    fn draw_advanced(&self, ui: &mut egui::Ui) {
        let snap = &self.gui_state.current;

        // Top row: note display + meters (compact).
        ui.horizontal(|ui| {
            // Note display.
            let note_color = if snap.note_active {
                ACCENT_GREEN
            } else {
                TEXT_DIM
            };
            ui.label(
                egui::RichText::new(&snap.note_name)
                    .color(note_color)
                    .size(40.0)
                    .strong(),
            );

            ui.separator();

            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(format!("Freq: {:.1} Hz", snap.frequency))
                        .color(TEXT_BRIGHT)
                        .size(12.0),
                );
                ui.label(
                    egui::RichText::new(format!("Vel: {}  PB: {}  CC74: {}",
                        snap.velocity,
                        format_pitch_bend(snap.pitch_bend),
                        snap.cc_brightness,
                    ))
                    .color(TEXT_DIM)
                    .size(11.0),
                );
                ui.label(
                    egui::RichText::new(format!(
                        "Conf: {:.0}%  Centroid: {:.0} Hz",
                        snap.confidence * 100.0,
                        snap.centroid_hz,
                    ))
                    .color(TEXT_DIM)
                    .size(11.0),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // MIDI activity indicator (larger in advanced mode).
                let midi_active = Instant::now() < self.gui_state.midi_flash_until;
                let indicator_color = if midi_active {
                    ACCENT_GREEN
                } else {
                    egui::Color32::from_rgb(40, 40, 50)
                };
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(20.0, 20.0),
                    egui::Sense::hover(),
                );
                ui.painter().circle_filled(rect.center(), 10.0, indicator_color);
            });
        });

        ui.separator();

        // Graphs section.
        ui.columns(2, |cols| {
            // Left column: waveform + pitch.
            cols[0].label(egui::RichText::new("Volume (RMS)").color(TEXT_DIM).size(11.0));
            draw_graph(
                &mut cols[0],
                &self.gui_state.rms_history,
                0.0,
                0.5,
                ACCENT_BLUE,
                80.0,
            );

            cols[0].add_space(4.0);
            cols[0].label(egui::RichText::new("Pitch (Hz)").color(TEXT_DIM).size(11.0));
            draw_graph(
                &mut cols[0],
                &self.gui_state.pitch_history,
                0.0,
                800.0,
                ACCENT_GREEN,
                80.0,
            );

            // Right column: velocity + centroid.
            cols[1].label(egui::RichText::new("Velocity").color(TEXT_DIM).size(11.0));
            draw_graph(
                &mut cols[1],
                &self.gui_state.velocity_history,
                0.0,
                127.0,
                ACCENT_ORANGE,
                80.0,
            );

            cols[1].add_space(4.0);
            cols[1].label(egui::RichText::new("Centroid (Hz)").color(TEXT_DIM).size(11.0));
            draw_graph(
                &mut cols[1],
                &self.gui_state.centroid_history,
                0.0,
                4000.0,
                ACCENT_PURPLE,
                80.0,
            );
        });

        ui.separator();

        // MIDI log.
        ui.label(egui::RichText::new("MIDI Log").color(TEXT_DIM).size(11.0));
        let log_height = ui.available_height().max(60.0);
        egui::ScrollArea::vertical()
            .max_height(log_height)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for entry in self.gui_state.midi_log.iter() {
                    let elapsed = entry.timestamp.elapsed();
                    let time_str = format!("{:.1}s", elapsed.as_secs_f32());
                    ui.label(
                        egui::RichText::new(format!("[{}] {}", time_str, entry.message))
                            .color(TEXT_DIM)
                            .size(10.0)
                            .family(egui::FontFamily::Monospace),
                    );
                }
            });
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
        ui.label(egui::RichText::new(label).color(TEXT_DIM).size(11.0));
        let normalized = (value / max_val).clamp(0.0, 1.0);
        let height = 12.0;
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

        // Background.
        ui.painter().rect_filled(
            rect,
            4.0,
            egui::Color32::from_rgb(35, 35, 45),
        );

        // Filled portion.
        let fill_rect = egui::Rect::from_min_size(
            rect.min,
            egui::vec2(rect.width() * normalized, rect.height()),
        );
        ui.painter().rect_filled(fill_rect, 4.0, color);

        // Value text.
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
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).color(TEXT_DIM).size(10.0));
        ui.label(
            egui::RichText::new(value)
                .color(TEXT_BRIGHT)
                .size(14.0)
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

    // Background.
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

    // Draw line.
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

// ---------------------------------------------------------------------------
// Dark theme setup
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

/// Launch the Voician GUI window.
pub fn run_gui(gui_state: GuiState) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 520.0])
            .with_min_inner_size([500.0, 400.0])
            .with_title("Voician — Voice to MIDI"),
        ..Default::default()
    };

    eframe::run_native(
        "Voician",
        options,
        Box::new(|_cc| Ok(Box::new(VoicianApp::new(gui_state)))),
    )
}
