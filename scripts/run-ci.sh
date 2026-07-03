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
# Gates (in order, fail-fast):
#   1. cargo test --tests                        — language test runner (parallel)
#   2. signature regen (rustdoc + adapter)      — python adapter
#   3. drift gate                               — porting-sdk diff_port_signatures.py
#   4. surface-fresh gate                       — porting-sdk check_surface_freshness.py
#   5. no-cheat gate                            — porting-sdk audit_no_cheat_tests.py
#   6. emission gate                            — porting-sdk diff_port_emission.py
#   7. fmt gate                                 — rustfmt (local: auto-fix; CI: --check)
#   8. lint gate                                — cargo clippy (Cargo.toml [lints] deny)
#   9. doc-audit gate                           — porting-sdk audit_docs.py
#  10. surface-diff gate                        — porting-sdk diff_port_surface.py
#
# Gates 7-10 were previously CI-only or unenforced (FMT not gated anywhere;
# clippy [lints] table unenforced; separate doc-audit.yml / surface-audit.yml
# workflows); folding them in restores the "run-ci.sh is canonical, CI just
# invokes it — no drift local vs CI" design. FMT + LINT are the Rust-internal
# source-quality pair (governed by PORT_PHILOSOPHY_RUST.md, not the parity
# gates); DOC-AUDIT + SURFACE-DIFF are cross-port parity checks.

set -u
set -o pipefail

PORT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PORT_NAME="signalwire-rust"

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

FAILED_GATES=""

run_gate() {
    local name="$1"; shift
    local description="$1"; shift
    local logfile
    logfile="$(mktemp)"
    "$@" >"$logfile" 2>&1
    local rc=$?
    if [ "$rc" -eq 0 ]; then
        echo "[$name] $description ... PASS"
        rm -f "$logfile"
        return 0
    fi
    echo "[$name] $description ... FAIL: exit $rc"
    sed 's/^/    /' "$logfile" | tail -40
    rm -f "$logfile"
    FAILED_GATES="$FAILED_GATES $name"
    return $rc
}

cd "$PORT_ROOT"

echo "==> running CI gates for $PORT_NAME (porting-sdk at $PORTING_SDK_DIR)"

# Gate 1: TEST — the language test runner. Delegates to the canonical
# scripts/run-tests.sh (cargo test --tests, PARALLEL — cargo's default). The
# mock-backed suites are session-isolated (relay: per-connection handshake
# `sessionid`; rest: per-test random project => unique Authorization header), so
# the shared mock servers are safe under concurrency without `--test-threads=1`.
# The few env-coupled unit tests serialize among themselves with a file-local
# lock. The script self-bootstraps the toolchain and runs from any CWD.
run_gate "TEST" "cargo test --tests (parallel) via scripts/run-tests.sh" \
    bash "$PORT_ROOT/scripts/run-tests.sh"

# Gate 1b: GEN-FRESH — the generated REST layer (src/rest/namespaces/generated/
# *_resources_generated.rs + client_tree_generated.rs + mod.rs + the adapter
# sidecar rest_signatures.json) must be in lockstep with the canonical specs +
# x-sdk-* markup. Fails if a spec/markup change wasn't regenerated (stale) or a
# generated file was hand-edited. Same shape as the go/ts REST GEN-FRESH gate.
run_gate "GEN-FRESH" "generated REST layer matches the canonical specs (generate_rest.py --check)" \
    python3 scripts/generate_rest.py --check

# Gate 1c–1e: GEN-FRESH for the READ-side typed-payload trees (SESSION_CHANGESET
# item D/H/I). Each must be in lockstep with its vendored spec source:
#   SWML-verb config types (src/swml/swml_verbs_generated.rs) <- schema.json $defs
#   RELAY-protocol wire types (src/relay/protocol_types_generated.rs) <- relay-protocol/*.json
#   SWAIG payloads (src/swaig/{post_prompt,swaig_request,swaig_actions}_generated.rs) <- swaig-specs/
# (the swml/swaig generators also emit the gen-payload signature sidecars the
# adapter consumes; GEN-FRESH gates those JSON files too). Fails if a spec changed
# without regenerating or the tree was hand-edited.
run_gate "GEN-FRESH-SWML" "generated SWML-verbs config tree matches schema.json (\$defs)" \
    python3 scripts/generate_swml_verbs.py --check

run_gate "GEN-FRESH-RELAY" "generated RELAY-protocol tree matches relay-protocol/*.json" \
    python3 scripts/generate_relay_protocol.py --check

run_gate "GEN-FRESH-SWAIG" "generated SWAIG payload tree matches swaig-specs/" \
    python3 scripts/generate_swaig_payloads.py --check

# Gate 1g: GEN-FRESH-TESTS — the generated full-mock REST wire-test suite
# (tests/rest_generated_<spec>.rs) must be in lockstep with the route-registry ×
# spec-operationId oracle. The generator captures the call plan from the REAL
# client (the rest-test-plan binary), joins it to the canonical spec operationIds,
# and emits one success + one error test per implemented route. Fails if a
# spec/markup/SDK change wasn't regenerated (stale) or a generated file was
# hand-edited (they carry a DO-NOT-EDIT header). Same shape as the ruby/php/go/ts
# REST test GEN-FRESH gate.
run_gate "GEN-FRESH-TESTS" "generated REST wire-test suite matches the route-registry × spec oracle (generate_rest_tests.py --check)" \
    python3 scripts/generate_rest_tests.py --check

# Gate 1f: SWAIG-COVERAGE — every engine response action in the vendored
# swaig-specs/swaig-response.yaml must be emittable by this port's FunctionResult
# (or signed off in porting-sdk/SWAIG_COVERAGE_ALLOWLIST.md). The shared checker's
# _sdk_emits_rust scraper captures the top-level keys of each
# `self.actions.push(json!({ 'key': … }))` — landing at 25 of the 27 engine actions
# (the 2 gaps, back_to_back_functions + user_event, are the shared signed-off
# allowlist), matching go/ts/php/ruby.
run_gate "SWAIG-COVERAGE" "every engine SWAIG action emittable (modulo allowlist)" \
    python3 "$PORTING_SDK_DIR/scripts/swaig_coverage.py" --check \
        --emission "$PORT_ROOT/src/swaig/function_result.rs"

# Gate 2: signature regen — adapter shells out to rustdoc nightly internally.
run_gate "SIGNATURES" "regenerate port_signatures.json (rustdoc + adapter)" \
    python3 scripts/enumerate_signatures.py

# Gate 3: drift gate
run_gate "DRIFT" "diff_port_signatures vs python reference" \
    python3 "$PORTING_SDK_DIR/scripts/diff_port_signatures.py" \
        --reference "$PORTING_SDK_DIR/python_signatures.json" \
        --port-signatures "$PORT_ROOT/port_signatures.json" \
        --surface-omissions "$PORT_ROOT/PORT_OMISSIONS.md" \
        --surface-additions "$PORT_ROOT/PORT_ADDITIONS.md" \
        --omissions "$PORT_ROOT/PORT_SIGNATURE_OMISSIONS.md"

# Gate 4: surface-fresh — close the Layer-B-not-gated hole. The DRIFT gate
# (Gate 3) only polices Layer A (port_signatures.json); port_surface.json can
# silently rot when a public symbol is added but the surface isn't regenerated.
# Regenerate the surface IN PLACE via the rust enumerator (regex over src/**.rs;
# no rustdoc needed, so it's independent of the SIGNATURES gate's cache), then
# compare the committed copy against the regen MODULO the volatile `generated_from`
# git-sha. We snapshot HEAD's committed surface first and ALWAYS restore the file
# afterward (the regen rewrites it in place, only bumping the sha line).
surface_fresh_gate() {
    git show HEAD:port_surface.json > /tmp/committed_surface.json 2>/dev/null \
        || cp "$PORT_ROOT/port_surface.json" /tmp/committed_surface.json
    # enumerate_surface.py writes port_surface.json directly (default --output).
    python3 scripts/enumerate_surface.py || { git checkout -- port_surface.json; return 1; }
    python3 "$PORTING_SDK_DIR/scripts/check_surface_freshness.py" \
        --committed /tmp/committed_surface.json \
        --fresh "$PORT_ROOT/port_surface.json"
    local rc=$?
    git checkout -- port_surface.json
    return $rc
}
run_gate "SURFACE-FRESH" "check_surface_freshness vs committed port_surface.json" \
    surface_fresh_gate

# Gate 5: no-cheat
run_gate "NO-CHEAT" "audit_no_cheat_tests" \
    python3 "$PORTING_SDK_DIR/scripts/audit_no_cheat_tests.py" --root "$PORT_ROOT"

# Gate 5b: REST-COVERAGE — every canonical REST route the SDK implements must be
# exercised with BOTH a success (2xx) AND an error (4xx/5xx) response on the
# correct on-the-wire path (parity). Measured by replaying the mock journal of the
# REST coverage suites through porting-sdk's rest_coverage checker. Accepted gaps —
# routes with no SDK method, malformed canonical routes, mock-router collisions —
# are allowlisted: the shared baseline (porting-sdk/REST_COVERAGE_BASELINE.md) +
# this port's REST_COVERAGE_GAPS.md. A stale entry (route now covered) fails the
# gate. Self-contained: spins its own mock, runs the coverage suites serially
# (--test-threads=1) so all traffic lands in one journal, then checks it. Same
# shape as python's/java's/typescript's/go's gate.
# Pick a free TCP port on 127.0.0.1: ask the OS for an ephemeral port (bind :0),
# read it back, release it. Never reuse a hardcoded port — concurrent runs (and
# leftover mocks) otherwise collide on a fixed port and the gate hangs forever
# waiting on a health check it can't satisfy.
pick_free_port() {
    python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1]); s.close()'
}
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
    # Wait for readiness; if the mock process dies (e.g. the OS-picked port was
    # grabbed in the race window), fail LOUD with the cause — never hang.
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
    # The GENERATED wire-test suites (tests/rest_generated_<spec>.rs) are the
    # authoritative REST coverage source: one success + one error test per
    # implemented route, emitted from the route-registry × spec-operationId oracle
    # (see scripts/generate_rest_tests.py). Run every generated namespace target
    # serially (--test-threads=1) so all traffic lands in one journal for the
    # coverage checker. The prior hand-written rest_*_coverage suites were deleted
    # once the generated suites reached coverage parity (item E).
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
run_gate "REST-COVERAGE" "every implemented REST route covered success+error (parity + allowlist)" \
    rest_coverage_gate

# Gate 5c: SPEC-PARITY — the REST surface must match the canonical spec in BOTH
# directions. REST-COVERAGE (5b) proves every route the SDK *implements* is
# exercised; this proves the set the SDK implements EQUALS the canonical spec
# (modulo checked-in gaps): no canonical route left unimplemented (A−B), no
# implemented route that matches no canonical route (B−A, i.e. invented surface).
# Set B is produced deterministically by the route-registry binary, which builds
# a stub-backed RestClient, invokes every namespace method, and reads back the
# routes the SDK actually dispatched (no hand-authored route list, no reflection
# — Rust has none). diff_spec_implementation.py matches that against the spec.
# Accepted not-implemented gaps live in the shared SPEC_IMPLEMENTATION_GAPS.md;
# a stale gap (now implemented) or unsanctioned divergence fails the gate. Same
# shape as go's/java's gate.
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
run_gate "SPEC-PARITY" "implemented REST routes == canonical spec (modulo gaps); deterministic Set B" \
    spec_parity_gate

# Gate 6: emission — byte-compare the SWAIG FunctionResult serialisation against
# Python's to_dict() over the shared 81-entry corpus. The drift gate (Gate 3)
# polices the SURFACE; this one polices the EMISSION (action shape/keys/values +
# the to_dict() envelope), the bug class the §6 sweep proved is otherwise drift-0
# and invisible to CI. Pure serialisation — no mock servers, no network; needs
# only signalwire-python adjacent (already required) + the emit_corpus example.
# The dump program is examples/emit_corpus.rs (cargo run --example emit_corpus).
run_gate "EMISSION" "diff_port_emission vs python to_dict() oracle" \
    python3 "$PORTING_SDK_DIR/scripts/diff_port_emission.py" \
        --dump-cmd 'cargo run --quiet --example emit_corpus' \
        --port-repo "$PORT_ROOT"

# Gate 6b: skill-contract — the sibling of EMISSION for built-in SKILLS. EMISSION
# byte-compares FunctionResult serialisation; this compares each skill's SWAIG
# tool contract (name/parameters/required/enum from register_tools()) against
# the Python reference. Catches a class drift/surface/emission can't see: a wrong
# `required`, a renamed/retyped param, an extra/missing tool. The dump program is
# examples/emit_skills.rs (cargo run --example emit_skills); dynamic skills are
# excluded + logged by the shared corpus. Same prereqs as EMISSION.
run_gate "SKILL-CONTRACT" "diff_skill_contracts vs python reference" \
    python3 "$PORTING_SDK_DIR/scripts/diff_skill_contracts.py" \
        --dump-cmd 'cargo run --quiet --example emit_skills' \
        --port-repo "$PORT_ROOT"

# Gate 7: FMT — the language format gate (rust: rustfmt). Delegates to the
# canonical scripts/run-format.sh (which self-bootstraps rustfmt and runs from any
# CWD). Governed by rustfmt.toml (style_edition 2024). Source-style only — proven
# surface/emission-neutral (a reformat leaves port_signatures.json byte-identical);
# a Rust-internal idiom gate, not parity.
#
# CI-AWARE behaviour (so the gate "just fixes it" locally but still guards CI):
#   * LOCAL ($CI unset)  → `run-format.sh` (apply): silently reformats your working
#     tree. No error, no manual step — you never have to run cargo fmt by hand. If
#     it rewrote anything we still PASS (the files are now clean) but the script
#     prints a note so you know to stage them.
#   * CI ($CI=true)      → `run-format.sh --check`: read-only safety net that FAILS
#     if unformatted code reached CI (a committer who didn't run run-ci.sh).
run_gate "FMT" "rustfmt via scripts/run-format.sh (local: auto-fix; CI: --check)" \
    bash "$PORT_ROOT/scripts/run-format.sh" ${CI:+--check}

# Gate 8: LINT — the language lint gate (rust: clippy). Delegates to the canonical
# scripts/run-lint.sh (which self-bootstraps clippy and runs from any CWD). Here
# that is clippy: Cargo.toml [lints.clippy] denies `all` + `pedantic` (with the
# documented per-lint allows), so any new finding is an `error`. `-D warnings`
# also promotes rustc warnings. `--all-targets` covers lib + bins + tests +
# examples (the same scope the burn-down cleared).
run_gate "LINT" "cargo clippy --all-targets via scripts/run-lint.sh" \
    bash "$PORT_ROOT/scripts/run-lint.sh"

# Gate 9: doc-audit — every method/class referenced in docs/ + examples/ fenced
# code blocks must resolve to a real symbol in port_surface.json (catches
# phantom-API doc promises). Mirrors .github/workflows/doc-audit.yml exactly.
# Uses the COMMITTED port_surface.json — Gate 4 already proved it's fresh.
run_gate "DOC-AUDIT" "audit_docs vs port_surface.json" \
    python3 "$PORTING_SDK_DIR/scripts/audit_docs.py" \
        --root "$PORT_ROOT" \
        --surface "$PORT_ROOT/port_surface.json" \
        --ignore "$PORT_ROOT/DOC_AUDIT_IGNORE.md"

# Gate 10: surface-diff — diff the port surface against the Python reference
# (omissions/additions accounted for in PORT_OMISSIONS.md / PORT_ADDITIONS.md).
# Gate 4 only checks the committed surface is FRESH (matches a regen); this
# checks it MATCHES PYTHON. Mirrors .github/workflows/surface-audit.yml.
run_gate "SURFACE-DIFF" "diff_port_surface vs python_surface.json" \
    python3 "$PORTING_SDK_DIR/scripts/diff_port_surface.py" \
        --reference "$PORTING_SDK_DIR/python_surface.json" \
        --port-surface "$PORT_ROOT/port_surface.json" \
        --omissions "$PORT_ROOT/PORT_OMISSIONS.md" \
        --additions "$PORT_ROOT/PORT_ADDITIONS.md"

# SWAIG-CLI — lightweight shared swaig-test mini-contract (NOT python parity;
# python's in-process simulator surface is reference-only). Black-box: invokes
# `cargo run --bin swaig-test --help` + golden invocations and asserts the shared
# verbs are documented and no-action errors (the cross-port majority default).
# Rust has no --simulate-serverless, so the no-serverless clause asserts the flag
# is rejected as an unknown option (no half-accept). --quiet keeps cargo's build
# chatter out of the captured help text.
run_gate "SWAIG-CLI" "swaig-test shared mini-contract (verbs/serverless-reject/default-action)" \
    python3 "$PORTING_SDK_DIR/scripts/audit_swaig_cli_contract.py" \
        --port rust \
        --cmd "cargo run --quiet --bin swaig-test --" \
        --require-url-model \
        --default-action-argv='--url|http://user:pass@127.0.0.1:1/' \
        --no-serverless-argv='--url|http://user:pass@127.0.0.1:1/|--simulate-serverless|lambda|--list-tools'

if [ -z "$FAILED_GATES" ]; then
    echo "==> CI PASS"
    exit 0
else
    echo "==> CI FAIL (gates:$FAILED_GATES )"
    exit 1
fi
