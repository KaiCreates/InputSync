<div align="center">

<!-- LOGO -->
```
┌ ● ● ● ──────────────────── ┐
│  ░░░░░░░░░░░░░░░░░░░░░░░░  │
│  ░  ▌                   ░  │
│  ░   ▌  ▬▬▬▬▬           ░  │
│  ░  ▌                   ░  │
│  ░░░░░░░░░░░░░░░░░░░░░░░░  │
└────────────────────────── ┘
        ████
      ████████
```

# InputSync

**Cross-platform KVM switch software — control multiple computers with one keyboard and mouse**

[![License: MIT](https://img.shields.io/badge/License-MIT-6c63ff.svg)](LICENSE)
[![Build](https://img.shields.io/github/actions/workflow/status/KaiCreates/InputSync/build.yml?label=Build&logo=github)](https://github.com/KaiCreates/InputSync/actions)
[![Version](https://img.shields.io/badge/version-1.1.0-3ecf8e)](https://github.com/KaiCreates/InputSync/releases)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-blue)](https://github.com/KaiCreates/InputSync/releases)
[![Downloads](https://img.shields.io/github/downloads/KaiCreates/InputSync/total?color=3ecf8e)](https://github.com/KaiCreates/InputSync/releases)

</div>

---

## What is InputSync?

InputSync is a lightweight, encrypted software KVM (Keyboard, Video, Mouse) switch. Run it on two or more computers on the same network and seamlessly share your keyboard and mouse between them — no hardware required.

Think of it like [Barrier](https://github.com/debauchee/barrier) or [InputLeap](https://github.com/input-leap/input-leap), but built from scratch in **pure Rust + egui** with end-to-end encryption baked in from day one. No browser engine. No Electron. No WebView2. Just a single native binary.

---

## Features

- **🔐 End-to-End Encrypted** — X25519 ECDH key exchange + ChaCha20-Poly1305; no plaintext ever leaves your machine
- **⚡ Ultra Low Latency** — UDP transport with delta-encoded events; input feels local
- **🎯 Session Codes** — 6-character alphanumeric codes to pair devices; no IP configuration required
- **🖱️ Screen Edge Switching** — move the cursor to the edge of the screen to automatically switch control to the client
- **🚫 Dead Zones & Dead Corners** — block edge triggers in configured screen regions to prevent accidental switching
- **🖥️ Cross-Platform** — Linux (X11) and Windows from a single Rust codebase
- **🪶 Tiny Footprint** — ~9 MB native binary; no background services; no runtime dependencies
- **🎨 Native Dark UI** — Terminal-themed interface built with egui; no browser engine or WebView2 required
- **🔒 Optional TLS** — Self-signed certificate (TOFU) transport layer on top of the encrypted stream
- **📋 Clipboard Sync** *(roadmap)* — Paste text across machines seamlessly
- **🔍 Local Discovery** *(roadmap)* — Auto-detect InputSync servers via mDNS

---

## Screenshots

```
┌─────────────────────────────────────────────────────┐
│  [>_] INPUTSYNC                              [IDLE] │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ⬡ HOST / SERVER                                    │
│  ┌───────────────────────────────────────────────┐  │
│  │            START SERVER                       │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
│  ─────────────────────────────────────────────────  │
│                                                     │
│  ◈ CONNECT TO SERVER                               │
│  Session Code  [ A B C 1 2 3 ]                     │
│  Server IP     [ 192.168.1.x  ]                    │
│  ┌───────────────────────────────────────────────┐  │
│  │                CONNECT                        │  │
│  └───────────────────────────────────────────────┘  │
│                                                     │
├─────────────────────────────────────────────────────┤
│  ● Ready — Start server or connect to one     v1.1  │
└─────────────────────────────────────────────────────┘
```

---

## Installation

### Linux (Ubuntu / Debian)

Download the `inputsync` binary from [Releases](https://github.com/KaiCreates/InputSync/releases/latest):

```bash
# Download the latest Linux binary
wget https://github.com/KaiCreates/InputSync/releases/latest/download/inputsync

# Make executable and run
chmod +x inputsync
./inputsync
```

**Required system libraries** (usually already present on most Linux desktops):
```
libgtk-3-0  libx11-6  libxtst6  libxdo3
```

### Linux (Fedora / RHEL / Arch)
```bash
# Build from source — see below
# RPM and AUR packages are on the roadmap
```

### Windows

1. Download `inputsync.exe` from [Releases](https://github.com/KaiCreates/InputSync/releases/latest)
2. Run it directly — no installer, no runtime dependencies required
3. Windows Defender may prompt on first launch; click **More info → Run anyway**

```powershell
# winget (coming soon)
winget install KaiCreates.InputSync
```

---

## Quick Start

### 1. Start the server (the machine with your keyboard/mouse)

1. Open InputSync
2. Click **Start Server**
3. Note the **6-character session code** and your **IP address**

```
Session Code:  ABC123
Address:       192.168.1.42:24800
```

### 2. Connect from the client (the machine to be controlled)

1. Open InputSync on the second machine
2. Enter the session code and server IP
3. Click **Connect**

### 3. Start controlling

Move your cursor to the **screen edge** — InputSync automatically forwards control to the connected client. Move it back to the server's edge to return. You can also toggle capture manually from the **Main** tab.

---

## How It Works

```
┌─────────────────────────────────────────────────────────┐
│                    YOUR NETWORK                         │
│                                                         │
│  ┌──────────────┐   TCP :24800   ┌──────────────────┐  │
│  │    SERVER    │◄──────────────►│    CLIENT        │  │
│  │  (Host PC)   │  Key Exchange  │ (Controlled PC)  │  │
│  │              │                │                  │  │
│  │  Captures    │   UDP :24801   │  Simulates       │  │
│  │  keyboard +  │───────────────►│  keyboard +      │  │
│  │  mouse       │  Encrypted     │  mouse           │  │
│  │              │  Input Events  │                  │  │
│  └──────────────┘                └──────────────────┘  │
└─────────────────────────────────────────────────────────┘

Session Flow:
  1. Server starts → generates random 6-char code (e.g. ABC123)
  2. Client connects via TCP → sends session code
  3. X25519 ECDH key exchange → derive ChaCha20-Poly1305 session key
  4. Nonce exchange (server + client nonces XOR'd — prevents replay)
  5. Session active → UDP stream of encrypted input events
  6. Session ends → code invalidates; fresh code on next start
```

**Port usage:**

| Port | Protocol | Purpose |
|------|----------|---------|
| 24800 | TCP | Control channel (handshake, key exchange) |
| 24801 | UDP | Server → Client input event stream |
| 24802 | UDP | Client receive socket |

---

## Security

InputSync was designed with security as a first-class concern.

| Component | Implementation |
|-----------|---------------|
| Key Exchange | X25519 ECDH (Curve25519) via `x25519-dalek` |
| Encryption | ChaCha20-Poly1305 AEAD via `chacha20poly1305` |
| Key Derivation | HKDF-SHA256 with session code as salt |
| Nonce Strategy | Per-packet counter XOR'd with combined server+client nonces |
| Session Binding | Code + IP pair; codes are single-use per server start |
| Data at Rest | No keys stored; fresh exchange every session |
| Optional Transport | TLS (self-signed TOFU) via `rustls` + `rcgen` |

**What is protected:** All keyboard and mouse events, including keystrokes, are encrypted before leaving your machine. An attacker on the same network cannot read your input or replay captured packets.

**Threat model limitations:** InputSync does not protect against a compromised machine acting as a legitimate server. Verify session codes out-of-band (in person or via secure message) when on untrusted networks.

---

## InputSync vs. Alternatives

| Feature | InputSync | Barrier | InputLeap | Synergy |
|---------|-----------|---------|-----------|---------|
| Open Source | ✅ MIT | ✅ GPL | ✅ GPL | ❌ Partial |
| Encryption | ✅ ChaCha20 | ⚠️ TLS optional | ⚠️ TLS optional | ✅ Paid tier |
| Session Codes | ✅ Built-in | ❌ Manual IP | ❌ Manual IP | ❌ Manual |
| Screen Edge Switch | ✅ Built-in | ✅ | ✅ | ✅ |
| Native Binary | ✅ Pure Rust | ❌ C++ | ❌ C++ | ❌ Large |
| No Browser Engine | ✅ egui | ❌ | ❌ | ❌ |
| Binary Size | ✅ ~9 MB | ❌ ~50 MB | ❌ ~50 MB | ❌ Large |
| Modern Codebase | ✅ Rust 2021 | ❌ Legacy C++ | ❌ Legacy C++ | ❌ Legacy |
| Linux Wayland | 🔜 Roadmap | ⚠️ Partial | ⚠️ Partial | ❌ No |

---

## System Requirements

### Linux
- Ubuntu 22.04+ / Debian 12+ / equivalent
- X11 display server (Wayland support on roadmap)
- 64-bit processor
- `libxtst6`, `libx11-6`, `libgtk-3-0`

### Windows
- Windows 10 (build 1903) or Windows 11
- 64-bit processor
- ~15 MB disk space
- No runtime dependencies — single portable `.exe`

### Network
- Both machines on the same local network (LAN/Wi-Fi)
- Firewall must allow TCP :24800 and UDP :24801–24802

---

## Building from Source

### Prerequisites
```bash
# Rust (stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Linux build dependencies
sudo apt install libgtk-3-dev libx11-dev libxtst-dev libxdo-dev
```

### Build

```bash
git clone https://github.com/KaiCreates/InputSync
cd InputSync/src-tauri

# Build release binary (~35 seconds, ~9 MB output)
cargo build --release

# Output: target/release/inputsync
./target/release/inputsync
```

### Cross-compile for Windows
Windows builds are handled by the GitHub Actions workflow (`.github/workflows/build.yml`) using `cross` or the Windows runner. Building locally requires a Windows host or MinGW toolchain.

---

## Troubleshooting

**Input not being captured on Linux**

> `rdev::listen` requires access to `/dev/input`. On some systems you may need to add your user to the `input` group:
> ```bash
> sudo usermod -aG input $USER
> # Then log out and back in
> ```

**"Server rejected connection" on the client**

> Double-check the session code (case-sensitive, uppercase letters and digits only). The code resets every time the server is restarted.

**Firewall blocking connections**

> On Linux:
> ```bash
> sudo ufw allow 24800/tcp
> sudo ufw allow 24801/udp
> sudo ufw allow 24802/udp
> ```
> On Windows, allow InputSync through Windows Defender Firewall when prompted.

**High input latency**

> Ensure both machines are on a wired or high-quality Wi-Fi connection. UDP packets may be delayed on congested networks. Check for VPN software that might be wrapping UDP traffic in TCP.

**Windows Defender flags the binary**

> InputSync is not code-signed yet. Click **More info → Run anyway** in the SmartScreen prompt. Code signing is on the roadmap for v1.2.

**Edge trigger fires accidentally**

> Configure dead corners or dead zones in the **Settings** tab to block edge triggers in specific screen regions.

---

## Roadmap

| Milestone | Status |
|-----------|--------|
| Core encryption + session system | ✅ v1.0.0 |
| Input capture (Linux X11 + Windows) | ✅ v1.0.0 |
| Input simulation (Linux + Windows) | ✅ v1.0.0 |
| Native egui UI (server + client panels, settings, logs) | ✅ v1.1.0 |
| Screen edge switching | ✅ v1.1.0 |
| Dead corners + dead zones | ✅ v1.1.0 |
| Optional TLS transport (TOFU) | ✅ v1.1.0 |
| Settings persistence (~/.local/share/inputsync) | ✅ v1.1.0 |
| Clipboard sync | 🔜 v1.2 |
| Linux Wayland support (libei) | 🔜 v1.2 |
| mDNS/Bonjour server discovery | 🔜 v1.2 |
| Configurable switch hotkey | 🔜 v1.2 |
| Multi-monitor layout configuration | 🔜 v1.3 |
| macOS support | 🔜 v1.3 |
| Multi-client support | 🔜 v1.3 |
| Code signing (Windows + Linux) | 🔜 v1.2 |
| RPM / Arch packages | 🔜 v1.2 |

---

## Credits

InputSync is built on these excellent open-source libraries:

| Library | Purpose |
|---------|---------|
| [eframe / egui](https://github.com/emilk/egui) | Native cross-platform GUI framework |
| [tokio](https://tokio.rs) | Async Rust runtime |
| [x25519-dalek](https://github.com/dalek-cryptography/x25519-dalek) | X25519 ECDH |
| [chacha20poly1305](https://github.com/RustCrypto/AEADs) | ChaCha20-Poly1305 AEAD |
| [rustls](https://github.com/rustls/rustls) + [rcgen](https://github.com/rustls/rcgen) | Optional TLS |
| [enigo](https://github.com/enigo-rs/enigo) | Cross-platform input simulation |
| [rdev](https://github.com/Narsil/rdev) | Cross-platform input capture |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, code style, and PR guidelines.

## Security

Found a vulnerability? See [SECURITY.md](SECURITY.md) for responsible disclosure.

## License

InputSync is released under the [MIT License](LICENSE).

Copyright © 2026 KaiCreates
