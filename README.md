# NeoResonance — Windows 11 Desktop Overlay Music Widget

A lightweight, native-feeling Windows 11 desktop music widget ("Glass Ribbon") built with **Tauri v2 (Rust backend)** and **React + TypeScript + Tailwind CSS (frontend)**.

![NeoResonance Design Reference](file:///C:/Users/shaur/.gemini/antigravity/brain/fab6740a-d571-46ea-8d9e-a8b4bcebe604/.user_uploaded/media_1795254422105_6af594dd.png)

NeoResonance sits directly on the Windows 11 desktop wallpaper layer, controls playing media via Windows SMTC, drives an audio-reactive ambient "Aura" backplate + 1px physics-based Bezier "String" wave via WASAPI loopback capture, and renders Apple Music-style synced lyrics with sub-word karaoke illumination.

---

## Key Features

1. **Windows 11 Desktop Overlay & Pinning (`src-tauri/src/window.rs`)**
   - Transparent, frameless capsule anchored at the bottom of the Z-order (`HWND_BOTTOM`) directly behind application windows.
   - Configured with `WS_EX_TOOLWINDOW` (hidden from `Alt+Tab`) and `WS_EX_NOACTIVATE` (prevents stealing window focus on playback interactions).
   - Desktop Pinning & `Win + D` survival: Intercepts `SC_MINIMIZE` in `WndProc` and attaches to the desktop wallpaper layer (`WorkerW`).
   - System Tray integration: Lock/Unlock position, toggle Aura & Wave, toggle Lyrics, and Quit.
   - Drag & Lock mode: Click the bottom-left lock icon or right-click anywhere to unlock and drag the widget; coordinates persist to `config.json`.

2. **System Media Transport Controls (SMTC) Engine (`src-tauri/src/smtc.rs`)**
   - Event-driven integration with Windows `GlobalSystemMediaTransportControlsSessionManager`.
   - Captures track title, artist, album, duration, playback status, and base64 album art.
   - Micro-controls for Previous, Play/Pause, Next, and interactive click-to-seek timeline.

3. **Audio-Reactive "Aura" & 1px "String" Baseline Wave (`src-tauri/src/audio.rs`)**
   - Real-time WASAPI loopback capture on the default audio endpoint.
   - FFT processing with `rustfft` (Hann window, 1024 samples) extracting sub-bass (20–150 Hz) and mids (200–2500 Hz).
   - Dynamic fluid ambient Aura backplate whose scale and glow intensity breathe on beat.
   - Continuous 1px Bezier "String" wave drawn on `<canvas>` at 60 FPS that vibrates with system audio and settles flat when paused.
   - Low-energy silence throttle to keep CPU usage near 0% when idle.

4. **Apple Music-Style Synced Lyrics (`src-tauri/src/lyrics.rs` + Frontend)**
   - Queries LRCLIB API with automatic in-memory caching.
   - High-precision `requestAnimationFrame` timeline interpolation.
   - Sub-word karaoke glow: voiced words dynamically illuminate with intense glow and subtle spring scale bump (`scale(1.025)`), while previously sung words dim smoothly.
   - Upcoming preview line rising into view.

---

## Quick Start

### Run Widget
```powershell
.\run.ps1
```

### Development Mode
```powershell
npm run tauri dev
```

### Production Build
```powershell
npm run build
cd src-tauri
cargo build --release
```
