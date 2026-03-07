#!/usr/bin/env bash
set -euo pipefail

# InputSync Build Script
# Produces: Linux .deb and Windows .exe installer

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${BLUE}[InputSync]${NC} $*"; }
ok()  { echo -e "${GREEN}[OK]${NC} $*"; }
warn(){ echo -e "${YELLOW}[WARN]${NC} $*"; }
fail(){ echo -e "${RED}[FAIL]${NC} $*"; exit 1; }

# ── Dependency checks ──────────────────────────────────────────────────────────
check_deps() {
    log "Checking build dependencies..."

    command -v cargo >/dev/null 2>&1 || fail "Rust/cargo not found. Install from https://rustup.rs"
    command -v npm   >/dev/null 2>&1 || fail "npm not found. Install Node.js LTS"
    command -v cargo-tauri >/dev/null 2>&1 || \
        cargo install tauri-cli --version "^2" --locked

    # Linux build deps
    if [[ "$OSTYPE" == "linux-gnu"* ]]; then
        for pkg in libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
                   libx11-dev libxtst-dev dpkg; do
            dpkg -l "$pkg" &>/dev/null || warn "Missing: $pkg (run: sudo apt install $pkg)"
        done
    fi
}

# ── Frontend ───────────────────────────────────────────────────────────────────
build_frontend() {
    log "Installing npm dependencies..."
    npm install --silent

    log "Building frontend..."
    npm run build
    ok "Frontend built → dist/"
}

# ── Linux .deb ─────────────────────────────────────────────────────────────────
build_linux() {
    log "Building Linux (x86_64) .deb package..."
    rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true

    cargo tauri build --target x86_64-unknown-linux-gnu --bundles deb

    DEB_PATH=$(find src-tauri/target/x86_64-unknown-linux-gnu/release/bundle/deb -name "*.deb" 2>/dev/null | head -1)
    if [[ -n "$DEB_PATH" ]]; then
        cp "$DEB_PATH" "InputSync-linux-x64.deb"
        ok "Linux .deb → $(pwd)/InputSync-linux-x64.deb"
    else
        warn "Could not locate .deb output"
    fi
}

# ── Windows .exe ───────────────────────────────────────────────────────────────
build_windows() {
    log "Building Windows (x86_64) NSIS installer..."

    # Check for MinGW cross-compiler
    if ! command -v x86_64-w64-mingw32-gcc >/dev/null 2>&1; then
        warn "MinGW not found. Install with: sudo apt install gcc-mingw-w64-x86-64"
        warn "Skipping Windows build."
        return
    fi

    rustup target add x86_64-pc-windows-gnu 2>/dev/null || true

    # Windows build requires specific env vars for WebView2
    TAURI_WINDOWS_WEBVIEW2_PATH="" \
    CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc \
    cargo tauri build --target x86_64-pc-windows-gnu --bundles nsis

    EXE_PATH=$(find src-tauri/target/x86_64-pc-windows-gnu/release/bundle/nsis -name "*.exe" 2>/dev/null | head -1)
    if [[ -n "$EXE_PATH" ]]; then
        cp "$EXE_PATH" "InputSync-windows-x64-setup.exe"
        ok "Windows .exe → $(pwd)/InputSync-windows-x64-setup.exe"
    else
        warn "Could not locate Windows .exe output"
    fi
}

# ── Main ───────────────────────────────────────────────────────────────────────
main() {
    TARGET="${1:-all}"

    echo ""
    echo "  ╔══════════════════════════════╗"
    echo "  ║   InputSync Build System     ║"
    echo "  ╚══════════════════════════════╝"
    echo ""

    check_deps
    build_frontend

    case "$TARGET" in
        linux)   build_linux ;;
        windows) build_windows ;;
        all)
            build_linux
            build_windows
            ;;
        *)
            fail "Unknown target: $TARGET (use: all | linux | windows)"
            ;;
    esac

    echo ""
    log "Build complete. Output files:"
    ls -lh InputSync-*.deb InputSync-*.exe 2>/dev/null || true
    echo ""
}

main "$@"
