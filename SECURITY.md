# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes       |

Only the latest release receives security patches. Users are strongly encouraged to stay on the current release.

---

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Please report security issues privately by emailing:

> **security-inputsync@proton.me** *(monitored by KaiCreates)*

Or use [GitHub's private vulnerability reporting](https://github.com/KaiCreates/InputSync/security/advisories/new).

Include in your report:
- A description of the vulnerability and its potential impact
- Steps to reproduce (proof-of-concept if available)
- Affected version(s)
- Any suggested mitigations

You will receive an acknowledgement within **48 hours** and a resolution timeline within **7 days**.

---

## Security Architecture

### Key Exchange
InputSync uses **X25519 Elliptic-Curve Diffie-Hellman** (via the `x25519-dalek` crate) to establish a shared secret between server and client. Keypairs are **ephemeral** — generated fresh for every session and never written to disk.

### Key Derivation
The raw ECDH shared secret is passed through **HKDF-SHA256** with the session code as the salt. This:
- Stretches the shared secret to a 256-bit symmetric key
- Binds the key to the session code (preventing cross-session reuse)
- Conforms to RFC 5869

### Payload Encryption
All input event packets are encrypted with **ChaCha20-Poly1305** (IETF variant, via the `chacha20poly1305` crate):
- 256-bit key derived via HKDF
- Per-packet nonce: `base_nonce XOR packet_counter` (prevents nonce reuse)
- 128-bit authentication tag on every packet (prevents tampering and replay)

### Session Binding
The session code appears in the HKDF salt, meaning a derived key from session `ABC123` cannot decrypt packets encrypted under session `XYZ789`. A client with a stolen key cannot silently switch to a different server.

### No Persistence
- No keys, session codes, or cryptographic material are stored on disk
- Every application restart generates a new session code and keypair
- There is no "remember this server" feature that could leak credentials

### Network Exposure
- TCP port **24800**: control/handshake only (closed after session establishment)
- UDP port **24801**: encrypted input events (server → client only)
- UDP port **24802**: client receive socket (local bind only)
- No internet connectivity required or initiated; designed for LAN use

### Known Limitations
- No certificate pinning (the session code acts as a shared secret for authentication)
- No protection against a malicious actor on the local network who can MitM before the handshake completes — use only on trusted networks
- Wayland capture (planned) may require elevated permissions depending on compositor

---

## Bug Bounty

There is currently no formal bug bounty program. Responsible disclosure is appreciated and contributors will be credited in the release notes.
