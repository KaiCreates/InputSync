# Changelog

All notable changes to InputSync are documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

### Planned — v1.1
- Clipboard sync across machines
- Linux Wayland support (libei)
- mDNS/Bonjour server auto-discovery
- Configurable switch hotkey (not hardcoded ScrollLock)
- Ping/pong latency measurement display
- Code signing for Windows SmartScreen bypass
- RPM and Arch (AUR) packages

### Planned — v1.2
- Multi-monitor layout configuration (edge-based switching)
- macOS support
- Screen edge cursor switching (like Barrier/InputLeap)
- Multi-client support with named client slots
