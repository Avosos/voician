# Voician — Real-Time Expressive Voice to MIDI Engine

A standalone real-time voice-to-MIDI engine written in Rust. Captures microphone audio, detects pitch using the YIN algorithm with FFT-based autocorrelation, extracts spectral brightness, and sends expressive MIDI output (notes with velocity, continuous pitch bend, and CC 74 brightness) to a virtual MIDI port (loopMIDI) for use with Ableton Live, FL Studio, or any DAW.

## Architecture

```
Microphone → [cpal/WASAPI] → Ring Buffer → ┌─ YIN Pitch Detection ──┐
              audio thread     lock-free   ├─ RMS Amplitude        │→ State Machine → [midir] → loopMIDI → DAW
                                           └─ Spectral Centroid ───┘    │ │ │ │
                                                                     │ │ │ └─ CC 74 (brightness)
                                                                     │ │ └─── Pitch Bend (continuous)
                                                                     │ └───── Velocity (from RMS)
                                                                     └─────── Note On / Off
```

### Modules

| Module         | Responsibility |
|---------------|---------------|
| `audio.rs`    | Microphone capture via cpal, mono downmix, lock-free ring buffer |
| `pitch.rs`    | YIN fundamental frequency detection with FFT-based autocorrelation |
| `analysis.rs` | RMS amplitude, spectral centroid (FFT), exponential smoothing filters |
| `midi.rs`     | MIDI output via midir — Note On/Off, Pitch Bend, CC, All Notes Off |
| `engine.rs`   | Processing loop, sliding window, note state machine, expressive MIDI |
| `main.rs`     | Initialization, Ctrl+C handling, main processing loop |

### Performance

- **Latency target**: < 30 ms end-to-end
- **Analysis**: 2048-sample window, 512-sample hop (~11.6 ms between detections)
- **Pitch range**: 80 Hz – 1000 Hz (E2 to B5)
- **Audio thread**: Lock-free ring buffer, zero allocations in callback
- **Pitch detection**: O(N log N) via FFT-based autocorrelation

### MIDI Output (Phase 2)

| Message | Source | Details |
|---------|--------|---------|
| Note On/Off | YIN pitch detection | Stability-filtered (2 frames ≈ 23 ms), velocity from RMS |
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

### 2. Install loopMIDI

loopMIDI creates virtual MIDI ports on Windows that bridge Voician to your DAW.

1. Download from [https://www.tobias-erichsen.de/software/loopmidi.html](https://www.tobias-erichsen.de/software/loopmidi.html)
2. Install and launch loopMIDI
3. Click the **+** button to create a new virtual MIDI port
4. The default name will be something like `loopMIDI Port` — leave it as-is
5. Keep loopMIDI running in the background

### 3. Build Voician

```powershell
cd C:\Users\Marius\Desktop\voician
cargo build --release
```

The optimized binary will be at `target\release\voician.exe`.

### 4. Run Voician

```powershell
cargo run --release
```

Or run the binary directly:

```powershell
.\target\release\voician.exe
```

**Expected output:**

```
╔════════════════════════════════════════════════╗
║        VOICIAN — Voice to MIDI Engine          ║
╠════════════════════════════════════════════════╣
║  Real-time voice pitch → MIDI note converter   ║
║  Sing or hum into your microphone to play!     ║
║  Press Ctrl+C to exit                          ║
╚════════════════════════════════════════════════╝

[main] Initializing MIDI output…
[midi] Available MIDI output ports:
  [0] loopMIDI Port ← auto-detected
[midi] Auto-selecting: loopMIDI Port (port 0)
[midi] Connected to: loopMIDI Port (channel 1)

[main] Initializing audio capture…
[audio] Input device : Microphone (USB Audio Device)
[audio] Capture started (44100 Hz, 1 ch → mono, ring buf 65536 samples)

[main] Engine running  (window=2048, hop=512, rate=44100 Hz)
[main] Listening… sing or hum into your mic!
```

Sing or hum — you'll see live MIDI events with expressive parameters:

```
  ● NOTE ON    60 (C4)  vel= 85  freq=  261.6 Hz  bend= 8192  cc74= 45  conf=0.95
  ○ NOTE OFF   60 (C4)  — silence
  ● NOTE ON    64 (E4)  vel= 72  freq=  329.6 Hz  bend= 8350  cc74= 62  conf=0.92
```

Press **Ctrl+C** to exit cleanly.

### 5. Connect to Ableton Live

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
| `SILENCE_RMS_THRESHOLD` | `0.012` | Minimum RMS to consider signal as voiced. Raise if getting false triggers from background noise. |
| `STABILITY_FRAMES` | `2` | Frames of consistent pitch before triggering NOTE_ON. Raise for less jitter, lower for faster response. |
| `STABILITY_TOLERANCE_SEMITONES` | `0.3` | Max semitone wobble allowed during stability check. |
| `NOTE_CHANGE_THRESHOLD_SEMITONES` | `0.5` | Semitone jump required to trigger a note change (OFF → Pending). |
| `YIN_THRESHOLD` | `0.15` | YIN aperiodicity threshold. Lower = stricter pitch detection (fewer false positives). |
| `PITCH_BEND_RANGE_SEMITONES` | `2.0` | Pitch bend range (± semitones). **Must match your DAW/synth setting** (see below). |
| `PITCH_BEND_DEADZONE` | `32` | Minimum 14-bit change before sending a new pitch bend message. |
| `CC_BRIGHTNESS` | `74` | MIDI CC number for spectral brightness. 74 = GM "Brightness" (filter cutoff). |
| `CENTROID_MIN_HZ` / `MAX_HZ` | `300` / `4000` | Spectral centroid range mapped to CC 0–127. Adjust for your voice. |
| `SMOOTH_ALPHA_PITCH` | `0.25` | EMA smoothing for pitch (higher = faster, noisier). |
| `SMOOTH_ALPHA_AMPLITUDE` | `0.15` | EMA smoothing for RMS amplitude. |
| `SMOOTH_ALPHA_CENTROID` | `0.20` | EMA smoothing for spectral centroid. |
| `MIN_FREQ_HZ` | `80.0` | Lowest detectable frequency. |
| `MAX_FREQ_HZ` | `1000.0` | Highest detectable frequency. |
| `WINDOW_SIZE` | `2048` | Analysis window in samples. Larger = better low-freq accuracy, higher latency. |
| `HOP_SIZE` | `512` | Samples between analyses. Smaller = more frequent detection, more CPU. |

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

## Troubleshooting

| Problem | Solution |
|---------|----------|
| "No audio input device found" | Check that a microphone is connected and enabled in Windows Sound settings |
| "No MIDI output ports found" | Start loopMIDI and create a virtual port |
| No sound in DAW | Ensure the MIDI track is armed and routed from loopMIDI |
| Too many false notes | Raise `SILENCE_RMS_THRESHOLD` or lower `YIN_THRESHOLD` |
| Notes feel sluggish | Lower `STABILITY_FRAMES` to 1 |
| Pitch bend feels wobbly | Decrease `SMOOTH_ALPHA_PITCH` (e.g. 0.15) for more smoothing |
| CC 74 not doing anything | Map CC 74 to a parameter in your synth (filter cutoff recommended) |
| Pitch bend sounds wrong | Ensure synth pitch bend range matches `PITCH_BEND_RANGE_SEMITONES` (±2) |
| High CPU usage | Increase `HOP_SIZE` (e.g., 1024) |

---

## License

MIT
