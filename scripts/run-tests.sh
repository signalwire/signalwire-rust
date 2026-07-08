#!/usr/bin/env bash
# run-tests.sh — the CANONICAL way to run the signalwire-rust test suite.
#
# Do NOT call `cargo test` directly anymore — call this. It self-bootstraps the
# toolchain and resolves the repo from its OWN path, so it works from ANY CWD.
#
# Default: `cargo test --tests` — the same invocation run-ci's TEST gate uses.
#   The mock-backed suites are session-isolated (relay: per-connection handshake
#   sessionid; rest: per-test random project => unique Authorization header), so
#   the shared mock is safe under cargo's default parallelism without
#   --test-threads=1.
#   (NOTE: there is a known bare-parallel mock-spawn port race across test
#   BINARIES — the real fix is free-port-per-binary in the harness, tracked
#   separately. This script runs `cargo test` the way run-ci already does; it
#   does NOT attempt to work around that here.)
#
# Optional filter: `run-tests.sh <filter>` passes the filter through as a cargo
#   test name/substring (e.g. `run-tests.sh logging`, `run-tests.sh test_logger_creation`).

source "$(dirname "${BASH_SOURCE[0]}")/_env.sh"

FILTER="${1:-}"
if [ -n "$FILTER" ]; then
    echo "==> TEST: cargo test --tests $FILTER   [$REPO]"
    "${CARGO[@]}" test --tests "$FILTER"
else
    echo "==> TEST: cargo test --tests   [$REPO]"
    "${CARGO[@]}" test --tests
fi
