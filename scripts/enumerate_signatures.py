#!/usr/bin/env python3
"""enumerate_signatures.py — emit port_signatures.json for the Rust SDK.

Phase 4-Rust of the cross-language signature audit. Pipeline:

    1. Run ``cargo +nightly rustdoc --lib -- -Z unstable-options
       --output-format json``. Produces target/doc/signalwire.json.
    2. Walk the resulting index: find every struct/enum that maps onto
       a Python class via CLASS_MODULE_MAP from enumerate_surface.py;
       extract its impl methods' signatures.
    3. Translate Rust types to canonical via porting-sdk/type_aliases.yaml
       (rust section).
    4. Emit port_signatures.json conforming to surface_schema_v2.json.

Notes:
    - Rustdoc-json schema is unstable; we pin against the rustc nightly
      pulled by ``rustup toolchain install nightly``. Bump in lockstep
      when needed; the wrapper is written against FORMAT_VERSION 57.
    - Rust has no defaults; every parameter is required.
    - ``&T`` / ``&mut T`` collapse to T (Rust borrowing is invisible to
      Python's type model).

Usage:
    python3 scripts/enumerate_signatures.py
    python3 scripts/enumerate_signatures.py --strict
    python3 scripts/enumerate_signatures.py --raw target/doc/signalwire.json
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

import yaml

HERE = Path(__file__).resolve().parent
PORT_ROOT = HERE.parent
PSDK = (PORT_ROOT.parent / "porting-sdk").resolve()
if not PSDK.is_dir():
    PSDK = Path("/usr/local/home/devuser/src/porting-sdk")

sys.path.insert(0, str(HERE))
from enumerate_surface import (  # type: ignore
    CLASS_MODULE_MAP, _module_path_for_class, _translate_class,
)


class TypeTranslationError(RuntimeError):
    pass


def load_aliases() -> dict[str, str]:
    data = yaml.safe_load((PSDK / "type_aliases.yaml").read_text(encoding="utf-8"))
    return {str(k): str(v) for k, v in data.get("aliases", {}).get("rust", {}).items()}


# ---------------------------------------------------------------------------
# Rust type translation (rustdoc-json types model)
# ---------------------------------------------------------------------------


def translate_rust_type(t, paths: dict, aliases: dict, context: str) -> str:
    """Translate a rustdoc-json type node to canonical form."""
    if t is None:
        return "void"
    if isinstance(t, str):
        # primitives are already strings in some places
        return aliases.get(t, t)
    if not isinstance(t, dict):
        raise TypeTranslationError(f"unexpected type node {t!r} at {context}")

    # primitive: { "primitive": "str" }
    if "primitive" in t:
        prim = t["primitive"]
        if prim in aliases:
            return aliases[prim]
        raise TypeTranslationError(f"unknown primitive {prim!r} at {context}")

    # generic: { "generic": "T" }  — type parameter; collapse to any
    if "generic" in t:
        return "any"

    # borrowed_ref: { "borrowed_ref": { "lifetime", "is_mutable", "type" } }
    if "borrowed_ref" in t:
        return translate_rust_type(t["borrowed_ref"]["type"], paths, aliases, context)

    # raw_pointer
    if "raw_pointer" in t:
        return translate_rust_type(t["raw_pointer"]["type"], paths, aliases, context)

    # slice: { "slice": { type } }
    if "slice" in t:
        inner = translate_rust_type(t["slice"], paths, aliases, context) if not isinstance(t["slice"], dict) else translate_rust_type(t["slice"], paths, aliases, context)
        # handle [u8] specifically as bytes
        slice_type = t["slice"]
        if isinstance(slice_type, dict) and slice_type.get("primitive") == "u8":
            return "bytes"
        return f"list<{translate_rust_type(slice_type, paths, aliases, context)}>"

    # array: { "array": { "type", "len" } }
    if "array" in t:
        arr = t["array"]
        if isinstance(arr, dict) and "type" in arr:
            elem_t = arr["type"]
            if isinstance(elem_t, dict) and elem_t.get("primitive") == "u8":
                return "bytes"
            return f"list<{translate_rust_type(elem_t, paths, aliases, context)}>"
        return "list<any>"

    # tuple: { "tuple": [ ... ] }
    if "tuple" in t:
        elements = t["tuple"]
        if not elements:
            return "void"
        parts = [translate_rust_type(e, paths, aliases, context) for e in elements]
        return f"tuple<{','.join(parts)}>"

    # resolved_path: { "resolved_path": { "path", "id", "args" } }
    if "resolved_path" in t:
        rp = t["resolved_path"]
        path = rp.get("path", "")
        path_id = rp.get("id")
        args = rp.get("args")

        # Resolve full dotted path via paths[id]
        full_path = path
        if path_id is not None:
            entry = paths.get(str(path_id)) or paths.get(path_id)
            if entry and "path" in entry:
                full_path = "::".join(entry["path"])

        last = full_path.split("::")[-1]

        # Direct alias hits (covers stdlib paths)
        for candidate in (full_path, last):
            if candidate in aliases:
                return aliases[candidate]

        # Stdlib generics: Option, Vec, HashMap, BTreeMap, Result, Box, Arc, Rc, etc.
        type_args = _extract_angle_args(args)
        if last in ("Option", "Optional"):
            inner = translate_rust_type(type_args[0], paths, aliases, context) if type_args else "any"
            return f"optional<{inner}>"
        if last in ("Vec", "VecDeque"):
            inner = translate_rust_type(type_args[0], paths, aliases, context) if type_args else "any"
            return f"list<{inner}>"
        if last in ("HashMap", "BTreeMap", "IndexMap"):
            if len(type_args) >= 2:
                k = translate_rust_type(type_args[0], paths, aliases, context)
                v = translate_rust_type(type_args[1], paths, aliases, context)
                return f"dict<{k},{v}>"
            return "dict<string,any>"
        if last in ("HashSet", "BTreeSet"):
            inner = translate_rust_type(type_args[0], paths, aliases, context) if type_args else "any"
            return f"list<{inner}>"
        if last == "Result":
            # Result<T, E> → T (the Err type is out-of-band in Python)
            inner = translate_rust_type(type_args[0], paths, aliases, context) if type_args else "any"
            return inner
        if last in ("Box", "Arc", "Rc", "Mutex", "RwLock"):
            inner = translate_rust_type(type_args[0], paths, aliases, context) if type_args else "any"
            return inner

        # SDK class — emit class:<canonical>
        canonical_name = _translate_class(last)
        if canonical_name in CLASS_MODULE_MAP:
            return f"class:{CLASS_MODULE_MAP[canonical_name]}.{canonical_name}"
        # Heuristic module path
        return f"class:signalwire.{last.lower()}.{canonical_name}"

    # impl_trait: { "impl_trait": [ ... ] }
    if "impl_trait" in t:
        return "any"

    # function_pointer: emit callable
    if "function_pointer" in t or "fn_pointer" in t:
        sig = t.get("function_pointer", t.get("fn_pointer", {}))
        sig_decl = sig.get("decl", sig.get("sig", {}))
        inputs = sig_decl.get("inputs", [])
        canon_args = [translate_rust_type(it[1] if isinstance(it, list) else it, paths, aliases, context) for it in inputs]
        output = sig_decl.get("output")
        canon_ret = translate_rust_type(output, paths, aliases, context) if output else "void"
        return f"callable<list<{','.join(canon_args)}>,{canon_ret}>"

    # dyn_trait: dyn Foo  →  any
    if "dyn_trait" in t:
        return "any"

    # qualified_path / projection
    if "qualified_path" in t:
        return "any"

    # pat / infer / etc. — fall back to any
    return "any"


def _extract_angle_args(args) -> list:
    if not args:
        return []
    if isinstance(args, dict) and "angle_bracketed" in args:
        ab = args["angle_bracketed"]
        out = []
        for a in ab.get("args", []):
            if isinstance(a, dict) and "type" in a:
                out.append(a["type"])
        return out
    return []


# ---------------------------------------------------------------------------
# Walking the rustdoc index
# ---------------------------------------------------------------------------


def collect(rust_doc: dict, aliases: dict) -> tuple[dict, list]:
    index = rust_doc["index"]
    paths = rust_doc["paths"]
    failures: list = []
    out_modules: dict = {}

    # Build a lookup: id → item
    def get(id_):
        return index.get(str(id_)) or index.get(id_)

    # Find all struct / enum / trait items + their impls
    for iid, item in index.items():
        inner = item.get("inner", {})
        if "struct" in inner:
            kind_inner = inner["struct"]
        elif "enum" in inner:
            kind_inner = inner["enum"]
        elif "trait" in inner:
            kind_inner = inner["trait"]
        else:
            continue
        struct_name = item.get("name")
        if not struct_name:
            continue
        impls = kind_inner.get("impls", [])

        # Determine canonical module for this class
        canonical_name = _translate_class(struct_name)
        if canonical_name not in CLASS_MODULE_MAP:
            continue  # port-only struct; would be in PORT_ADDITIONS
        mod = CLASS_MODULE_MAP[canonical_name]

        methods_out: dict = {}
        for impl_id in impls:
            impl_item = get(impl_id)
            if not impl_item:
                continue
            impl_inner = impl_item.get("inner", {}).get("impl", {})
            # Skip trait impls that pull in unrelated methods (stdlib derives etc.)
            trait = impl_inner.get("trait")
            if trait is not None:
                # Allow impls of SDK-relevant traits but skip stdlib derives.
                # rustdoc emits trait { path, id }; only keep impls whose
                # trait path is part of the SDK.
                trait_path = trait.get("path", "") if isinstance(trait, dict) else ""
                if trait_path in (
                    "Debug", "Clone", "Default", "PartialEq", "Eq",
                    "Hash", "Send", "Sync", "Drop", "From", "TryFrom",
                    "Display", "Error", "Iterator", "IntoIterator",
                    "Future", "Serialize", "Deserialize",
                ):
                    continue
            for method_id in impl_inner.get("items", []):
                method_item = get(method_id)
                if not method_item:
                    continue
                m_inner = method_item.get("inner", {})
                if "function" not in m_inner:
                    continue
                method_native = method_item.get("name", "")
                if not method_native:
                    continue
                if method_native.startswith("_") and not method_native.startswith("__"):
                    continue
                # Translate name
                if method_native == "new":
                    method_canonical = "__init__"
                else:
                    method_canonical = method_native
                ctx = f"{mod}.{canonical_name}.{method_canonical}"
                try:
                    sig = build_signature(m_inner["function"], paths, aliases, ctx)
                except TypeTranslationError as e:
                    failures.append(str(e))
                    continue
                if method_canonical in methods_out:
                    continue
                methods_out[method_canonical] = sig

        if not methods_out:
            continue

        out_modules.setdefault(mod, {"classes": {}})
        out_modules[mod]["classes"][canonical_name] = {
            "methods": dict(sorted(methods_out.items())),
        }

    sorted_modules = {}
    for k in sorted(out_modules):
        entry = out_modules[k]
        sorted_modules[k] = {
            "classes": {
                cls: entry["classes"][cls] for cls in sorted(entry["classes"])
            }
        }
    return {
        "version": "2",
        "generated_from": f"signalwire-rust via cargo rustdoc-json (FORMAT_VERSION {rust_doc.get('format_version')})",
        "modules": sorted_modules,
    }, failures


def build_signature(fn: dict, paths: dict, aliases: dict, context: str) -> dict:
    sig = fn.get("sig", {})
    inputs = sig.get("inputs", [])
    params_out: list = []
    is_method = False
    for i, entry in enumerate(inputs):
        if not isinstance(entry, list) or len(entry) != 2:
            continue
        name, t = entry
        if i == 0 and name == "self":
            params_out.append({"name": "self", "kind": "self"})
            is_method = True
            continue
        canon = translate_rust_type(t, paths, aliases, f"{context}[{name}]")
        params_out.append({
            "name": name,
            "type": canon,
            "required": True,  # Rust has no defaults
        })

    output = sig.get("output")
    return_canon = translate_rust_type(output, paths, aliases, f"{context}[->]") if output else "void"
    # ::new() returns Self in Rust; translate as void per __init__ convention
    if context.endswith(".__init__") and return_canon != "void":
        return_canon = "void"
    return {"params": params_out, "returns": return_canon}


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def run_dump() -> dict:
    cp = subprocess.run(
        ["cargo", "+nightly", "rustdoc", "--lib", "--", "-Z", "unstable-options",
         "--output-format", "json"],
        cwd=PORT_ROOT, capture_output=True, text=True, timeout=600,
    )
    if cp.returncode != 0:
        raise RuntimeError(f"cargo rustdoc failed:\n{cp.stderr}")
    out = PORT_ROOT / "target" / "doc" / "signalwire.json"
    if not out.is_file():
        raise RuntimeError(f"rustdoc did not produce {out}")
    return json.loads(out.read_text(encoding="utf-8"))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--raw", type=Path, default=None)
    parser.add_argument("--out", type=Path, default=PORT_ROOT / "port_signatures.json")
    parser.add_argument("--strict", action="store_true")
    args = parser.parse_args()

    aliases = load_aliases()
    if args.raw and args.raw.is_file():
        rust_doc = json.loads(args.raw.read_text(encoding="utf-8"))
    else:
        rust_doc = run_dump()

    canonical, failures = collect(rust_doc, aliases)
    if failures:
        print(f"enumerate_signatures: {len(failures)} translation failure(s)", file=sys.stderr)
        for f in failures[:30]:
            print(f"  - {f}", file=sys.stderr)
        if len(failures) > 30:
            print(f"  ... ({len(failures) - 30} more)", file=sys.stderr)
        if args.strict:
            return 1

    args.out.write_text(json.dumps(canonical, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    n_mods = len(canonical["modules"])
    n_methods = sum(sum(len(c["methods"]) for c in m.get("classes", {}).values()) for m in canonical["modules"].values())
    print(f"enumerate_signatures: wrote {args.out} ({n_mods} modules, {n_methods} methods)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
