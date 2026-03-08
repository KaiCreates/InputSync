# Changelog

All notable changes to InputSync are documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [1.1.1] — 2026-03-08

### Fixed

**Linux — .deb install fails with file conflict against old `input-sync` package:**
- Installing `inputsync_1.1.1_amd64.deb` on a machine that previously had the old
  Tauri-based `input-sync` package would fail: dpkg refused to overwrite shared icon
  files (`/usr/share/icons/hicolor/*/apps/inputsync.png`) owned by `input-sync`.
- Added `Conflicts: input-sync` and `Replaces: input-sync` to the package metadata.
  dpkg now automatically removes the old package when installing InputSync 1.1.1+.
- The "Broken pipe / lzma write error" was a side-effect of dpkg aborting mid-extract
  on the conflict — resolved by the same fix.

**Linux — "Illegal instruction (core dumped)" on non-AVX2 CPUs:**
- Binary was compiled with `target-cpu=native` (AVX2/AVX512 instructions) and cached
  dependency objects were reused even after changing the CPU target. Full `cargo clean`
  plus `-C target-feature=-avx,-avx2,-avx512f` ensures the binary runs on any x86_64
  CPU, not just Haswell-era and newer.

---

## [1.1.0] — 2026-03-08

### Changed

**Architecture — Full migration from Tauri/WebView2/React to pure Rust + egui:**
- Replaced the entire Tauri + React frontend with a native egui (eframe 0.31 + glow) UI — no browser engine, no WebView2, no Node.js required at build or runtime
- Single portable native binary (~9 MB stripped) replaces the .deb/.exe installer package
- Windows no longer requires WebView2 Runtime — completely self-contained executable

### Added

- **Screen edge switching** — moving the cursor to any configured screen edge automatically forwards control to the connected client
- **Dead corners** — configurable corner regions that block edge triggers to prevent accidental switching
- **Dead zones** — configurable rectangular screen regions that suppress edge activation
- **Optional TLS transport** — self-signed TOFU certificate via `rcgen` + `rustls` + `tokio-rustls`; layered on top of the existing ChaCha20 session encryption
- **Settings persistence** — all configuration saved to `~/.local/share/inputsync/config.json` via serde_json
- **In-app log viewer** — real-time log output in the Logs tab via `egui_logger`
- **Mini screen-map widget** — clickable Painter-based widget in Settings for configuring edge targets
- Tabs: Main | Settings | Logs

### Fixed

- **Server restart after stop** — `Start Server` now auto-stops any running server; no stale state
- Stopped TCP listener now releases port immediately on shutdown

---

## [1.0.2] — 2026-03-07

### Fixed

**Windows/Linux — Server restart (os error 98 "Address already in use"):**
- Stopping and restarting the server showed "Address already in use" error — the TCP accept loop held port 24800 open indefinitely because the shutdown signal was never delivered (the oneshot receiver was immediately dropped). Replaced the broken oneshot with a `CancellationToken`; the TCP loop now exits and drops the `TcpListener` the instant `Stop Server` is clicked, releasing the port immediately.
- `Start Server` now auto-stops any previously running server instead of returning "Server already running" — clicking Start Server always works even if a prior session wasn't explicitly stopped.

**Windows — WebView2 required even on stripped Windows (Windows Lite, etc.):**
- The app required WebView2 to be pre-installed or downloaded from the internet, breaking on lightweight/locked-down Windows installations. Switched NSIS installer to `offlineInstaller` mode — the full WebView2 runtime is now bundled inside the `.exe` installer (~150 MB total). No internet connection, no Microsoft services, and no manual WebView2 installation required.

---

## [1.0.1] — 2026-03-07

### Fixed

**Windows:**
- Installer appeared frozen during WebView2 download — NSIS hook now detects whether WebView2 is present. If missing, shows a dialog before the download begins: "The installer is NOT frozen — please wait." If already installed (most Windows 10/11), skips silently.
- App silently failed to open when WebView2 was missing or damaged — replaced the silent panic with a native Windows MessageBoxW error dialog with exact fix steps and a download link.

**Linux (Wayland/Hyprland):**
- App showed no UI on Wayland compositors (Hyprland, etc.) — .desktop entry now launches with `GDK_BACKEND=x11 WEBKIT_DISABLE_DMABUF_RENDERER=1` to force XWayland.
- Tray icon caused a startup panic — icons regenerated as 8-bit RGBA (were 16-bit, causing `ImageBufferSize` mismatch in the tray icon loader).

---

## [1.0.0] — 2025-03-07

### Added
- X25519 ECDH key exchange with HKDF-SHA256 session key derivation
- ChaCha20-Poly1305 authenticated encryption on all input events
- 6-character alphanumeric session code pairing system
- TCP control channel (port 24800) for handshake and session management
- UDP event channel (ports 24801/24802) for low-latency input forwarding
- Cross-platform input capture via `rdev` (Linux X11, Windows)
- Cross-platform input simulation via `enigo` (Linux X11, Windows)
- Mouse, keyboard, and scroll wheel event forwarding
- Relative mouse movement (resolution-independent across screens)
- ScrollLock hotkey to toggle input forwarding without touching the UI
- Bounded input event channel with backpressure (drops stale packets)
- Counter-window UDP resync (up to 64 packets ahead on packet loss)
- Dedicated OS thread for input simulation (enigo compatibility)
- Pixel-art CRT terminal logo with blinking cursor SVG animation
- Dark theme React UI: Server panel, Client connect panel, Status bar
- Session code copy-to-clipboard button
- Linux `.deb` package (Ubuntu/Debian)
- Windows NSIS installer (`.exe`) via GitHub Actions CI
- GitHub Actions workflow: builds Linux + Windows on every tagged release

### Security
- All keyboard and mouse events encrypted before leaving the network adapter
- Nonces are XOR-combined from both server and client contributions
- Forward-only counter window prevents replay of captured packets
- Session codes are single-use per server start

---

## [Unreleased]

### Planned — v1.2
- Clipboard sync across machines
- Linux Wayland support (libei)
- mDNS/Bonjour server auto-discovery
- Configurable switch hotkey (not hardcoded ScrollLock)
- Ping/pong latency measurement display
- Code signing for Windows SmartScreen bypass
- RPM and Arch (AUR) packages

### Planned — v1.3
- Multi-monitor layout configuration
- macOS support
- Multi-client support with named client slots
