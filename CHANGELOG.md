# Changelog

All notable changes to InputSync are documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
