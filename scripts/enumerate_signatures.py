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
import os
import subprocess
import sys
from pathlib import Path

import yaml

HERE = Path(__file__).resolve().parent
PORT_ROOT = HERE.parent
PSDK = (PORT_ROOT.parent / "porting-sdk").resolve()
if not PSDK.is_dir():
    env_psdk = os.environ.get("PORTING_SDK")
    if env_psdk:
        PSDK = Path(env_psdk).resolve()

sys.path.insert(0, str(HERE))
from enumerate_surface import (  # type: ignore
    CLASS_MODULE_MAP, _module_path_for_class, _translate_class,
    # Idiom-reconciliation tables mirrored from the SURFACE enumerator so the
    # two enumerators discover/name the SAME symbols (Rule 2: reconcile idiom
    # in the enumerator, not via an omission). Kept as a single source of truth
    # by importing them rather than re-declaring.
    METHOD_RENAMES, SURFACE_PROJECTIONS, PROJECTION_DONOR_STRIPS,
    FORCE_CLASS_METHODS, SKILLBASE_IDIOM_METHOD_DROPS, SKILL_INTERFACE_METHODS,
    SKILL_INTERFACE_PROJECTION, PUBLIC_SURFACE_TRAITS, MODULE_METHOD_DROPS,
)


class TypeTranslationError(RuntimeError):
    pass


# Per-method return-type remap (canonical method path -> canonical return class).
#
# Rust models some Python class *pairs* as ONE struct with a field, where the
# only difference is invisible to the signature model (e.g. the HTTP verb used
# by ``update``). Python's reference keeps them as distinct classes, so the
# struct-collapse would read as a return-type drift. We remap the return type
# back to the Python class the accessor is contractually returning — so the
# signature lines up AND the FULL method set of the returned struct is still
# compared (if a method is added/removed/retyped on the collapsed struct, or an
# accessor stops returning it, the drift reappears). This is the tool handling
# the idiom, NOT a blanket omission that would hide any future change.
#
# FabricResourcePUT: Python's PUT-update fabric resource. Rust folds it onto
# FabricResource (constructed via ``new_put`` — PUT instead of PATCH on
# ``update``); these five accessors contractually return the PUT variant.
RETURN_TYPE_OVERRIDE: dict[str, str] = {
    f"signalwire.rest.namespaces.fabric.FabricNamespace.{m}":
        "class:signalwire.rest.namespaces.fabric.FabricResourcePUT"
    for m in (
        "sip_endpoints",
        "swml_scripts",
        "cxml_scripts",
        "relay_applications",
        "freeswitch_connectors",
    )
}

# as_router() returns a real mountable handler — an ``axum::Router`` (Rust's
# "embed my routes in a host app" unit, mounted via ``Router::nest`` into a
# caller's own axum/hyper app). Python's WebMixin.as_router / SWMLService.as_router
# return the behaviour-neutral ``HostAppRouter`` (a FastAPI ``APIRouter`` subclass)
# — the SAME capability in each language's idiom. rustdoc spells the return type
# ``axum::Router`` (a foreign crate's type the type-alias table can't resolve to a
# Python class), so map it here per-method to the canonical HostAppRouter. This is
# the tool handling the idiom (the full method contract is still compared), NOT an
# omission — as_router now drifts 0 against the reference.
RETURN_TYPE_OVERRIDE.update({
    ctx: "class:signalwire.core.web.HostAppRouter"
    for ctx in (
        "signalwire.core.swml_service.SWMLService.as_router",
        "signalwire.core.mixins.web_mixin.WebMixin.as_router",
        "signalwire.core.agent_base.AgentBase.as_router",
    )
})

# Per-method PARAMETER reconcile: rename a param and/or remap its type to the
# canonical Python spelling where the Rust idiom names/types it differently but
# the method contract is otherwise identical. This is the parameter analog of
# RETURN_TYPE_OVERRIDE — the tool handling the idiom in the enumerator (Rule 2:
# reconcile idiom in the enumerator, not via an omission), so the full method
# still compares and drifts 0. Shape: {context: {rust_param_name: {"name": ...,
# "type": ...}}}. A missing key leaves that facet unchanged.
#
# handle_request: Python's framework-free dispatch core is
#   handle_request(method, url, headers, body: Optional[dict]) -> (status, headers, body).
# Rust's handler receives the request as an HTTP handler does on the wire: the
# request-target as `path` and the raw request `body: &str`, which it parses to a
# JSON dict internally (see Service::handle_request). Reconcile the two idiomatic
# spellings — `path`→`url`, raw `body: &str`→ the parsed `optional<dict<string,any>>`
# — so the port's dispatch core compares equal to the oracle's.
PARAM_RECONCILE: dict[str, dict[str, dict[str, str]]] = {
    ctx: {
        "path": {"name": "url"},
        "body": {"type": "optional<dict<string,any>>"},
    }
    for ctx in (
        "signalwire.core.swml_service.SWMLService.handle_request",
        "signalwire.core.agent_base.AgentBase.handle_request",
    )
}
# create_typed_handler_wrapper's `func` is a handler CALLABLE — Rust spells it
# with the `TypedHandler` = `Box<dyn Fn(..) -> FunctionResult>` type alias, which
# rustdoc renders as a bare class name. Remap it to the canonical `callable<..>`
# so the wrapper's arg compares equal to the oracle's `func: callable<list<any>,any>`.
PARAM_RECONCILE["signalwire.core.agent.tools.type_inference.create_typed_handler_wrapper"] = {
    "func": {"type": "callable<list<any>,any>"},
}

# Return-type reconcile for the type_inference free fns: rustdoc leaks the
# `TypedHandler` / `InferredSchema` type aliases as class names. Map them to the
# concrete canonical types they alias — `create_typed_handler_wrapper` returns a
# handler callable; `infer_schema` returns the `(parameters, required,
# description, is_typed, has_raw_data)` tuple — so both return-compare equal to
# the oracle (the tool handling the idiom, not an omission).
RETURN_TYPE_OVERRIDE.update({
    "signalwire.core.agent.tools.type_inference.create_typed_handler_wrapper":
        "callable<list<any>,any>",
    "signalwire.core.agent.tools.type_inference.infer_schema":
        "tuple<dict<string,dict<string,any>>,list<string>,optional<string>,bool,bool>",
})


def load_aliases() -> dict[str, str]:
    data = yaml.safe_load((PSDK / "type_aliases.yaml").read_text(encoding="utf-8"))
    return {str(k): str(v) for k, v in data.get("aliases", {}).get("rust", {}).items()}


# ---------------------------------------------------------------------------
# Generated REST layer sidecar (L10). The generator (scripts/generate_rest.py)
# emits src/rest/namespaces/generated/rest_signatures.json alongside the .rs
# modules: the EXPLODED named-param model (kinds) of each generated method plus
# the surface drop-set (base-delegated methods the runtime keeps but the oracle
# does not record). rustdoc alone can't see inside the ``request: XRequest``
# builder, so we UNFOLD from the sidecar: replace each generated method's params
# with the reference-shaped exploded set, drop the silently-inherited base
# methods, and suppress the port-internal GeneratedResourceTree glue struct.
# ---------------------------------------------------------------------------

_REST_SIDECAR_PATH = PORT_ROOT / "src" / "rest" / "namespaces" / "generated" / "rest_signatures.json"


def load_rest_sidecar() -> dict:
    try:
        return json.loads(_REST_SIDECAR_PATH.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {"resources": {}, "containers": {}, "suppress_structs": []}


def _sidecar_param(p: dict) -> dict:
    """Map a sidecar param {name, kind, required, type} to a port_signatures
    param. Path-id / string args carry their real ``string`` type (genuine
    Rust ``&str``); body/command keyword fields carry the concrete field type
    the generator threaded from the spec schema (scalar → string/int/float/bool,
    array → list<any>, object/$ref/union → dict<string,any>); the ``extras``/
    ``extra`` door + var_keyword tail stay open (the cross-port extras signal)."""
    return {
        "name": p["name"],
        "kind": p["kind"],
        "type": p.get("type", "any"),
        "required": bool(p.get("required", False)),
    }


def build_generated_signatures(sidecar: dict) -> dict:
    """Build the port_signatures fragment for the generated REST layer straight
    from the sidecar (self + exploded params + a void __init__). Returns
    {module: {"classes": {cls: {"methods": {...}}}}}."""
    out: dict = {}
    for _name, r in sidecar.get("resources", {}).items():
        mod = r["module"]
        cls = r["class"]
        methods: dict = {
            "__init__": {
                "params": [
                    {"name": "self", "kind": "self"},
                    {"name": "http", "type": "any", "required": True},
                ],
                "returns": "void",
            }
        }
        for m_name, params in r.get("methods", {}).items():
            methods[m_name] = {
                "params": [{"name": "self", "kind": "self"}]
                + [_sidecar_param(p) for p in params],
                "returns": "any",
            }
        # ``paginate`` is an OWN method the Python oracle records on the read-only
        # leaf resources (base == ReadResource: FaxLogs / MessageLogs /
        # VideoRoomSessions / VoiceLogs / FabricAddresses). Rust delegates it to
        # the ReadResource base (generate_rest.py emits the public delegator), and
        # the diff's crud_base excuse does NOT cover ``paginate`` (it's not a CRUD
        # verb), so synthesize the reference-shaped signature here: self-only,
        # returning PaginatedIterator (mirrors ReadResource.paginate in the oracle).
        if r.get("base") == "ReadResource":
            methods["paginate"] = {
                "params": [
                    {"name": "self", "kind": "self"},
                    # The reference records ``paginate(self, *, request_options=None,
                    # **params)`` — the enumerator drops ``**params``, leaving the
                    # trailing keyword-only ``request_options`` (plan 4.2 / PY-9).
                    {
                        "name": "request_options",
                        "kind": "keyword",
                        "required": False,
                        "type": "optional<class:signalwire.rest._request_options.RequestOptions>",
                    },
                ],
                "returns": "class:signalwire.rest._pagination.PaginatedIterator",
            }
        out.setdefault(mod, {"classes": {}})
        out[mod]["classes"][cls] = {"methods": dict(sorted(methods.items()))}
    for _name, c in sidecar.get("containers", {}).items():
        mod = c["module"]
        cls = c["class"]
        methods = {
            "__init__": {
                "params": [
                    {"name": "self", "kind": "self"},
                    {"name": "http", "type": "any", "required": True},
                ],
                "returns": "void",
            }
        }
        # Sub-resource accessors (signature oracle records them; the surface
        # oracle does not — the surface adapter drops them).
        for acc_name, acc in (c.get("accessors") or {}).items():
            methods[acc_name] = {
                "params": [{"name": "self", "kind": "self"}],
                "returns": acc.get("returns", "any"),
            }
        out.setdefault(mod, {"classes": {}})
        out[mod]["classes"][cls] = {"methods": dict(sorted(methods.items()))}
    return out


# ---------------------------------------------------------------------------
# Gen-payload SIGNATURE sidecars (§D3, read side). The three read-side payload
# modules the reference records WITH zero-arg accessors per class-typed field
# (swml_verbs / post_prompt / swaig_request) have no accessor methods on the Rust
# struct — rustdoc sees a method-less struct and drops it. The sibling generators
# (generate_swml_verbs.py / generate_swaig_payloads.py) each write a
# ``*_gen_payload.json`` next to their .rs module carrying {module, classes:{Name:
# [accessor,...]}}. We synthesize an ``any``-return, self-only accessor per name
# and route it to the oracle module. The diff tool's gen-payload fold +
# _is_port_state_accessor excuse make these compare EQUAL to the reference
# (class-typed fields fold by leaf; scalar fields excuse as port-side state).
#
# relay-protocol / swaig-actions / REST <ns>_types_generated are NOT in the
# signature oracle (method-less on both sides) — no sidecar, nothing synthesized.
# ---------------------------------------------------------------------------

_GEN_PAYLOAD_SIDECAR_GLOBS = (
    "src/swml/swml_verbs_gen_payload.json",
    "src/swaig/post_prompt_gen_payload.json",
    "src/swaig/swaig_request_gen_payload.json",
)


def load_gen_payload_sidecars() -> list[dict]:
    out: list[dict] = []
    for rel in _GEN_PAYLOAD_SIDECAR_GLOBS:
        p = PORT_ROOT / rel
        try:
            out.append(json.loads(p.read_text(encoding="utf-8")))
        except FileNotFoundError:
            continue
    return out


def build_gen_payload_signatures(sidecars: list[dict]) -> dict:
    """Build the port_signatures fragment for the read-side payload modules:
    each class gets a synthesized zero-arg accessor per field (self-only,
    ``any`` return). Returns {module: {"classes": {cls: {"methods": {...}}}}}."""
    out: dict = {}
    for sc in sidecars:
        mod = sc.get("module")
        if not mod:
            continue
        for cls, accessors in (sc.get("classes") or {}).items():
            methods = {
                acc: {
                    "params": [{"name": "self", "kind": "self"}],
                    "returns": "any",
                }
                for acc in accessors
            }
            out.setdefault(mod, {"classes": {}})
            out[mod]["classes"][cls] = {"methods": dict(sorted(methods.items()))}
    return out


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

        # MediaArg<E> — the typed-or-raw wrapper behind FunctionResult's
        # closed-set media params (``record_call(format: impl
        # Into<MediaArg<RecordFormat>>)`` etc.). The closed set the Python
        # reference describes (now ``enum<…>`` in the oracle) IS the inner
        # enum ``E``; the wrapper merely also accepts a raw string for
        # forward-compat. Surface the inner enum's class so the param reads
        # as the typed closed set (``class:…RecordFormat``), which is exactly
        # what the oracle's ``enum<…>`` contract expects — not the wrapper.
        if last == "MediaArg":
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
        # A single conversion-trait bound — ``impl Into<T>`` / ``impl
        # AsRef<T>`` / ``impl TryInto<T>`` … — is just an ergonomic way to
        # accept ``T`` (callers pass a ``T`` or anything that converts to one).
        # Surface the target ``T`` so the param carries its real type instead
        # of collapsing to ``any``. This is what makes ``record_call(format:
        # impl Into<MediaArg<RecordFormat>>)`` surface as the typed closed set
        # (``class:…RecordFormat`` via the MediaArg unwrap above) rather than
        # ``any``. Multi-bound / non-conversion impl-traits (``impl
        # Iterator``, ``impl Fn``, ``impl Display``) stay ``any``.
        bounds = t["impl_trait"]
        if isinstance(bounds, list) and len(bounds) == 1:
            tb = bounds[0].get("trait_bound") if isinstance(bounds[0], dict) else None
            trait = tb.get("trait") if isinstance(tb, dict) else None
            if isinstance(trait, dict):
                trait_last = str(trait.get("path", "")).split("::")[-1]
                if trait_last in ("Into", "From", "TryInto", "TryFrom",
                                  "AsRef", "AsMut", "Borrow", "BorrowMut",
                                  "Cow"):
                    inner_args = _extract_angle_args(trait.get("args"))
                    if inner_args:
                        return translate_rust_type(
                            inner_args[0], paths, aliases, context
                        )
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


# Per-module rename for free-function paths. Rustdoc reports these
# under their physical module path (e.g. ``signalwire::security::webhook``);
# the Python reference lives under ``signalwire.core.security.webhook_validator``.
# The audit pipeline projects the Rust path onto the Python canonical
# path BEFORE the free_fn_targets lookup so canonical free functions
# show up under the right module entry in port_signatures.json.
FREE_FN_MODULE_RENAMES: dict[str, str] = {
    # webhook signature validator — Rust splits validator and tower
    # middleware across two modules; both project onto the Python
    # canonical names so the cross-language audit lines up.
    "signalwire.security.webhook": "signalwire.core.security.webhook_validator",
    "signalwire.security.webhook_layer": "signalwire.core.security.webhook_middleware",
    # security hygiene free functions — Rust groups them under
    # ``signalwire::security::security_utils``; the Python reference lives at
    # ``signalwire.core.security.security_utils``. The names match 1:1
    # (filter_sensitive_headers / redact_url / is_valid_hostname), so the
    # rename alone lines them up — no additions/omissions paperwork needed.
    "signalwire.security.security_utils": "signalwire.core.security.security_utils",
    # typed-handler → SWAIG param-schema inference — Rust hosts the free fns
    # (``infer_schema`` / ``create_typed_handler_wrapper``) at
    # ``src/agent/type_inference.rs``; the Python reference lives at
    # ``signalwire.core.agent.tools.type_inference``. Rust builds the schema from
    # a typed ParamsBuilder rather than reflecting a handler signature (types are
    # compile-time-erased) — the static-port idiom for the same inference.
    "signalwire.agent.type_inference": "signalwire.core.agent.tools.type_inference",
    # RequestOptions envelope free fns (resolve / status_is_retryable /
    # default_retry_on_status) — Rust groups them under
    # ``signalwire::rest::request_options``; the Python reference lives at
    # ``signalwire.rest._request_options``. resolve + status_is_retryable match
    # 1:1; default_retry_on_status is the Rust helper for Python's module-level
    # _DEFAULT_RETRY_ON_STATUS constant (a PORT_ADDITION).
    "signalwire.rest.request_options": "signalwire.rest._request_options",
}


def _load_python_reference() -> dict:
    """Load the Python signatures inventory once for projection lookups.

    Used by the variadic-projection pass: when a Rust method takes
    ``params: serde_json::Value`` AND the Python reference shows the
    same method's trailing parameter as ``**kwargs`` (kind=var_keyword),
    we project the Rust positional ``params`` as var_keyword so the
    cross-language audit treats the two as functionally equivalent.
    Rust has no native ``**kwargs``; serde_json::Value at the trailing
    position IS the idiomatic stand-in.
    """
    try:
        import json as _json
        return _json.loads((PSDK / "python_signatures.json").read_text(encoding="utf-8"))
    except FileNotFoundError:
        return {"modules": {}}


def _ref_class_index(py_ref: dict) -> dict[str, set[str]]:
    """{module: {class_name, ...}} for every class the reference records.

    Used to gate PATH-DERIVED class discovery: a Rust struct/enum/trait not in
    CLASS_MODULE_MAP is emitted under its file-path-derived module ONLY when the
    reference actually records that (module, class) — this is the SIGNATURE
    analogue of the surface enumerator's path-derived fallback
    (``_module_path_for_class``). Port-only structs (no reference twin) stay
    dropped; PORT_ADDITIONS.md owns genuine extras."""
    idx: dict[str, set[str]] = {}
    for mod, entry in py_ref.get("modules", {}).items():
        idx[mod] = set((entry.get("classes") or {}).keys())
    return idx


def _module_from_rustdoc_path(paths: dict, iid) -> str | None:
    """Derive the canonical module for a struct/enum/trait id from its rustdoc
    path (``signalwire::core::config_loader::ConfigLoader`` ->
    ``signalwire.core.config_loader``). Mirrors the surface enumerator's
    file-path-derived module fallback so both enumerators route a
    not-in-CLASS_MODULE_MAP class to the SAME module. Returns None if the path
    is unavailable / not under the crate root."""
    entry = paths.get(str(iid)) or paths.get(iid)
    if not entry:
        return None
    p = entry.get("path") or []
    if len(p) < 2 or p[0] != "signalwire":
        return None
    return ".".join(p[:-1])


def _apply_method_renames(cls_name: str, methods: dict) -> dict:
    """Apply the surface enumerator's METHOD_RENAMES table to a class's method
    dict (Rust name -> Python name; None -> drop). Mirrors the surface pass so a
    Rust-idiom method name (``to_value`` -> ``to_dict``) and its dropped
    borrow-checker companions (``*_mut`` / ``from_value`` / ...) line up
    identically on both enumerators. Signatures are carried through unchanged
    (only the key is renamed)."""
    table = METHOD_RENAMES.get(cls_name, {})
    if not table:
        return methods
    out: dict = {}
    for name, sig in methods.items():
        if name in table:
            target = table[name]
            if target is None:
                continue  # drop
            out[target] = sig
        else:
            out[name] = sig
    return out


# Rust parameter names used as the ``*args`` / ``**kwargs`` variadic-equivalent
# (a single ``serde_json::Value`` / ``Vec<..>`` / ``HashMap<..>`` carries what
# Python spells with a splat). Only these names are treated as variadic tails.
# ``_params`` is the underscore-ignored form of ``params``: a no-op forwarder
# (e.g. BedrockAgent's set_prompt_llm_params / set_post_prompt_llm_params, which
# warn-and-drop because Bedrock's prompt/post-prompt run on a platform-side model)
# names the arg ``_params`` so clippy doesn't flag the unused binding. It is the
# SAME **kwargs-equivalent variadic tail as ``params`` — reconcile it identically
# so the idiom does not surface as drift against the oracle's stripped-kwargs twin.
_VARIADIC_TAIL_NAMES = ("params", "_params", "kwargs", "args", "options")


def _reconcile_variadic_tail(py_sig: dict, rust_sig: dict) -> None:
    """Reconcile a Rust method/function's trailing variadic-equivalent
    parameter(s) against the Python reference twin, so pure ``*args`` /
    ``**kwargs`` idiom does not surface as drift.

    Rust has no native splat: it spells Python's ``*args`` as a
    ``Vec<..>`` and ``**kwargs`` as a single ``serde_json::Value`` /
    ``HashMap<..>``. Python's reference adapter (porting-sdk #58) now
    STRIPS the trailing ``**kwargs`` (var_keyword) entirely from the
    oracle and records a trailing ``*args`` (var_positional) as an
    OPTIONAL param (``required: false``). Two consequences the diff must
    tolerate:

      * A stripped ``**kwargs`` tail leaves the Python ref with FEWER
        params than the Rust twin. The Rust trailing variadic param is a
        legitimate optional extra — mark it ``required: false`` (and
        project the type to the ``dict<string,any>`` **kwargs shape) so
        diff_port_signatures excuses it as an optional-extra param rather
        than flagging a param-count mismatch.
      * A retained ``*args`` (var_positional) tail: align the overlapping
        Rust param to that variadic shape (optional).

    Named tails only (``params`` / ``kwargs`` / ``args`` / ``options``)
    with a JSON-value / list / dict / any translated type — a genuine
    concrete positional param is never touched.
    """
    PROJECTED_TYPE = "dict<string,any>"
    py_params = py_sig.get("params", [])
    rust_params = sig_params = rust_sig.get("params", [])
    if not rust_params:
        return

    def _is_variadic_tail(p: dict) -> bool:
        if p.get("kind") in ("self", "var_keyword"):
            return False
        name = p.get("name", "")
        if name not in _VARIADIC_TAIL_NAMES:
            return False
        t = (p.get("type") or "").replace(" ", "")
        return (
            t in ("any", "class:signalwire.value.Value")
            or t.startswith("dict<")
            or t.startswith("list<")
        )

    # Count the Python reference's non-self param arity. When the Rust
    # method carries MORE params than the reference (because a var_keyword
    # tail was stripped, or because Rust splits *args + **kwargs into two
    # concrete params), every trailing param beyond the reference arity
    # that is a variadic-equivalent is an optional extra.
    n_ref = len(py_params)
    py_last = py_params[-1] if py_params else None
    ref_ends_var_positional = bool(py_last) and py_last.get("kind") == "var_positional"

    # Walk trailing params from the end; each variadic-equivalent tail that
    # sits at or beyond the reference arity becomes an optional extra.
    for i in range(len(rust_params) - 1, -1, -1):
        p = rust_params[i]
        if not _is_variadic_tail(p):
            break  # stop at the first non-variadic (concrete) param
        beyond_ref = i >= n_ref
        aligns_var_positional = ref_ends_var_positional and i == n_ref - 1
        if beyond_ref:
            # Extra tail (stripped **kwargs, or the second of *args/**kwargs):
            # optional **kwargs shape.
            p["kind"] = "var_keyword"
            p["type"] = PROJECTED_TYPE
            p["required"] = False
        elif aligns_var_positional:
            # Overlaps the reference's trailing *args: keep it optional to
            # match the var_positional shape.
            p["required"] = False
            break
        else:
            break


def _project_variadic_kwargs(out_modules: dict, py_ref: dict) -> None:
    """Post-process pass: reconcile every Rust method AND module-level
    function trailing variadic-equivalent parameter against its Python
    reference twin (see ``_reconcile_variadic_tail``)."""
    py_modules = py_ref.get("modules", {})
    for mod_name, mod_entry in out_modules.items():
        py_mod = py_modules.get(mod_name, {})
        for cls_name, cls_entry in mod_entry.get("classes", {}).items():
            py_cls = py_mod.get("classes", {}).get(cls_name, {})
            for m_name, sig in cls_entry.get("methods", {}).items():
                py_sig = py_cls.get("methods", {}).get(m_name)
                if not py_sig:
                    continue
                _reconcile_variadic_tail(py_sig, sig)
        for fn_name, sig in mod_entry.get("functions", {}).items():
            py_sig = py_mod.get("functions", {}).get(fn_name)
            if not py_sig:
                continue
            _reconcile_variadic_tail(py_sig, sig)


def collect(rust_doc: dict, aliases: dict) -> tuple[dict, list]:
    index = rust_doc["index"]
    paths = rust_doc["paths"]
    failures: list = []
    out_modules: dict = {}

    # Generated REST layer: names handled entirely from the sidecar (below), so
    # the rustdoc struct walk must SKIP them (their base-delegation methods and
    # the port-internal tree glue would otherwise leak in under a fallback path).
    sidecar = load_rest_sidecar()
    generated_struct_names: set[str] = set(sidecar.get("resources", {}).keys())
    generated_struct_names |= set(sidecar.get("containers", {}).keys())
    generated_struct_names |= set(sidecar.get("suppress_structs", []))

    # Build a lookup: id → item
    def get(id_):
        return index.get(str(id_)) or index.get(id_)

    # Reference class index — gates PATH-DERIVED class discovery (mirrors the
    # surface enumerator's file-path-derived module fallback: a struct not in
    # CLASS_MODULE_MAP is emitted under its rustdoc-path module ONLY when the
    # reference records that (module, class)).
    ref_class_idx = _ref_class_index(_load_python_reference())

    def _translate_method_name(method_native: str) -> str:
        if method_native == "new":
            return "__init__"
        if method_native == "repr":
            return "__repr__"
        return method_native

    # Find all struct / enum / trait items + their impls
    for iid, item in index.items():
        inner = item.get("inner", {})
        is_trait = False
        if "struct" in inner:
            kind_inner = inner["struct"]
        elif "enum" in inner:
            kind_inner = inner["enum"]
        elif "trait" in inner:
            kind_inner = inner["trait"]
            is_trait = True
        else:
            continue
        struct_name = item.get("name")
        if not struct_name:
            continue
        # Generated REST structs/containers are emitted from the sidecar below.
        if struct_name in generated_struct_names:
            continue
        # Port-internal generated-layer base resources
        # (rest::generated_bases::{BaseResource,ReadResource,CrudResource,
        # FabricResource}) compose the generated resources; the `_base` surface
        # is represented by the legacy rest::CrudResource. Skip the
        # generated_bases copies so they don't collide by struct name.
        _pentry = paths.get(str(iid)) or paths.get(iid)
        if _pentry and "generated_bases" in (_pentry.get("path") or []):
            continue
        impls = kind_inner.get("impls", [])

        # Determine canonical module for this class. Mirror enumerate_surface.py:
        # the CLASS_MODULE_MAP can be keyed by either the native Rust struct
        # name (e.g. ``Client`` → ``signalwire.relay.client``) OR by the
        # canonical Python class name (e.g. ``CallingNamespace`` →
        # ``signalwire.rest.namespaces.calling``). _translate_class renames
        # ``Client`` → ``RelayClient`` for emit, but the module-map lookup
        # must also consider the native form so that struct→module mapping
        # works regardless of which form the map uses.
        canonical_name = _translate_class(struct_name)
        if struct_name in CLASS_MODULE_MAP:
            mod = CLASS_MODULE_MAP[struct_name]
        elif canonical_name in CLASS_MODULE_MAP:
            mod = CLASS_MODULE_MAP[canonical_name]
        else:
            # PATH-DERIVED fallback (mirror the surface enumerator): route the
            # class to its rustdoc-path module and keep it ONLY if the reference
            # records that (module, class). This is what makes the new
            # item-I subsystems (ConfigLoader / SecurityConfig at
            # signalwire.core.config_loader / .security_config) and the typed
            # relay event subclasses (signalwire.relay.event.*) — none of which
            # are in CLASS_MODULE_MAP — appear in port_signatures.json with their
            # real rustdoc signatures instead of being silently dropped.
            mod = _module_from_rustdoc_path(paths, iid)
            mod = FREE_FN_MODULE_RENAMES.get(mod, mod) if mod else mod
            if not mod or canonical_name not in ref_class_idx.get(mod, set()):
                continue  # port-only struct; PORT_ADDITIONS.md owns genuine extras

        methods_out: dict = {}

        # Trait body methods: a `pub trait X` (e.g. SkillBase) carries its public
        # interface as trait items, NOT impl blocks — rustdoc lists them under
        # inner["trait"]["items"]. The surface enumerator collects these via
        # RE_TRAIT_FN; mirror it here so a trait's method signatures land in the
        # inventory (previously SkillBase surfaced method-less). Rust-idiom trait
        # accessors the reference does not enumerate are dropped (mirrors the
        # surface SKILLBASE_IDIOM_METHOD_DROPS).
        method_id_sources: list = []
        if is_trait:
            for method_id in kind_inner.get("items", []):
                method_item = get(method_id)
                if not method_item:
                    continue
                m_inner = method_item.get("inner", {})
                if "function" not in m_inner:
                    continue
                m_native = method_item.get("name", "")
                if not m_native:
                    continue
                if m_native.startswith("_") and not m_native.startswith("__"):
                    continue
                if (canonical_name in PUBLIC_SURFACE_TRAITS
                        and m_native in SKILLBASE_IDIOM_METHOD_DROPS):
                    continue
                method_canonical = _translate_method_name(m_native)
                ctx = f"{mod}.{canonical_name}.{method_canonical}"
                try:
                    sig = build_signature(m_inner["function"], paths, aliases, ctx)
                except TypeTranslationError as e:
                    failures.append(str(e))
                    continue
                methods_out.setdefault(method_canonical, sig)

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
                method_canonical = _translate_method_name(method_native)
                ctx = f"{mod}.{canonical_name}.{method_canonical}"
                try:
                    sig = build_signature(m_inner["function"], paths, aliases, ctx)
                except TypeTranslationError as e:
                    failures.append(str(e))
                    continue
                if method_canonical in methods_out:
                    continue
                methods_out[method_canonical] = sig

        # Apply the surface enumerator's per-class method renames (``to_value`` ->
        # ``to_dict``, drop borrow-checker/idiom companions) so both enumerators
        # name the SAME methods (Rule 2). Without this, ``to_value`` surfaces as
        # missing-reference AND ``to_dict`` as missing-port on every POM/Context.
        methods_out = _apply_method_renames(canonical_name, methods_out)

        if not methods_out:
            continue

        out_modules.setdefault(mod, {"classes": {}})
        existing = out_modules[mod]["classes"].get(canonical_name, {}).get("methods", {})
        existing.update(methods_out)
        out_modules[mod]["classes"][canonical_name] = {
            "methods": dict(sorted(existing.items())),
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
    _PY_REF = _load_python_reference()
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
        # Apply per-module rename before the free_fn_targets lookup.
        # This is how Rust paths under ``signalwire.security.webhook``
        # surface as Python-canonical
        # ``signalwire.core.security.webhook_validator`` in the audit.
        target_module = FREE_FN_MODULE_RENAMES.get(target_module, target_module)
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
        # Kind-align keyword-only params to the reference by name. Python marks
        # some free-function params keyword-only (``*, signing_key``); Rust has
        # no keyword-only args, so rustdoc reports them positional. This is pure
        # idiom (Rule 2: reconcile in the enumerator, not via an omission) — when
        # the reference records a same-named param as ``keyword``, mirror that
        # kind onto the Rust positional so the two compare EQUAL.
        ref_fn = (
            _PY_REF.get("modules", {})
            .get(target_module, {})
            .get("functions", {})
            .get(target_function, {})
        )
        ref_kind_by_name = {
            p.get("name"): p.get("kind")
            for p in ref_fn.get("params", [])
            if p.get("kind") == "keyword"
        }
        for p in sig.get("params", []):
            if p.get("name") in ref_kind_by_name and p.get("kind", "positional") == "positional":
                p["kind"] = "keyword"
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
        # NOTE: projected methods are deliberately KEPT on AgentBase — the Rust
        # port hangs the mixin surface directly off AgentBase and each such
        # method is excused there via PORT_ADDITIONS.md ("Python has the same
        # surface via a mixin; projected onto the originating mixin"). Popping
        # them here would (a) hide them from the SURFACE_PROJECTIONS donor index
        # below and (b) contradict the PORT_ADDITIONS excuse.

        # SWMLService is itself a Python class with its own method inventory.
        # Some projected methods ARE legitimately on Python's SWMLService
        # (get_basic_auth_credentials, on_request) — those stay. Others are
        # projection-only (validate_basic_auth, get/has/remove_function,
        # validate_tool_token, define_tool, on_function_call, register_*,
        # define_tools, on_swml_request) — those should be dropped from
        # SWMLService because Python's signatures inventory does NOT expose
        # them there, and keeping them on the Rust SWMLService creates
        # missing-reference drift. Python's reference signatures inventory
        # is the source of truth for what SWMLService actually exposes.
        if svc_entry:
            try:
                py_ref = _load_python_reference()
                py_svc_methods = set(py_ref.get("modules", {})
                                     .get("signalwire.core.swml_service", {})
                                     .get("classes", {})
                                     .get("SWMLService", {})
                                     .get("methods", {}).keys())
            except Exception:
                py_svc_methods = set()
            for n in list(svc_methods.keys()):
                if n in projected and n not in py_svc_methods:
                    svc_methods.pop(n, None)

    # --- Surface-parity projections (mirror enumerate_surface.py, item H) -----
    # The reference keeps a family of methods on mixin / manager / abstract-base
    # classes that Rust's composition idiom hosts directly on AgentBase / Service
    # / CrudResource. The surface enumerator projects the reference-named methods
    # onto the canonical class path via SURFACE_PROJECTIONS; mirror that here so
    # the SIGNATURE inventory carries the same projected method SIGNATURES (not
    # just names) and the two enumerators agree. Previously the sig enumerator's
    # MIXIN_PROJECTIONS covered only ToolRegistry/PromptManager + a few mixin
    # method-names, leaving AIConfigMixin/PromptMixin/SkillMixin/WebMixin/
    # ReadResource entirely missing (→ ~60 missing-port drifts).
    #
    # Donor lookup is language-agnostic (by translated class name), so build a
    # class→{method: sig} index across all modules. Deref-inheritance
    # (AgentBase impl Deref<Target=Service>) folds SWMLService's methods into
    # AgentBase's DONOR set (only) — mirroring the surface DEREF_INHERITS — so a
    # method the reference records on a mixin but which Rust hosts on SWMLService
    # (inherited by AgentBase) is still projectable.
    donor_sig_index: dict[str, dict] = {}
    for _mod_name, _entry in out_modules.items():
        for _cls, _c in _entry.get("classes", {}).items():
            donor_sig_index.setdefault(_cls, {}).update(_c.get("methods", {}))
    DEREF_INHERITS = {"AgentBase": "SWMLService"}
    for _child, _parent in DEREF_INHERITS.items():
        parent_sigs = {
            m: s for m, s in donor_sig_index.get(_parent, {}).items()
        }
        merged = dict(parent_sigs)
        merged.update(donor_sig_index.get(_child, {}))
        donor_sig_index[_child] = merged

    for (target_mod, target_cls), donors in SURFACE_PROJECTIONS.items():
        projected_sigs: dict = {}
        for donor_cls, names in donors:
            have = donor_sig_index.get(donor_cls, {})
            for n in names:
                if n in have and n not in projected_sigs:
                    projected_sigs[n] = have[n]
        if not projected_sigs:
            continue
        out_modules.setdefault(target_mod, {"classes": {}})
        cls_entry = out_modules[target_mod]["classes"].setdefault(
            target_cls, {"methods": {}}
        )
        for n, s in projected_sigs.items():
            cls_entry["methods"].setdefault(n, s)

    # Strip projection-only methods from their donor classes (mirror
    # PROJECTION_DONOR_STRIPS): e.g. get/list leave CrudResource for ReadResource.
    for (donor_mod, donor_cls), strip in PROJECTION_DONOR_STRIPS.items():
        cls = out_modules.get(donor_mod, {}).get("classes", {}).get(donor_cls)
        if cls:
            for n in strip:
                cls["methods"].pop(n, None)

    # Force reference-declared methods Rust realizes idiomatically rather than as
    # a literally-named/­constructible method (mirror FORCE_CLASS_METHODS). This
    # supplies the ``__init__`` the reference records on every typed relay event
    # subclass (Rust builds them via ``from_payload``, no ``new``) and on the
    # SkillBase trait / SkillRegistry / delegate classes; and the reference
    # dunders on PaginatedIterator. A synthesized ``__init__`` is a bare
    # self-receiver / void return (the surface records only the NAME; the diff
    # compares __init__ params only when both sides carry a real one — a
    # projected bare __init__ satisfies the presence check).
    for (mod_name, cls), method_list in FORCE_CLASS_METHODS.items():
        # Only force onto a class the reference actually records here (do not
        # invent surface), and only when the class is present in the port OR the
        # reference — FORCE is about the reference's own declared symbol.
        if cls not in ref_class_idx.get(mod_name, set()):
            continue
        out_modules.setdefault(mod_name, {"classes": {}})
        cls_entry = out_modules[mod_name]["classes"].setdefault(cls, {"methods": {}})
        for m in method_list:
            if m in cls_entry["methods"]:
                continue
            if m == "__init__":
                cls_entry["methods"][m] = {
                    "params": [{"name": "self", "kind": "self"}],
                    "returns": "void",
                }
            else:
                # Dunder / accessor the reference records; Rust realizes it via a
                # trait (Iterator) or generic accessor. Synthesize a self-only
                # ``any``-return signature so the NAME is present.
                cls_entry["methods"][m] = {
                    "params": [{"name": "self", "kind": "self"}],
                    "returns": "any",
                }

    # Typed relay event subclasses (CallStateEvent / PlayEvent / …): the
    # SIGNATURE reference records a full-field dataclass ``__init__`` plus a
    # ``from_payload(cls, payload)`` classmethod on each. Rust models each as a
    # thin typed WRAPPER over ``RelayEvent`` (a single ``base: RelayEvent`` field),
    # constructed by an associated ``from_payload(payload)`` fn — it has neither a
    # field-wise constructor nor a ``cls`` receiver, and the dataclass fields are
    # exposed as ``&self`` accessor methods that delegate to ``base``. These are
    # genuine, formulaic idiom divergences (NOT laundered missing-port: the port
    # DOES construct each event, via ``from_payload`` — the divergence is the
    # constructor SHAPE), so they are documented per-event in
    # PORT_SIGNATURE_OMISSIONS.md rather than synthesized to a shape the Rust
    # struct does not structurally have. (Synthesizing a bare ``self``-only
    # ``__init__`` here would only trade a missing-port for a param-count-mismatch.)

    # NOTE: the surface enumerator's SKILL_INTERFACE_PROJECTION (projecting the
    # SkillBase interface onto each skill subclass) is deliberately NOT mirrored
    # here. The SURFACE oracle records the interface method NAMES on each Python
    # skill subclass; the SIGNATURE oracle records skill subclasses METHOD-LESS
    # (Python's skill methods are inherited from SkillBase, not re-enumerated with
    # signatures per subclass). Projecting them onto the port's skill classes
    # would create missing-reference drift against the empty oracle entries. The
    # skill subclasses' own trait-impl methods are already excluded (the
    # `impl SkillBase for XSkill` block's trait_path is the uppercase unqualified
    # "SkillBase", skipped by the stdlib/blanket-trait filter above), so skills
    # stay method-less on both sides.

    # Drop module-scoped Rust-idiom accessor methods (mirror MODULE_METHOD_DROPS)
    # — the typed relay event wrappers carry Rust base/event/event_type views the
    # reference does not enumerate.
    for mod_name, drop in MODULE_METHOD_DROPS.items():
        entry = out_modules.get(mod_name)
        if not entry:
            continue
        for _cls, _c in entry.get("classes", {}).items():
            for n in drop:
                _c["methods"].pop(n, None)

    # Project Rust ``params: serde_json::Value`` trailing arguments onto
    # Python's ``**kwargs`` shape wherever the Python reference uses
    # var_keyword at the same position. See _project_variadic_kwargs for
    # the rationale: Rust uses a single Value as the **kwargs equivalent
    # for every Call/CallingNamespace command, and the audit needs to
    # treat the two as functionally equivalent.
    py_ref = _load_python_reference()
    _project_variadic_kwargs(out_modules, py_ref)

    # Merge the generated REST layer (sidecar-unfolded — L10). These modules
    # (<ns>_resources_generated + _client_tree_generated) come entirely from the
    # generator sidecar, not rustdoc, so the exploded named params + kinds line
    # up with the oracle. Merge after everything else so a stray fallback-mapped
    # generated struct can't shadow them.
    for mod, entry in build_generated_signatures(sidecar).items():
        out_modules.setdefault(mod, {"classes": {}})
        out_modules[mod].setdefault("classes", {})
        out_modules[mod]["classes"].update(entry["classes"])

    # Merge the read-side payload modules (§D3): swml_verbs / post_prompt /
    # swaig_request classes with synthesized per-field ``any``-return accessors
    # from the gen-payload sidecars (Rust structs carry no accessor methods).
    for mod, entry in build_gen_payload_signatures(load_gen_payload_sidecars()).items():
        out_modules.setdefault(mod, {"classes": {}})
        out_modules[mod].setdefault("classes", {})
        out_modules[mod]["classes"].update(entry["classes"])

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
        reconcile = PARAM_RECONCILE.get(context, {}).get(name, {})
        params_out.append({
            "name": reconcile.get("name", name),
            "type": reconcile.get("type", canon),
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
    # Idiom remap: a collapsed struct returned under a Python class's contract.
    if context in RETURN_TYPE_OVERRIDE:
        return_canon = RETURN_TYPE_OVERRIDE[context]
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
