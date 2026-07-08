#!/usr/bin/env bash
# run-lint.sh — the CANONICAL way to lint signalwire-rust (rust: clippy).
#
# Do NOT call `cargo clippy` directly anymore — call this. It self-bootstraps the
# toolchain and resolves the repo from its OWN path, so it works from ANY CWD.
#
# Default: `cargo clippy --all-targets --all-features -- -D warnings` — lib + bins
# + tests + examples; Cargo.toml [lints.clippy] denies all+pedantic and -D warnings
# promotes rustc warnings, so any finding is an error (exit non-zero).
#
# --fix: `cargo clippy --fix` — apply clippy's machine-applicable suggestions to
#        the working tree (then still report). Requires a clean/committed tree
#        unless --allow-dirty is understood; we pass --allow-dirty --allow-staged
#        so it works in a dev loop.

source "$(dirname "${BASH_SOURCE[0]}")/_env.sh"

sw_ensure_components

if [ "${1:-}" = "--fix" ]; then
    echo "==> LINT (fix): cargo clippy --fix --all-targets   [$REPO]"
    "${CARGO[@]}" clippy --fix --all-targets --all-features \
        --allow-dirty --allow-staged -- -D warnings
elif [ -n "${1:-}" ]; then
    echo "usage: run-lint.sh [--fix]" >&2
    exit 2
else
    echo "==> LINT: cargo clippy --all-targets   [$REPO]"
    "${CARGO[@]}" clippy --all-targets --all-features -- -D warnings
fi
