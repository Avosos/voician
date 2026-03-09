// ============================================================================
// main.rs — Voician entry point (Phase 5: Hybrid engine + Pro GUI)
// ============================================================================
//
// Threading model:
//   • Audio thread  – managed by cpal/WASAPI (high-priority OS thread).
//                     Pushes mono f32 samples into a lock-free ring buffer.
//   • Engine thread – reads from ring buffer, runs hybrid YIN+CREPE pitch
//                     detection + MIDI output. Reads live params from GUI.
//   • Main thread   – runs the egui/eframe GUI, reads snapshots at ~60 FPS,
//                     writes tuning params to shared state.
//
// On window close, the engine thread and audio thread shut down gracefully.
// ============================================================================

mod analysis;
mod audio;
mod crepe;
mod engine;
mod gui;
mod midi;
mod pitch;
mod state;

use ringbuf::traits::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() {
    // --- Banner (console) ---------------------------------------------------
    println!();
    println!("╔═══════════════════════════════════════════════════╗");
    println!("║    VOICIAN — Voice to MIDI Engine  (Phase 5)       ║");
    println!("╠═══════════════════════════════════════════════════╣");
    println!("║  Hybrid YIN + CREPE neural pitch detection         ║");
    println!("║  Live-tunable parameters via GUI sidebar           ║");
    println!("║  Expressive MIDI: velocity, pitch bend, CC 74      ║");
    println!("╚═══════════════════════════════════════════════════╝");
    println!();

    // --- Shutdown flag -------------------------------------------------------
    let running = Arc::new(AtomicBool::new(true));
    {
        let r = running.clone();
        ctrlc::set_handler(move || {
            println!("\n[main] Ctrl+C received — shutting down…");
            r.store(false, Ordering::SeqCst);
        })
        .expect("Failed to set Ctrl+C handler");
    }

    // --- Shared parameters (GUI → Engine) ------------------------------------
    let params = state::create_shared_params();

    // --- Create channels for engine → GUI ------------------------------------
    let (snapshot_tx, snapshot_rx) = state::create_snapshot_channel();
    let (midi_log_tx, midi_log_rx) = state::create_midi_log_channel();

    // --- Initialize MIDI output (non-interactive) ----------------------------
    println!("[main] Initializing MIDI output…");
    let midi_result = midi::MidiController::connect(Some(midi_log_tx));
    let midi_port_name = midi_result.port_name.clone();
    let midi_connected = midi_result.connected;

    if !midi_result.available_ports.is_empty() {
        println!("[midi] Available ports:");
        for (i, name) in midi_result.available_ports.iter().enumerate() {
            println!("  [{}] {}", i, name);
        }
    }
    println!();

    // --- Initialize CREPE neural pitch detector ------------------------------
    println!("[main] Loading CREPE pitch model (ONNX Runtime)…");
    let crepe_detector = match crepe::CrepeDetector::initialize("crepe_full.onnx") {
        Ok(det) => {
            println!("[main] CREPE model loaded successfully.");
            det
        }
        Err(e) => {
            eprintln!("[main] Failed to load CREPE model: {}", e);
            eprintln!(
                "[main] Please download 'crepe_full.onnx' and place it in the project root."
            );
            eprintln!("[main]   See README.md for instructions.");
            return;
        }
    };
    println!();

    // --- Initialize audio capture --------------------------------------------
    println!("[main] Initializing audio capture…");
    let (audio_capture, mut consumer) = match audio::start_capture(running.clone()) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("[main] Audio capture failed: {}", e);
            eprintln!("[main] Please ensure a microphone is connected and try again.");
            return;
        }
    };
    let sample_rate = audio_capture.sample_rate;
    println!();

    // --- Create engine (on a dedicated thread) -------------------------------
    println!(
        "[main] Engine: window={}, hop={}, rate={} Hz, mode=Hybrid",
        engine::WINDOW_SIZE,
        engine::HOP_SIZE,
        sample_rate,
    );

    let engine_params = params.clone();
    let engine_running = running.clone();
    let engine_handle = std::thread::Builder::new()
        .name("voician-engine".into())
        .spawn(move || {
            let mut engine_inst = engine::Engine::new(
                crepe_detector,
                midi_result.controller,
                sample_rate as f32,
                snapshot_tx,
                engine_params,
            );

            let mut read_buffer = vec![0.0f32; 2048];

            while engine_running.load(Ordering::Relaxed) {
                let n = consumer.pop_slice(&mut read_buffer);

                if n > 0 {
                    engine_inst.process_samples(&read_buffer[..n]);
                } else {
                    std::thread::sleep(Duration::from_micros(500));
                }
            }

            drop(engine_inst);
            println!("[engine] Stopped.");
        })
        .expect("Failed to spawn engine thread");

    println!("[main] Engine thread started. Launching GUI…\n");

    // --- Build GUI state and launch ------------------------------------------
    let gui_state = state::GuiState::new(
        snapshot_rx,
        midi_log_rx,
        midi_port_name,
        midi_connected,
        sample_rate,
        params,
    );

    // GUI runs on the main thread (required by eframe/winit).
    let gui_result = gui::run_gui(gui_state);

    // --- GUI closed — shut down everything -----------------------------------
    println!("[main] GUI closed. Shutting down…");
    running.store(false, Ordering::SeqCst);

    let _ = engine_handle.join();
    drop(audio_capture);

    if let Err(e) = gui_result {
        eprintln!("[main] GUI error: {}", e);
    }

    println!("[main] Goodbye!");
}
