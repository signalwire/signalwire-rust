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


# Free-function targets to project from rustdoc into the canonical
# inventory. Keyed by (canonical_module, canonical_function) so the
# walker only emits things that the Python reference also has at the
# module level (no port-only additions sneak in here — PORT_ADDITIONS.md
# is the place for extras).
def _collect_free_function_targets() -> set[tuple[str, str]]:
    targets: set[tuple[str, str]] = set()
    # Read the Python reference's module-level functions directly so the
    # set stays in sync with whatever the oracle currently exposes.
    try:
        import json as _json
        ref = _json.loads((PSDK / "python_signatures.json").read_text(encoding="utf-8"))
        for mod_name, mod_entry in ref.get("modules", {}).items():
            for fn_name in (mod_entry.get("functions") or {}).keys():
                targets.add((mod_name, fn_name))
    except FileNotFoundError:
        pass
    return targets


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
                # rustdoc emits trait { path, id }; only keep impls whose
                # trait path is part of the SDK. Skip ALL stdlib traits.
                trait_path = trait.get("path", "") if isinstance(trait, dict) else ""
                if trait_path in (
                    "Debug", "Clone", "Default", "PartialEq", "Eq",
                    "Hash", "Send", "Sync", "Drop", "From", "TryFrom",
                    "Display", "Error", "Iterator", "IntoIterator",
                    "Future", "Serialize", "Deserialize",
                    "Borrow", "BorrowMut", "AsRef", "AsMut", "ToOwned",
                    "Into", "Deref", "DerefMut", "CloneToUninit",
                    "Pointable", "Any", "TypeId", "Unpin",
                    "PartialOrd", "Ord", "Copy", "Sized",
                    "FnOnce", "Fn", "FnMut",
                ):
                    continue
                # Drop blanket impls: any trait path that isn't part of SDK
                if trait_path and not trait_path.startswith("signalwire"):
                    # Most stdlib trait paths are unqualified (Debug, Borrow);
                    # if it's not in our allow-skip list and starts with a
                    # capital letter, conservatively skip it too.
                    if trait_path[0:1].isupper():
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

    # Free-function walk — module-level ``pub fn`` items that are not
    # inherent or trait methods. Project the rustdoc path
    # ``signalwire::utils::url_validator::validate_url`` onto the Python
    # canonical ``signalwire.utils.url_validator.validate_url`` (free
    # function, no class). Only declarations whose canonical Python path
    # corresponds to an existing module entry in the Python reference
    # are emitted; everything else is treated as a port-only extension
    # and dropped (PORT_ADDITIONS.md owns surface-level extras).
    free_fn_targets = _collect_free_function_targets()
    for iid, item in index.items():
        inner = item.get("inner", {})
        if "function" not in inner:
            continue
        # Only pub items; rustdoc emits everything but we still want to
        # filter on visibility.
        if item.get("visibility") != "public":
            continue
        path_id = item.get("id")
        path_entry = paths.get(str(path_id)) or paths.get(path_id)
        if not path_entry or path_entry.get("kind") != "function":
            continue
        rust_path = path_entry.get("path", [])
        if len(rust_path) < 2 or rust_path[0] != "signalwire":
            continue
        # Skip impl-level fns (those have a struct/enum segment somewhere
        # in the path that is not a module). The free-function walk only
        # cares about path-rooted fns.
        target_module = ".".join(rust_path[:-1])
        target_function = rust_path[-1]
        if (target_module, target_function) not in free_fn_targets:
            continue
        try:
            sig = build_signature(
                inner["function"], paths, aliases,
                f"{target_module}.{target_function}",
            )
        except TypeTranslationError as e:
            failures.append(str(e))
            continue
        # Free functions have no receiver; strip a leading self if rustdoc
        # somehow emitted one (it shouldn't, but be defensive).
        params = sig.get("params", [])
        if params and params[0].get("kind") == "self":
            sig["params"] = params[1:]
        out_modules.setdefault(target_module, {"classes": {}})
        out_modules[target_module].setdefault("functions", {})
        out_modules[target_module]["functions"][target_function] = sig

    # Mixin/manager projections — the Rust ``Service`` (renamed
    # SWMLService) inherits to AgentBase. Project Service-side methods
    # to canonical Python mixin / manager paths so the audit lines up.
    MIXIN_PROJECTIONS: dict[tuple[str, str], list[str]] = {
        ("signalwire.core.agent.tools.registry", "ToolRegistry"): [
            "define_tool", "register_swaig_function",
            "has_function", "get_function", "get_all_functions",
            "remove_function",
        ],
        ("signalwire.core.mixins.tool_mixin", "ToolMixin"): [
            "define_tool", "on_function_call", "register_swaig_function",
            "define_tools",
        ],
        ("signalwire.core.mixins.auth_mixin", "AuthMixin"): [
            "validate_basic_auth", "get_basic_auth_credentials",
        ],
        ("signalwire.core.mixins.state_mixin", "StateMixin"): [
            "validate_tool_token",
        ],
        ("signalwire.core.mixins.web_mixin", "WebMixin"): [
            "on_request", "on_swml_request",
        ],
        # Python additionally extracted a ``PromptManager`` class that
        # PromptMixin delegates to.  The user-facing surface is
        # identical (``agent.prompt_manager.X`` ≡ ``agent.X``).  Project
        # the same set of AgentBase methods to PromptManager so the
        # cross-language audit treats both paths as covered.  Rust
        # exposes the prompt methods directly on AgentBase rather than
        # under a separate PromptMixin namespace, so this is the only
        # prompt-side projection needed here.
        ("signalwire.core.agent.prompt.manager", "PromptManager"): [
            "define_contexts", "get_contexts", "get_post_prompt", "get_prompt",
            "get_raw_prompt",
            "prompt_add_section", "prompt_add_subsection", "prompt_add_to_section",
            "prompt_has_section", "set_post_prompt", "set_prompt_pom",
            "set_prompt_text",
        ],
    }
    svc_entry = out_modules.get("signalwire.core.swml_service", {}).get("classes", {}).get("SWMLService")
    ab_entry = out_modules.get("signalwire.core.agent_base", {}).get("classes", {}).get("AgentBase")
    if svc_entry or ab_entry:
        svc_methods = svc_entry["methods"] if svc_entry else {}
        ab_methods = ab_entry["methods"] if ab_entry else {}
        combined = {**svc_methods, **ab_methods}
        projected: set[str] = set()
        for (target_mod, target_cls), expected in MIXIN_PROJECTIONS.items():
            present = {m: combined[m] for m in expected if m in combined}
            if not present:
                continue
            out_modules.setdefault(target_mod, {"classes": {}})
            out_modules[target_mod]["classes"].setdefault(target_cls, {"methods": {}})
            out_modules[target_mod]["classes"][target_cls]["methods"].update(present)
            projected.update(present)
        # Drop projected methods from AgentBase only — keep them on
        # SWMLService (which Python keeps as the primary host of tool/auth
        # methods). This matches the .NET adapter pattern.
        if ab_entry:
            for n in projected:
                ab_methods.pop(n, None)

        # on_swml_request lives only on WebMixin in Python (NOT on
        # SWMLService). The Rust port emits it on SWMLService since
        # Service.rs is the reflection target. Drop the duplicate from
        # SWMLService so the projection-only WebMixin entry is kept and
        # the diff doesn't flag it as a port-only method on SWMLService.
        # on_request stays — Python's SWMLService inherits it from WebMixin
        # in the canonical reference.
        if svc_entry:
            svc_methods.pop("on_swml_request", None)

    sorted_modules = {}
    for k in sorted(out_modules):
        entry = out_modules[k]
        out_entry: dict = {}
        if entry.get("classes"):
            out_entry["classes"] = {
                cls: entry["classes"][cls] for cls in sorted(entry["classes"])
            }
        if entry.get("functions"):
            out_entry["functions"] = {
                fn: entry["functions"][fn] for fn in sorted(entry["functions"])
            }
        if out_entry:
            sorted_modules[k] = out_entry
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
    is_ctor = context.endswith(".__init__")
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
    # Constructors have no Rust receiver but Python's canonical signature
    # includes ``self`` first. Synthesize it so __init__ shapes line up.
    if is_ctor and not is_method:
        params_out.insert(0, {"name": "self", "kind": "self"})

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
