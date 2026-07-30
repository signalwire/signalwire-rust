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

# STRICT-MOCKS 400-mode (plan §2.2c): strict is the default now.
export MOCK_SIGNALWIRE_STRICT="${MOCK_SIGNALWIRE_STRICT:-1}"
export MOCK_RELAY_STRICT="${MOCK_RELAY_STRICT:-1}"

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

pick_free_port() {
    python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'
}

# SURFACE-FRESH — close the Layer-B-not-gated hole. Regenerate the surface IN PLACE
# via the rust enumerator (regex over src/**.rs; no rustdoc, so independent of the
# SIGNATURES gate's cache), compare against the committed copy MODULO the volatile
# generated_from git-sha, then always restore the file.
surface_fresh_gate() {
    # Scratch under the repo-local, gitignored .sw-tmp/ (never machine-wide /tmp,
    # which invites cross-run pollution + leftover files).
    mkdir -p "$PORT_ROOT/.sw-tmp"
    local committed="$PORT_ROOT/.sw-tmp/committed_surface.json"
    git show HEAD:port_surface.json > "$committed" 2>/dev/null \
        || cp "$PORT_ROOT/port_surface.json" "$committed"
    # The enumerator writes the native-name sidecar alongside the surface, so both
    # are restored — leaving a regenerated sidecar behind would make the tree dirty
    # for every later gate.
    python3 scripts/enumerate_surface.py || {
        git checkout -- port_surface.json port_surface_native.json
        return 1
    }
    python3 "$PORTING_SDK_DIR/scripts/check_surface_freshness.py" \
        --committed "$committed" \
        --fresh "$PORT_ROOT/port_surface.json"
    local rc=$?
    git checkout -- port_surface.json port_surface_native.json
    return $rc
}

# REST-COVERAGE — every implemented REST route covered success+error. Self-
# contained: spins its own mock, runs the generated wire-test suites serially, then
# checks the journal for BOTH coverage AND wire-truth (STRICT-MOCKS §2.2a: any
# journaled wire_violation reds the gate — respelling-proof, since it reads the
# mock's own spec-vs-wire judgement).
rest_coverage_gate() {
    local port
    port="$(pick_free_port)" || { echo "could not allocate a free port" >&2; return 1; }
    local mock_pkg_parent="$PORTING_SDK_DIR/test_harness/mock_signalwire"
    export PYTHONPATH="$mock_pkg_parent${PYTHONPATH:+:$PYTHONPATH}"
    # Mock log under the repo-local, gitignored .sw-tmp/ (never machine-wide /tmp).
    mkdir -p "$PORT_ROOT/.sw-tmp"
    local mock_log="$PORT_ROOT/.sw-tmp/rest_cov_mock_rust.$$.log"
    python3 -m mock_signalwire --host 127.0.0.1 --port "$port" --log-level error \
        >"$mock_log" 2>&1 &
    local mock_pid=$!
    # shellcheck disable=SC2064
    trap "kill $mock_pid 2>/dev/null" RETURN
    local i ready=0
    for i in $(seq 1 60); do
        if ! kill -0 "$mock_pid" 2>/dev/null; then
            echo "mock_signalwire died on port $port — log:" >&2
            cat "$mock_log" >&2
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
        --test rest_generated_messages \
        --test rest_generated_project \
        --test rest_generated_projects \
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
        --gap-baseline "$PORTING_SDK_DIR/REST_COVERAGE_GAP_BASELINE.md" || return 1
    # STRICT-MOCKS §2.2a — fail the gate on ANY journaled wire_violation. The shared
    # helper reads the same live mock journal and exits non-zero on any offender
    # (see porting-sdk/scripts/assert_no_wire_violations.py). WIRE_VIOLATIONS_ALLOW.md
    # holds ONLY owner-signed spec-gap parks.
    python3 "$PORTING_SDK_DIR/scripts/assert_no_wire_violations.py" \
        --rest-mock-url "http://127.0.0.1:$port" \
        --allowlist "$PORT_ROOT/WIRE_VIOLATIONS_ALLOW.md"
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

# ROUTE-COLLISION (expansion) — no duplicate CRUD base + no split route classes.
# Needs the port's route-registry (same deterministic Set B the SPEC-PARITY gate
# builds via the route-registry binary). Enforcing (no --report-only); the port's
# ROUTE_COLLISION_ALLOW.md, when present, is honored by the gate.
route_collision_gate() {
    local reg
    reg="$(mktemp -t rust_route_collision.XXXXXX.json)"
    # shellcheck disable=SC2064
    trap "rm -f '$reg'" RETURN
    cargo run --quiet --bin route-registry >"$reg" || return 1
    python3 "$PORTING_SDK_DIR/scripts/route_collision.py" \
        --port rust --repo . --registry-json "$reg"
}

# ARTIFACT-DENY (Day-one) — authoritative --listing mode. Feed the REAL published
# package file listing (`cargo package --list`) to artifact_deny.py rather than the
# git-ls-files proxy, which over-reports files tracked in-repo but excluded from the
# published crate. --allow-dirty so an uncommitted tree (this very run) still lists.
dayone_artifact_deny() {
    cargo package --list --allow-dirty 2>/dev/null \
        | python3 "$PORTING_SDK_DIR/scripts/artifact_deny.py" --port rust --listing -
}

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

# PERF-BASELINE (r5 deep_perf_baseline / porting-sdk scripts/perf) — the nightly
# SWML-render ratchet (P2). Builds the release `perf-swml` bench (which prints
# `P2 default <µs/doc>`), folds it into perf_results.json, and ratchets against
# the committed perf_baseline.json via the shared perf_baseline.py checker.
# Release build is heavy → nightly-only (tier=nightly, deferred behind the cheap
# wave). Its noise control: a >15% regression must reproduce on an immediate
# re-measure before it reds (measure → check → on-red confirm-measure → recheck).
perf_baseline_gate() {
    local checker="$PORTING_SDK_DIR/scripts/perf_baseline.py"
    if [ ! -f "$checker" ]; then
        echo "PERF-BASELINE: perf_baseline.py not present in porting-sdk — skipping (harness not yet adopted)"
        return 0
    fi
    local results="$PORT_ROOT/.sw-tmp/perf_results.json"
    mkdir -p "$PORT_ROOT/.sw-tmp"
    _perf_measure() {
        local out
        out="$(cargo run --quiet --release --bin perf-swml -- 1000 2>/dev/null)" || return 1
        # Fold the single `P2 default <µs>` line into the shared perf_results shape.
        python3 - "$out" "$results" <<'PY'
import json, sys
line, out = sys.argv[1], sys.argv[2]
metrics = {}
for ln in line.splitlines():
    p = ln.split()
    if len(p) == 3:
        metrics.setdefault(p[0], {})[p[1]] = float(p[2])
json.dump({"port": "rust", "toolchain": "cargo", "metrics": metrics}, open(out, "w"), indent=2, sort_keys=True)
PY
    }
    _perf_measure || { echo "PERF-BASELINE: bench build/run failed"; return 1; }
    if python3 "$checker" --baseline "$PORT_ROOT/perf_baseline.json" --results "$results"; then
        return 0
    fi
    # Confirm pass (two-consecutive-medians noise control): re-measure, re-check.
    echo "PERF-BASELINE: over threshold — confirming with a re-measure..."
    _perf_measure || { echo "PERF-BASELINE: confirm bench run failed"; return 1; }
    python3 "$checker" --baseline "$PORT_ROOT/perf_baseline.json" --results "$results"
}

# ---- register gates ----------------------------------------------------------
sched_init "$@"

# STRICT-MOCKS §2.2b — run under MOCK_RELAY_STRICT=1 so an unknown RELAY frame
# field / duplicate command-id is rejected (400) rather than silently accepted.
# The relay-backed tests self-spawn `python -m mock_relay`, which inherits this
# env var from run-ci's process. rust's RELAY suite passes clean under strict.
sched_gate TEST defer=1 desc="cargo test --tests (parallel) via scripts/run-tests.sh (STRICT-MOCKS: MOCK_RELAY_STRICT=1)" \
    -- env MOCK_RELAY_STRICT=1 bash "$PORT_ROOT/scripts/run-tests.sh"

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

# SIGNATURES-FRESH — the sibling hole. SURFACE-FRESH guards port_surface.json;
# NOTHING guarded port_signatures.json, which is DRIFT's INPUT — so a stale copy
# means the parity gate compares against a fiction and reports clean.
# Scheduled STANDALONE here, exactly like SEMVER-DIFF above: rust's run-ci does not
# read _surface_commands.py, so a table entry there would be silently skipped.
sched_gate SIGNATURES-FRESH res=surface desc="committed port_signatures.json matches a fresh regen" \
    -- python3 "$PORTING_SDK_DIR/scripts/suites/_signatures_fresh.py" \
        --port rust --repo "$PORT_ROOT" --porting-sdk "$PORTING_SDK_DIR"

# TYPE-EROSION: a port may not erase a type the reference DECLARES. compare_param treats
# `any` on EITHER side as matching anything, so a port emitting `any` silently satisfies
# every reference declaration — an unlimited opt-out. ConciergeAgent.hours_of_operation is
# declared optional<dict<string,string>> and go still shipped a bare string, with no gate
# red. RATCHET, not a hard gate: dynamic languages cannot always express a type, so this
# banks the current count and fails only on REGRESSION. Drive the number DOWN; never up.
sched_gate TYPE-EROSION desc="port did not erase a reference-declared param type (ratchet 13)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_type_erosion.py" --port rust --repo "$PORT_ROOT" --max 13

# RELAY-VERB-RESULT-LOCK: the ~52 calling.* verbs on relay::Call return
# Result<_, RelayError> (RUST-1). DRIFT tolerates a reversion (the enumerator
# unwraps Result<T,E>→T so a bare-return verb records the same concrete type),
# so this parses src/relay/call.rs to lock the flip against a silent revert.
# The gate SCRIPT lives on porting-sdk wave/1-aplus; PR CI currently checks out
# porting-sdk main (post-#50 unpin), where it is not yet present. Guard on the
# script's presence so PR #51 stays green against a main psdk while enforcing the
# instant psdk carries it (wave/1-aplus locally, and unconditionally once the
# Wave-1 psdk merges to main — at which point this guard becomes a no-op). This
# is an availability-guard on a psdk-side gate, NOT a silenced local gate: the
# verb→Result flip itself is proven by cargo tests + relay_mock_verb_result.
if [ -f "$PORTING_SDK_DIR/scripts/rust_relay_verb_result_lock.py" ]; then
    sched_gate RELAY-VERB-RESULT-LOCK desc="every relay::Call calling.* verb returns Result<_, RelayError>" \
        -- python3 "$PORTING_SDK_DIR/scripts/rust_relay_verb_result_lock.py" \
            --call-source "$PORT_ROOT/src/relay/call.rs" --min-locked 40
else
    echo "run-ci: RELAY-VERB-RESULT-LOCK skipped — gate script not in this porting-sdk" \
         "checkout (on wave/1-aplus; enforces once psdk carries it / merges to main)." >&2
fi

sched_gate NO-CHEAT desc="audit_no_cheat_tests" \
    -- python3 "$PORTING_SDK_DIR/scripts/audit_no_cheat_tests.py" --root "$PORT_ROOT"

sched_gate COORDINATED-PASS desc="a non-main porting-sdk pin must be declared on the PR (Coordinated-With: line or coordinated-pass label)" \
    -- python3 "$PORTING_SDK_DIR/scripts/coordinated_pass.py" --porting-sdk "$PORTING_SDK_DIR"

sched_gate COORDINATED-REFS desc="every coordinated-set checkout (porting-sdk + python oracle + matrix ports) uses PORTING_SDK_REF, not a literal ref" \
    -- python3 "$PORTING_SDK_DIR/scripts/check_coordinated_refs.py" --repo "$PORT_ROOT"

sched_gate REST-COVERAGE defer=1 desc="every implemented REST route covered success+error (parity + allowlist)" \
    --fn rest_coverage_gate

sched_gate SPEC-PARITY defer=1 desc="implemented REST routes == canonical spec (modulo gaps); deterministic Set B" \
    --fn spec_parity_gate

sched_gate EMISSION desc="diff_port_emission vs python to_dict() oracle" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_emission.py" \
        --dump-cmd 'cargo run --quiet --example emit_corpus' \
        --port-repo "$PORT_ROOT"

# BEHAVIORAL-* (Layer D) — 5 wire-shape differs vs the python oracle. Each runs a
# tiny `<surface>_dump` example that emits ONLY JSON on stdout and structurally
# compares it against signalwire-python's behavior for the same corpus. The python
# oracle is auto-resolved by the differ exactly as EMISSION does (no --python-sdk
# flag; --python-sdk defaults to the adjacent/installed signalwire package, which is
# the sibling checkout in CI). The 5 dump examples are prebuilt ONCE below so cargo
# emits no build noise onto the dump's stdout mid-gate.
cargo build --quiet \
    --example wire_dump --example swml_dump --example state_dump \
    --example http_dump --example wire_relay_dump 2>/dev/null || true

# WAIT-LIVENESS PREBUILD (plan §2.9) — the liveness differ starts a wall-clock
# deadline (~40s) the moment it launches its dump-cmd and measures how long
# `Action::wait()` blocks. If the FIRST `cargo run --example wait_liveness_dump`
# had to COMPILE the example (cold target), that build time was charged against
# the liveness deadline and the rust nightly went RED 07-17/18. Build the example
# HERE (outside any gate's clock) so the gate's `cargo run --quiet` is a near-
# instant exec and the measurement excludes build time. Same rationale as the
# BEHAVIORAL-* dump prebuild above. envelope-dump prebuilt for the same reason
# (ENVELOPE gate runs a dump-cmd).
cargo build --quiet --example wait_liveness_dump --bin envelope-dump --bin ai-chat-dump 2>/dev/null || true

# SECURITY-GATE PREBUILD — same rationale: SECURE-DEFAULT's differ runs a
# dump-cmd, and SECRET-SCRUB-LIVE's differ arms a wall-clock deadline the moment
# it launches its dump (it also gets --prebuild-cmd below, belt-and-braces so the
# exclusion is gate-local and independent of ordering).
cargo build --quiet --example secure_default_dump --example secret_scrub_dump 2>/dev/null || true

sched_gate BEHAVIORAL-WIRE desc="diff_port_wire vs python oracle (Layer D)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_wire.py" \
        --port rust \
        --dump-cmd 'cargo run -q --example wire_dump 2>/dev/null'

sched_gate BEHAVIORAL-SWML desc="diff_port_swml vs python oracle (Layer D)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_swml.py" \
        --port rust \
        --dump-cmd 'cargo run -q --example swml_dump 2>/dev/null'

sched_gate BEHAVIORAL-STRICT-RENDER desc="diff_port_strict_render vs python oracle (Wave-2 P#5)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_strict_render.py" \
        --port rust \
        --dump-cmd 'cargo run -q --example strict_render_dump 2>/dev/null'

sched_gate BEHAVIORAL-STATE desc="diff_port_state vs python oracle (Layer D)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_state.py" \
        --port rust \
        --dump-cmd 'cargo run -q --example state_dump 2>/dev/null'

sched_gate BEHAVIORAL-HTTP desc="diff_port_http vs python oracle (Layer D)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_http.py" \
        --port rust \
        --dump-cmd 'cargo run -q --example http_dump 2>/dev/null'

sched_gate BEHAVIORAL-WIRE-RELAY desc="diff_port_wire_relay vs python oracle (Layer D)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_wire_relay.py" \
        --port rust \
        --dump-cmd 'cargo run -q --example wire_relay_dump 2>/dev/null'

sched_gate ENVELOPE desc="diff_port_envelope vs python oracle: conn-refused typed error + request-options retry/timeout/idempotency (Layer D, plan 4.2/1.3b)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_envelope.py" \
        --port rust \
        --dump-cmd 'cargo run -q --bin envelope-dump 2>/dev/null'

# SECURE-DEFAULT (A1 / PSDK-4a) — the wire manifestation of `secure`. Drives
# AgentBase through the shared secure_default corpus: a default-secure tool and a
# `secure = false` tool, renders the SWML, and compares the per-fixture
# {secure_default_true, wire_reflects_secure} classification against the python
# oracle. `wire_reflects_secure` is measured off the RENDERED document — a per-tool
# `__token` must be present for the secure tool and absent for the insecure one — so
# this gate proves the port does not silently ship tools as unauthenticated. A fast
# in-process SWML render (no live mock) → per-PR tier.
#
# --prebuild-cmd, like WAIT-LIVENESS and SECRET-SCRUB-LIVE below: the differ arms
# a 120s wall-clock deadline the moment it launches the dump, and `cargo run`
# BLOCKS on the cargo build lock whenever a concurrently-scheduled gate is
# building. Gates run in parallel (SW_CI_JOBS), so the line-415 prebuild does not
# make this gate ordering-independent on its own — it only warms the artifact,
# it does not stop a LATER sibling's build from holding the lock while this gate
# waits on it. Without the flag that lock wait is charged to the deadline and the
# gate reports "dump HUNG (> 120s)" for a dump that takes ~5s: measured 5.0s idle
# vs 10.1s under concurrent CI load, and a red in two consecutive full runs.
sched_gate SECURE-DEFAULT desc="define_tool's secure state reaches the wire as the per-tool __token (Layer D vs python oracle)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_secure_default.py" \
        --port rust \
        --prebuild-cmd "cargo build --quiet --example secure_default_dump" \
        --dump-cmd 'cargo run -q --example secure_default_dump 2>/dev/null'

# The three STATIC security source-checks (PSDK-5/6, A5). rust wired NONE of them
# even though all three pass, so a regression at any of these sites would have
# shipped silently. Cheap greps over the port's own source → per-PR.
#   SECRET-SCRUB  — the raw-frame-log credential-leak SHAPE. The per-PR companion
#                   to SECRET-SCRUB-LIVE below (static catches the shape, live
#                   catches the leaks a shape grep cannot see).
#   TLS-VERIFY    — no hardcoded TLS-verify-off construct in the builtin-skill /
#                   HTTP-client source.
#   CA-VAR        — the REST source reads SIGNALWIRE_REST_CA_FILE and the RELAY
#                   source reads SIGNALWIRE_RELAY_CA_FILE (exact fleet names).
sched_gate SECRET-SCRUB desc="no raw-frame credential-log shape in the relay/skill source (static leg)" \
    -- python3 "$PORTING_SDK_DIR/scripts/secret_scrub.py" --port rust --repo .

sched_gate TLS-VERIFY desc="no TLS-verify-off construct in the builtin-skill / HTTP-client source" \
    -- python3 "$PORTING_SDK_DIR/scripts/tls_verify.py" --port rust --repo .

sched_gate CA-VAR desc="REST reads SIGNALWIRE_REST_CA_FILE and RELAY reads SIGNALWIRE_RELAY_CA_FILE (exact fleet names)" \
    -- python3 "$PORTING_SDK_DIR/scripts/ca_var_parity.py" --port rust --repo .

# TOKEN-INTEROP — property 3 of the SWAIG tool-token contract: a token this port MINTS
# must validate under the REFERENCE's own decoder. SECURE-DEFAULT proves a token is
# minted and the fleet keying check proves the HMAC key; NEITHER sees the base64
# ENVELOPE, so a port can ship correct-key correct-HMAC tokens that no other
# implementation accepts — in production every secure tool call then fails auth. THIS
# port is where the defect class was first proven: it minted with URL_SAFE_NO_PAD, and
# the reference's urlsafe_b64decode RAISES on a stripped '='. Our own decoder tolerated
# it, so round-tripping against ourselves could never catch it. This gate is what keeps
# the fix (URL_SAFE) from silently regressing. One mint + a pure-python validation →
# cheap, per-PR (a security property must not wait for nightly).
sched_gate TOKEN-INTEROP desc="a token this port mints validates under the reference's decoder (padded urlsafe base64, ':'-signed / '.'-enveloped, hex HMAC keyed by the secret_key string)" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_token_interop.py" --port rust \
        --mint-cmd 'cargo run --quiet --example token_interop_mint'

# AI-CHAT (COORDINATED pass rust:ai-chat-client <-> porting-sdk:ai-chat-client):
# wire-behavioral gate for the AIChatClient. Drives the ai-chat-dump binary through
# the shared ai_chat_corpus against porting-sdk's in-process mock_ai_chat and asserts
# the client speaks the AI Chat JSON-RPC protocol per the vendored spec
# (ai-chat-specs/ai-chat.yaml). The gate script (diff_port_ai_chat.py) + mock live on
# the porting-sdk `ai-chat-client` branch, so during the coordinated pass
# PORTING_SDK_REF pins that branch; until it lands on porting-sdk main this gate
# skip-passes (the differ script is simply absent).
sched_gate AI-CHAT desc="AIChatClient speaks the AI Chat protocol per the vendored spec (mock_ai_chat wire-behavioral)" \
    -- bash -c 'if [ -f "$1/scripts/diff_port_ai_chat.py" ]; then python3 "$1/scripts/diff_port_ai_chat.py" --port rust --dump-cmd "cargo run -q --bin ai-chat-dump 2>/dev/null"; else echo "[ai-chat] diff_port_ai_chat.py not on porting-sdk main yet — skip-pass (coordinated-branch dep: porting-sdk ai-chat-client)"; fi' _ "$PORTING_SDK_DIR"

sched_gate SKILL-CONTRACT desc="diff_skill_contracts vs python reference" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_skill_contracts.py" \
        --dump-cmd 'cargo run --quiet --example emit_skills' \
        --port-repo "$PORT_ROOT"

sched_gate FMT defer=1 desc="rustfmt via scripts/run-format.sh (local: auto-fix; CI: --check)" \
    -- bash "$PORT_ROOT/scripts/run-format.sh" ${CI:+--check}

sched_gate LINT defer=1 desc="cargo clippy --all-targets via scripts/run-lint.sh" \
    -- bash "$PORT_ROOT/scripts/run-lint.sh"

# PY-LINT — ruff (lint + format) over the 7 hand-written Python programs under
# scripts/. LINT above covers the whole Rust tree; this covers the Python that
# DECIDES WHAT THE GATES COMPARE — enumerate_surface.py feeds SURFACE-FRESH,
# enumerate_signatures.py feeds SIGNATURES/DRIFT, and the three generators feed
# GEN-FRESH*. None of it was linted by anything before 2026-07-30; the first
# pass found a live silent-success defect in both enumerators (an unreadable
# input yielded a short-but-valid oracle at rc=0). Rule selection mirrors the
# reference implementation (signalwire-python/pyproject.toml); config in
# ruff.toml. Dual-mode exactly like FMT: LOCAL applies fixes in place, CI ($CI
# set) passes --check for read-only verification, so an unformatted commit is
# never green locally and red in CI on the very formatting the local run applied.
# Cheap (sub-second, no toolchain), so it is NOT deferred.
sched_gate PY-LINT desc="ruff over scripts/*.py via scripts/run-pylint.sh (local: apply, CI: check)" \
    -- bash "$PORT_ROOT/scripts/run-pylint.sh" ${CI:+--check}

# --native-names is load-bearing, not optional. port_surface.json holds the FOLDED
# surface (reference spellings), so without the native-name sidecar every accessor
# or options-struct member the enumerator folds becomes unresolvable in this crate's
# own docs — which are correct, compiling code. See build_native_names() in
# scripts/enumerate_surface.py.
sched_gate DOC-AUDIT res=surface desc="audit_docs vs port_surface.json (+ native-name sidecar)" \
    -- python3 "$PORTING_SDK_DIR/scripts/audit_docs.py" \
        --root "$PORT_ROOT" \
        --surface "$PORT_ROOT/port_surface.json" \
        --native-names "$PORT_ROOT/port_surface_native.json" \
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

sched_gate DOC-LANG-PURITY res=dayone desc="no python-verbatim docs in a non-python port" \
    -- python3 "$PORTING_SDK_DIR/scripts/doc_lang_purity.py" --port rust --repo .
sched_gate DOC-LINKS res=dayone desc="every relative markdown link resolves to a tracked file" \
    -- python3 "$PORTING_SDK_DIR/scripts/doc_links.py" --port rust --repo .

sched_gate README-INCLUDE res=dayone desc="doc code blocks are byte-identical to their gate-compiled fixture regions" \
    -- python3 "$PORTING_SDK_DIR/scripts/readme_include.py" --port rust --repo .
sched_gate ROOT-HYGIENE res=dayone desc="no audit/scratch clutter tracked at repo root (allowlist ROOT_HYGIENE_ALLOW.md)" \
    -- python3 "$PORTING_SDK_DIR/scripts/root_hygiene.py" --port rust --repo .
sched_gate IGNORE-LEDGER-VERIFY res=dayone desc="no laundered false-absence entries in DOC_AUDIT_IGNORE.md (strict: reason/approver/date required)" \
    -- python3 "$PORTING_SDK_DIR/scripts/ignore_ledger_verify.py" --port rust --repo . --require-fields
sched_gate META-CONSISTENT tier=nightly res=dayone desc="package metadata consistency" \
    -- python3 "$PORTING_SDK_DIR/scripts/meta_consistent.py" --port rust --repo .
sched_gate ARTIFACT-DENY res=dayone desc="no porting artifacts in the PUBLISHED package (authoritative listing)" \
    --fn dayone_artifact_deny

# WIRED-MODES (plan §1.6 / D7) — merge-coherence guard. WIRED_MODES.md declares the
# load-bearing ENV/MODE lines this run-ci MUST keep (MOCK_RELAY_STRICT / REST strict
# default / assert_no_wire_violations). The check greps run-ci for each and fails
# loud if a merge silently drops one — the guard the strict-mocks × Part-5 merge race
# proved we need (a dropped strict export makes a gate green-and-vacuous).
# GATE-INVENTORY NOTE (§2.16): this gate has no per-port allowlist; the manifest IS
# the checked-in declaration. A retired mode requires editing WIRED_MODES.md in the
# same change with a reason. Self-tested: removing any declared line reds it (D6).
sched_gate WIRED-MODES res=dayone desc="load-bearing run-ci modes declared in WIRED_MODES.md are all present (merge-coherence guard)" \
    -- python3 "$PORTING_SDK_DIR/scripts/check_wired_modes.py" --port rust --repo .

# ---- expansion gates (backlog burned to zero; now enforcing) -----------------
sched_gate GEN-TYPE-DEGENERACY desc="generated typed I/O is not degenerate (modulo GEN_TYPE_DEGENERACY_ALLOW.md)" \
    -- python3 "$PORTING_SDK_DIR/scripts/gen_type_degeneracy.py" --port rust --repo .
sched_gate PUBLIC-JARGON desc="no porting/internal jargon leaked into public docs/identifiers" \
    -- python3 "$PORTING_SDK_DIR/scripts/public_jargon.py" --port rust --repo .
sched_gate ROUTE-COLLISION desc="no duplicate CRUD base / split route classes (route-registry × modulo ROUTE_COLLISION_ALLOW.md)" \
    --fn route_collision_gate
sched_gate GEN-IDIOM desc="generated code is not lint-excluded (idiom parity with hand-written)" \
    -- python3 "$PORTING_SDK_DIR/scripts/gen_idiom.py" --port rust --repo .
sched_gate RELEASE-FRESH desc="publish path is gated (gates-before-publish); release freshness" \
    -- python3 "$PORTING_SDK_DIR/scripts/release_fresh.py" --port rust --repo .

# SEMVER-DIFF — the version bump must match the public-API change. Floor is the
# committed port_signatures.baseline.json (baseline_version 3.0.0); the current
# generated surface must not regress it (or the version must major-bump). deps on
# SIGNATURES so it diffs the freshly-regenerated port_signatures.json.
# WAVE-1: report-only in-wave (owner-FINAL, re-anchor at cut, D5). GATE_ENFORCEMENT_PLAN.md
# D5a defers the version-line decision to the real release — "no bump churn now;
# perl/rust 4.0.0 declarations stay as-is; unified-vs-per-port decided at cut time" — so
# an intentional in-wave breaking change must REPORT rather than block. Eight ports get
# this hold via the SURFACE suite (_surface_commands.py passes semver_report_only=True);
# rust and python schedule SEMVER-DIFF standalone and so must pass the flag here.
sched_gate SEMVER-DIFF deps=SIGNATURES desc="version bump matches the public-API surface change vs the committed baseline floor" \
    -- python3 "$PORTING_SDK_DIR/scripts/semver_diff.py" --port rust --repo "$PORT_ROOT" --report-only

# ---- §D1 packaging -----------------------------------------------------------
# PACKAGE-SMOKE builds+installs+imports the real published artifact (cargo build
# + install + a smoke that constructs RestClient). ~heavy → defer.
sched_gate PACKAGE-SMOKE tier=nightly defer=1 desc="published crate builds, installs, and imports (real artifact smoke)" \
    -- python3 "$PORTING_SDK_DIR/scripts/package_smoke.py" --port rust --repo .

# PERF-BASELINE — nightly SWML-render (P2) ratchet vs committed perf_baseline.json
# (release bench build → heavy → nightly-only, deferred behind the cheap wave).
sched_gate PERF-BASELINE tier=nightly defer=1 desc="SWML 20-verb render µs/doc within the ratchet threshold vs perf_baseline.json (r5 P2)" \
    --fn perf_baseline_gate

# ---- §G anti-laundering ledger -----------------------------------------------
sched_gate SUPPRESSION-LEDGER res=dayone desc="no un-ledgered broad analyzer suppressions (#![allow] modulo SUPPRESSION_LEDGER.md)" \
    -- python3 "$PORTING_SDK_DIR/scripts/suppression_ledger.py" --port rust --repo .

# ---- §C1 doc/example execution gates -----------------------------------------
# SNIPPET-COMPILE (~29s, typecheck WITH the real crate) is cheap → blocking.
# DOC-CLI line-detects swaig-test invocations (no built binary to probe) → cheap,
# blocking. EXAMPLES-RUN + SNIPPET-RUN self-skip for a compiled port (no cargo
# run target / non-dynamic) — SNIPPET-COMPILE covers them — but are wired the
# same as python (defer=1) so the gate graduates automatically if a run target
# is added.
sched_gate SNIPPET-COMPILE tier=nightly desc="documented code snippets compile against the real crate" \
    -- python3 "$PORTING_SDK_DIR/scripts/snippet_compile.py" --port rust --repo .

sched_gate DOC-CLI desc="documented swaig-test invocations parse against the real CLI" \
    -- python3 "$PORTING_SDK_DIR/scripts/doc_cli.py" --port rust --repo .

# Wave-3 doc/API-truth gates — deterministic source/doc analysis (no build, no
# mock, ~1.3s for all six). Per-PR tier: cheap enough to catch doc/API drift at
# PR time rather than a day later in nightly.
sched_gate ERROR-ENVELOPE desc="REST error carries the full (status,body,url,method) envelope + raised on >=400" \
    -- python3 "$PORTING_SDK_DIR/scripts/error_envelope.py" --port rust --repo "$PORT_ROOT"
sched_gate DEAD-PUBLIC-ERROR desc="exported error types are raised/caught/user-signalled (no dead error surface)" \
    -- python3 "$PORTING_SDK_DIR/scripts/dead_public_error.py" --port rust --repo "$PORT_ROOT"
sched_gate PAGINATION-WIRED desc="shipped iterator-protocol paginator is wired into list()" \
    -- python3 "$PORTING_SDK_DIR/scripts/pagination_wired.py" --port rust --repo "$PORT_ROOT"
sched_gate DOC-ENV desc="documented SIGNALWIRE_*/SWML_* env vars <=> code-read vars agree" \
    -- python3 "$PORTING_SDK_DIR/scripts/doc_env.py" --port rust --repo "$PORT_ROOT"
sched_gate COUNT-CLAIM desc="numeric doc claims (skills/namespaces) match reality" \
    -- python3 "$PORTING_SDK_DIR/scripts/count_claim.py" --port rust --repo "$PORT_ROOT"
sched_gate ACCESSOR-TRUTH desc="documented backtick method() refs exist in source" \
    -- python3 "$PORTING_SDK_DIR/scripts/accessor_truth.py" --port rust --repo "$PORT_ROOT"

# ---- gate-enforcement quartet (§2.1-2.4) -------------------------------------
# DOC-WIRE (§2.1) — the wire SHAPE emitted by the documented REST examples must be
# spec-clean. doc_wire.py spawns the mock in flag mode, exports MOCK_SIGNALWIRE_PORT,
# runs the doc_wire_dump example (which replays the README / rest/docs / rest/examples
# REST calls against the mock), then reads the mock's wire_violations journal. Per-PR
# (a single quick example run). rust's red list is empty (its wire keys already match
# the spec — `areacode`, not `area_code`; nested play params:{text}).
sched_gate DOC-WIRE desc="doc-example REST wire shapes emit no unknown-field/dup-id violations against the strict mock" \
    -- python3 "$PORTING_SDK_DIR/scripts/doc_wire.py" --port rust --repo "$PORT_ROOT" \
        --runner "cargo run --quiet --example doc_wire_dump"

# STATUS-CLAIM (§2.3) — doc status phrases ("not implemented", "no … adapter",
# "pending", …) must match shipped reality. Per-PR (cheap, deterministic doc/source
# scan) so a false status claim is caught at PR time. rust's red list was clean once
# the rest/README + docs/sdk_features "reqwest" claims were corrected to "ureq".
sched_gate STATUS-CLAIM res=surface desc="doc status claims (not-implemented/adapter/pending) match shipped reality" \
    -- python3 "$PORTING_SDK_DIR/scripts/status_claim.py" --port rust --repo "$PORT_ROOT" \
        --surface "$PORT_ROOT/port_surface.json"

# WAIT-LIVENESS (§2.4) — the RELAY Action::wait() liveness contract: wait() BLOCKS
# until the deferred completing event arrives, then returns with the finished state
# (never a no-op that returns at t~=0, never a hang). examples/wait_liveness_dump.rs
# drives a real mock_relay, arms deferred completing events, drives Action::wait, and
# emits the liveness classification; the differ compares it to the python golden.
# Real-time behavioral check → tier=nightly (deferred behind the cheap wave). rust
# passes all three cases (play / record / nested re-entrant).
# The python oracle is auto-resolved by the differ (no --python-sdk flag; it
# defaults to the adjacent/installed signalwire package — the sibling checkout in
# CI), exactly as the BEHAVIORAL-* / EMISSION gates above do.
# §2.9: pass --prebuild-cmd so the differ compiles the dump example UNTIMED before
# it starts the liveness wall-clock deadline — a cold cargo build must NOT be
# charged against the ~40s deadline (that reded rust's nightly 07-17/18). The
# prebuild above also warms it, but wiring --prebuild-cmd makes the exclusion
# gate-local and independent of gate ordering.
sched_gate WAIT-LIVENESS tier=nightly defer=1 desc="RELAY Action::wait() blocks-until-event liveness matches the python golden" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_wait_liveness.py" --port rust \
        --prebuild-cmd "cargo build --quiet --example wait_liveness_dump" \
        --dump-cmd "cargo run --quiet --example wait_liveness_dump"

# SECRET-SCRUB-LIVE (PSDK-5) — the BEHAVIORAL leg of the credential-hygiene
# contract, complementing the STATIC source grep. examples/secret_scrub_dump.rs
# drives a real RELAY connect + an inbound authorization.state re-auth frame at
# SIGNALWIRE_LOG_LEVEL=debug with the corpus sentinels, captures the SDK's OWN
# debug output (via a child re-exec, since Logger writes to fd 2), and classifies
# each sentinel {leaked}. The differ compares against the python golden (all
# false). A static grep cannot express this: the leak we found here was NOT at a
# frame-log site — it was the project id echoed back inside the connect response's
# `identity` field, which survives a key-shape-only scrub.
# Live debug-level relay drive → tier=nightly (per the enterprise report's
# "static per-PR + behavioral nightly"), deferred behind the cheap wave.
# --prebuild-cmd keeps a cold cargo build off the differ's wall-clock deadline,
# same rationale as WAIT-LIVENESS above.
sched_gate SECRET-SCRUB-LIVE tier=nightly defer=1 desc="no credential sentinel reaches the debug log on a live RELAY connect + re-auth drive" \
    -- python3 "$PORTING_SDK_DIR/scripts/diff_port_secret_scrub.py" --port rust \
        --prebuild-cmd "cargo build --quiet --example secret_scrub_dump" \
        --dump-cmd "cargo run --quiet --example secret_scrub_dump"

# STRICT-MOCKS (§2.2) — nightly re-run of the RELAY suite with MOCK_RELAY_STRICT=1.
sched_gate STRICT-MOCKS tier=nightly defer=1 desc="RELAY suite passes with the mock in 400-on-violation strict mode (MOCK_RELAY_STRICT=1)" \
    --fn strict_mocks_gate

# STRICT-MOCKS: MOCK_RELAY_STRICT=1 for parity with TEST/STRICT-MOCKS above (both
# gates self-skip for rust today — compiled port, no cargo run target for
# examples/snippets — but stay wired so they graduate automatically if a run
# target is added, exactly like the reference).
sched_gate SNIPPET-RUN tier=nightly defer=1 desc="dynamic-port doc snippets run to a zero exit against the mock (compiled port: self-skips; STRICT-MOCKS: MOCK_RELAY_STRICT=1)" \
    -- env MOCK_RELAY_STRICT=1 python3 "$PORTING_SDK_DIR/scripts/snippet_run.py" --port rust --repo . --report-only

sched_gate EXAMPLES-RUN tier=nightly defer=1 desc="shipped examples load/start against the mock (compiled port: self-skips; STRICT-MOCKS: MOCK_RELAY_STRICT=1)" \
    -- env MOCK_RELAY_STRICT=1 python3 "$PORTING_SDK_DIR/scripts/examples_run.py" --port rust --repo .

# DOC-SURFACE — rustdoc coverage floor on the public surface (pub items, excluding
# pub(crate)). The floor is pinned in .doc_surface_floor and ratchets up via
# --write-floor. BLOCKING and pinned at 100.0 as of the 2026-07-29 burn: every public
# item carries a `///`, so a new undocumented one is a real regression, not a note.
# Cheap (a pure text scan, no build), so per-PR rather than nightly.
sched_gate DOC-SURFACE desc="public rustdoc coverage floor (.doc_surface_floor ratchet; 100% — blocking)" \
    -- python3 "$PORTING_SDK_DIR/scripts/doc_surface.py" --port rust --repo .

sched_run
rc=$?
if [ "$rc" -eq 0 ]; then
    echo "==> CI PASS"
else
    echo "==> CI FAIL (gates:$FAILED_GATES )"
fi
exit "$rc"
