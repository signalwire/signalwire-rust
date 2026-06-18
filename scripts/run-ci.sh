#!/usr/bin/env bash
# run-ci.sh — canonical local-and-CI gate runner for signalwire-rust.
#
# Same script invoked locally (`bash scripts/run-ci.sh`) AND by the
# GitHub Actions workflow. No drift between local and CI behavior.
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

# Gate 1: cargo test (PARALLEL — cargo's default). The mock-backed suites are
# session-isolated (relay: per-connection handshake `sessionid`; rest: per-test
# random project => unique Authorization header), so the shared mock servers are
# safe under concurrency without `--test-threads=1`. The few env-coupled unit
# tests serialize among themselves with a file-local lock. No cross-test serial
# crutch, no cross-binary flock.
run_gate "TEST" "cargo test --tests (parallel)" \
    cargo test --tests

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

# Gate 7: FMT — the language format gate (rust: rustfmt). Canonical gate name is
# language-neutral (FMT); each port runs its own formatter under it. Governed by
# rustfmt.toml (style_edition 2024). Source-style only — proven surface/emission-
# neutral (a reformat leaves port_signatures.json byte-identical); a Rust-internal
# idiom gate, not parity.
#
# CI-AWARE behaviour (so the gate "just fixes it" locally but still guards CI):
#   * LOCAL ($CI unset)  → `cargo fmt --all` in FIX mode: silently reformats your
#     working tree. No error, no manual step — you never have to run cargo fmt by
#     hand. If it had to rewrite anything, we still PASS (the files are now clean)
#     but print a note so you know to stage them.
#   * CI ($CI=true)      → `cargo fmt --all -- --check`: read-only safety net that
#     FAILS if unformatted code reached CI (a committer who didn't run run-ci.sh).
# Pinned to +stable: the SIGNATURES gate installs nightly (rustdoc-json) which can
# become the default toolchain and may lack rustfmt/clippy; +stable is robust.
fmt_gate() {
    if [ -n "${CI:-}" ]; then
        cargo +stable fmt --all -- --check
    else
        cargo +stable fmt --all
        local rc=$?
        if [ "$rc" -eq 0 ]; then
            # fmt rewrites in place and exits 0 whether or not it changed files;
            # surface any reformatting so the dev knows to `git add` it.
            if ! git diff --quiet 2>/dev/null; then
                echo "    (FMT auto-applied formatting to your working tree — review & stage)"
            fi
        fi
        return $rc
    fi
}
run_gate "FMT" "rustfmt (local: auto-fix; CI: --check)" fmt_gate

# Gate 8: LINT — the language lint gate (rust: clippy). The canonical gate name
# is language-neutral (LINT); each port runs its own linter under it. Here that
# is clippy: Cargo.toml [lints.clippy] denies `all` + `pedantic` (with the
# documented per-lint allows), so any new finding is an `error`. `-D warnings`
# also promotes rustc warnings. `--all-targets` covers lib + bins + tests +
# examples (the same scope the burn-down cleared).
run_gate "LINT" "cargo clippy --all-targets (lint gate)" \
    cargo +stable clippy --all-targets --all-features -- -D warnings

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

if [ -z "$FAILED_GATES" ]; then
    echo "==> CI PASS"
    exit 0
else
    echo "==> CI FAIL (gates:$FAILED_GATES )"
    exit 1
fi
