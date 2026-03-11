#!/usr/bin/env bash
#
# Zapcode installer — Run AI code. Safely. Instantly.
#
# Detects your project type and installs Zapcode from the appropriate registry.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/TheUncharted/zapcode/master/install.sh | bash
#
#   Or with options:
#   curl -fsSL https://raw.githubusercontent.com/TheUncharted/zapcode/master/install.sh | bash -s -- --lang ts
#   curl -fsSL https://raw.githubusercontent.com/TheUncharted/zapcode/master/install.sh | bash -s -- --lang python
#   curl -fsSL https://raw.githubusercontent.com/TheUncharted/zapcode/master/install.sh | bash -s -- --lang rust

set -euo pipefail

# ── Colors ──────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${BLUE}>${NC} $*"; }
ok()    { echo -e "${GREEN}✓${NC} $*"; }
warn()  { echo -e "${YELLOW}!${NC} $*"; }
fail()  { echo -e "${RED}✗${NC} $*"; exit 1; }

# ── Parse arguments ─────────────────────────────────────────────────
LANG=""
VERSION="${ZAPCODE_VERSION:-latest}"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --lang|-l)      LANG="$2"; shift 2 ;;
        --version|-v)   VERSION="$2"; shift 2 ;;
        --help|-h)
            echo "Zapcode installer"
            echo ""
            echo "Usage: install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --lang, -l <ts|python|rust>    Target language (default: auto-detect)"
            echo "  --version, -v <version>        Package version (default: latest)"
            echo "  --help, -h                     Show this help"
            exit 0
            ;;
        *) fail "Unknown option: $1" ;;
    esac
done

echo ""
echo -e "${BOLD}Zapcode installer${NC} — Run AI code. Safely. Instantly."
echo ""

# ── Auto-detect language from project files ─────────────────────────
if [[ -z "$LANG" ]]; then
    if [[ -f "package.json" ]]; then
        LANG="ts"
    elif [[ -f "pyproject.toml" ]] || [[ -f "setup.py" ]] || [[ -f "requirements.txt" ]]; then
        LANG="python"
    elif [[ -f "Cargo.toml" ]]; then
        LANG="rust"
    else
        info "Could not auto-detect project type."
        info "Run with --lang <ts|python|rust>"
        echo ""
        echo "  For TypeScript/JavaScript: install.sh --lang ts"
        echo "  For Python:                install.sh --lang python"
        echo "  For Rust:                  install.sh --lang rust"
        exit 1
    fi
    ok "Detected project type: ${LANG}"
fi

# ── Helpers ─────────────────────────────────────────────────────────
check_cmd() {
    command -v "$1" &>/dev/null
}

# ── Install based on language ───────────────────────────────────────
case "$LANG" in
    ts|js|typescript|javascript|node)
        if ! check_cmd node; then
            fail "Node.js is required. Install it from https://nodejs.org"
        fi
        ok "Node.js found ($(node --version))"

        # Detect package manager
        if check_cmd bun; then
            PM="bun"
        elif check_cmd pnpm; then
            PM="pnpm"
        elif check_cmd yarn; then
            PM="yarn"
        elif check_cmd npm; then
            PM="npm"
        else
            fail "No package manager found. Install npm, yarn, pnpm, or bun."
        fi
        ok "Using package manager: ${PM}"

        # Install from npm
        PKG="@unchartedfr/zapcode"
        if [[ "$VERSION" == "latest" ]]; then
            PKG="${PKG}@beta"
        else
            PKG="${PKG}@${VERSION}"
        fi

        info "Installing ${PKG}..."
        case "$PM" in
            npm)  npm install "$PKG" ;;
            yarn) yarn add "$PKG" ;;
            pnpm) pnpm add "$PKG" ;;
            bun)  bun add "$PKG" ;;
        esac
        ok "Installed ${PKG}"

        echo ""
        ok "Ready! Import in your code:"
        echo ""
        echo "  import { Zapcode } from '@unchartedfr/zapcode';"
        echo ""
        echo "  const z = new Zapcode('1 + 2 * 3');"
        echo "  const result = z.run();"
        echo "  console.log(result.output);  // 7"
        ;;

    python|py)
        if check_cmd python3; then
            PYTHON="python3"
        elif check_cmd python; then
            PYTHON="python"
        else
            fail "Python is required. Install it from https://python.org"
        fi
        ok "Python found ($(${PYTHON} --version))"

        # Detect package manager
        if check_cmd uv; then
            PY_PM="uv"
        elif check_cmd pip3; then
            PY_PM="pip3"
        elif check_cmd pip; then
            PY_PM="pip"
        else
            fail "No Python package manager found. Install uv (https://docs.astral.sh/uv/) or pip."
        fi
        ok "Using package manager: ${PY_PM}"

        # Install from PyPI
        PKG="zapcode"
        if [[ "$VERSION" != "latest" ]]; then
            PKG="${PKG}==${VERSION}"
        fi

        info "Installing ${PKG}..."
        case "$PY_PM" in
            uv)  uv pip install "$PKG" --prerelease=allow ;;
            *)   ${PY_PM} install "$PKG" --pre ;;
        esac
        ok "Installed ${PKG}"

        echo ""
        ok "Ready! Import in your code:"
        echo ""
        echo "  from zapcode import Zapcode"
        echo ""
        echo "  z = Zapcode('1 + 2 * 3')"
        echo "  result = z.run()"
        echo "  print(result['output'])  # 7"
        ;;

    rust|rs)
        if ! check_cmd cargo; then
            fail "Rust is required. Install it from https://rustup.rs"
        fi
        ok "Rust found ($(rustc --version | cut -d' ' -f2))"

        if [[ ! -f "Cargo.toml" ]]; then
            fail "No Cargo.toml found. Create a Rust project first: cargo init"
        fi

        # Add dependency
        if grep -q "zapcode-core" Cargo.toml 2>/dev/null; then
            ok "zapcode-core already in Cargo.toml"
        else
            if [[ "$VERSION" == "latest" ]]; then
                cargo add zapcode-core
            else
                cargo add "zapcode-core@${VERSION}"
            fi
            ok "Added zapcode-core to Cargo.toml"
        fi

        echo ""
        ok "Ready! Use in your code:"
        echo ""
        echo "  use zapcode_core::{ZapcodeRun, Value, ResourceLimits};"
        echo ""
        echo "  let runner = ZapcodeRun::new("
        echo "      \"1 + 2 * 3\".to_string(),"
        echo "      vec![], vec![], ResourceLimits::default()"
        echo "  )?;"
        echo "  let result = runner.run_simple()?;"
        echo "  assert_eq!(result, Value::Int(7));"
        ;;

    *)
        fail "Unknown language: $LANG. Use: ts, python, or rust"
        ;;
esac

echo ""
echo -e "${GREEN}${BOLD}Done!${NC} Zapcode is ready."
echo -e "Docs: ${BLUE}https://github.com/TheUncharted/zapcode${NC}"
