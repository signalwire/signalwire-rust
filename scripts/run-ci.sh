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

# Gate-enforcement plan (Part D): rust's Wave-A widened findings are BLOCKING, not
# report-only. The shared wave-A gates (count_claim / dead_public_error / audit_docs
# / status_claim / semver_diff) default to report-only (SW_WAVE_A_REPORT_ONLY unset →
# report-only); setting it to 0 makes every newly-caught wave-A violation count toward
# the exit code. rust's wave-A red list has been burned to zero, so this stays green.
export SW_WAVE_A_REPORT_ONLY=0

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

# ---- Part 5: the per-gate --fn helpers are now DEAD — reproduced in the suites ---
# surface_fresh_gate (SURFACE-FRESH), rest_coverage_gate (REST-COVERAGE),
# spec_parity_gate (SPEC-PARITY), route_collision_gate (ROUTE-COLLISION), and
# dayone_artifact_deny (ARTIFACT-DENY) used to be defined here as `--fn` gate bodies.
# Those exact bodies are now reproduced INSIDE the Part-5 suites
# (scripts/suites/_surface_fresh.py, _rest_coverage.py, _spec_parity.py, the
# ROUTE-COLLISION member of _surface_commands.py, _artifact_deny.py), so they are no
# longer defined here. pick_free_port() likewise moved into the suites. Byte-identity
# vs the old per-gate path is proven by porting-sdk's tests/test_suite_parity*.py.
#
# STRICT-MOCKS stays a standalone `--fn` (its RELAY-strict re-run is not a suite
# member) — its helper is retained below.

# STRICT-MOCKS (§2.2) — re-run the RELAY integration suite with the mock in STRICT
# mode (MOCK_RELAY_STRICT=1: mock_relay 400s an unknown field or a duplicate id
# instead of tolerantly journaling it), so a wire-shape regression the tolerant
# mock would swallow fails loud. The relay tests self-spawn `python -m mock_relay`,
# which inherits MOCK_RELAY_STRICT from this env. rust's RELAY suite passes clean
# under strict today (empty red list). tier=nightly (a second full RELAY pass is
# heavy) + defer. The mock package is discovered adjacently by the harness.
strict_mocks_gate() {
    local mock_relay_parent="$PORTING_SDK_DIR/test_harness/mock_relay"
    MOCK_RELAY_STRICT=1 PYTHONPATH="$mock_relay_parent${PYTHONPATH:+:$PYTHONPATH}" \
        cargo test --quiet \
            --test relay_mock_actions \
            --test relay_mock_connect \
            --test relay_mock_event_dispatch \
            --test relay_mock_inbound_call \
            --test relay_mock_messaging \
            --test relay_mock_outbound_call \
            --test relay_mock_smoke \
            --test relay_mock_typed_convenience \
            --test relay_mock_typed_errors \
            -- --test-threads=1
}

# ---- register gates ----------------------------------------------------------
sched_init "$@"

# HEAVY (deferred behind the cheap wave for S1 fail-fast).
sched_gate TEST defer=1 desc="cargo test --tests (parallel) via scripts/run-tests.sh" \
    -- bash "$PORT_ROOT/scripts/run-tests.sh"

# ---- Part 5 gate SUITES ------------------------------------------------------
# The former per-gate SIGNATURES/DRIFT/SURFACE-*/SEMVER-DIFF/GEN-TYPE-DEGENERACY/
# GEN-IDIOM/ROUTE-COLLISION/GEN-FRESH*/BEHAVIORAL-*/EMISSION/ERROR-ENVELOPE/
# PAGINATION-WIRED/DOC-WIRE/REST-COVERAGE/SPEC-PARITY/SKILL-CONTRACT/SWAIG-*/
# WAIT-LIVENESS/DOC-*/COUNT-CLAIM/ACCESSOR-TRUTH/STATUS-CLAIM/README-INCLUDE/
# *-LEDGER/PACKAGE-SMOKE/META-CONSISTENT/ARTIFACT-DENY/RELEASE-FRESH gates now run
# under 6 SUITE engines. Each suite emits every original gate NAME as a
# `[SUITE:RULE] ... PASS/FAIL` rule ID (failure identity + allowlists + finding
# output unchanged). A suite exits nonzero iff any of its rules fails. Byte-identity
# vs the old per-gate path is proven by porting-sdk/tests/test_suite_parity*.py.
#
# The `--fn` helpers the old gates used (surface_fresh_gate, rest_coverage_gate,
# spec_parity_gate, route_collision_gate, dayone_artifact_deny, pick_free_port) are
# reproduced INSIDE the suites, so they are no longer defined here.
#
# Former single-gate scheduler features preserved by the suites internally:
#   * SIGNATURES→DRIFT ordering, the SEMVER-DIFF-reads-SIGNATURES data dep, and the
#     SURFACE-FRESH regenerate-then-restore all live inside the SURFACE suite.
#   * mixed tiers are split with --rules: PACKAGE + BEHAVIORAL each schedule a
#     per-PR line and a nightly line (nightly members broken out below).
# RUST-SPECIFIC: rust's SURFACE suite ALSO carries ROUTE-COLLISION, GEN-TYPE-DEGENERACY,
# and GEN-IDIOM (the expansion gates); rust's behavioral RELAY rule keeps rust's
# hyphen spelling BEHAVIORAL-WIRE-RELAY. SURFACE-FRESH regenerates port_surface.json
# in place (and restores it) and DOC-AUDIT/STATUS-CLAIM read it, so the SURFACE and
# DOC-TRUTH suites share res=surface (mutually exclusive), exactly as the old per-gate
# SURFACE-FRESH/SURFACE-DIFF/DOC-AUDIT/STATUS-CLAIM surface mutex did.

# SURFACE (parity spine): SIGNATURES→DRIFT ordered, SURFACE-FRESH regen/restore,
# SURFACE-DIFF, SEMVER-DIFF, GEN-TYPE-DEGENERACY, GEN-IDIOM, ROUTE-COLLISION — all
# read the one enumeration. res=surface: SURFACE-FRESH regenerates port_surface.json
# in place (and restores it), so it must not overlap DOC-TRUTH's reads of it.
sched_gate SURFACE res=surface desc="surface parity suite (SIGNATURES/DRIFT/SURFACE-FRESH/SURFACE-DIFF/GEN-TYPE-DEGENERACY/ROUTE-COLLISION/GEN-IDIOM/SEMVER-DIFF)" \
    -- python3 "$PORTING_SDK_DIR/scripts/suites/surface.py" --port rust --repo "$PORT_ROOT"

# GEN (regen-from-specs family): the 5 GEN-FRESH rules. rust scheduled these in the
# cheap S2 wave (no defer), preserved here.
sched_gate GEN desc="generated-code freshness suite (GEN-FRESH/-SWML/-RELAY/-SWAIG/-TESTS)" \
    -- python3 "$PORTING_SDK_DIR/scripts/suites/gen.py" --port rust --repo "$PORT_ROOT"

# BEHAVIORAL (one Layer-D pass per rule): the per-PR rules. WAIT-LIVENESS (nightly)
# is the separate line below. defer=1: REST-COVERAGE (spins the mock + serial cargo
# --test crates) is the heavy member. NOTE rust's hyphen spelling BEHAVIORAL-WIRE-RELAY.
sched_gate BEHAVIORAL defer=1 desc="behavioral suite (REST-COVERAGE/SPEC-PARITY/EMISSION/BEHAVIORAL-*/SKILL-CONTRACT/SWAIG-COVERAGE/SWAIG-CLI/ERROR-ENVELOPE/PAGINATION-WIRED/DOC-WIRE)" \
    -- python3 "$PORTING_SDK_DIR/scripts/suites/behavioral.py" --port rust --repo "$PORT_ROOT" \
        --rules REST-COVERAGE,SPEC-PARITY,EMISSION,BEHAVIORAL-WIRE,BEHAVIORAL-SWML,BEHAVIORAL-STATE,BEHAVIORAL-HTTP,BEHAVIORAL-WIRE-RELAY,SKILL-CONTRACT,SWAIG-COVERAGE,SWAIG-CLI,ERROR-ENVELOPE,PAGINATION-WIRED,DOC-WIRE

sched_gate BEHAVIORAL-NIGHTLY tier=nightly defer=1 desc="behavioral suite, nightly rules (WAIT-LIVENESS)" \
    -- python3 "$PORTING_SDK_DIR/scripts/suites/behavioral.py" --port rust --repo "$PORT_ROOT" \
        --rules WAIT-LIVENESS

# DOC-TRUTH (one markdown walk): DOC-AUDIT/DOC-LINKS/DOC-LANG-PURITY/DOC-ENV/
# COUNT-CLAIM/ACCESSOR-TRUTH/STATUS-CLAIM/README-INCLUDE. res=surface: DOC-AUDIT +
# STATUS-CLAIM read rust's on-disk port_surface.json, which the SURFACE suite
# regenerates+restores.
sched_gate DOC-TRUTH res=surface desc="doc-truth suite (DOC-AUDIT/DOC-LINKS/DOC-LANG-PURITY/DOC-ENV/COUNT-CLAIM/ACCESSOR-TRUTH/STATUS-CLAIM/README-INCLUDE)" \
    -- python3 "$PORTING_SDK_DIR/scripts/suites/doc_truth.py" --port rust --repo "$PORT_ROOT"

# LEDGER: SUPPRESSION-LEDGER + IGNORE-LEDGER-VERIFY.
sched_gate LEDGER res=dayone desc="ledger governance suite (SUPPRESSION-LEDGER/IGNORE-LEDGER-VERIFY)" \
    -- python3 "$PORTING_SDK_DIR/scripts/suites/ledger.py" --port rust --repo "$PORT_ROOT"

# PACKAGE: per-PR rules (ARTIFACT-DENY/RELEASE-FRESH); nightly rules (META-CONSISTENT/
# PACKAGE-SMOKE) on the separate line below.
sched_gate PACKAGE res=dayone desc="package suite, per-PR rules (ARTIFACT-DENY/RELEASE-FRESH)" \
    -- python3 "$PORTING_SDK_DIR/scripts/suites/package.py" --port rust --repo "$PORT_ROOT" \
        --rules ARTIFACT-DENY,RELEASE-FRESH

sched_gate PACKAGE-NIGHTLY tier=nightly defer=1 res=dayone desc="package suite, nightly rules (META-CONSISTENT/PACKAGE-SMOKE)" \
    -- python3 "$PORTING_SDK_DIR/scripts/suites/package.py" --port rust --repo "$PORT_ROOT" \
        --rules META-CONSISTENT,PACKAGE-SMOKE

# ---- gates that stay standalone (native toolchains + singletons) -------------
sched_gate NO-CHEAT desc="audit_no_cheat_tests" \
    -- python3 "$PORTING_SDK_DIR/scripts/audit_no_cheat_tests.py" --root "$PORT_ROOT"

sched_gate FMT defer=1 desc="rustfmt via scripts/run-format.sh (local: auto-fix; CI: --check)" \
    -- bash "$PORT_ROOT/scripts/run-format.sh" ${CI:+--check}

sched_gate LINT defer=1 desc="cargo clippy --all-targets via scripts/run-lint.sh" \
    -- bash "$PORT_ROOT/scripts/run-lint.sh"

sched_gate PUBLIC-JARGON desc="no porting/internal jargon leaked into public docs/identifiers" \
    -- python3 "$PORTING_SDK_DIR/scripts/public_jargon.py" --port rust --repo .

sched_gate ROOT-HYGIENE res=dayone desc="no audit/scratch clutter tracked at repo root (allowlist ROOT_HYGIENE_ALLOW.md)" \
    -- python3 "$PORTING_SDK_DIR/scripts/root_hygiene.py" --port rust --repo .

# ---- §C1 doc/example/CLI execution gates -------------------------------------
# SNIPPET-COMPILE (~29s, typecheck WITH the real crate) → tier=nightly. DOC-CLI
# line-detects swaig-test invocations (no built binary to probe) → cheap, blocking.
# EXAMPLES-RUN + SNIPPET-RUN self-skip for a compiled port (no cargo run target /
# non-dynamic) — SNIPPET-COMPILE covers them — but stay wired (nightly, defer) so
# the gate graduates automatically if a run target is added.
sched_gate SNIPPET-COMPILE tier=nightly desc="documented code snippets compile against the real crate" \
    -- python3 "$PORTING_SDK_DIR/scripts/snippet_compile.py" --port rust --repo .

sched_gate DOC-CLI desc="documented swaig-test invocations parse against the real CLI" \
    -- python3 "$PORTING_SDK_DIR/scripts/doc_cli.py" --port rust --repo .

# DEAD-PUBLIC-ERROR stays standalone (source analysis of exported error types — not
# a doc-truth/behavioral rule).
sched_gate DEAD-PUBLIC-ERROR desc="exported error types are raised/caught/user-signalled (no dead error surface)" \
    -- python3 "$PORTING_SDK_DIR/scripts/dead_public_error.py" --port rust --repo "$PORT_ROOT"

sched_gate SNIPPET-RUN tier=nightly defer=1 desc="dynamic-port doc snippets run to a zero exit against the mock (compiled port: self-skips)" \
    -- python3 "$PORTING_SDK_DIR/scripts/snippet_run.py" --port rust --repo . --report-only

sched_gate EXAMPLES-RUN tier=nightly defer=1 desc="shipped examples load/start against the mock (compiled port: self-skips)" \
    -- python3 "$PORTING_SDK_DIR/scripts/examples_run.py" --port rust --repo .

# STRICT-MOCKS (§2.2) — nightly re-run of the RELAY suite with MOCK_RELAY_STRICT=1.
sched_gate STRICT-MOCKS tier=nightly defer=1 desc="RELAY suite passes with the mock in 400-on-violation strict mode (MOCK_RELAY_STRICT=1)" \
    --fn strict_mocks_gate

sched_run
rc=$?
if [ "$rc" -eq 0 ]; then
    echo "==> CI PASS"
else
    echo "==> CI FAIL (gates:$FAILED_GATES )"
fi
exit "$rc"
