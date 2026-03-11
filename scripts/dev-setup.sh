#!/usr/bin/env bash
#
# Zapcode dev setup — Sets up a contributor development environment.
#
# Usage:
#   ./scripts/dev-setup.sh
#
# What it does:
#   1. Ensures Rust toolchain with rustfmt + clippy
#   2. Installs wasm-pack and maturin (if missing)
#   3. Installs Node.js dependencies for JS bindings
#   4. Builds all crates
#   5. Runs tests to verify everything works

set -euo pipefail

# ── Colors ──────────────────────────────────────────────────────────
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${BLUE}>${NC} $*"; }
ok()    { echo -e "${GREEN}✓${NC} $*"; }
fail()  { echo -e "${RED}✗${NC} $*"; exit 1; }

check_cmd() { command -v "$1" &>/dev/null; }

echo ""
echo -e "${BOLD}Zapcode dev setup${NC}"
echo ""

# ── Rust toolchain ──────────────────────────────────────────────────
if check_cmd rustc; then
    ok "Rust $(rustc --version | cut -d' ' -f2)"
else
    info "Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --quiet
    source "$HOME/.cargo/env"
    ok "Rust installed"
fi

info "Ensuring rustfmt and clippy..."
rustup component add rustfmt clippy --quiet 2>/dev/null
ok "rustfmt + clippy"

# ── wasm-pack ───────────────────────────────────────────────────────
if check_cmd wasm-pack; then
    ok "wasm-pack found"
else
    info "Installing wasm-pack..."
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    ok "wasm-pack installed"
fi

# ── WASM target ─────────────────────────────────────────────────────
info "Ensuring wasm32 target..."
rustup target add wasm32-unknown-unknown --quiet 2>/dev/null
ok "wasm32-unknown-unknown target"

# ── Node.js ─────────────────────────────────────────────────────────
if check_cmd node; then
    ok "Node.js $(node --version)"
else
    fail "Node.js is required. Install it from https://nodejs.org"
fi

info "Installing JS binding dependencies..."
(cd crates/zapcode-js && npm install --quiet)
ok "JS dependencies"

# ── Python + maturin ────────────────────────────────────────────────
if check_cmd python3 || check_cmd python; then
    PYTHON=$(check_cmd python3 && echo python3 || echo python)
    ok "Python $(${PYTHON} --version | cut -d' ' -f2)"

    if check_cmd maturin; then
        ok "maturin found"
    else
        info "Installing maturin..."
        if check_cmd uv; then
            uv tool install maturin --quiet
        elif check_cmd pip3; then
            pip3 install maturin --quiet
        elif check_cmd pip; then
            pip install maturin --quiet
        fi
        ok "maturin installed"
    fi
else
    echo -e "${BLUE}>${NC} Python not found — skipping Python bindings setup"
fi

# ── Build ───────────────────────────────────────────────────────────
echo ""
info "Building all crates..."
cargo build --workspace 2>&1 | tail -1
ok "Build complete"

# ── Tests ───────────────────────────────────────────────────────────
info "Running core tests..."
cargo test -p zapcode-core 2>&1 | tail -1
ok "Tests passed"

# ── Lint check ──────────────────────────────────────────────────────
info "Running clippy..."
cargo clippy --all-targets -- -D warnings 2>&1 | tail -1
ok "Clippy clean"

info "Checking formatting..."
cargo fmt -- --check 2>&1 || fail "Run 'cargo fmt' to fix formatting"
ok "Formatting clean"

# ── Done ────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}${BOLD}Dev environment ready!${NC}"
echo ""
echo "  cargo test              — run tests"
echo "  cargo bench             — run benchmarks"
echo "  cargo clippy            — lint"
echo "  cargo fmt               — format"
echo ""
