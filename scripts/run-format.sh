#!/usr/bin/env bash
# run-format.sh — the CANONICAL way to format signalwire-rust (rust: rustfmt).
#
# Do NOT call `cargo fmt` directly anymore — call this. It self-bootstraps the
# toolchain and resolves the repo from its OWN path, so it works from ANY CWD.
#
# Modes:
#   (default)  APPLY   — `cargo fmt --all`: reformat the tree in place, exit 0
#                        even if it changed files (local dev convenience).
#   --check    VERIFY  — `cargo fmt --all -- --check`: read-only, exit non-zero
#                        if anything is unformatted (the CI FMT gate).
#
# Formats BOTH hand-written and generated code — the generated tree is emitted
# formatter-clean by construction, so --check stays green.

source "$(dirname "${BASH_SOURCE[0]}")/_env.sh"

sw_ensure_components

MODE="apply"
if [ "${1:-}" = "--check" ]; then
    MODE="check"
elif [ -n "${1:-}" ]; then
    echo "usage: run-format.sh [--check]" >&2
    exit 2
fi

if [ "$MODE" = "check" ]; then
    echo "==> FMT (check): cargo fmt --all -- --check   [$REPO]"
    "${CARGO[@]}" fmt --all -- --check
else
    echo "==> FMT (apply): cargo fmt --all   [$REPO]"
    "${CARGO[@]}" fmt --all
    # fmt exits 0 whether or not it rewrote files; surface any change so the dev
    # knows to `git add` it.
    if ! git diff --quiet 2>/dev/null; then
        echo "    (FMT auto-applied formatting to your working tree — review & stage)"
    fi
fi
