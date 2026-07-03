#!/usr/bin/env python3
"""Generate the typed SWML-verbs CONFIG surface for signalwire-rust.

The RUST realization of SESSION_CHANGESET_FOR_PORTS.md item D2 — the
``signalwire.core.swml_verbs_generated`` module — mirroring python's
``swml_verbs_generated.py`` and ruby/php's ``generate_swml_verbs.py``.

Source: the CANONICAL porting-sdk ``schema.json`` ``$defs``. Emits the 155
method-less SWML config types the Python SURFACE oracle records (the reference's
``_SwmlVerbs`` verb-METHOD protocol is ``_``-prefixed and NOT part of the
cross-port surface oracle, so only the CONFIG type surface is emitted):

  1. One method-less Rust ``struct`` per ``$defs`` OBJECT schema (133).
     scalar/array/oneOf/anyOf/allOf alias -> NOT surfaced (matches the reference).
  2. One ``<Verb>Config`` struct per SWMLMethod.anyOf verb whose inner schema is
     an inline object / oneOf union (22) — the flattened UNION of the verb's
     variant properties. Hand-written verbs (answer/hangup/ai/play/say) excluded.

  133 + 22 = 155 == the oracle exactly (0 missing / 0 extra).

Output: a single module ``src/swml/swml_verbs_generated.rs`` (namespace
``signalwire::swml::swml_verbs_generated``), routed to the oracle module
``signalwire.core.swml_verbs_generated`` BY the FILE PATH in both enumerators
(names collide with REST wire types + SDK classes; the path route wins). The
reference records these WITH a zero-arg accessor per class-typed field on the
SIGNATURE oracle — Rust structs carry no accessor methods, so the generator also
writes a gen-payload SIGNATURE SIDECAR (``swml_verbs_gen_payload.json``) the sig
enumerator unfolds into synthesized ``any``-return accessors (excused / folded by
the diff tool). The SURFACE enumerator records these method-less (the split the
reference itself has).

Usage:
    python3 scripts/generate_swml_verbs.py            # write into the repo tree
    python3 scripts/generate_swml_verbs.py --check    # GEN-FRESH: fail if stale
    python3 scripts/generate_swml_verbs.py --out DIR  # scratch: emit into DIR
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import re
import sys
from pathlib import Path


def _load_rest_generator():
    here = Path(__file__).resolve().parent
    spec = importlib.util.spec_from_file_location("generate_rest", here / "generate_rest.py")
    if spec is None or spec.loader is None:  # pragma: no cover
        raise SystemExit("generate_swml_verbs.py: cannot load generate_rest.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


GR = _load_rest_generator()

MODULE_FILE = "swml/swml_verbs_generated.rs"
SIDECAR_FILE = "swml/swml_verbs_gen_payload.json"
ORACLE_MODULE = "signalwire.core.swml_verbs_generated"
HAND_WRITTEN_VERBS = {"answer", "hangup", "ai", "play", "say"}


def resolve_porting_sdk() -> Path:
    return GR.resolve_porting_sdk()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _load_defs(psdk: Path) -> dict:
    doc = json.loads((psdk / "schema.json").read_text())
    defs = doc.get("$defs")
    if not defs:
        raise SystemExit("generate_swml_verbs.py: schema.json has no $defs")
    return defs


def _ref_leaf(ref: str) -> str:
    return ref.rsplit("/", 1)[-1] if ref else ref


def _type_str(node: dict):
    t = node.get("type")
    if isinstance(t, list):
        return next((x for x in t if x != "null"), None)
    return t


def _pascal(s: str) -> str:
    parts = re.split(r"[_\-\s.]", s)
    return "".join(w[:1].upper() + w[1:] for w in parts if w)


def _flatten_union(defs: dict, node) -> dict:
    """UNION of properties across allOf/oneOf/anyOf, following $ref (mirrors the
    reference _flatten_union / go flattenUnion). First-seen wins."""
    out: dict = {}

    def walk(n) -> None:
        if not n:
            return
        ref = n.get("$ref")
        if ref:
            walk(defs.get(_ref_leaf(ref)))
            return
        for sub in n.get("allOf") or []:
            walk(sub)
        for name, psc in (n.get("properties") or {}).items():
            out.setdefault(name, psc)
        for sub in n.get("oneOf") or []:
            walk(sub)
        for sub in n.get("anyOf") or []:
            walk(sub)

    walk(node)
    return out


def build_types(psdk: Path) -> list[tuple[str, dict, str]]:
    """Return [(rs_name, properties, desc)] in reference declaration order."""
    defs = _load_defs(psdk)
    out: list[tuple[str, dict, str]] = []
    seen: set = set()

    def add(rs_name: str, props: dict, desc: str) -> None:
        if rs_name in seen:
            return
        seen.add(rs_name)
        out.append((rs_name, props, desc))

    # 1. One struct per OBJECT $defs schema.
    for raw_name, node in defs.items():
        if not isinstance(node, dict) or not GR.is_object_schema(node):
            continue
        add(GR.type_name(raw_name), node.get("properties") or {},
            f"schema.json $defs schema {raw_name!r}")

    # 2. One <Verb>Config struct per flattenable SWMLMethod.anyOf verb.
    sm = defs.get("SWMLMethod")
    if sm:
        for ref in sm.get("anyOf") or []:
            wrapper = _ref_leaf(ref.get("$ref", ""))
            wdef = defs.get(wrapper)
            if not wdef or not (wdef.get("properties") or {}):
                continue
            verb = next(iter(wdef["properties"].keys()))
            if verb in HAND_WRITTEN_VERBS:
                continue
            inner = wdef["properties"][verb]
            if _type_str(inner) == "string" or inner.get("$ref"):
                continue
            has_inline = _type_str(inner) == "object" and bool(inner.get("properties"))
            if not inner.get("oneOf") and not has_inline:
                continue
            props = _flatten_union(defs, inner)
            if not props:
                continue
            add(GR.type_name(_pascal(verb) + "Config"), props,
                f"flattened SWMLMethod verb {verb!r} config")

    return out


def build_module(psdk: Path) -> str:
    types = build_types(psdk)
    desc = "Generated SWML-verb config types from porting-sdk/schema.json ($defs)."
    src = GR.TYPES_HEADER.format(gen="generate_swml_verbs.py", desc=desc) + "\n"
    for rs_name, props, tdesc in types:
        src += "\n" + GR.emit_methodless_struct(rs_name, props, tdesc,
                                                "generate_swml_verbs.py") + "\n"
    return GR._rustfmt(src)


def build_sidecar(psdk: Path) -> str:
    """The gen-payload signature sidecar: {module, classes:{Name:[accessor,...]}}
    the sig enumerator unfolds into synthesized ``any``-return accessors."""
    types = build_types(psdk)
    classes = {rs_name: GR.gen_payload_accessors(props) for rs_name, props, _ in types}
    payload = {
        "_comment": ("Generated by scripts/generate_swml_verbs.py; DO NOT EDIT. "
                     "Per-class zero-arg accessor names for the read-side SWML-verb "
                     "payloads; enumerate_signatures.py synthesizes an any-return "
                     "accessor per name (gen-payload §D3)."),
        "module": ORACLE_MODULE,
        "classes": dict(sorted(classes.items())),
    }
    return json.dumps(payload, indent=2, sort_keys=True) + "\n"


def build_outputs(psdk: Path) -> dict[str, str]:
    return {
        MODULE_FILE: build_module(psdk),
        SIDECAR_FILE: build_sidecar(psdk),
    }


def main(argv: list) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="GEN-FRESH: exit non-zero if stale")
    ap.add_argument("--out", default="", help="scratch: emit into this dir")
    args = ap.parse_args(argv)

    psdk = resolve_porting_sdk()
    outs = build_outputs(psdk)
    out_dir = Path(args.out) if args.out else repo_root() / "src"

    if args.check:
        stale: list = []
        for fn, src in outs.items():
            p = out_dir / fn
            if not p.is_file() or p.read_text() != src:
                stale.append(str(p))
        if stale:
            sys.stderr.write("GEN-FRESH FAIL: %d generated SWML-verb file(s) stale:\n" % len(stale))
            for s in stale:
                sys.stderr.write("  - %s\n" % s)
            return 1
        print("GEN-FRESH: generated SWML-verb files match porting-sdk/schema.json ($defs).")
        return 0

    for fn, src in outs.items():
        p = out_dir / fn
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(src)
    print(f"generated {len(outs)} SWML-verb file(s) into {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
