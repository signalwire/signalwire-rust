#!/usr/bin/env bash
# run-ci.sh — canonical local-and-CI gate runner for signalwire-rust.
#
# Same script invoked locally (`bash scripts/run-ci.sh`) AND by the
# GitHub Actions workflow. No drift between local and CI behavior.
#
# FMT / LINT / TEST entry points are the canonical scripts (callable standalone
# from any CWD, self-bootstrapping the toolchain):
#   scripts/run-format.sh   (rust: cargo fmt; --check for CI verify-only)
#   scripts/run-lint.sh     (rust: cargo clippy --all-targets; --fix for autofix)
#   scripts/run-tests.sh    (rust: cargo test --tests; optional name filter)
# All three + this run-ci source the shared scripts/_env.sh bootstrap.
#
# GATE SCHEDULING (porting-sdk/scripts/gate_scheduler.sh — CI_PERF S1 + S2):
#   Gates run CONCURRENTLY up to a cap (SW_CI_JOBS, default nproc), scheduled by
#   their DATA dependencies:
#     * S2 concurrent wave: the pure-Python side-effect-free gates (all GEN-FRESH*,
#       DRIFT, NO-CHEAT, EMISSION, SKILL-CONTRACT, SWAIG-COVERAGE, SURFACE-DIFF,
#       DOC-AUDIT, SWAIG-CLI) overlap — they share no mutable state.
#     * S1 fail-fast: heavy gates (TEST, LINT, FMT, REST-COVERAGE, SPEC-PARITY,
#       SIGNATURES via rustdoc) are deferred behind the cheap wave, so a trivial
#       cheap-gate failure surfaces in seconds; --fail-fast aborts before TEST.
#   HARD ordering is data-dependency ONLY:
#     * DRIFT reads port_signatures.json that SIGNATURES writes → deps=SIGNATURES.
#     * SURFACE-FRESH regenerates port_surface.json in place (and restores it);
#       SURFACE-DIFF + DOC-AUDIT read it → all three share res=surface.
#   Per-gate PASS/FAIL + the FAILED_GATES tally preserved exactly; each gate's output
#   captured + replayed atomically.
#
# Flags:
#   --fail-fast   stop launching new gates at the first failure (local dev loop).

set -u
set -o pipefail

PORT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT_NAME="signalwire-rust"

# sccache: availability-gated compiler cache (pure speedup, no-op when absent).
# The canonical run-{tests,format,lint}.sh gates source scripts/_env.sh and get
# this automatically; run-ci ALSO invokes cargo directly for several gates
# (REST-COVERAGE, route-registry, emit_corpus/emit_skills, swaig-test), so wire
# the same gate here in run-ci's own process. We do NOT `source _env.sh` (it sets
# `set -e`, and run-ci deliberately runs every gate to collect failures). Mirror
# only the availability gate — see scripts/_env.sh for the declaration/rationale.
if [ -z "${RUSTC_WRAPPER:-}" ] && command -v sccache >/dev/null 2>&1; then
    export RUSTC_WRAPPER=sccache
    if [ -z "${SCCACHE_DIR:-}" ]; then
        export SCCACHE_DIR="$PORT_ROOT/.sw-tmp/sccache"
    fi
    mkdir -p "$SCCACHE_DIR" 2>/dev/null || true
fi

resolve_porting_sdk() {
    if [ -n "${PORTING_SDK:-}" ] && [ -d "$PORTING_SDK/scripts" ]; then
        echo "$PORTING_SDK"
        return 0
    fi
    if [ -d "$PORT_ROOT/../porting-sdk/scripts" ]; then
        (cd "$PORT_ROOT/../porting-sdk" && pwd)
        return 0
    fi
    return 1
}

PORTING_SDK_DIR="$(resolve_porting_sdk)" || {
    echo "FATAL: porting-sdk not found, clone it adjacent to this repo" >&2
    echo "       (expected $PORT_ROOT/../porting-sdk or \$PORTING_SDK env var)" >&2
    exit 2
}

# shellcheck source=/dev/null
source "$PORTING_SDK_DIR/scripts/gate_scheduler.sh"

cd "$PORT_ROOT"

echo "==> running CI gates for $PORT_NAME (porting-sdk at $PORTING_SDK_DIR)"

pick_free_port() {
    python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'
}

# SURFACE-FRESH — close the Layer-B-not-gated hole. Regenerate the surface IN PLACE
# via the rust enumerator (regex over src/**.rs; no rustdoc, so independent of the
# SIGNATURES gate's cache), compare against the committed copy MODULO the volatile
# generated_from git-sha, then always restore the file.
surface_fresh_gate() {
    git show HEAD:port_surface.json > /tmp/committed_surface.json 2>/dev/null \
        || cp "$PORT_ROOT/port_surface.json" /tmp/committed_surface.json
    python3 scripts/enumerate_surface.py || { git checkout -- port_surface.json; return 1; }
    python3 "$PORTING_SDK_DIR/scripts/check_surface_freshness.py" \
        --committed /tmp/committed_surface.json \
        --fresh "$PORT_ROOT/port_surface.json"
    local rc=$?
    git checkout -- port_surface.json
    return $rc
}

# REST-COVERAGE — every implemented REST route covered success+error. Self-
# contained: spins its own mock, runs the generated wire-test suites serially, then
# checks the journal.
rest_coverage_gate() {
    local port
    port="$(pick_free_port)" || { echo "could not allocate a free port" >&2; return 1; }
    local mock_pkg_parent="$PORTING_SDK_DIR/test_harness/mock_signalwire"
    export PYTHONPATH="$mock_pkg_parent${PYTHONPATH:+:$PYTHONPATH}"
    python3 -m mock_signalwire --host 127.0.0.1 --port "$port" --log-level error \
        >/tmp/rest_cov_mock_rust.$$.log 2>&1 &
    local mock_pid=$!
    # shellcheck disable=SC2064
    trap "kill $mock_pid 2>/dev/null" RETURN
    local i ready=0
    for i in $(seq 1 60); do
        if ! kill -0 "$mock_pid" 2>/dev/null; then
            echo "mock_signalwire died on port $port — log:" >&2
            cat "/tmp/rest_cov_mock_rust.$$.log" >&2
            return 1
        fi
        if python3 -c "import urllib.request; urllib.request.urlopen('http://127.0.0.1:$port/__mock__/health',timeout=1)" 2>/dev/null; then
            ready=1
            break
        fi
        sleep 0.5
    done
    if [ "$ready" -ne 1 ]; then
        echo "mock_signalwire on port $port not healthy within 30s" >&2
        return 1
    fi
    python3 -c "import urllib.request; urllib.request.urlopen(urllib.request.Request('http://127.0.0.1:$port/__mock__/journal/reset',method='POST'),timeout=5).read()"
    MOCK_SIGNALWIRE_PORT="$port" cargo test \
        --test rest_generated_calling \
        --test rest_generated_chat \
        --test rest_generated_datasphere \
        --test rest_generated_fabric \
        --test rest_generated_fax \
        --test rest_generated_logs \
        --test rest_generated_message \
        --test rest_generated_project \
        --test rest_generated_pubsub \
        --test rest_generated_relay_rest \
        --test rest_generated_video \
        --test rest_generated_voice \
        -- --test-threads=1 || return 1
    python3 -m mock_signalwire.rest_coverage \
        --mock-url "http://127.0.0.1:$port" \
        --spec-root "$PORTING_SDK_DIR/rest-apis" \
        --allowlist "$PORTING_SDK_DIR/REST_COVERAGE_BASELINE.md" \
        --allowlist "$PORT_ROOT/REST_COVERAGE_GAPS.md" \
        --gap-baseline "$PORTING_SDK_DIR/REST_COVERAGE_GAP_BASELINE.md"
}

# SPEC-PARITY — implemented REST routes == canonical spec (both directions). Set B
# is produced deterministically by the route-registry binary.
spec_parity_gate() {
    local reg
    reg="$(mktemp -t rust_route_registry.XXXXXX.json)"
    # shellcheck disable=SC2064
    trap "rm -f '$reg'" RETURN
    cargo run --quiet --bin route-registry >"$reg" || return 1
    python3 "$PORTING_SDK_DIR/scripts/diff_spec_implementation.py" \
        --registry-json "$reg" \
        --gaps "$PORTING_SDK_DIR/SPEC_IMPLEMENTATION_GAPS.md"
}

# ---- register gates ----------------------------------------------------------
sched_init "$@"

sched_gate TEST defer=1 desc="cargo test --tests (parallel) via scripts/run-tests.sh" \
    -- bash "$PORT_ROOT/scripts/run-tests.sh"

sched_gate GEN-FRESH desc="generated REST layer matches the canonical specs (generate_rest.py --check)" \
    -- python3 scripts/generate_rest.py --check

sched_gate GEN-FRESH-SWML desc="generated SWML-verbs config tree matches schema.json (\$defs)" \
    -- python3 scripts/generate_swml_verbs.py --check

sched_gate GEN-FRESH-RELAY desc="generated RELAY-protocol tree matches relay-protocol/*.json" \
    -- python3 scripts/generate_relay_protocol.py --check

sched_gate GEN-FRESH-SWAIG desc="generated SWAIG payload tree matches swaig-specs/" \
    -- python3 scripts/generate_swaig_payloads.py --check

sched_gate GEN-FRESH-TESTS desc="generated REST wire-test suite matches the route-registry × spec oracle (generate_rest_tests.py --check)" \
    -- python3 scripts/generate_rest_tests.py --check

sched_gate SWAIG-COVERAGE desc="every engine SWAIG action emittable (modulo allowlist)" \
    -- python3 "$PORTING_SDK_DIR/scripts/swaig_coverage.py" --check \
        --emission "$PORT_ROOT/src/swaig/function_result.rs"

# SIGNATURES shells out to rustdoc (heavy) but DRIFT deps on it, so it is NOT
# deferred — deferring a writer that a cheap gate reads would stall the wave.
sched_gate SIGNATURES desc="regenerate port_signatures.json (rustdoc + adapter)" \
    -- python3 scripts/enumerate_signatures.py

sched_gate DRIFT deps=SIGNATURES desc="diff_port_signatures vs python reference" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_signatures.py" \
        --reference "$PORTING_SDK_DIR/python_signatures.json" \
        --port-signatures "$PORT_ROOT/port_signatures.json" \
        --surface-omissions "$PORT_ROOT/PORT_OMISSIONS.md" \
        --surface-additions "$PORT_ROOT/PORT_ADDITIONS.md" \
        --omissions "$PORT_ROOT/PORT_SIGNATURE_OMISSIONS.md"

sched_gate SURFACE-FRESH res=surface desc="check_surface_freshness vs committed port_surface.json" \
    --fn surface_fresh_gate

sched_gate NO-CHEAT desc="audit_no_cheat_tests" \
    -- python3 "$PORTING_SDK_DIR/scripts/audit_no_cheat_tests.py" --root "$PORT_ROOT"

sched_gate REST-COVERAGE defer=1 desc="every implemented REST route covered success+error (parity + allowlist)" \
    --fn rest_coverage_gate

sched_gate SPEC-PARITY defer=1 desc="implemented REST routes == canonical spec (modulo gaps); deterministic Set B" \
    --fn spec_parity_gate

sched_gate EMISSION desc="diff_port_emission vs python to_dict() oracle" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_emission.py" \
        --dump-cmd 'cargo run --quiet --example emit_corpus' \
        --port-repo "$PORT_ROOT"

sched_gate SKILL-CONTRACT desc="diff_skill_contracts vs python reference" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_skill_contracts.py" \
        --dump-cmd 'cargo run --quiet --example emit_skills' \
        --port-repo "$PORT_ROOT"

sched_gate FMT defer=1 desc="rustfmt via scripts/run-format.sh (local: auto-fix; CI: --check)" \
    -- bash "$PORT_ROOT/scripts/run-format.sh" ${CI:+--check}

sched_gate LINT defer=1 desc="cargo clippy --all-targets via scripts/run-lint.sh" \
    -- bash "$PORT_ROOT/scripts/run-lint.sh"

sched_gate DOC-AUDIT res=surface desc="audit_docs vs port_surface.json" \
    -- python3 "$PORTING_SDK_DIR/scripts/audit_docs.py" \
        --root "$PORT_ROOT" \
        --surface "$PORT_ROOT/port_surface.json" \
        --ignore "$PORT_ROOT/DOC_AUDIT_IGNORE.md"

sched_gate SURFACE-DIFF res=surface desc="diff_port_surface vs python_surface.json" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_surface.py" \
        --reference "$PORTING_SDK_DIR/python_surface.json" \
        --port-surface "$PORT_ROOT/port_surface.json" \
        --omissions "$PORT_ROOT/PORT_OMISSIONS.md" \
        --additions "$PORT_ROOT/PORT_ADDITIONS.md"

sched_gate SWAIG-CLI desc="swaig-test shared mini-contract (verbs/serverless-reject/default-action)" \
    -- python3 "$PORTING_SDK_DIR/scripts/audit_swaig_cli_contract.py" \
        --port rust \
        --cmd "cargo run --quiet --bin swaig-test --" \
        --require-url-model \
        --default-action-argv='--url|http://user:pass@127.0.0.1:1/' \
        --no-serverless-argv='--url|http://user:pass@127.0.0.1:1/|--simulate-serverless|lambda|--list-tools'

sched_run
rc=$?
if [ "$rc" -eq 0 ]; then
    echo "==> CI PASS"
else
    echo "==> CI FAIL (gates:$FAILED_GATES )"
fi
exit "$rc"
