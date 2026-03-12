// ============================================================================
// strudel.rs — WebSocket bridge: Voician → Strudel live-coding environment
// ============================================================================
//
// Architecture:
//   • HTTP server on port 9000 serves the embedded Strudel HTML page
//   • WebSocket server on port 9001 streams real-time pitch/note data
//   • The Strudel page connects via WebSocket and generates patterns
//     from the detected voice input
//
// Data flow:
//   Engine → crossbeam channel → WS server → browser → @strudel/web
// ============================================================================

use crossbeam_channel::Receiver;
use serde::Serialize;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tungstenite::{accept, Message};

/// Message sent to the Strudel frontend over WebSocket.
#[derive(Debug, Clone, Serialize)]
pub struct StrudelMessage {
    pub note_name: String,
    pub midi_note: Option<u8>,
    pub frequency: f32,
    pub velocity: u8,
    pub note_active: bool,
    pub rms: f32,
    pub confidence: f32,
    pub centroid_hz: f32,
    pub pitch_bend: u16,
    pub cc_brightness: u8,
}

/// Start the HTTP server that serves the Strudel page.
pub fn start_http_server(running: Arc<AtomicBool>) {
    std::thread::Builder::new()
        .name("strudel-http".into())
        .spawn(move || {
            let listener = match TcpListener::bind("127.0.0.1:9000") {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[strudel] HTTP server failed to bind port 9000: {}", e);
                    return;
                }
            };
            listener
                .set_nonblocking(true)
                .expect("Cannot set non-blocking");

            println!("[strudel] HTTP server running at http://127.0.0.1:9000");

            while running.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut buf = [0u8; 4096];
                        let _ = stream.read(&mut buf);
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            STRUDEL_HTML.len(),
                            STRUDEL_HTML
                        );
                        let _ = stream.write_all(response.as_bytes());
                        let _ = stream.flush();
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
            println!("[strudel] HTTP server stopped.");
        })
        .expect("Failed to spawn strudel HTTP thread");
}

/// Start the WebSocket server that streams pitch data to the Strudel page.
pub fn start_ws_server(rx: Receiver<StrudelMessage>, running: Arc<AtomicBool>) {
    std::thread::Builder::new()
        .name("strudel-ws".into())
        .spawn(move || {
            let listener = match TcpListener::bind("127.0.0.1:9001") {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[strudel] WebSocket server failed to bind port 9001: {}", e);
                    return;
                }
            };
            listener
                .set_nonblocking(true)
                .expect("Cannot set non-blocking");

            println!("[strudel] WebSocket server running at ws://127.0.0.1:9001");

            while running.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, addr)) => {
                        println!("[strudel] WebSocket client connected: {}", addr);
                        let rx_clone = rx.clone();
                        let r = running.clone();
                        std::thread::Builder::new()
                            .name("strudel-ws-client".into())
                            .spawn(move || {
                                handle_ws_client(stream, rx_clone, r);
                            })
                            .ok();
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }
            println!("[strudel] WebSocket server stopped.");
        })
        .expect("Failed to spawn strudel WS thread");
}

fn handle_ws_client(stream: TcpStream, rx: Receiver<StrudelMessage>, running: Arc<AtomicBool>) {
    stream.set_nonblocking(false).ok();
    stream
        .set_read_timeout(Some(std::time::Duration::from_millis(100)))
        .ok();

    let mut ws = match accept(stream) {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("[strudel] WebSocket handshake failed: {}", e);
            return;
        }
    };

    // Throttle: send at most ~30 messages per second
    let send_interval = std::time::Duration::from_millis(33);
    let mut last_send = std::time::Instant::now();

    while running.load(Ordering::Relaxed) {
        // Drain channel, keep only the latest message
        let mut latest: Option<StrudelMessage> = None;
        while let Ok(msg) = rx.try_recv() {
            latest = Some(msg);
        }

        if let Some(msg) = latest {
            if last_send.elapsed() >= send_interval {
                let json = match serde_json::to_string(&msg) {
                    Ok(j) => j,
                    Err(_) => continue,
                };
                if ws.send(Message::Text(json.into())).is_err() {
                    println!("[strudel] WebSocket client disconnected.");
                    return;
                }
                last_send = std::time::Instant::now();
            }
        }

        // Check for incoming messages (close, ping)
        match ws.read() {
            Ok(Message::Close(_)) => {
                println!("[strudel] WebSocket client closed connection.");
                return;
            }
            Ok(Message::Ping(data)) => {
                let _ = ws.send(Message::Pong(data));
            }
            Err(tungstenite::Error::Io(ref e))
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut => {}
            Err(_) => {
                return;
            }
            _ => {}
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// Open the Strudel page in the user's default browser.
pub fn open_browser() {
    if let Err(e) = open::that("http://127.0.0.1:9000") {
        eprintln!("[strudel] Failed to open browser: {}", e);
    }
}

// ---------------------------------------------------------------------------
// Embedded Strudel HTML page
// ---------------------------------------------------------------------------

const STRUDEL_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Voician × Strudel</title>
  <script src="https://unpkg.com/@strudel/web@1.0.3"></script>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body {
      background: #0f1119; color: #e0e0e0;
      font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
      min-height: 100vh; display: flex; flex-direction: column;
    }
    header {
      background: #161822; padding: 12px 24px;
      display: flex; align-items: center; justify-content: space-between;
      border-bottom: 1px solid #2a2d3a;
    }
    header h1 { font-size: 18px; font-weight: 600; color: #7dd3fc; }
    header .status { font-size: 13px; color: #888; }
    header .status.connected { color: #4ade80; }
    .controls {
      background: #161822; padding: 12px 24px;
      display: flex; gap: 12px; align-items: center; flex-wrap: wrap;
      border-bottom: 1px solid #2a2d3a;
    }
    button {
      background: #2563eb; color: #fff; border: none; padding: 8px 18px;
      border-radius: 6px; cursor: pointer; font-size: 13px; font-family: inherit;
      transition: background 0.15s;
    }
    button:hover { background: #3b82f6; }
    button:disabled { background: #334155; color: #666; cursor: not-allowed; }
    button.stop { background: #dc2626; }
    button.stop:hover { background: #ef4444; }
    button.active { background: #16a34a; }
    select, input {
      background: #1e2030; color: #e0e0e0; border: 1px solid #2a2d3a;
      padding: 6px 12px; border-radius: 6px; font-size: 13px; font-family: inherit;
    }
    label { font-size: 13px; color: #94a3b8; }
    .main-area { display: flex; flex: 1; min-height: 0; }
    .voice-panel {
      width: 260px; background: #161822; padding: 16px;
      border-right: 1px solid #2a2d3a; overflow-y: auto;
    }
    .voice-panel h3 { color: #7dd3fc; font-size: 14px; margin-bottom: 12px; }
    .meter { margin-bottom: 12px; }
    .meter-label { font-size: 11px; color: #888; margin-bottom: 4px; }
    .meter-bar {
      height: 8px; background: #1e1e2e; border-radius: 4px; overflow: hidden;
    }
    .meter-fill {
      height: 100%; background: #4ade80; border-radius: 4px;
      transition: width 0.08s ease-out;
    }
    .note-display {
      font-size: 48px; font-weight: 700; text-align: center;
      color: #555; padding: 12px 0; margin-bottom: 8px;
      transition: color 0.1s;
    }
    .note-display.active { color: #4ade80; }
    .freq-display {
      text-align: center; font-size: 14px; color: #888; margin-bottom: 16px;
    }
    .note-history {
      font-size: 12px; color: #666; padding: 8px;
      background: #1e2030; border-radius: 6px; min-height: 40px;
      max-height: 120px; overflow-y: auto; word-break: break-all;
    }
    .code-area {
      flex: 1; display: flex; flex-direction: column; min-width: 0;
    }
    .code-editor {
      flex: 1; padding: 16px; overflow: auto;
    }
    textarea {
      width: 100%; height: 100%; background: #1e2030; color: #e0e0e0;
      border: 1px solid #2a2d3a; border-radius: 8px; padding: 16px;
      font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace;
      font-size: 14px; line-height: 1.6; resize: none;
    }
    textarea:focus { outline: 2px solid #2563eb; border-color: transparent; }
    .pattern-log {
      background: #161822; padding: 12px 24px;
      border-top: 1px solid #2a2d3a; font-size: 12px; color: #666;
      max-height: 80px; overflow-y: auto;
    }
  </style>
</head>
<body>
  <header>
    <h1>🎤 Voician × Strudel</h1>
    <div class="status" id="ws-status">⏳ Connecting to Voician...</div>
  </header>

  <div class="controls">
    <button id="btn-play" onclick="startPlaying()">▶ Play</button>
    <button id="btn-stop" class="stop" onclick="stopPlaying()" disabled>■ Stop</button>
    <label>Mode:
      <select id="mode-select" onchange="onModeChange()">
        <option value="follow">🎵 Melodic Follow</option>
        <option value="harmonize">🎹 Auto Harmonize</option>
        <option value="arpeggio">🔄 Arpeggio Builder</option>
        <option value="rhythm">🥁 Rhythm to Drums</option>
        <option value="ambient">🌊 Ambient Texture</option>
        <option value="custom">✏️ Custom Pattern</option>
      </select>
    </label>
    <label>Synth:
      <select id="synth-select">
        <option value="sawtooth">Sawtooth</option>
        <option value="sine">Sine</option>
        <option value="square">Square</option>
        <option value="triangle" selected>Triangle</option>
        <option value="piano">Piano</option>
        <option value="gm_epiano1">Electric Piano</option>
        <option value="gm_strings1">Strings</option>
        <option value="gm_flute">Flute</option>
      </select>
    </label>
    <label>BPM: <input type="number" id="bpm-input" value="120" min="40" max="300" style="width:60px"></label>
  </div>

  <div class="main-area">
    <div class="voice-panel">
      <h3>Voice Input</h3>
      <div class="note-display" id="note-display">---</div>
      <div class="freq-display" id="freq-display">0.0 Hz</div>

      <div class="meter">
        <div class="meter-label">Volume</div>
        <div class="meter-bar"><div class="meter-fill" id="vol-meter" style="width:0%"></div></div>
      </div>
      <div class="meter">
        <div class="meter-label">Confidence</div>
        <div class="meter-bar"><div class="meter-fill" id="conf-meter" style="width:0%"></div></div>
      </div>
      <div class="meter">
        <div class="meter-label">Brightness</div>
        <div class="meter-bar"><div class="meter-fill" id="bright-meter" style="width:0%"></div></div>
      </div>

      <h3 style="margin-top:16px">Note History</h3>
      <div class="note-history" id="note-history"></div>
    </div>

    <div class="code-area">
      <div class="code-editor">
        <textarea id="code-editor" spellcheck="false">// Voician → Strudel pattern (auto-generated)
// Select a mode above or write your own pattern.
// Use $note, $freq, $vel, $brightness as live voice variables.

note("c4 e4 g4 c5").s("triangle").lpf(800)
</textarea>
      </div>
    </div>
  </div>

  <div class="pattern-log" id="pattern-log">Ready. Connect Voician and click Play.</div>

<script>
  // =========================================================================
  // State
  // =========================================================================
  let ws = null;
  let playing = false;
  let currentNote = null;
  let currentFreq = 0;
  let currentVel = 0;
  let currentBrightness = 0;
  let currentRms = 0;
  let noteActive = false;
  let noteHistory = [];
  const MAX_HISTORY = 32;

  // Track recent notes for pattern generation
  let recentNotes = [];
  const MAX_RECENT = 16;
  let lastPatternUpdate = 0;
  const PATTERN_UPDATE_INTERVAL = 500; // ms

  // =========================================================================
  // WebSocket connection to Voician
  // =========================================================================
  function connectWS() {
    ws = new WebSocket('ws://127.0.0.1:9001');
    ws.onopen = () => {
      document.getElementById('ws-status').textContent = '✅ Connected to Voician';
      document.getElementById('ws-status').className = 'status connected';
    };
    ws.onclose = () => {
      document.getElementById('ws-status').textContent = '❌ Disconnected — retrying...';
      document.getElementById('ws-status').className = 'status';
      setTimeout(connectWS, 2000);
    };
    ws.onerror = () => {};
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        handleVoicianData(data);
      } catch(e) {}
    };
  }

  function handleVoicianData(data) {
    currentNote = data.note_name;
    currentFreq = data.frequency;
    currentVel = data.velocity;
    currentBrightness = data.cc_brightness;
    currentRms = data.rms;
    noteActive = data.note_active;

    // Update display
    const noteEl = document.getElementById('note-display');
    noteEl.textContent = currentNote || '---';
    noteEl.className = noteActive ? 'note-display active' : 'note-display';
    document.getElementById('freq-display').textContent = currentFreq.toFixed(1) + ' Hz';
    document.getElementById('vol-meter').style.width = Math.min(currentRms * 500, 100) + '%';
    document.getElementById('conf-meter').style.width = (data.confidence * 100) + '%';
    document.getElementById('bright-meter').style.width = (currentBrightness / 127 * 100) + '%';

    // Collect notes for pattern generation
    if (noteActive && data.midi_note != null) {
      const now = Date.now();
      const last = recentNotes[recentNotes.length - 1];
      if (!last || last.note !== data.midi_note || (now - last.time) > 300) {
        recentNotes.push({ note: data.midi_note, name: data.note_name, vel: data.velocity, time: now });
        if (recentNotes.length > MAX_RECENT) recentNotes.shift();

        // Update note history display
        noteHistory.push(data.note_name);
        if (noteHistory.length > MAX_HISTORY) noteHistory.shift();
        document.getElementById('note-history').textContent = noteHistory.join(' → ');
      }

      // Auto-update pattern
      if (playing && (now - lastPatternUpdate) > PATTERN_UPDATE_INTERVAL) {
        lastPatternUpdate = now;
        updateLivePattern();
      }
    }
  }

  // =========================================================================
  // Strudel integration
  // =========================================================================
  let strudelInitialized = false;

  async function initStrudelOnce() {
    if (strudelInitialized) return;
    await initStrudel();
    strudelInitialized = true;
  }

  async function startPlaying() {
    await initStrudelOnce();
    playing = true;
    document.getElementById('btn-play').disabled = true;
    document.getElementById('btn-stop').disabled = false;
    document.getElementById('btn-play').classList.add('active');
    updateLivePattern();
    log('Playing started. Sing into your mic!');
  }

  function stopPlaying() {
    playing = false;
    hush();
    document.getElementById('btn-play').disabled = false;
    document.getElementById('btn-stop').disabled = true;
    document.getElementById('btn-play').classList.remove('active');
    log('Stopped.');
  }

  // =========================================================================
  // Pattern generation modes
  // =========================================================================

  function updateLivePattern() {
    if (!playing) return;
    const mode = document.getElementById('mode-select').value;
    const synth = document.getElementById('synth-select').value;
    const bpm = parseInt(document.getElementById('bpm-input').value) || 120;

    try {
      setcps(bpm / 60 / 4); // Convert BPM to cycles per second

      if (mode === 'custom') {
        // Custom mode: evaluate user's code from the textarea
        const code = document.getElementById('code-editor').value;
        evalStrudelCode(code);
        return;
      }

      const noteNames = recentNotes.slice(-8).map(n => n.name.toLowerCase());
      if (noteNames.length === 0) return;

      let pattern;
      const noteStr = noteNames.join(' ');

      switch (mode) {
        case 'follow':
          pattern = buildFollowPattern(noteStr, synth);
          break;
        case 'harmonize':
          pattern = buildHarmonizePattern(noteNames, synth);
          break;
        case 'arpeggio':
          pattern = buildArpeggioPattern(noteNames, synth);
          break;
        case 'rhythm':
          pattern = buildRhythmPattern(synth);
          break;
        case 'ambient':
          pattern = buildAmbientPattern(noteNames, synth);
          break;
        default:
          pattern = buildFollowPattern(noteStr, synth);
      }

      document.getElementById('code-editor').value = pattern;
      evalStrudelCode(pattern);
      log('Pattern updated: ' + mode + ' (' + noteNames.length + ' notes)');
    } catch (e) {
      log('Error: ' + e.message);
    }
  }

  function evalStrudelCode(code) {
    try {
      const fn = new Function(
        'note', 'sound', 's', 'stack', 'setcps', 'hush',
        'sine', 'saw', 'square', 'tri', 'perlin',
        'rev', 'jux', 'slow', 'fast',
        code
      );
      // We use strudel's global functions directly since initStrudel() exposes them
      eval(code);
    } catch(e) {
      // If eval fails, just log it
      log('Code error: ' + e.message);
    }
  }

  // --- Mode: Melodic Follow ---
  function buildFollowPattern(noteStr, synth) {
    const usesSample = synth.startsWith('gm_') || synth === 'piano';
    const sCmd = usesSample ? `.s("${synth}")` : `.s("${synth}")`;
    const lpf = currentBrightness > 0 ? `.lpf(${200 + currentBrightness * 40})` : '';
    return `note("${noteStr}")${sCmd}${lpf}.room(0.3).gain(0.7)`;
  }

  // --- Mode: Auto Harmonize ---
  function buildHarmonizePattern(noteNames, synth) {
    const root = noteNames[noteNames.length - 1];
    // Build a simple triad from the most recent note
    const lines = [
      `note("${noteNames.join(' ')}").s("${synth}").room(0.3).gain(0.5)`,
      `note("${noteNames.join(' ')}").add(note("4")).s("${synth}").room(0.4).gain(0.3)`,
      `note("${noteNames.join(' ')}").add(note("7")).s("${synth}").room(0.5).gain(0.25)`,
    ];
    return `stack(\n  ${lines.join(',\n  ')}\n)`;
  }

  // --- Mode: Arpeggio Builder ---
  function buildArpeggioPattern(noteNames, synth) {
    const unique = [...new Set(noteNames)];
    const fast = Math.max(2, unique.length);
    return `note("${unique.join(' ')}").s("${synth}").fast(${fast}).lpf(sine.range(400, 3000).slow(4)).room(0.4).gain(0.6)`;
  }

  // --- Mode: Rhythm to Drums ---
  function buildRhythmPattern(synth) {
    // Convert recent note timings into a drum pattern
    const now = Date.now();
    const recent = recentNotes.filter(n => (now - n.time) < 4000);
    if (recent.length < 2) return `s("bd sd hh*4 hh").gain(0.8)`;

    // Map note onsets to 16th-note grid
    const span = recent[recent.length - 1].time - recent[0].time;
    const gridSize = 16;
    let grid = new Array(gridSize).fill('~');

    recent.forEach(n => {
      const pos = Math.round(((n.time - recent[0].time) / Math.max(span, 1)) * (gridSize - 1));
      // Higher notes = hi-hat, mid = snare, low = kick
      if (n.note > 72) grid[pos] = 'hh';
      else if (n.note > 60) grid[pos] = 'sd';
      else grid[pos] = 'bd';
    });

    return `s("${grid.join(' ')}").gain(0.8)`;
  }

  // --- Mode: Ambient Texture ---
  function buildAmbientPattern(noteNames, synth) {
    const unique = [...new Set(noteNames)].slice(-4);
    return `note("${unique.join(' ')}").s("${synth}").slow(4)\n  .jux(rev).room(0.8).delay(0.5).lpf(sine.range(200, 2000).slow(8))\n  .gain(0.4)`;
  }

  // =========================================================================
  // UI helpers
  // =========================================================================
  function onModeChange() {
    const mode = document.getElementById('mode-select').value;
    const editor = document.getElementById('code-editor');
    if (mode === 'custom') {
      editor.removeAttribute('readonly');
      editor.style.opacity = '1';
    } else {
      editor.style.opacity = '0.8';
    }
    if (playing) updateLivePattern();
  }

  function log(msg) {
    const el = document.getElementById('pattern-log');
    const time = new Date().toLocaleTimeString();
    el.textContent = `[${time}] ${msg}`;
  }

  // Allow manual evaluation with Ctrl+Enter in custom mode
  document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key === 'Enter') {
      e.preventDefault();
      const mode = document.getElementById('mode-select').value;
      if (mode === 'custom' && playing) {
        const code = document.getElementById('code-editor').value;
        try {
          eval(code);
          log('Custom pattern evaluated.');
        } catch(err) {
          log('Error: ' + err.message);
        }
      }
    }
  });

  // =========================================================================
  // Init
  // =========================================================================
  connectWS();
</script>
</body>
</html>"##;
