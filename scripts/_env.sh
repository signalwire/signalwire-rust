#!/usr/bin/env bash
# _env.sh — shared bootstrap for the canonical run-format / run-lint / run-tests
# scripts (and sourced by run-ci.sh's FMT/LINT/TEST gates).
#
# Requirement 0 (SCRIPT-RELATIVE PATHS): resolve the repo root from THIS file's
# own location, never from $PWD, so every caller operates on the code under this
# repo regardless of the working directory it was invoked from.
#
# Sourcing contract: a caller does
#     source "$(dirname "${BASH_SOURCE[0]}")/_env.sh"
# as its FIRST real line. After sourcing, $REPO is the repo root and the CWD is
# $REPO. The caller must have already set `set -euo pipefail` OR rely on this
# file to set it (we set it here too, so a bare `bash scripts/run-*.sh` is safe).

set -euo pipefail

# Resolve the directory THIS file lives in (scripts/), following one level of
# symlink on ${BASH_SOURCE[0]} if present, then the repo root above it.
_ENV_SRC="${BASH_SOURCE[0]}"
if [ -L "$_ENV_SRC" ]; then
    _ENV_SRC="$(readlink "$_ENV_SRC")"
fi
SCRIPT_DIR="$(cd "$(dirname "$_ENV_SRC")" && pwd)"
REPO="$(dirname "$SCRIPT_DIR")"
cd "$REPO"

# --- Rust toolchain self-bootstrap (CWD-independent) -------------------------
# Ensure cargo/rustup resolve no matter the caller's shell setup. Homebrew's
# rustup keeps the shims under its opt prefix; the standard rustup install keeps
# them under ~/.cargo/bin. Add both to PATH if present so a fresh shell works.
if [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi
if [ -d "/opt/homebrew/opt/rustup/bin" ]; then
    export PATH="/opt/homebrew/opt/rustup/bin:$PATH"
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "FATAL: cargo not found on PATH." >&2
    echo "       Install the Rust toolchain: https://rustup.rs" >&2
    echo "       (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)" >&2
    exit 1
fi

# Ensure the rustfmt + clippy components are present on the stable toolchain we
# invoke (the scripts pin +stable — see below). rustup makes this idempotent;
# add them only when missing so a warm machine pays nothing.
sw_ensure_components() {
    if ! command -v rustup >/dev/null 2>&1; then
        # No rustup (e.g. a distro rustc): trust cargo has fmt/clippy; the tool
        # invocation itself will fail loud below if not.
        return 0
    fi
    local missing=""
    if ! rustup component list --toolchain stable --installed 2>/dev/null | grep -q '^rustfmt'; then
        missing="$missing rustfmt"
    fi
    if ! rustup component list --toolchain stable --installed 2>/dev/null | grep -q '^clippy'; then
        missing="$missing clippy"
    fi
    if [ -n "$missing" ]; then
        echo "==> installing missing stable components:$missing" >&2
        rustup component add --toolchain stable $missing >&2 || {
            echo "FATAL: could not install rustfmt/clippy via rustup." >&2
            echo "       Try: rustup component add --toolchain stable rustfmt clippy" >&2
            exit 1
        }
    fi
}

# CARGO invocation prefix. We pin +stable because the SIGNATURES gate installs a
# nightly toolchain (for rustdoc-json) which can become the default and may lack
# rustfmt/clippy; +stable is robust. Callers use "${CARGO[@]}".
if command -v rustup >/dev/null 2>&1; then
    CARGO=(cargo +stable)
else
    CARGO=(cargo)
fi
