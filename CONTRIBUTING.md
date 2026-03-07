# Contributing to InputSync

Thank you for considering a contribution. This document explains how to get
started, coding standards, and how to submit work.

---

## Reporting Bugs

1. Search [existing issues](https://github.com/KaiCreates/InputSync/issues) first.
2. If none match, open a **Bug Report** using the issue template.
3. Include:
   - OS and version (e.g. Ubuntu 24.04, Windows 11)
   - InputSync version
   - Steps to reproduce (exact)
   - Expected vs. actual behaviour
   - Relevant logs (run with `RUST_LOG=debug` for verbose output)

## Requesting Features

1. Open a **Feature Request** issue.
2. Describe the use case, not just the implementation.
3. Tag with `enhancement`.

---

## Development Setup

### Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust | stable ≥ 1.77 | `rustup.rs` |
| Node.js | ≥ 18 LTS | `nodejs.org` |
| Tauri CLI | 2.x | `cargo install tauri-cli --version "^2" --locked` |

**Linux extra dependencies:**
```bash
sudo apt install \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  libx11-dev libxtst-dev build-essential
```

### Clone and Run

```bash
git clone https://github.com/KaiCreates/InputSync
cd InputSync

npm install

# Hot-reload dev mode (frontend + Tauri)
cargo tauri dev

# Build release
./build.sh linux    # .deb
./build.sh windows  # .exe (needs MinGW: sudo apt install gcc-mingw-w64-x86-64)
```

---

## Project Structure

```
InputSync/
├── src/                      # React/TypeScript frontend
│   ├── App.tsx               # Root component + status polling
│   ├── components/
│   │   ├── PixelLogo.tsx     # SVG pixel-art logo
│   │   ├── ServerPanel.tsx   # Host/Server UI
│   │   ├── ClientPanel.tsx   # Client connect UI
│   │   └── StatusBar.tsx     # Bottom status bar
│   └── styles.css            # Global CSS variables
├── src-tauri/
│   └── src/
│       ├── core/
│       │   ├── crypto.rs     # X25519 + ChaCha20-Poly1305
│       │   ├── protocol.rs   # Binary packet format
│       │   └── session.rs    # Session code generation
│       ├── input/
│       │   ├── capture.rs    # rdev input capture
│       │   └── simulation.rs # enigo input simulation
│       ├── network/
│       │   ├── server.rs     # TCP listener + UDP broadcaster
│       │   └── client.rs     # TCP handshake + UDP receiver
│       ├── commands.rs       # Tauri command handlers
│       ├── state.rs          # Shared app state
│       └── lib.rs            # App entry, Tauri builder
└── .github/workflows/        # CI/CD
```

---

## Coding Standards

### Rust

- Format with `cargo fmt` before committing.
- Lint with `cargo clippy -- -D warnings` (no warnings allowed).
- All public functions must have doc comments (`///`).
- Prefer `anyhow::Result` for error returns from functions; `thiserror` for
  error types exposed to callers.
- No `unwrap()` or `expect()` in production paths — use `?` or explicit handling.

### TypeScript / React

- Format with Prettier (`.prettierrc` if added, otherwise default settings).
- `tsc --noEmit` must pass with zero errors.
- Functional components only; no class components.
- Props interfaces defined explicitly.
- No `any` types.

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <summary>

[optional body]
[optional footer]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`, `ci`

Examples:
```
feat(network): add counter-window UDP resync for packet loss recovery
fix(capture): use relative mouse movement to handle different screen resolutions
docs: add CONTRIBUTING.md and issue templates
```

---

## Submitting Pull Requests

1. Fork the repo and create a branch from `main`:
   ```bash
   git checkout -b fix/my-bug-description
   ```
2. Make your changes, following the coding standards above.
3. Run the full test suite:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt --check
   npx tsc --noEmit
   ```
4. Push and open a PR against `main`.
5. Fill in the PR template completely.
6. A maintainer will review within a few days.

### PR Requirements

- [ ] Tests pass (`cargo test`)
- [ ] No clippy warnings
- [ ] Frontend type-checks (`tsc --noEmit`)
- [ ] CHANGELOG.md updated under `[Unreleased]`
- [ ] Docs updated if behaviour changes

---

## Brand Colors

For UI consistency:

| Name | Hex | Usage |
|------|-----|-------|
| Accent | `#6c63ff` | Primary buttons, focus rings, logo |
| Success | `#3ecf8e` | Connected state, capture active |
| Danger | `#f04444` | Disconnect button, errors |
| Warning | `#f59e0b` | Warnings |
| Background | `#0f1117` | App background |
| Surface | `#1a1d27` | Cards, header, status bar |
| Border | `#2d3250` | Dividers, input borders |
| Text | `#e8eaf6` | Primary text |
| Muted | `#7c84a6` | Labels, secondary text |

---

## Security Contributions

Please do **not** open public issues for security vulnerabilities.
See [SECURITY.md](SECURITY.md) for the responsible disclosure process.
