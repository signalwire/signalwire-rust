#!/usr/bin/env bash
# run-pylint.sh — the CANONICAL Python linter/formatter for signalwire-rust (ruff).
#
# This is a Rust SDK, and the Rust tree has always been covered end-to-end by
# `cargo clippy --all-targets` (lib + bins + tests + examples) at full
# strictness. But the repo also ships 7 hand-written Python programs under
# scripts/ (~8.2k lines), and until 2026-07-30 NO gate linted or formatted a
# single line of them. Five of the seven emit artifacts CI reads as ground
# truth: port_surface.json (SURFACE-FRESH), port_signatures.json
# (SIGNATURES/DRIFT), and the generated REST/SWML/SWAIG source trees
# (GEN-FRESH*). That was the highest-stakes code in the repo held to no bar.
#
# Modes (mirroring scripts/run-format.sh so the two behave identically):
#   (default)  APPLY  — `ruff check --fix` + `ruff format`: fix in place.
#   --check    VERIFY — `ruff check` + `ruff format --check`: read-only, exits
#                       non-zero on any finding. This is the CI mode.
#
# Config lives in ruff.toml at the repo root and MIRRORS the reference
# implementation's rule selection (signalwire-python/pyproject.toml).

source "$(dirname "${BASH_SOURCE[0]}")/_env.sh"

cd "$REPO" || exit 1

# --config is PINNED, never left to auto-discovery. ruff resolves config by
# walking UP from the TARGET path, so the file it picks depends on the CWD and
# on how the target is spelled. Measured on this repo: running `ruff check
# scripts/` from a foreign CWD reported 69 findings where the pinned config
# reports 76 — the per-file-ignores anchored to the wrong root and 7 findings
# silently changed status. A gate whose ruleset depends on where it was invoked
# from is not a gate.
CFG="$REPO/ruff.toml"

if ! command -v ruff >/dev/null 2>&1; then
    echo "ERROR: ruff not found on PATH." >&2
    echo "       It lints + formats the 7 hand-written Python programs under scripts/," >&2
    echo "       five of which produce artifacts the CI gates read as ground truth." >&2
    echo "       Install it with:  pip install ruff   (or: brew install ruff)" >&2
    exit 1
fi

if [ ! -f "$CFG" ]; then
    echo "ERROR: $CFG missing — refusing to lint against ruff's built-in defaults." >&2
    exit 1
fi

# Fail loud rather than silently passing on an empty file set — a gate that
# checks nothing is worse than no gate.
if ! find scripts -name '*.py' -print -quit | grep -q .; then
    echo "ERROR: no Python sources found under scripts/ to lint." >&2
    exit 1
fi

if [ "${1:-}" = "--check" ]; then
    echo "==> PY-LINT (check): ruff check + ruff format --check   [$REPO]"
    ruff check --config "$CFG" scripts/ || exit 1
    ruff format --config "$CFG" --check scripts/ || exit 1
elif [ -n "${1:-}" ]; then
    echo "usage: run-pylint.sh [--check]" >&2
    exit 2
else
    echo "==> PY-LINT (apply): ruff check --fix + ruff format   [$REPO]"
    ruff check --config "$CFG" --fix scripts/ || exit 1
    ruff format --config "$CFG" scripts/ || exit 1
    # A residual finding --fix cannot resolve must still fail the gate, and the
    # formatter must be a no-op on its own output.
    ruff check --config "$CFG" scripts/ || exit 1
    ruff format --config "$CFG" --check scripts/ || exit 1
fi
