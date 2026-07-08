#!/usr/bin/env python3
"""Generate the typed SWAIG read-side payload surface for signalwire-rust.

The RUST realization of SESSION_CHANGESET_FOR_PORTS.md item D1 — the three
``signalwire.core.*_generated`` SWAIG payload modules — mirroring python's
``generate_swaig_request`` / ``generate_post_prompt`` / ``generate_swaig_actions``
and ruby/php's ``generate_swaig_payloads.py``.

Source: the vendored porting-sdk ``swaig-specs/*.yaml`` (from mod_openai):

  * ``swaig-request.yaml``  -> signalwire.core.swaig_request_generated  (2 types)
        SwaigRequest (+ the inline ``argument`` object lifted to SwaigArgument).
  * ``post-prompt.yaml``    -> signalwire.core.post_prompt_generated    (14 types)
        one type per components/schemas OBJECT schema; the oneOf alias
        ``PostPromptCallLogEntry`` is NOT surfaced (15 schemas - 1 alias = 14).
  * ``swaig-response.yaml`` -> signalwire.core.swaig_actions_generated  (4 types)
        one ``<Verb>Action`` type per action key whose value is an object-with-
        properties (a bare object OR the object variant of a oneOf).

  2 + 14 + 4 = 20 == the surface oracle EXACTLY (0 missing / 0 extra).

post-prompt + swaig-request types are in the SIGNATURE oracle WITH a zero-arg
accessor per class-typed field — Rust structs carry no accessor methods, so this
generator writes a gen-payload SIGNATURE SIDECAR per module the sig enumerator
unfolds into synthesized ``any``-return accessors. swaig-actions are NOT in the
sig oracle -> method-less, no sidecar. All three modules route to their oracle
module BY FILE PATH in both enumerators (names collide with the SWAIG SDK classes
FunctionResult / ParameterSchema).

Output: three modules under src/swaig/
  post_prompt_generated.rs   (+ post_prompt_gen_payload.json)
  swaig_request_generated.rs (+ swaig_request_gen_payload.json)
  swaig_actions_generated.rs

Usage:
    python3 scripts/generate_swaig_payloads.py            # write into the repo tree
    python3 scripts/generate_swaig_payloads.py --check    # GEN-FRESH: fail if stale
    python3 scripts/generate_swaig_payloads.py --out DIR  # scratch: emit into DIR
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
        raise SystemExit("generate_swaig_payloads.py: cannot load generate_rest.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


GR = _load_rest_generator()

PP_FILE = "swaig/post_prompt_generated.rs"
PP_SIDECAR = "swaig/post_prompt_gen_payload.json"
PP_ORACLE = "signalwire.core.post_prompt_generated"
SR_FILE = "swaig/swaig_request_generated.rs"
SR_SIDECAR = "swaig/swaig_request_gen_payload.json"
SR_ORACLE = "signalwire.core.swaig_request_generated"
SA_FILE = "swaig/swaig_actions_generated.rs"


def resolve_porting_sdk() -> Path:
    return GR.resolve_porting_sdk()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _load_yaml(path: Path) -> dict:
    import yaml  # type: ignore[import-untyped]

    return yaml.safe_load(path.read_text())


def _pascal_verb(verb: str) -> str:
    parts = [p for p in re.split(r"[._\-\s]", verb) if p]
    return "".join(w[:1].upper() + w[1:] for w in parts)


def _build_swaig_request(psdk: Path) -> list[tuple[str, dict, str]]:
    spec = _load_yaml(psdk / "swaig-specs" / "swaig-request.yaml")
    schema = spec["components"]["schemas"]["SwaigRequest"]
    props = schema.get("properties", {})
    out: list[tuple[str, dict, str]] = []
    arg = props.get("argument")
    if isinstance(arg, dict) and arg.get("properties"):
        out.append(("SwaigArgument", arg["properties"], "inline swaig-request `argument` object"))
    out.append(("SwaigRequest", props, "swaig-request `SwaigRequest` schema"))
    return out


def _build_post_prompt(psdk: Path) -> list[tuple[str, dict, str]]:
    spec = _load_yaml(psdk / "swaig-specs" / "post-prompt.yaml")
    schemas = spec["components"]["schemas"]
    out: list[tuple[str, dict, str]] = []
    seen: set = set()
    for raw_name, node in schemas.items():
        if not isinstance(node, dict) or not GR.is_object_schema(node):
            continue
        rs_name = GR.type_name(raw_name)
        if rs_name in seen:
            continue
        seen.add(rs_name)
        out.append((rs_name, node.get("properties") or {},
                    f"post-prompt components/schemas {raw_name!r}"))
    return out


def _build_swaig_actions(psdk: Path) -> list[tuple[str, dict, str]]:
    spec = _load_yaml(psdk / "swaig-specs" / "swaig-response.yaml")
    actions = spec["components"]["schemas"]["SwaigAction"]["properties"]

    def _is_obj(s) -> bool:
        return isinstance(s, dict) and s.get("type") == "object" and bool(s.get("properties"))

    out: list[tuple[str, dict, str]] = []
    seen: set = set()
    for verb in sorted(actions):
        schema = actions[verb]
        if not isinstance(schema, dict):
            continue
        branches = schema.get("oneOf") or ([schema] if _is_obj(schema) else [])
        obj_i = 0
        for b in branches:
            if not _is_obj(b):
                continue
            obj_i += 1
            rs_name = GR.type_name(_pascal_verb(verb) + "Action" + ("" if obj_i == 1 else str(obj_i)))
            if rs_name in seen:
                continue
            seen.add(rs_name)
            out.append((rs_name, b.get("properties") or {},
                        f"swaig-response action {verb!r} value object"))
    return out


def _module_src(types: list[tuple[str, dict, str]], desc: str, gen: str) -> str:
    src = GR.TYPES_HEADER.format(gen=gen, desc=desc) + "\n"
    for rs_name, props, tdesc in types:
        src += "\n" + GR.emit_methodless_struct(rs_name, props, tdesc, gen) + "\n"
    return GR._rustfmt(src)


def _sidecar_src(types: list[tuple[str, dict, str]], oracle: str, gen: str) -> str:
    classes = {rs_name: GR.gen_payload_accessors(props) for rs_name, props, _ in types}
    payload = {
        "_comment": (f"Generated by scripts/{gen}; DO NOT EDIT. Per-class zero-arg "
                     "accessor names for the read-side SWAIG payloads; "
                     "enumerate_signatures.py synthesizes an any-return accessor per "
                     "name (gen-payload §D3)."),
        "module": oracle,
        "classes": dict(sorted(classes.items())),
    }
    return json.dumps(payload, indent=2, sort_keys=True) + "\n"


def build_outputs(psdk: Path) -> dict[str, str]:
    specs_dir = psdk / "swaig-specs"
    if not specs_dir.is_dir():
        raise SystemExit(
            f"generate_swaig_payloads.py: {specs_dir} not found (need porting-sdk adjacency)")
    pp = _build_post_prompt(psdk)
    sr = _build_swaig_request(psdk)
    sa = _build_swaig_actions(psdk)
    return {
        PP_FILE: _module_src(pp, "Generated SWAIG post-prompt payload types.",
                             "generate_swaig_payloads.py"),
        PP_SIDECAR: _sidecar_src(pp, PP_ORACLE, "generate_swaig_payloads.py"),
        SR_FILE: _module_src(sr, "Generated SWAIG request payload types.",
                             "generate_swaig_payloads.py"),
        SR_SIDECAR: _sidecar_src(sr, SR_ORACLE, "generate_swaig_payloads.py"),
        SA_FILE: _module_src(sa, "Generated SWAIG response-action config types.",
                             "generate_swaig_payloads.py"),
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
            sys.stderr.write("GEN-FRESH FAIL: %d generated SWAIG-payload file(s) stale:\n" % len(stale))
            for s in stale:
                sys.stderr.write("  - %s\n" % s)
            return 1
        print("GEN-FRESH: generated SWAIG-payload files match porting-sdk/swaig-specs/.")
        return 0

    for fn, src in outs.items():
        p = out_dir / fn
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(src)
    print(f"generated {len(outs)} SWAIG-payload file(s) into {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
