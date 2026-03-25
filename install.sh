#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────
#  Navi Installer
#  Installs Navi + ast-grep with all prerequisites
# ─────────────────────────────────────────────

NAVI_REPO="https://github.com/keanji-x/Navi.git"

# Colors (disabled if not a terminal)
if [ -t 1 ]; then
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  RED='\033[0;31m'
  CYAN='\033[0;36m'
  BOLD='\033[1m'
  NC='\033[0m'
else
  GREEN='' YELLOW='' RED='' CYAN='' BOLD='' NC=''
fi

info()  { echo -e "${CYAN}[INFO]${NC}  $*"; }
ok()    { echo -e "${GREEN}[OK]${NC}    $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC}  $*"; }
fail()  { echo -e "${RED}[FAIL]${NC}  $*"; exit 1; }

# ── Step 1: Check / Install Rust toolchain ───

install_rust() {
  info "Installing Rust toolchain via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
  ok "Rust installed: $(rustc --version)"
}

ensure_cargo() {
  if command -v cargo &>/dev/null; then
    ok "cargo found: $(cargo --version)"
  else
    if command -v rustup &>/dev/null; then
      warn "rustup found but cargo not in PATH, sourcing env..."
      source "$HOME/.cargo/env" 2>/dev/null || true
      if command -v cargo &>/dev/null; then
        ok "cargo found: $(cargo --version)"
      else
        install_rust
      fi
    else
      install_rust
    fi
  fi
}

# ── Step 2: Install / Update Navi ────────────

install_navi() {
  if command -v navi &>/dev/null; then
    local current
    current=$(navi --help 2>/dev/null | head -1 || echo "")
    info "Navi already installed, updating to latest..."
    cargo install --git "$NAVI_REPO" --force
  else
    info "Installing Navi from source..."
    cargo install --git "$NAVI_REPO"
  fi
  ok "Navi installed: $(navi --help 2>/dev/null | head -1)"
}

# ── Step 3: Install / Update ast-grep ────────

install_ast_grep() {
  if command -v ast-grep &>/dev/null; then
    local current
    current=$(ast-grep --version 2>/dev/null || echo "unknown")
    info "ast-grep already installed ($current), checking for updates..."
    cargo install ast-grep 2>&1 | tail -3
  else
    info "Installing ast-grep..."
    cargo install ast-grep
  fi
  ok "ast-grep installed: $(ast-grep --version 2>/dev/null)"
}

# ── Step 4: Verify ───────────────────────────

verify() {
  echo ""
  echo -e "${BOLD}── Verification ──${NC}"
  local all_ok=true

  for cmd in cargo navi ast-grep; do
    if command -v "$cmd" &>/dev/null; then
      ok "$cmd  →  $(command -v "$cmd")"
    else
      warn "$cmd not found in PATH"
      all_ok=false
    fi
  done

  echo ""
  if [ "$all_ok" = true ]; then
    echo -e "${GREEN}${BOLD}✅ All set! Run 'navi init' in your project to get started.${NC}"
  else
    echo -e "${YELLOW}${BOLD}⚠  Some tools missing. You may need to restart your shell or run:${NC}"
    echo '   source "$HOME/.cargo/env"'
  fi
}

# ── Main ─────────────────────────────────────

echo ""
echo -e "${BOLD}🧭 Navi Installer${NC}"
echo "─────────────────────────────────────"
echo ""

ensure_cargo
echo ""
install_navi
echo ""
install_ast_grep
echo ""
verify
