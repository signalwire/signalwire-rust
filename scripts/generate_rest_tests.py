#!/usr/bin/env python3
"""Generate the full-mock REST wire-test suite for signalwire-rust.

This is the Rust realisation of porting-sdk/REST_TEST_GENERATOR_RULES.md (the
portable REST *test* generator; reference:
generate_python_rest_types.py::generate_rest_tests, mirrors:
signalwire-ruby/scripts/generate_rest_tests.py + signalwire-php + signalwire-go +
signalwire-typescript). For every REST route the SDK actually implements it
emits, into tests/rest_generated_<spec>.rs (one integration-test target per
namespace, directly under tests/ — cargo only auto-discovers .rs files at the
top level of tests/, never a subdirectory, so a tests/rest/generated/ layout
would silently never run):

  - a SUCCESS test: call the real SDK method against the shared mock_signalwire
    harness (common::mocktest::client), assert the mock journaled the expected
    (method, matched_route);
  - an ERROR test: arm a 500 for that route, assert the SDK returns
    SignalWireRestError with .status_code() == 500.

The assertion oracle is INDEPENDENT of the resource generator (RULES §1):
  - the (method, path) to call + the call expression (chain, member, sentinel
    args) come from the call plan (src/bin/rest_test_plan.rs — captured from the
    REAL client via the recording StubTransport), NOT re-derived here;
  - the matched_route to assert comes from the OpenAPI operationId
    (<spec_dir>.<operationId>) — the same value the mock derives its route table
    from. A generated test therefore catches SDK-vs-contract drift, not a
    generator self-snapshot.

Inputs joined by (METHOD, normalized-path) (RULES §2): the plan's per-route call
entries (path params already {id}) × the spec operationIds (spec path normalized
the SAME way before the join). Routing collisions are resolved
longest-template-wins (RULES §7) so the asserted route is the one the mock
ACTUALLY journals (e.g. GET /rooms/{id} vs GET /rooms/{name}).

Call args are sentinel-faithful and type-correct BY CONSTRUCTION (RULES §4/§6):
rest_test_plan.rs emits, per required param, the exact Rust literal the generated
call needs — a path id -> "x", a query map -> &std::collections::HashMap::new(),
a body Value -> &serde_json::json!({}), a generated request struct ->
Mod::XRequest::new(<required sentinels>). Proven type-correct by the plan binary
itself compiling and dispatching every call with zero capture errors.

GEN-FRESH: `--check` reproduces the committed *_generated.rs and exits non-zero
if any file differs. Resolves porting-sdk via $PORTING_SDK or sibling.

Usage:
    python3 scripts/generate_rest_tests.py           # (re)write the test files
    python3 scripts/generate_rest_tests.py --check   # GEN-FRESH: fail if stale
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover
    sys.stderr.write("generate_rest_tests.py requires PyYAML (pip install pyyaml)\n")
    raise


# ---------------------------------------------------------------------------
# Resolution.
# ---------------------------------------------------------------------------

def resolve_porting_sdk() -> Path:
    env = os.environ.get("PORTING_SDK")
    if env and (Path(env) / "rest-apis").is_dir():
        return Path(env).resolve()
    here = Path(__file__).resolve()
    for parent in here.parents:
        cand = parent.parent / "porting-sdk"
        if (cand / "rest-apis").is_dir():
            return cand.resolve()
    raise SystemExit(
        "generate_rest_tests.py: porting-sdk not found (set $PORTING_SDK or clone adjacent)"
    )


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


# ---------------------------------------------------------------------------
# 1. Capture from the real client (RULES §3) — the rest-test-plan binary.
#    Each entry: {method, path ({id}-normalized), chain, member, args}.
# ---------------------------------------------------------------------------

def load_plan() -> list[dict]:
    proc = subprocess.run(
        ["cargo", "run", "--quiet", "--bin", "rest-test-plan"],
        cwd=str(repo_root()),
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        sys.stderr.write(proc.stderr)
        raise SystemExit("rest-test-plan exited non-zero — plan capture incomplete")
    out = proc.stdout
    i = out.find("{")
    if i > 0:
        out = out[i:]
    data = json.loads(out)
    if data.get("errors"):
        for e in data["errors"]:
            sys.stderr.write(f"  plan capture error: {e}\n")
        raise SystemExit(
            f"rest-test-plan reported {len(data['errors'])} capture error(s) — plan incomplete"
        )
    return data["plan"]


# ---------------------------------------------------------------------------
# 2. The join — plan routes × spec operationIds by (method, normalized-path).
# ---------------------------------------------------------------------------

_BRACE = re.compile(r"\{[^}]+\}")


def norm_params(p: str) -> str:
    """Every {param} -> {id} (the plan already does this to its paths; do it to
    the spec path so renamed params — {token_id}, {name} — line up)."""
    return _BRACE.sub("{id}", p)


def wire_key(p: str) -> str:
    """Every {param} -> X: the wire-identical key used for collision ranking."""
    return _BRACE.sub("X", p)


def spec_prefix(doc: dict) -> str:
    url = ((doc.get("servers") or [{}])[0]).get("url", "")
    i = url.find("signalwire.com")
    return url[i + len("signalwire.com"):] if i >= 0 else ""


def spec_dirs_with_openapi(psdk: Path) -> list[str]:
    root = psdk / "rest-apis"
    out = [
        d.name
        for d in root.iterdir()
        if d.is_dir() and (d / "openapi.yaml").is_file()
    ]
    return sorted(out)


def build_index(psdk: Path, spec_dirs: list[str]) -> tuple[dict[str, str], dict[str, tuple[int, str]]]:
    """Return (op_by, wire_winner):
      op_by:       "METHOD normPath" -> <spec>.<operationId>   (a route exists)
      wire_winner: "METHOD wireKey"  -> (orig_len, <spec>.<operationId>)
                   the longest original template — the route the mock journals.
    """
    op_by: dict[str, str] = {}
    wire_winner: dict[str, tuple[int, str]] = {}
    verbs = ("get", "post", "put", "patch", "delete")

    for spec in spec_dirs:
        doc = yaml.safe_load((psdk / "rest-apis" / spec / "openapi.yaml").read_text())
        prefix = spec_prefix(doc)
        for path_key, body in (doc.get("paths") or {}).items():
            orig = prefix + path_key
            full = _BRACE.sub("{id}", orig)
            wk = _BRACE.sub("X", orig)
            for verb in verbs:
                op = body.get(verb)
                if not isinstance(op, dict):
                    continue
                op_id = op.get("operationId")
                if not op_id:
                    continue
                route = f"{spec}.{op_id}"
                op_by[f"{verb.upper()} {full}"] = route
                wkey = f"{verb.upper()} {wk}"
                cur = wire_winner.get(wkey)
                if cur is None or len(orig) > cur[0]:
                    wire_winner[wkey] = (len(orig), route)
    return op_by, wire_winner


def build_rows(plan: list[dict], op_by: dict[str, str],
               wire_winner: dict[str, tuple[int, str]]) -> tuple[list[dict], list[str]]:
    """One row per plan entry that has a spec op. Row carries the op_id the mock
    actually journals (longest-template winner). Entries with no spec op are
    coverage findings (returned separately), not generator bugs."""
    rows: list[dict] = []
    uncovered: list[str] = []
    for e in plan:
        method = e["method"]
        np = norm_params(e["path"])
        if f"{method} {np}" not in op_by:
            uncovered.append(f"{'.'.join(e['chain'])}.{e['member']} ({method} {np})")
            continue
        winner = wire_winner.get(f"{method} {wire_key(e['path'])}")
        if winner is None:
            uncovered.append(f"{'.'.join(e['chain'])}.{e['member']} ({method} {np})")
            continue
        op_id = winner[1]
        spec = op_id[: op_id.index(".")]
        rows.append({
            "method": method,
            "path": np,
            "op_id": op_id,
            "spec": spec,
            "chain": e["chain"],
            "member": e["member"],
            "args": e["args"],
        })
    return rows, uncovered


# ---------------------------------------------------------------------------
# 3. Emit — one tests/rest/generated/<spec>_generated.rs per spec namespace.
# ---------------------------------------------------------------------------

def slug(chain: list[str], member: str) -> str:
    """A stable, unique-per-file test-method fragment from the call chain +
    member. e.g. (["video","rooms"], "list_streams") -> video_rooms_list_streams."""
    raw = "_".join([*chain, member])
    return re.sub(r"_+$", "", re.sub(r"[^A-Za-z0-9]+", "_", raw))


def call_expr(chain: list[str], member: str, args: list[str]) -> str:
    """The literal Rust call `c.ns().res().member(args)`."""
    accessors = "".join(f".{seg}()" for seg in chain)
    arglist = ", ".join(args)
    return f"c{accessors}.{member}({arglist})"


# Every generated file `use`s the full module-alias set the plan's arg literals
# reference (calling -> cg, fabric -> fabric_gen, …). Emitting all of them in
# every file keeps the generator per-spec-agnostic; `#![allow(unused_imports)]`
# tolerates the aliases a given namespace doesn't use. The alias names MUST match
# the tokens rest_test_plan.rs emits in its `args`.
HEADER_TMPL = """// Code generated by scripts/generate_rest_tests.py; DO NOT EDIT.
//
// AUTO-GENERATED full-mock REST wire tests for the '{spec}' namespace — regenerate:
//   python3 scripts/generate_rest_tests.py
//
// Each route the SDK implements (captured from the real client by the
// rest-test-plan binary, joined to the spec operationId) gets a SUCCESS test
// (call it, assert method + matched_route on the mock journal) and an ERROR test
// (arm a 500, assert SignalWireRestError with .status_code() == 500). The
// assertion oracle is the spec operationId — independent of the resource
// generator — so these catch SDK-vs-contract drift, not a generator self-snapshot.
// Full-mock harness fixtures (common::mocktest).

#![allow(unused_imports)]

#[path = "common/mod.rs"]
mod common;

use serde_json::json;
use signalwire::rest::namespaces::generated::calling_resources_generated as cg;
use signalwire::rest::namespaces::generated::chat_resources_generated as chat_gen;
use signalwire::rest::namespaces::generated::datasphere_resources_generated as datasphere_gen;
use signalwire::rest::namespaces::generated::fabric_resources_generated as fabric_gen;
use signalwire::rest::namespaces::generated::messages_resources_generated as messages_gen;
use signalwire::rest::namespaces::generated::project_resources_generated as project_gen;
use signalwire::rest::namespaces::generated::pubsub_resources_generated as pubsub_gen;
use signalwire::rest::namespaces::generated::relay_rest_resources_generated as relay_gen;
use signalwire::rest::namespaces::generated::video_resources_generated as video_gen;
"""


def rustfmt(src: str) -> str:
    """Run the emitted source through rustfmt (stdin/stdout) so the committed
    generated files are exactly what the FMT gate produces — otherwise a long
    call expression the generator emits on one line gets reflowed by `cargo fmt`
    and GEN-FRESH-TESTS vs FMT would perpetually disagree (RULES §6: generated
    tests are strict/format-clean BY CONSTRUCTION). Pinned to +stable to match
    run-ci.sh's FMT gate. Fails loud if rustfmt is unavailable or errors."""
    cmds = [
        ["rustup", "run", "stable", "rustfmt", "--edition", "2024", "--emit", "stdout"],
        ["rustfmt", "--edition", "2024", "--emit", "stdout"],
    ]
    last = None
    for cmd in cmds:
        try:
            last = subprocess.run(cmd, input=src, capture_output=True, text=True)
        except FileNotFoundError:
            continue
        if last.returncode == 0:
            return last.stdout
    if last is not None:
        sys.stderr.write(last.stderr)
    raise SystemExit("rustfmt failed on generated REST test source (need `rustup run stable rustfmt` or `rustfmt` on PATH)")


def emit_spec_file(spec: str, rows: list[dict]) -> str:
    body = HEADER_TMPL.format(spec=spec)
    for r in rows:
        name = r["_name"]
        call = r["_call"]
        method = r["method"]
        op_id = r["op_id"]
        body += f"""
#[test]
fn test_{name}_success() {{
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let _ = {call};
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "{method}");
    assert_eq!(e.matched_route.as_deref(), Some("{op_id}"));
}}

#[test]
fn test_{name}_error() {{
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("{op_id}", 500, json!({{"error": "x"}}));
    let err = {call}.expect_err("expected a 500 error");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("{op_id}"));
}}
"""
    return body


# ---------------------------------------------------------------------------
# Driver.
# ---------------------------------------------------------------------------

def build_outputs(psdk: Path) -> tuple[dict[str, str], list[str], int]:
    plan = load_plan()
    spec_dirs = spec_dirs_with_openapi(psdk)
    op_by, wire_winner = build_index(psdk, spec_dirs)
    rows, uncovered = build_rows(plan, op_by, wire_winner)

    by_spec: dict[str, list[dict]] = {}
    for row in rows:
        by_spec.setdefault(row["spec"], []).append(row)

    outs: dict[str, str] = {}
    n_routes = 0
    for spec in sorted(by_spec):
        srows = by_spec[spec]
        # Deterministic ordering: sort by (chain-join + member + method).
        srows.sort(key=lambda r: (".".join(r["chain"]), r["member"], r["method"]))
        used: set[str] = set()
        for r in srows:
            base = slug(r["chain"], r["member"])
            name = base
            k = 2
            while name in used:
                name = f"{base}_{k}"
                k += 1
            used.add(name)
            r["_name"] = name
            r["_call"] = call_expr(r["chain"], r["member"], r["args"])
            n_routes += 1
        fn = f"rest_generated_{spec.replace('-', '_')}.rs"
        outs[fn] = rustfmt(emit_spec_file(spec, srows))

    return outs, uncovered, n_routes


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="GEN-FRESH: exit non-zero if stale")
    ap.add_argument("--out", default="", help="scratch: emit into this dir")
    args = ap.parse_args(argv)

    psdk = resolve_porting_sdk()
    outs, uncovered, n_routes = build_outputs(psdk)

    out_dir = Path(args.out) if args.out else (repo_root() / "tests")

    if uncovered:
        sys.stderr.write(
            f"\nUNCOVERED ({len(uncovered)} plan route(s) with no spec op — coverage findings):\n"
        )
        for u in sorted(set(uncovered)):
            sys.stderr.write(f"  - {u}\n")

    if args.check:
        stale = []
        for fn, src in outs.items():
            p = out_dir / fn
            if not p.is_file() or p.read_text() != src:
                stale.append(str(p))
        expected = set(outs.keys())
        if out_dir.is_dir():
            for p in sorted(out_dir.glob("rest_generated_*.rs")):
                if p.name not in expected:
                    stale.append(f"{p} (leftover — not in generator output)")
        if stale:
            sys.stderr.write("GEN-FRESH FAIL: %d generated REST test file(s) stale:\n" % len(stale))
            for s in stale:
                sys.stderr.write(f"  - {s}\n")
            return 1
        total = sum(src.count("#[test]") for src in outs.values())
        print(
            f"GEN-FRESH: {len(outs)} generated REST test file(s) up to date "
            f"({total} tests, {n_routes} routes)."
        )
        return 0

    out_dir.mkdir(parents=True, exist_ok=True)
    expected = set(outs.keys())
    for p in sorted(out_dir.glob("rest_generated_*.rs")):
        if p.name not in expected:
            p.unlink()
    for fn, src in outs.items():
        (out_dir / fn).write_text(src)
    total = sum(src.count("#[test]") for src in outs.values())
    print(
        f"generated {len(outs)} REST test file(s) into {out_dir} "
        f"({total} tests across {len(outs)} namespaces, {n_routes} routes covered)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
