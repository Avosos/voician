# Voician — Real-Time Voice to MIDI with CREPE Neural Pitch Detection

A standalone real-time voice-to-MIDI engine written in Rust. Captures microphone audio, detects pitch using the **CREPE neural network** via ONNX Runtime for near-perfect accuracy, extracts spectral brightness, and sends expressive MIDI output (notes with velocity, continuous pitch bend, and CC 74 brightness) to a virtual MIDI port (loopMIDI) for use with Ableton Live, FL Studio, or any DAW.

Includes a real-time GUI built with egui showing pitch, waveform, MIDI status, and all parameters.

## Architecture

```
                                                        ┌─────────────────────────┐
Microphone → [cpal/WASAPI] → Ring Buffer ──┬──────────→ │ Native-rate analysis    │
              audio thread     lock-free   │            │  • RMS amplitude        │
                                           │            │  • Spectral centroid    │
                                           │            └───────────┬─────────────┘
                                           │                        │
                                           ▼                        ▼
                                    ┌─────────────┐     ┌─────────────────────────┐
                                    │ Resample    │     │ State Machine           │
                                    │ → 16 kHz    │     │  Silent→Pending→Active  │
                                    └──────┬──────┘     │  • Note On/Off          │
                                           │            │  • Pitch Bend           │
                                           ▼            │  • CC 74 (brightness)   │
                                    ┌─────────────┐     │  • Velocity             │
                                    │ CREPE ONNX  │────→└───────────┬─────────────┘
                                    │ [1,1024]    │                 │
                                    │ → freq+conf │                 ▼
                                    └─────────────┘          [midir] → loopMIDI → DAW
                                                                    │
                                                                    ▼
                                                             ┌─────────────┐
                                                             │  egui GUI   │
                                                             │  ~60 FPS    │
                                                             └─────────────┘
```

### Modules

| Module         | Responsibility |
|---------------|---------------|
| `crepe.rs`    | CREPE neural pitch detection via ONNX Runtime — loads model, normalizes audio, runs inference, softmax + weighted refinement |
| `engine.rs`   | Dual pipeline (native-rate + 16 kHz resampled), CREPE frame accumulation, note state machine, expressive MIDI |
| `audio.rs`    | Microphone capture via cpal, mono downmix, lock-free ring buffer |
| `analysis.rs` | RMS amplitude, spectral centroid (FFT), exponential smoothing filters |
| `midi.rs`     | MIDI output via midir — Note On/Off, Pitch Bend, CC, All Notes Off |
| `gui.rs`      | Real-time egui GUI — pitch display, waveform, MIDI status, minimal/advanced modes |
| `state.rs`    | Shared state types, snapshot channel, GUI state management |
| `pitch.rs`    | Legacy YIN pitch detection (kept for reference, not used in Phase 4) |
| `main.rs`     | Initialization — loads CREPE model, spawns engine thread, launches GUI |

### Performance

- **Pitch detection**: CREPE neural network (360-bin output, 20-cent resolution)
- **Latency**: ~64 ms per CREPE frame (1024 samples at 16 kHz) + resampler delay
- **Spectral analysis**: 2048-sample window, 512-sample hop at native rate
- **Pitch range**: 80 Hz – 1000 Hz (E2 to B5)
- **Audio thread**: Lock-free ring buffer, zero allocations in callback
- **ONNX Runtime**: Level 3 graph optimization, single intra-op thread

### MIDI Output

| Message | Source | Details |
|---------|--------|---------|
| Note On/Off | CREPE pitch detection | Stability-filtered (2 frames), velocity from RMS |
| Pitch Bend | Sub-semitone pitch deviation | ±2 semitone range, deadzone-filtered, EMA smoothed (α=0.25) |
| CC 74 | Spectral centroid (brightness) | Mapped 300–4000 Hz → 0–127, EMA smoothed (α=0.20) |

---

## Step-by-Step Setup

### 1. Install Rust

If you don't have Rust installed:

1. Go to [https://rustup.rs](https://rustup.rs)
2. Download and run `rustup-init.exe`
3. Follow the prompts (accept defaults)
4. **Restart your terminal** after installation
5. Verify:
   ```
   rustc --version
   cargo --version
   ```

### 2. Download the CREPE ONNX Model

Voician requires the CREPE pitch detection model in ONNX format. You have two options:

#### Option A: Download a Pre-Converted ONNX Model

If a `crepe_full.onnx` file is available (e.g., from a release or shared link), simply place it in the project root:

```
C:\Users\Marius\Desktop\voician\crepe_full.onnx
```

#### Option B: Convert from TensorFlow

1. Install Python dependencies:
   ```bash
   pip install tensorflow crepe tf2onnx
   ```

2. Run the conversion script:
   ```python
   import crepe
   import tensorflow as tf
   import tf2onnx

   # Load the CREPE "full" model
   model = crepe.core.build_and_load_model("full")

   # Convert to ONNX
   input_spec = [tf.TensorSpec(shape=(1, 1024), dtype=tf.float32, name="input")]
   model_proto, _ = tf2onnx.convert.from_keras(model, input_signature=input_spec)

   with open("crepe_full.onnx", "wb") as f:
       f.write(model_proto.SerializeToString())

   print("Saved crepe_full.onnx")
   ```

3. Copy the resulting `crepe_full.onnx` to the project root.

**Model details:**
- Input: `[1, 1024]` float32 (1024 audio samples at 16 kHz)
- Output: `[1, 360]` float32 (360 pitch bins, 20 cents each, covering ~32 Hz to ~1975 Hz)
- Size: ~80 MB for the "full" model

### 3. Install loopMIDI

loopMIDI creates virtual MIDI ports on Windows that bridge Voician to your DAW.

1. Download from [https://www.tobias-erichsen.de/software/loopmidi.html](https://www.tobias-erichsen.de/software/loopmidi.html)
2. Install and launch loopMIDI
3. Click the **+** button to create a new virtual MIDI port
4. The default name will be something like `loopMIDI Port` — leave it as-is
5. Keep loopMIDI running in the background

### 4. Build Voician

```powershell
cd C:\Users\Marius\Desktop\voician
cargo build --release
```

The optimized binary will be at `target\release\voician.exe`.

> **Note:** First build will download the ONNX Runtime shared library (~50 MB). Subsequent builds are fast.

### 5. Run Voician

```powershell
cargo run --release
```

Or run the binary directly:

```powershell
.\target\release\voician.exe
```

**Expected console output:**

```
╔═══════════════════════════════════════════════════╗
║    VOICIAN — Voice to MIDI Engine  (Phase 4)       ║
╠═══════════════════════════════════════════════════╣
║  CREPE neural pitch detection (ONNX Runtime)       ║
║  Expressive voice → MIDI with velocity, pitch      ║
║  bend, and CC 74 brightness                        ║
╚═══════════════════════════════════════════════════╝

[main] Loading CREPE pitch model (ONNX Runtime)…
[main] CREPE model loaded successfully.

[main] Initializing MIDI output…
[midi] Auto-selecting: loopMIDI Port (port 0)
[midi] Connected to: loopMIDI Port (channel 1)

[main] Initializing audio capture…
[audio] Input device : Microphone (USB Audio Device)
[audio] Capture started (44100 Hz, 1 ch → mono)

[main] Engine thread started. Launching GUI…
```

The GUI window will open showing real-time pitch, waveform, and MIDI status. Sing or hum — you'll see live MIDI events:

```
  NOTE ON    60 (C4)  vel= 85  freq=  261.6 Hz  conf=0.95
  NOTE OFF   60 (C4)  -> E4
  NOTE ON    64 (E4)  vel= 72  freq=  329.6 Hz  conf=0.92
```

### 6. Connect to Ableton Live

1. Open **Ableton Live**
2. Go to **Options → Preferences → Link, Tempo & MIDI**
3. Under **MIDI Ports**, find `loopMIDI Port`
4. Enable **Track** and **Remote** for the loopMIDI Input port
5. Close Preferences
6. Create a **MIDI track** (Ctrl+Shift+T)
7. Set the track's **MIDI From** dropdown to `loopMIDI Port`
8. Load any instrument (e.g., Analog, Wavetable, or a VST)
9. **Arm the track** (click the record-arm button)
10. Run Voician and start singing — the instrument will play!

### Connecting to Other DAWs

The same principle applies to any DAW:

- **FL Studio**: Options → MIDI Settings → enable loopMIDI Port as input
- **Reaper**: Options → Preferences → MIDI Devices → enable loopMIDI
- **Logic Pro**: (macOS only — use a different virtual MIDI driver)
- **Bitwig**: Settings → Controllers → add Generic MIDI keyboard on loopMIDI

---

## Tuning Parameters

Key constants in `src/engine.rs` you can adjust:

| Constant | Default | Description |
|----------|---------|-------------|
| `CONFIDENCE_THRESHOLD` | `0.60` | Minimum CREPE confidence to accept a pitch. Raise if getting false triggers (0.7–0.8 for noisy environments). |
| `SILENCE_RMS_THRESHOLD` | `0.012` | Minimum RMS to consider signal as voiced. Raise if getting false triggers from background noise. |
| `STABILITY_FRAMES` | `2` | Frames of consistent pitch before triggering NOTE_ON. Raise for less jitter, lower for faster response. |
| `STABILITY_TOLERANCE_SEMITONES` | `0.3` | Max semitone wobble allowed during stability check. |
| `NOTE_CHANGE_THRESHOLD_SEMITONES` | `0.5` | Semitone jump required to trigger a note change (OFF → Pending). |
| `PITCH_BEND_RANGE_SEMITONES` | `2.0` | Pitch bend range (± semitones). **Must match your DAW/synth setting** (see below). |
| `PITCH_BEND_DEADZONE` | `32` | Minimum 14-bit change before sending a new pitch bend message. |
| `CC_BRIGHTNESS` | `74` | MIDI CC number for spectral brightness. 74 = GM "Brightness" (filter cutoff). |
| `CENTROID_MIN_HZ` / `MAX_HZ` | `300` / `4000` | Spectral centroid range mapped to CC 0–127. Adjust for your voice. |
| `SMOOTH_ALPHA_PITCH` | `0.25` | EMA smoothing for pitch (higher = faster, noisier). |
| `SMOOTH_ALPHA_AMPLITUDE` | `0.15` | EMA smoothing for RMS amplitude. |
| `SMOOTH_ALPHA_CENTROID` | `0.20` | EMA smoothing for spectral centroid. |
| `MIN_FREQ_HZ` | `80.0` | Lowest detectable frequency. |
| `MAX_FREQ_HZ` | `1000.0` | Highest detectable frequency. |
| `WINDOW_SIZE` | `2048` | Native-rate analysis window in samples (for RMS + centroid). |
| `HOP_SIZE` | `512` | Samples between native-rate analyses. |

---

## DAW Pitch Bend Range Setup

Voician sends pitch bend with a ±2 semitone range by default. Your synth must match:

### Ableton Live
- **Analog / Wavetable / Drift**: Already default ±2 semitones — no change needed
- **Operator**: Pitch Bend Range is in the Pitch envelope section
- **VST/AU plugins**: Check the plugin's pitch bend range setting and set to ±2

### FL Studio
- In the instrument's settings, find Pitch Bend Range and set to 2

### General Rule
- If your synth's pitch bend range is different (e.g. ±12), change `PITCH_BEND_RANGE_SEMITONES` in `src/engine.rs` to match

## CC 74 Brightness Mapping

CC 74 is sent continuously while singing. To use it in your DAW:

1. In Ableton: right-click any knob → "Map to MIDI" → sing (Voician sends CC 74)
2. Or route CC 74 directly to a synth's filter cutoff / brightness parameter
3. The value tracks the "brightness" of your voice — say "eee" for bright, "ooo" for dark

---

## How CREPE Works

[CREPE](https://github.com/marl/crepe) (Convolutional Representation for Pitch Estimation) is a deep neural network trained on a large dataset of vocal and musical audio. It processes 1024 samples of 16 kHz audio and outputs a probability distribution over 360 pitch bins (20 cents each), spanning from ~32 Hz to ~1975 Hz.

Voician's implementation:
1. **Resamples** audio from the native sample rate (e.g. 44.1/48 kHz) down to 16 kHz using linear interpolation
2. **Normalizes** each 1024-sample frame to zero mean and unit variance
3. **Runs inference** through the ONNX model (single thread, Level 3 optimization)
4. **Applies softmax** to convert logits to probabilities
5. **Refines** the peak bin using a weighted average of neighboring bins for sub-bin accuracy
6. The resulting frequency and confidence are fed into the state machine

This approach provides significantly better accuracy than traditional autocorrelation (YIN) methods, especially for:
- Breathy or quiet singing
- Fast pitch transitions (melisma, vibrato)
- Mixed voice/falsetto registers
- Noisy environments

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "Failed to load CREPE model" | Ensure `crepe_full.onnx` is in the project root directory |
| "No audio input device found" | Check that a microphone is connected and enabled in Windows Sound settings |
| "No MIDI output ports found" | Start loopMIDI and create a virtual port |
| No sound in DAW | Ensure the MIDI track is armed and routed from loopMIDI |
| Too many false notes | Raise `CONFIDENCE_THRESHOLD` (e.g. 0.75) or `SILENCE_RMS_THRESHOLD` |
| Notes feel sluggish | Lower `STABILITY_FRAMES` to 1 |
| Pitch bend feels wobbly | Decrease `SMOOTH_ALPHA_PITCH` (e.g. 0.15) for more smoothing |
| CC 74 not doing anything | Map CC 74 to a parameter in your synth (filter cutoff recommended) |
| Pitch bend sounds wrong | Ensure synth pitch bend range matches `PITCH_BEND_RANGE_SEMITONES` (±2) |
| High CPU usage | Use `crepe_small.onnx` or `crepe_tiny.onnx` instead (if available) |

---

## License

MIT
