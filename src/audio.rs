// ============================================================================
// audio.rs — Real-time microphone capture via cpal
// ============================================================================
//
// Captures audio from the default input device and pushes mono f32 samples
// into a lock-free SPSC ring buffer. The audio callback runs on a dedicated
// high-priority thread managed by the OS audio subsystem (WASAPI on Windows).
//
// The ring buffer consumer is returned to the caller for processing on a
// separate thread, ensuring the audio callback never blocks.
// ============================================================================

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleRate, Stream, StreamConfig, BufferSize};
use ringbuf::{traits::*, HeapRb, HeapCons};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Ring buffer capacity in mono samples (~1.5 seconds at 44100 Hz).
/// Sized large enough to absorb scheduling jitter without dropping frames.
const RING_BUFFER_CAPACITY: usize = 65536;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Handle that keeps the audio stream alive.
/// Drop this to stop capture.
pub struct AudioCapture {
    _stream: Stream,
    pub sample_rate: u32,
    #[allow(dead_code)]
    pub channels: u16,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Start capturing audio from the default input device.
///
/// Returns:
/// - `AudioCapture` handle (must be kept alive for the duration of capture)
/// - `HeapCons<f32>` ring buffer consumer for reading mono audio samples
///
/// The callback converts multi-channel input to mono by taking the first
/// channel only.
pub fn start_capture(
    running: Arc<AtomicBool>,
) -> Result<(AudioCapture, HeapCons<f32>)> {
    // --- Select host and device ------------------------------------------------
    let host = cpal::default_host();

    let device = host
        .default_input_device()
        .context("No audio input device found. Check that a microphone is connected.")?;

    let device_name = device
        .name()
        .unwrap_or_else(|_| "Unknown".to_string());
    println!("[audio] Input device : {}", device_name);

    // --- Query default config --------------------------------------------------
    let supported = device
        .default_input_config()
        .context("Failed to query default input config")?;

    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels();

    println!(
        "[audio] Device config: {} Hz, {} ch, {:?}",
        sample_rate,
        channels,
        supported.sample_format()
    );

    // Use the device's native sample rate and channel count for maximum
    // compatibility. We down-mix to mono in the callback.
    let config = StreamConfig {
        channels,
        sample_rate: SampleRate(sample_rate),
        buffer_size: BufferSize::Default,
    };

    // --- Create lock-free ring buffer ------------------------------------------
    let rb = HeapRb::<f32>::new(RING_BUFFER_CAPACITY);
    let (mut producer, consumer) = rb.split();

    // --- Build input stream (f32) ----------------------------------------------
    let num_channels = channels as usize;
    let running_clone = running.clone();

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Bail early when shutdown is requested.
                if !running_clone.load(Ordering::Relaxed) {
                    return;
                }

                // Down-mix to mono: take the first channel of each frame.
                // This is allocation-free — only atomic writes to the ring buffer.
                if num_channels == 1 {
                    for &sample in data {
                        let _ = producer.try_push(sample);
                    }
                } else {
                    for frame in data.chunks(num_channels) {
                        let _ = producer.try_push(frame[0]);
                    }
                }
            },
            move |err| {
                eprintln!("[audio] Stream error: {}", err);
            },
            None, // None = no timeout (blocking)
        )
        .context("Failed to build audio input stream. Ensure microphone permissions are granted.")?;

    stream.play().context("Failed to start audio stream")?;

    println!(
        "[audio] Capture started ({} Hz, {} ch → mono, ring buf {} samples)",
        sample_rate, channels, RING_BUFFER_CAPACITY
    );

    Ok((
        AudioCapture {
            _stream: stream,
            sample_rate,
            channels,
        },
        consumer,
    ))
}
