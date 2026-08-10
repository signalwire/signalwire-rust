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
    - Rust has NO language-level default parameter values (there is no
      ``fn f(x: i32 = 5)``), so a plain positional param is genuinely
      required and carries no ``default``. Where the port DOES express a
      default, it does so through one of three source constructs, which
      ``extract_defaults`` recovers from the rustdoc ``span`` (see that
      section's docstring). Anything else honestly stays required.
    - Rust also has no OMITTABLE arguments, so ``required`` is read from
      the type, not the arity: an ``Option<T>`` param models absence
      (``None`` IS the don't-supply-it call) and is ``required: false``;
      a bare ``T`` is ``required: true``. See the PARAMETER OPTIONALITY
      section for the one enumerated reference-side exception.
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
import re
import subprocess
import sys
from pathlib import Path

import yaml

HERE = Path(__file__).resolve().parent
PORT_ROOT = HERE.parent
sys.path.insert(0, str(HERE))
from enumerate_surface import (  # type: ignore
    # SHARE the resolved porting-sdk path rather than re-deriving it. Both
    # enumerators read the same oracle, and a duplicated resolver is the shape
    # that lets the two gates disagree about where the oracle lives — the same
    # duplication hazard that made a rename table apply in one gate and not the
    # other (go, typescript, php, and rust's own METHOD_RENAMES keying bug).
    PSDK,
    CLASS_MODULE_MAP,
    _translate_class,
    # Idiom-reconciliation tables mirrored from the SURFACE enumerator so the
    # two enumerators discover/name the SAME symbols (Rule 2: reconcile idiom
    # in the enumerator, not via an omission). Kept as a single source of truth
    # by importing them rather than re-declaring.
    METHOD_RENAMES,
    SURFACE_PROJECTIONS,
    PROJECTION_DONOR_STRIPS,
    FORCE_CLASS_METHODS,
    SKILLBASE_IDIOM_METHOD_DROPS,
    PUBLIC_SURFACE_TRAITS,
    MODULE_METHOD_DROPS,
    MODULE_METHOD_DROP_EXCEPTIONS,
    PUBLIC_FIELD_RENAMES,
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
    f"signalwire.rest.namespaces.fabric.FabricNamespace.{m}": "class:signalwire.rest.namespaces.fabric.FabricResourcePUT"
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
RETURN_TYPE_OVERRIDE.update(
    dict.fromkeys(
        (
            "signalwire.core.swml_service.SWMLService.as_router",
            "signalwire.core.mixins.web_mixin.WebMixin.as_router",
            "signalwire.core.agent_base.AgentBase.as_router",
        ),
        "class:signalwire.core.web.HostAppRouter",
    )
)

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
PARAM_RECONCILE[
    "signalwire.core.agent.tools.type_inference.create_typed_handler_wrapper"
] = {
    "func": {"type": "callable<list<any>,any>"},
}

# EXPLICIT-RECEIVER ELISION for public-surface trait methods. Python binds a
# skill to its agent once (``self.agent``, set by the loader) and every interface
# method reads it off ``self``; Rust's ``SkillBase`` is not agent-bound (a skill
# is a ``&self`` trait object and cannot hold a ``&mut AgentBase`` across the
# borrow), so the agent is threaded in as an explicit argument:
# ``fn register_tools(&self, agent: &mut AgentBase)`` vs
# ``def register_tools(self) -> None``. That argument IS Python's ``self.agent``
# — the same receiver, spelled positionally because the borrow checker forbids
# storing it. This is IDIOM, folded here at the enumerator (Rule 2) rather than
# documented as 14 near-identical omissions (SkillBase + every concrete skill),
# which is exactly the shape §3 forbids. The method still compares in full: a
# changed return type, a NEW parameter, or a disappeared method all still drift.
# Shape: {trait: {method: {param names to elide}}}.
SURFACE_TRAIT_RECEIVER_PARAMS: dict[str, dict[str, set[str]]] = {
    "SkillBase": {"register_tools": {"agent"}},
}

# KWARGS-MAP EXPLOSION. Rust has neither keyword arguments nor `**kwargs`, so a
# reference method whose parameters are a flat set of optional named keys is
# spelled as ONE `&Map<String, Value>` the body then reads BY THOSE NAMES:
#
#   Python  AIVerbHandler.build_config(self, prompt_text=None, prompt_pom=None,
#                                      contexts=None, post_prompt=None,
#                                      post_prompt_url=None, swaig=None, **kwargs)
#   Rust    fn build_config(&self, args: &Map<String, Value>) -> Value
#             args.get("prompt_text") / .get("prompt_pom") / .get("contexts") /
#             .get("post_prompt") / .get("post_prompt_url") / .get("swaig")
#
# The port accepts every reference key; only the CALL SPELLING differs. Explode
# the single map slot into the reference's recorded parameter list so the method
# compares in FULL — this is the enumerator folding idiom (Rule 2), the same
# treatment `build_ai_chat_signatures` gives the options-object collapse, not an
# omission that would blind the gate to a key going missing.
#
# The explosion is ORACLE-SOURCED: the emitted params are copied from the
# reference's own recorded signature, so it cannot invent a parameter the
# reference lacks and it tracks the oracle without a hand-maintained list. Shape:
# {"module.Class.method": "rust map param name"}.
KWARGS_MAP_EXPLODE: dict[str, str] = {
    "signalwire.core.swml_handler.AIVerbHandler.build_config": "args",
}

# Return-type reconcile for the type_inference free fns: rustdoc leaks the
# `TypedHandler` / `InferredSchema` type aliases as class names. Map them to the
# concrete canonical types they alias — `create_typed_handler_wrapper` returns a
# handler callable; `infer_schema` returns the `(parameters, required,
# description, is_typed, has_raw_data)` tuple — so both return-compare equal to
# the oracle (the tool handling the idiom, not an omission).
RETURN_TYPE_OVERRIDE.update(
    {
        "signalwire.core.agent.tools.type_inference.create_typed_handler_wrapper": "callable<list<any>,any>",
        "signalwire.core.agent.tools.type_inference.infer_schema": "tuple<dict<string,dict<string,any>>,list<string>,optional<string>,bool,bool>",
    }
)


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

_REST_SIDECAR_PATH = (
    PORT_ROOT / "src" / "rest" / "namespaces" / "generated" / "rest_signatures.json"
)


def _require_oracle(path: Path) -> dict:
    """Read a REQUIRED reference oracle, or abort naming the file.

    Every oracle read in this module gates which members get emitted or how they
    get projected. Degrading an unreadable one to ``{}`` does not fail loudly --
    it makes the enumerator write a SHORT-BUT-VALID port_signatures.json and exit
    0, and that artifact is exactly what the SIGNATURES and DRIFT gates then
    compare the port against. The port gets blamed for omissions it never had.
    The oracle ships with porting-sdk; absence is a broken checkout, never a
    supported degraded mode."""
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(
            f"enumerate_signatures: cannot read the reference oracle {path}: {exc}\n"
            "Continuing would write a short-but-valid port_signatures.json and "
            "exit 0, so DRIFT would compare the port against a fiction."
        ) from exc


def load_rest_sidecar() -> dict:
    """The generated REST layer's adapter sidecar. FAIL LOUD if absent.

    It carries the typed-param unfold for every generated resource, so an empty
    sidecar silently reduces the whole generated REST surface to its raw
    builder shape and still exits 0 — a short inventory that DRIFT then reads as
    the port's real signatures. generate_rest.py commits this file next to the
    code it describes; its absence is a broken tree, not a supported mode."""
    try:
        return json.loads(_REST_SIDECAR_PATH.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SystemExit(
            f"enumerate_signatures: cannot read the REST sidecar {_REST_SIDECAR_PATH}: {exc}\n"
            "Re-run scripts/generate_rest.py; continuing would emit a short "
            "port_signatures.json at rc=0."
        ) from exc


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
    for r in sidecar.get("resources", {}).values():
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
    for c in sidecar.get("containers", {}).values():
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
# swaig-actions joined this set once the generator started emitting the response
# ENVELOPE types: the reference records SwaigAction.{context_switch, hold,
# playback_bg, transfer} and SwaigResponse.action as class-typed fields, so the
# module IS in the signature oracle and needs its sidecar like the other three.
#
# relay-protocol / REST <ns>_types_generated are NOT in the signature oracle
# (method-less on both sides) — no sidecar, nothing synthesized.
# ---------------------------------------------------------------------------

_GEN_PAYLOAD_SIDECAR_GLOBS = (
    "src/swml/swml_verbs_gen_payload.json",
    "src/swaig/post_prompt_gen_payload.json",
    "src/swaig/swaig_request_gen_payload.json",
    "src/swaig/swaig_actions_gen_payload.json",
)


def load_gen_payload_sidecars() -> list[dict]:
    # FAIL LOUD: each sidecar contributes a whole read-side payload MODULE to the
    # inventory. Skipping an unreadable one drops that module entirely while the
    # run still exits 0. generate_swaig_payloads.py commits all of them.
    out: list[dict] = []
    for rel in _GEN_PAYLOAD_SIDECAR_GLOBS:
        p = PORT_ROOT / rel
        try:
            out.append(json.loads(p.read_text(encoding="utf-8")))
        except (OSError, json.JSONDecodeError) as exc:
            raise SystemExit(
                f"enumerate_signatures: cannot read the SWAIG payload sidecar {p}: {exc}\n"
                "Re-run scripts/generate_swaig_payloads.py; continuing would drop "
                "that payload module from port_signatures.json at rc=0."
            ) from exc
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
            inner = (
                translate_rust_type(type_args[0], paths, aliases, context)
                if type_args
                else "any"
            )
            return f"optional<{inner}>"
        if last in ("Vec", "VecDeque"):
            inner = (
                translate_rust_type(type_args[0], paths, aliases, context)
                if type_args
                else "any"
            )
            return f"list<{inner}>"
        if last in ("HashMap", "BTreeMap", "IndexMap"):
            if len(type_args) >= 2:
                k = translate_rust_type(type_args[0], paths, aliases, context)
                v = translate_rust_type(type_args[1], paths, aliases, context)
                return f"dict<{k},{v}>"
            return "dict<string,any>"
        if last in ("HashSet", "BTreeSet"):
            inner = (
                translate_rust_type(type_args[0], paths, aliases, context)
                if type_args
                else "any"
            )
            return f"list<{inner}>"
        if last == "Result":
            # Result<T, E> → T (the Err type is out-of-band in Python)
            return (
                translate_rust_type(type_args[0], paths, aliases, context)
                if type_args
                else "any"
            )
        if last in ("Box", "Arc", "Rc", "Mutex", "RwLock"):
            return (
                translate_rust_type(type_args[0], paths, aliases, context)
                if type_args
                else "any"
            )

        # MediaArg<E> — the typed-or-raw wrapper behind FunctionResult's
        # closed-set media params (``record_call(format: impl
        # Into<MediaArg<RecordFormat>>)`` etc.). The closed set the Python
        # reference describes (now ``enum<…>`` in the oracle) IS the inner
        # enum ``E``; the wrapper merely also accepts a raw string for
        # forward-compat. Surface the inner enum's class so the param reads
        # as the typed closed set (``class:…RecordFormat``), which is exactly
        # what the oracle's ``enum<…>`` contract expects — not the wrapper.
        if last == "MediaArg":
            return (
                translate_rust_type(type_args[0], paths, aliases, context)
                if type_args
                else "any"
            )

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
                if trait_last in (
                    "Into",
                    "From",
                    "TryInto",
                    "TryFrom",
                    "AsRef",
                    "AsMut",
                    "Borrow",
                    "BorrowMut",
                    "Cow",
                ):
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
        canon_args = [
            translate_rust_type(
                it[1] if isinstance(it, list) else it, paths, aliases, context
            )
            for it in inputs
        ]
        output = sig_decl.get("output")
        canon_ret = (
            translate_rust_type(output, paths, aliases, context) if output else "void"
        )
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
        return [
            a["type"] for a in ab.get("args", []) if isinstance(a, dict) and "type" in a
        ]
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
    # FAIL LOUD: an unreadable oracle here yields an EMPTY target set, so every
    # canonical free function silently stops being projected and the inventory
    # ships short at rc=0.
    ref = _require_oracle(PSDK / "python_signatures.json")
    for mod_name, mod_entry in ref.get("modules", {}).items():
        for fn_name in mod_entry.get("functions") or {}:
            targets.add((mod_name, fn_name))
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
    # FAIL LOUD: an empty reference silently disables the variadic projection, so
    # every `params: serde_json::Value` tail stays positional and DRIFT reports
    # kind-mismatches the port does not actually have.
    return _require_oracle(PSDK / "python_signatures.json")


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


_SIG_ORACLE_MEMBERS: dict[tuple[str, str], set[str]] | None = None


def _sig_oracle_members() -> dict[tuple[str, str], set[str]]:
    """(module, class) -> the members the reference SIGNATURE oracle records.

    The authority for the oracle-gated idiom drops below. This is the SIGNATURE
    twin of ``enumerate_surface._oracle_class_members``; every oracle exclusion
    must be gated in BOTH enumerators in lockstep or the two gates become
    mutually exclusive (that happened once fleet-wide and three lanes root-caused
    it independently — porting-sdk CAMPAIGN_STATE §9.4).

    FAILS LOUD on an unresolvable oracle: a silently-empty oracle would make
    every gated drop apply again and emit a valid-looking snapshot missing the
    members, which is the resolver trap that cost dotnet/go/cpp a full CI
    investigation each.
    """
    global _SIG_ORACLE_MEMBERS
    if _SIG_ORACLE_MEMBERS is None:
        ref = _load_python_reference()
        if not ref.get("modules"):
            raise SystemExit(
                "enumerate_signatures: cannot read the reference signature oracle at "
                f"{PSDK / 'python_signatures.json'}.\n"
                "  Set $PORTING_SDK to the porting-sdk checkout, or clone it as a sibling\n"
                "  of this repo. Refusing to emit a snapshot with the oracle-gated members\n"
                "  silently dropped."
            )
        out: dict[tuple[str, str], set[str]] = {}
        for mod, inv in ref.get("modules", {}).items():
            for cls, ce in (inv.get("classes") or {}).items():
                methods = ce.get("methods")
                if isinstance(methods, dict):
                    out[(mod, cls)] = set(methods.keys())
        _SIG_ORACLE_MEMBERS = out
    return _SIG_ORACLE_MEMBERS


_SIG_ORACLE_SIGNATURES: dict[tuple[str, str], dict] | None = None


def _sig_oracle_signatures() -> dict[tuple[str, str], dict]:
    """(module, class) -> {member: the reference's recorded SIGNATURE}.

    The members twin above answers "does the reference have this member"; this
    answers "and what does it look like", which is what the arity-idiom
    collision resolver in ``_apply_method_renames`` needs to decide WHICH of two
    Rust spellings of one reference method is the closer match. Same fail-loud
    oracle resolution (``_sig_oracle_members`` raises first if unreadable)."""
    global _SIG_ORACLE_SIGNATURES
    if _SIG_ORACLE_SIGNATURES is None:
        _sig_oracle_members()  # fail loud on an unresolvable oracle
        ref = _load_python_reference()
        out: dict[tuple[str, str], dict] = {}
        for mod, inv in ref.get("modules", {}).items():
            for cls, ce in (inv.get("classes") or {}).items():
                methods = ce.get("methods")
                if isinstance(methods, dict):
                    out[(mod, cls)] = methods
        _SIG_ORACLE_SIGNATURES = out
    return _SIG_ORACLE_SIGNATURES


def _apply_method_renames(
    cls_name: str, methods: dict, module: str | None = None, py_cls: str | None = None
) -> dict:
    """Apply the surface enumerator's METHOD_RENAMES table to a class's method
    dict (Rust name -> Python name; None -> drop). Mirrors the surface pass so a
    Rust-idiom method name (``to_value`` -> ``to_dict``) and its dropped
    borrow-checker companions (``*_mut`` / ``from_value`` / ...) line up
    identically on both enumerators. Signatures are carried through unchanged
    (only the key is renamed).

    A ``None`` drop is ORACLE-GATED, exactly as in the surface enumerator: the
    drop asserts "the reference records no such member here", so when the oracle
    DOES record it the drop is stale and the Rust accessor IS that member. Pass
    ``module``/``py_cls`` (the CANONICAL post-translate key — the same key space
    the emitter writes into) to enable the gate; without them the drop is
    unconditional, preserving the pre-gate behaviour for callers that have no
    canonical key yet.

    ARITY-IDIOM COLLISION (``add_section`` + ``add_section_with`` ->
    ``add_section``): Rust has no default arguments and no overloading, so ONE
    reference method with optional kwargs is spelled as a minimum-required
    entry-point plus a richer ``_with`` / ``_full`` / ``_with_options``
    companion. Both spellings ARE that single reference method. When a rename
    makes two Rust methods land on the same reference name, the one whose
    parameter list is CLOSER TO THE REFERENCE wins — otherwise the fold would be
    dict-order dependent and would routinely keep the truncated spelling, which
    is precisely the drift we are folding away. Ties keep the first arrival."""
    table = METHOD_RENAMES.get(cls_name, {})
    param_table = PUBLIC_FIELD_RENAMES.get(cls_name, {})
    if not table and not param_table:
        return methods
    recorded: set[str] = set()
    ref_members: dict = {}
    if module is not None and py_cls is not None:
        recorded = _sig_oracle_members().get((module, py_cls), set())
        ref_members = _sig_oracle_signatures().get((module, py_cls), {})
    out: dict = {}
    for name, sig in methods.items():
        sig = _apply_param_renames(sig, param_table)
        if name in table:
            target = table[name]
            if target is None:
                if name in recorded:
                    out[name] = sig  # stale drop — the reference has this member
                continue
        else:
            target = name
        if target in out and out[target] is not sig:
            if _closer_to_reference(sig, out[target], ref_members.get(target)):
                out[target] = sig
            continue
        out[target] = sig
    return out


def _apply_param_renames(sig: dict, param_table: dict[str, str]) -> dict:
    """Apply a class's ``PUBLIC_FIELD_RENAMES`` spelling map to a method's
    PARAMETER names.

    The same Rust-vs-reference spelling split that a struct FIELD has, its
    methods' parameters have: ``Section.numbered_bullets`` is the field, and
    ``add_subsection(.., numbered_bullets)`` is the parameter that sets it. The
    reference records the camelCase WIRE key (``numberedBullets``) in both
    places, so one table governs both — renaming only the field would leave the
    parameter reading as drift for a spelling we have already adjudicated."""
    if not param_table:
        return sig
    params = (sig or {}).get("params")
    if not params or not any(p.get("name") in param_table for p in params):
        return sig
    out = dict(sig)
    out["params"] = [
        {**p, "name": param_table.get(p["name"], p["name"])} for p in params
    ]
    return out


def _named_params(sig: dict) -> list[str]:
    """The comparable parameter NAMES of a signature — the receiver (``self``)
    is positional plumbing on both sides and never distinguishes two spellings
    of the same method."""
    return [p["name"] for p in (sig or {}).get("params", []) if p.get("kind") != "self"]


def _closer_to_reference(
    candidate: dict, incumbent: dict, ref_sig: dict | None
) -> bool:
    """Does ``candidate`` match the reference signature better than ``incumbent``?

    Two spellings of one reference method differ by ARITY — that is the whole
    point of the idiom (``put`` / ``put_with_options``, ``add_section`` /
    ``add_section_with``). So score on how close each spelling's arity is to the
    reference's, and use the reference's own parameter NAMES only to break a tie.

    Name-matching must NOT be the primary score: the extra parameter frequently
    carries a Rust-idiom spelling (``options`` for the reference's
    ``request_options``, ``data`` for ``body``), so both spellings hit the same
    small set of shared names and a name-first rule silently keeps the TRUNCATED
    one — reinstating the exact drift this fold exists to remove."""
    cand = _named_params(candidate)
    inc = _named_params(incumbent)
    if not ref_sig:
        # Nothing to aim at: the truncated spelling can only ever be a subset of
        # the fuller one, so more parameters is strictly more capability.
        return len(cand) > len(inc)
    ref = _named_params(ref_sig)
    cand_gap, inc_gap = abs(len(cand) - len(ref)), abs(len(inc) - len(ref))
    if cand_gap != inc_gap:
        return cand_gap < inc_gap
    return len(set(ref) & set(cand)) > len(set(ref) & set(inc))


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
    rust_params = rust_sig.get("params", [])
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

    # Recover the port's real parameter defaults from source (rustdoc carries no
    # function bodies; ``span`` is the join key). ``fn_defaults`` is keyed by
    # rustdoc item id, so each build_signature call gets exactly its own fn's
    # defaults — a param with none stays required, never a fabricated value.
    _reader = _SourceReader(PORT_ROOT)
    fn_defaults, options_defaults = extract_defaults(index, _reader)

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

    # PUBLIC-SURFACE-TRAIT DEFAULT BODIES: {trait: {method: signature}}.
    #
    # A Rust trait DEFAULT is inherited public API on every implementor exactly
    # as a Python base-class method is on every subclass — ``Datasphere`` does not
    # re-declare ``get_instance_key`` in its ``impl SkillBase`` block, but
    # ``skill.get_instance_key()`` compiles and runs. The reference oracle records
    # those inherited members PER SUBCLASS (Python enumerates the resolved MRO),
    # so an implementor that leans on the default must still enumerate it or it
    # reads as missing-port. Collected here once, projected (oracle-gated) onto
    # each implementor below. Idiom accessors are dropped at collection time, the
    # same set the surface enumerator drops from an ``impl SkillBase for X`` body.
    def _elide_receiver_params(trait_name: str, method_native: str, sig: dict) -> dict:
        """Drop the explicit-receiver argument(s) a public-surface trait method
        threads in where Python reads them off ``self`` (see
        SURFACE_TRAIT_RECEIVER_PARAMS). Returns ``sig`` unchanged when the
        (trait, method) pair declares none."""
        elide = SURFACE_TRAIT_RECEIVER_PARAMS.get(trait_name, {}).get(method_native)
        if not elide:
            return sig
        kept = [p for p in sig.get("params", []) if p.get("name") not in elide]
        if len(kept) == len(sig.get("params", [])):
            return sig
        return {**sig, "params": kept}

    surface_trait_defaults: dict[str, dict] = {}
    for _titem in index.values():
        _tname = _titem.get("name")
        if _tname not in PUBLIC_SURFACE_TRAITS:
            continue
        _tinner = _titem.get("inner", {}).get("trait")
        if not _tinner:
            continue
        _bucket = surface_trait_defaults.setdefault(_tname, {})
        for _mid in _tinner.get("items", []):
            _mi = get(_mi_id := _mid)
            if not _mi or "function" not in _mi.get("inner", {}):
                continue
            _mn = _mi.get("name", "")
            if not _mn or _mn.startswith("_"):
                continue
            if _mn in SKILLBASE_IDIOM_METHOD_DROPS:
                continue
            # Required (body-less) trait items are NOT inherited implementations —
            # every implementor must write its own, and that one lands via the
            # impl-block walk. Only DEFAULT-bodied items are inherited surface.
            if not _mi["inner"]["function"].get("has_body"):
                continue
            try:
                _bucket[_translate_method_name(_mn)] = _elide_receiver_params(
                    _tname,
                    _mn,
                    build_signature(
                        _mi["inner"]["function"],
                        paths,
                        aliases,
                        f"{_tname}.{_mn}",
                        defaults=fn_defaults.get(str(_mi_id)),
                    ),
                )
            except TypeTranslationError as e:
                failures.append(str(e))

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
        _ppath = (_pentry.get("path") or []) if _pentry else []
        if "generated_bases" in _ppath:
            continue
        # Generated REST TYPE structs (rest::namespaces::generated::types::
        # <ns>_types_generated::*) are the request/response payload shapes. They
        # are emitted from the sidecar like the resources, but unlike the
        # resources they are NOT listed in `generated_struct_names` (that set
        # covers resources/containers/suppressed only), so the struct walk picked
        # them up and routed them through CLASS_MODULE_MAP BY BARE NAME.
        #
        # That collides whenever a generated payload type shares a name with a
        # hand-written SDK class. `messages_types_generated::Message` and the
        # RELAY `relay::message::Message` both mapped to
        # signalwire.relay.message, and which one survived depended on rustdoc's
        # index ORDERING — so a rustdoc rebuild alone could flip
        # `Message.direction` between `optional<string>` (the real
        # `Option<&str>` accessor) and `optional<class:Value>` (the generated
        # payload field), staling the committed snapshot with no source change.
        # Skip them by path, exactly as generated_bases is skipped above.
        if any(seg.endswith("_types_generated") for seg in _ppath):
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
                if (
                    canonical_name in PUBLIC_SURFACE_TRAITS
                    and m_native in SKILLBASE_IDIOM_METHOD_DROPS
                ):
                    continue
                method_canonical = _translate_method_name(m_native)
                ctx = f"{mod}.{canonical_name}.{method_canonical}"
                try:
                    sig = build_signature(
                        m_inner["function"],
                        paths,
                        aliases,
                        ctx,
                        defaults=fn_defaults.get(str(method_id)),
                    )
                except TypeTranslationError as e:
                    failures.append(str(e))
                    continue
                if canonical_name in PUBLIC_SURFACE_TRAITS:
                    sig = _elide_receiver_params(canonical_name, m_native, sig)
                methods_out.setdefault(method_canonical, sig)

        # Which PUBLIC_SURFACE_TRAITS this class implements (see the trait-impl
        # walk below). Populated as we walk, consumed by the trait-DEFAULT
        # projection after it.
        implemented_surface_traits: set[str] = set()

        for impl_id in impls:
            impl_item = get(impl_id)
            if not impl_item:
                continue
            impl_inner = impl_item.get("inner", {}).get("impl", {})
            # Skip trait impls that pull in unrelated methods (stdlib derives etc.)
            trait = impl_inner.get("trait")
            is_surface_trait_impl = False
            if trait is not None:
                # rustdoc emits trait { path, id }; only keep impls whose
                # trait path is part of the SDK. Skip ALL stdlib traits.
                trait_path = trait.get("path", "") if isinstance(trait, dict) else ""
                # PUBLIC-SURFACE TRAIT IMPLS ARE REAL SURFACE. ``impl SkillBase
                # for Math`` is how Rust spells what Python spells as
                # ``class MathSkill(SkillBase)`` overriding setup/register_tools/
                # get_hints/... — the trait-impl methods ARE the port's
                # implementation of the reference's per-subclass override set.
                # The blanket "unqualified + capitalized => stdlib, skip" rule
                # below would swallow them (SkillBase is unqualified and
                # capitalized), which is why every skill subclass previously
                # enumerated METHOD-LESS. The SURFACE enumerator has always
                # collected these (PUBLIC_SURFACE_TRAITS + RE_TRAIT_FN); mirror
                # it here so the two enumerators discover the same symbols.
                if trait_path in PUBLIC_SURFACE_TRAITS:
                    is_surface_trait_impl = True
                    implemented_surface_traits.add(trait_path)
                # Skip everything else: stdlib/derive traits (Debug, Clone,
                # Borrow, Serialize, ...) and blanket impls all present as an
                # unqualified, capitalized trait path, and none of them are
                # reference surface. An SDK trait that IS surface belongs in
                # PUBLIC_SURFACE_TRAITS above, not in a second allow-list here.
                elif (
                    trait_path
                    and not trait_path.startswith("signalwire")
                    and trait_path[0:1].isupper()
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
                # Rust-idiom trait accessors the reference does not enumerate
                # (name/description/params/version/...) — same drop the surface
                # enumerator applies to every ``impl SkillBase for X`` body.
                if (
                    is_surface_trait_impl
                    and method_native in SKILLBASE_IDIOM_METHOD_DROPS
                ):
                    continue
                method_canonical = _translate_method_name(method_native)
                # ORACLE-GATE the trait-impl contribution: a public-surface trait
                # impl supplies the reference's per-subclass OVERRIDE set, and the
                # SIGNATURE oracle is the authority on which members it records
                # there (e.g. it records ``build_config`` on AIVerbHandler but not
                # ``validate_config``/``get_verb_name``, which live on the
                # SWMLVerbHandler base). Emitting an un-recorded trait method onto
                # the concrete class would manufacture a missing-reference drift
                # out of a method that IS the base's. Same gate shape as the
                # field-synthesis and idiom-drop passes: cannot over-emit, cannot
                # go stale.
                if (
                    is_surface_trait_impl
                    and mod
                    and method_canonical
                    not in _sig_oracle_members().get((mod, canonical_name), set())
                ):
                    continue
                ctx = f"{mod}.{canonical_name}.{method_canonical}"
                try:
                    sig = build_signature(
                        m_inner["function"],
                        paths,
                        aliases,
                        ctx,
                        defaults=fn_defaults.get(str(method_id)),
                    )
                except TypeTranslationError as e:
                    failures.append(str(e))
                    continue
                if is_surface_trait_impl:
                    sig = _elide_receiver_params(trait_path, method_native, sig)
                if method_canonical in methods_out:
                    continue
                methods_out[method_canonical] = sig

        # INHERITED TRAIT-DEFAULT PROJECTION, oracle-gated. ``impl SkillBase for
        # Datasphere`` overrides only some of the interface; the rest resolve to
        # the trait's default bodies and are still callable on the concrete skill
        # — the Rust spelling of Python's inherited-from-the-base methods, which
        # the reference oracle records on EACH subclass. Project a default onto
        # this class only when the reference records that same NAME on that same
        # CLASS, so the projection cannot invent surface and cannot go stale as
        # the oracle grows (this replaced nothing: the signature enumerator used
        # to exclude skill trait-impls entirely on the premise that the oracle
        # recorded skill subclasses method-less, which stopped being true when
        # the oracle went from 7 to 18 skill modules).
        if implemented_surface_traits and mod:
            _recorded_here = _sig_oracle_members().get((mod, canonical_name), set())
            for _tname in sorted(implemented_surface_traits):
                for _mname, _msig in surface_trait_defaults.get(_tname, {}).items():
                    if _mname in methods_out or _mname not in _recorded_here:
                        continue
                    methods_out[_mname] = _msig

        # PUBLIC-FIELD ACCESSOR SYNTHESIS, oracle-gated — the SIGNATURE twin of the
        # surface enumerator's public-field emission. A bare ``pub`` struct field is
        # a public read of that member, but a field is not a ``pub fn``, so the
        # impl-block walk above never records it. Synthesize the reference's
        # field-read shape (self-only, returns the field's translated type) for
        # every public field the reference oracle records on THIS class.
        #
        # Oracle-gated for the same two reasons as the surface side: it cannot
        # over-emit (nothing the reference lacks can appear), and it cannot go stale
        # (a newly-recorded oracle member starts emitting with no table to edit).
        if "struct" in inner:
            # ``kind`` is a dict for a plain (named-field) struct and a bare
            # string for a unit/tuple struct — only the former has named fields.
            _sk = kind_inner.get("kind")
            _plain = _sk.get("plain") if isinstance(_sk, dict) else None
            _field_ids = (_plain or {}).get("fields") or []
            _recorded = _sig_oracle_members().get((mod, canonical_name), set())
            for _fid in _field_ids:
                _f = get(_fid)
                if not _f or _f.get("visibility") != "public":
                    continue
                _fname = _f.get("name")
                if not _fname or _fname.startswith("_"):
                    continue
                # Fold field-spelling idiom before the gate (shared table, keyed
                # by the RUST struct name — same key space, both enumerators).
                _fname = PUBLIC_FIELD_RENAMES.get(struct_name, {}).get(_fname, _fname)
                if _fname not in _recorded:
                    continue
                if _fname in methods_out:
                    continue  # a real accessor method already carries this name
                _fnode = (_f.get("inner") or {}).get("struct_field")
                try:
                    _ftype = translate_rust_type(
                        _fnode,
                        paths,
                        aliases,
                        f"{mod}.{canonical_name}.{_fname}",
                    )
                except TypeTranslationError as e:
                    failures.append(str(e))
                    continue
                methods_out[_fname] = {"params": [_S], "returns": _ftype}

        # Apply the surface enumerator's per-class method renames (``to_value`` ->
        # ``to_dict``, drop borrow-checker/idiom companions) so both enumerators
        # name the SAME methods (Rule 2). Without this, ``to_value`` surfaces as
        # missing-reference AND ``to_dict`` as missing-port on every POM/Context.
        #
        # ⚠ KEY SPACE. ``METHOD_RENAMES`` is keyed by the RUST struct name, and the
        # surface enumerator looks it up PRE-translate. This call used
        # ``canonical_name`` (POST-translate), so for every class CLASS_RENAME_MAP
        # aliases — SwaigFunction→SWAIGFunction, Client→RelayClient,
        # Service→SWMLService, SwmlBuilder→SWMLBuilder, AiVerbHandler→AIVerbHandler,
        # McpGateway→MCPGatewaySkill, WikipediaSearch→WikipediaSearchSkill — the
        # table silently never applied HERE while it did on the surface side, so a
        # name folded in one gate and not the other. Look up by ``struct_name``
        # (pre-alias, matching the table's key space) and pass the canonical
        # (module, class) separately for the ORACLE GATE, which must be keyed by
        # the name the emitter EMITS. Never mix the two spaces.
        methods_out = _apply_method_renames(
            struct_name,
            methods_out,
            module=mod,
            py_cls=canonical_name,
        )

        if not methods_out:
            continue

        out_modules.setdefault(mod, {"classes": {}})
        existing = (
            out_modules[mod]["classes"].get(canonical_name, {}).get("methods", {})
        )
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
                inner["function"],
                paths,
                aliases,
                f"{target_module}.{target_function}",
                defaults=fn_defaults.get(str(iid)),
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
            if (
                p.get("name") in ref_kind_by_name
                and p.get("kind", "positional") == "positional"
            ):
                p["kind"] = "keyword"
        out_modules.setdefault(target_module, {"classes": {}})
        out_modules[target_module].setdefault("functions", {})
        out_modules[target_module]["functions"][target_function] = sig

    # Mixin/manager projections — the Rust ``Service`` (renamed
    # SWMLService) inherits to AgentBase. Project Service-side methods
    # to canonical Python mixin / manager paths so the audit lines up.
    MIXIN_PROJECTIONS: dict[tuple[str, str], list[str]] = {
        ("signalwire.core.agent.tools.registry", "ToolRegistry"): [
            "define_tool",
            "register_swaig_function",
            "has_function",
            "get_function",
            "get_all_functions",
            "remove_function",
        ],
        ("signalwire.core.mixins.tool_mixin", "ToolMixin"): [
            "define_tool",
            "on_function_call",
            "register_swaig_function",
            "define_tools",
        ],
        ("signalwire.core.mixins.auth_mixin", "AuthMixin"): [
            "validate_basic_auth",
            "get_basic_auth_credentials",
        ],
        ("signalwire.core.mixins.state_mixin", "StateMixin"): [
            "validate_tool_token",
        ],
        ("signalwire.core.mixins.web_mixin", "WebMixin"): [
            "on_request",
            "on_swml_request",
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
            "define_contexts",
            "get_contexts",
            "get_post_prompt",
            "get_prompt",
            "get_raw_prompt",
            "prompt_add_section",
            "prompt_add_subsection",
            "prompt_add_to_section",
            "prompt_has_section",
            "set_post_prompt",
            "set_prompt_pom",
            "set_prompt_text",
        ],
    }
    svc_entry = (
        out_modules.get("signalwire.core.swml_service", {})
        .get("classes", {})
        .get("SWMLService")
    )
    ab_entry = (
        out_modules.get("signalwire.core.agent_base", {})
        .get("classes", {})
        .get("AgentBase")
    )
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
            # No try/except here on purpose. The old `except Exception:
            # py_svc_methods = set()` was the most damaging swallow in this file:
            # an empty set makes the loop below drop EVERY projected method off
            # SWMLService (the drop condition is `n not in py_svc_methods`), so a
            # transient oracle read error silently emptied a whole class and the
            # run still exited 0. _load_python_reference() now aborts naming the
            # file, which is the correct behaviour for a missing oracle.
            py_ref = _load_python_reference()
            py_svc_methods = set(
                py_ref.get("modules", {})
                .get("signalwire.core.swml_service", {})
                .get("classes", {})
                .get("SWMLService", {})
                .get("methods", {})
                .keys()
            )
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
    for _entry in out_modules.values():
        for _cls, _c in _entry.get("classes", {}).items():
            donor_sig_index.setdefault(_cls, {}).update(_c.get("methods", {}))
    DEREF_INHERITS = {"AgentBase": "SWMLService"}
    for _child, _parent in DEREF_INHERITS.items():
        parent_sigs = dict(donor_sig_index.get(_parent, {}).items())
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
        exceptions = MODULE_METHOD_DROP_EXCEPTIONS.get(mod_name, {})
        for _cls, _c in entry.get("classes", {}).items():
            cls_drop = drop - exceptions.get(_cls, set())
            # ORACLE-GATED, in lockstep with the surface enumerator's identical
            # gate: a module-scoped idiom drop applies only to a name the
            # reference does NOT record on that class, so it self-retires as the
            # oracle grows instead of needing a hand edit.
            recorded = _sig_oracle_members().get((mod_name, _cls), set())
            for n in cls_drop - recorded:
                _c["methods"].pop(n, None)

    # Project Rust ``params: serde_json::Value`` trailing arguments onto
    # Python's ``**kwargs`` shape wherever the Python reference uses
    # var_keyword at the same position. See _project_variadic_kwargs for
    # the rationale: Rust uses a single Value as the **kwargs equivalent
    # for every Call/CallingNamespace command, and the audit needs to
    # treat the two as functionally equivalent.
    py_ref = _load_python_reference()

    # Explode a collapsed ``&Map<String, Value>`` kwargs slot into the reference's
    # own recorded named parameters (see KWARGS_MAP_EXPLODE). Runs BEFORE the
    # variadic-tail projection so the exploded shape is what that pass sees.
    for _path, _map_param in KWARGS_MAP_EXPLODE.items():
        _mod, _cls, _meth = _path.rsplit(".", 2)
        _ref_sig = _sig_oracle_signatures().get((_mod, _cls), {}).get(_meth)
        _port_sig = (
            out_modules.get(_mod, {})
            .get("classes", {})
            .get(_cls, {})
            .get("methods", {})
        ).get(_meth)
        if not _ref_sig or not _port_sig:
            continue  # class/method genuinely absent → stays a real drift
        _params = _port_sig.get("params", [])
        _names = [p.get("name") for p in _params]
        if _map_param not in _names:
            continue  # the collapse is gone (port now spells them out) → nothing to do
        _idx = _names.index(_map_param)
        _exploded = [
            dict(p) for p in _ref_sig.get("params", []) if p.get("kind") != "self"
        ]
        _port_sig["params"] = _params[:_idx] + _exploded + _params[_idx + 1 :]

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

    # Merge the ai_chat.client reference-shaped signatures OVER the rustdoc-derived
    # builder/options-object/struct-literal shapes (see build_ai_chat_signatures):
    # AIChatClient methods + the dataclass/error __init__s. This folds the ai_chat
    # signature idiom in the enumerator (Rule 2) so no ai_chat entry is needed in
    # PORT_SIGNATURE_OMISSIONS.md. Overwrite (not setdefault) the AIChatClient
    # methods so the collapsed options-object shapes are replaced by the exploded
    # reference kwargs; the dataclass/error classes are otherwise method-less.
    for mod, entry in build_ai_chat_signatures().items():
        out_modules.setdefault(mod, {"classes": {}})
        out_modules[mod].setdefault("classes", {})
        for cls, cls_entry in entry["classes"].items():
            existing = out_modules[mod]["classes"].setdefault(cls, {"methods": {}})
            existing.setdefault("methods", {})
            existing["methods"].update(cls_entry["methods"])

    # Merge the read-side payload modules (§D3): swml_verbs / post_prompt /
    # swaig_request classes with synthesized per-field ``any``-return accessors
    # from the gen-payload sidecars (Rust structs carry no accessor methods).
    for mod, entry in build_gen_payload_signatures(load_gen_payload_sidecars()).items():
        out_modules.setdefault(mod, {"classes": {}})
        out_modules[mod].setdefault("classes", {})
        out_modules[mod]["classes"].update(entry["classes"])

    # KEYWORD-ONLY KIND MIRROR (methods). The reference marks some params
    # keyword-only (``def paginate(self, *, request_options=None, **params)``);
    # RUST HAS NO KEYWORD-ONLY ARGUMENTS, so rustdoc necessarily reports every
    # param positional. That is pure idiom, reconciled in the enumerator rather
    # than excused (Rule 2) — exactly the mirror already applied to the free
    # functions above (see ``ref_kind_by_name``), lifted to methods so the two
    # paths share one rule instead of the class side silently going unmirrored.
    #
    # It is a MIRROR, not an assertion: the kind is copied only onto a param the
    # reference records under the SAME NAME and only when Rust reports it
    # positional. A param the reference does not declare keyword-only keeps its
    # positional kind, so this cannot manufacture agreement where none exists.
    for mod_name, mod_entry in out_modules.items():
        ref_classes = _PY_REF.get("modules", {}).get(mod_name, {}).get("classes", {})
        for cls_name, cls_entry in (mod_entry.get("classes") or {}).items():
            ref_methods = ref_classes.get(cls_name, {}).get("methods", {})
            for meth_name, sig in (cls_entry.get("methods") or {}).items():
                ref_kw = {
                    p.get("name")
                    for p in ref_methods.get(meth_name, {}).get("params", [])
                    if p.get("kind") == "keyword"
                }
                if not ref_kw:
                    continue
                for p in sig.get("params", []):
                    if (
                        p.get("name") in ref_kw
                        and p.get("kind", "positional") == "positional"
                    ):
                        p["kind"] = "keyword"

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
        "construction": build_construction(
            sorted_modules,
            index,
            paths,
            aliases,
            failures,
            options_defaults=options_defaults,
        ),
    }, failures


# ---------------------------------------------------------------------------
# Construction contract (porting-sdk ALLOWLIST_DISCIPLINE.md §10)
# ---------------------------------------------------------------------------

# Rust expresses a wide many-optional-kwarg constructor as an OPTIONS STRUCT:
# the reference's ``AgentBase.__init__(name, route, host, port, …)`` becomes
# ``AgentBase::new(AgentOptions::new(name).route(…).host(…))``. The options
# struct's public FIELDS are the construction parameter set — same capability,
# different spelling — so they satisfy the construction contract rather than
# being N port-only additions plus one blanket ``__init__`` signature omission.
# The fluent ``with_*``/setter methods over those fields are the builder FACE of
# the same struct and fold onto the field they set (they add no capability the
# field does not already carry) — exactly the RequestOptions field+setter
# precedent, which emits ONE member per field.
#
# Native Rust options-struct name -> the canonical Python class it constructs.
_OPTIONS_CONSTRUCTS: dict[str, str] = {
    "AgentOptions": "signalwire.core.agent_base.AgentBase",
    "ServiceOptions": "signalwire.core.swml_service.SWMLService",
    "WebServiceOptions": "signalwire.web.web_service.WebService",
    "BedrockOptions": "signalwire.agents.bedrock.BedrockAgent",
    "RequestOptions": "signalwire.rest._request_options.RequestOptions",
    # Prefabs. The reference constructs each with a wide all-defaulted kwarg
    # list (`FAQBotAgent(faqs=[], suggest_related=True, persona=None,
    # name="faq_bot", route="/faq")`); Rust carries the same set on an options
    # struct that derives its defaults from `Default`, so the zero-argument
    # reference program ports as `FAQBotAgent::default()`.
    "ConciergeOptions": "signalwire.prefabs.concierge.ConciergeAgent",
    "FAQBotOptions": "signalwire.prefabs.faq_bot.FAQBotAgent",
    "InfoGathererOptions": "signalwire.prefabs.info_gatherer.InfoGathererAgent",
    "ReceptionistOptions": "signalwire.prefabs.receptionist.ReceptionistAgent",
    "SurveyOptions": "signalwire.prefabs.survey.SurveyAgent",
}

# Field-name canonicalization (ADAPTER_CONTRACT rule 3: names are translated to
# Python-canonical form at adapter time). Rust splits the reference's
# ``basic_auth: tuple[str, str]`` into two scalar fields because Rust struct
# literals have no anonymous-tuple-field idiom; the pair IS the reference's
# single configurable, so the user-half carries the canonical name and the
# password-half folds onto it. Keyed by (canonical class, native field).
_CONSTRUCTION_FIELD_RENAMES: dict[tuple[str, str], str | None] = {
    ("signalwire.core.agent_base.AgentBase", "basic_auth_user"): "basic_auth",
    ("signalwire.core.agent_base.AgentBase", "basic_auth_password"): None,
    ("signalwire.core.swml_service.SWMLService", "basic_auth_user"): "basic_auth",
    ("signalwire.core.swml_service.SWMLService", "basic_auth_password"): None,
    ("signalwire.agents.bedrock.BedrockAgent", "basic_auth_user"): "basic_auth",
    ("signalwire.agents.bedrock.BedrockAgent", "basic_auth_password"): None,
}

# The reference type for a canonicalized field whose Rust spelling is a
# projection of a differently-shaped reference param (the basic_auth pair
# above). Keyed the same way as the rename table, by the CANONICAL name.
_CONSTRUCTION_FIELD_TYPES: dict[tuple[str, str], str] = {
    (
        "signalwire.core.agent_base.AgentBase",
        "basic_auth",
    ): "optional<tuple<string,string>>",
    (
        "signalwire.core.swml_service.SWMLService",
        "basic_auth",
    ): "optional<tuple<string,string>>",
    (
        "signalwire.agents.bedrock.BedrockAgent",
        "basic_auth",
    ): "optional<tuple<string,string>>",
}


def build_construction(
    modules: dict,
    index: dict,
    paths: dict,
    aliases: dict,
    failures: list,
    options_defaults: dict | None = None,
) -> dict:
    """Return ``{"module.Class": {"params": {name: {type, required[, default]}}}}``.

    A NAME-KEYED set (order/arity/mechanism are idiom; the named set is the
    capability) — see porting-sdk ALLOWLIST_DISCIPLINE.md §10. Two sources, in
    precedence order:

      1. the class's own ``__init__`` (Rust ``new``) params, when construction
         is a plain constructor;
      2. its OPTIONS STRUCT's public fields, when construction goes through an
         options struct (``AgentBase::new(AgentOptions…)``).

    ``required`` mirrors the source. A Rust options-struct field is optional by
    construction when it is ``Option<T>`` or the struct has a ``Default``/``new``
    that fills it; only the non-``Option`` fields of a struct whose ``new`` takes
    them are required. Where the reference marks a param required and the struct
    does not (or vice versa), that is a real ``construction-required-flip`` for
    review, not something to paper over here.

    ``default`` carries the options struct's own field initializer (mechanism C
    of ``extract_defaults``) — the value a caller who never touches that field
    actually gets. A field whose initializer is not a static literal carries NO
    ``default``, exactly as if it had none.
    """
    options_defaults = options_defaults or {}
    out: dict = {}

    def _params_from_init(sig: dict) -> dict:
        params: dict = {}
        for p in sig.get("params", []):
            if not isinstance(p, dict):
                continue
            if (p.get("kind") or "positional") in (
                "self",
                "cls",
                "var_keyword",
                "var_positional",
            ):
                continue
            name = p.get("name")
            if not name or name.startswith("_"):
                continue
            ptype = p.get("type", "any")
            # A constructor that takes the options struct is the MECHANISM, not
            # a configurable — its fields are unfolded below.
            if isinstance(ptype, str) and ptype.startswith("class:"):
                short = ptype.rsplit(".", 1)[-1]
                if short in _OPTIONS_CONSTRUCTS:
                    continue
            entry = {
                "type": ptype,
                "required": bool(p.get("required", True)),
            }
            if "default" in p:
                entry["default"] = p["default"]
                entry["required"] = False
            params[name] = entry
        return params

    for mod, entry in modules.items():
        for cls, cinfo in entry.get("classes", {}).items():
            init = cinfo.get("methods", {}).get("__init__")
            if isinstance(init, dict):
                params = _params_from_init(init)
                if params:
                    out[f"{mod}.{cls}"] = {"params": dict(sorted(params.items()))}

    # Options-struct fields: each public field names one construction param.
    def _get(id_):
        return index.get(str(id_)) or index.get(id_)

    for item in index.values():
        struct_name = item.get("name")
        target = _OPTIONS_CONSTRUCTS.get(struct_name or "")
        if not target:
            continue
        inner = item.get("inner", {})
        if "struct" not in inner:
            continue
        kind_inner = inner["struct"].get("kind") or {}
        field_ids = (kind_inner.get("plain") or {}).get("fields") or []
        # Which fields the struct's OWN constructor demands. `AgentOptions::new
        # (name)` takes `name` and defaults the other nine; `WebServiceOptions`
        # / `BedrockOptions` derive `Default` and demand nothing. Only a field
        # the struct cannot be built without is REQUIRED — a defaulted scalar
        # (`auto_answer: bool`) is optional even though its type is not
        # `Option<T>`, exactly as the reference records it defaulted.
        ctor_required: set[str] = set()
        has_ctor = False
        for impl_id in inner["struct"].get("impls", []):
            impl_item = _get(impl_id)
            if not impl_item:
                continue
            impl_inner = impl_item.get("inner", {}).get("impl", {})
            if impl_inner.get("trait") is not None:
                continue  # `Default` and friends demand nothing
            for m_id in impl_inner.get("items", []):
                m_item = _get(m_id)
                if not m_item or m_item.get("name") != "new":
                    continue
                m_fn = (m_item.get("inner") or {}).get("function")
                if not m_fn:
                    continue
                has_ctor = True
                for pname, _ptype in (m_fn.get("sig") or {}).get("inputs", []):
                    if pname not in ("self", "cls"):
                        ctor_required.add(pname)
        params = out.setdefault(target, {"params": {}})["params"]
        for fid in field_ids:
            f_item = _get(fid)
            if not f_item or f_item.get("visibility") != "public":
                continue
            fname = f_item.get("name")
            if not fname or fname.startswith("_"):
                continue
            key = (target, fname)
            if key in _CONSTRUCTION_FIELD_RENAMES:
                canonical = _CONSTRUCTION_FIELD_RENAMES[key]
                if canonical is None:
                    continue  # folded onto its sibling half
                fname = canonical
            f_type_node = (f_item.get("inner") or {}).get("struct_field")
            ctx = f"{target}.{fname}"
            override = _CONSTRUCTION_FIELD_TYPES.get((target, fname))
            if override is not None:
                ftype = override
            else:
                try:
                    ftype = translate_rust_type(f_type_node, paths, aliases, ctx)
                except TypeTranslationError as e:
                    failures.append(str(e))
                    continue
            # An options-struct field is optional by construction unless the
            # struct's own `new` demands it. `Option<T>` fields and defaulted
            # scalars are settable-or-skippable; a struct with no inherent `new`
            # (Default-only) demands nothing at all.
            native = f_item.get("name")
            required = (
                has_ctor
                and native in ctor_required
                and not ftype.startswith("optional<")
            )
            entry = {"type": ftype, "required": required}
            # The field's own initializer in `<Struct>::new` / `::default` is the
            # value a caller who never sets it gets — the port's real default for
            # this construction param. Keyed by the NATIVE field name (what the
            # source-side extraction saw), attached under the canonical one.
            # A field whose initializer is not a static literal is absent from
            # the map and correctly carries no `default`.
            _fdefs = options_defaults.get(struct_name or "", {})
            if native in _fdefs:
                entry["default"] = _fdefs[native]
                if not required:
                    entry["required"] = False
            # The class's own `__init__` (if any) already declared this name with
            # its real required flag — a real ctor param's flag wins.
            params.setdefault(fname, entry)
        out[target]["params"] = dict(sorted(params.items()))

    return dict(sorted(out.items()))


# ---------------------------------------------------------------------------
# PARAMETER DEFAULT VALUES
#
# Rust has NO language-level default parameter values. A plain positional param
# (``fn pay(&mut self, timeout: i64, ...)``) is genuinely required: the caller
# MUST pass a value, and there is no default to record. Emitting one would be a
# fabrication, so those params keep ``required: true`` and NO ``default`` key.
#
# The port DOES express a default through three source constructs, and for those
# the value a caller-who-supplies-nothing actually gets is recoverable:
#
#   A. ZERO-ARG SIBLING CONSTRUCTOR. A public zero-arg fn whose whole body is a
#      single ``Self::<other>(<literals>)`` delegation. The literals it passes
#      ARE the defaults of the delegated-to fn's params, positionally.
#      (``SessionManager::with_defaults() -> Self::new(900)`` — 900 is what a
#      caller gets for ``token_expiry_secs`` without supplying one.)
#
#   B. ``Option<T>`` PARAM + ``<param>.unwrap_or(<literal>)``. Passing ``None``
#      is the "don't supply it" call, and the ``unwrap_or`` literal is what the
#      caller then gets. (``AgentServer::with_log_level(host: Option<&str>)`` →
#      ``host.unwrap_or("0.0.0.0")``.)
#
#   C. OPTIONS-STRUCT FIELD INITIALIZERS. Rust spells a wide many-optional-kwarg
#      constructor as an options struct (see ``_OPTIONS_CONSTRUCTS``); the
#      struct's own ``new``/``default`` is a struct literal whose per-field
#      initializers are exactly the reference's per-kwarg defaults
#      (``AgentOptions::new`` → ``auto_answer: true, token_expiry_secs: 3600,
#      record_format: "mp4"``).
#
# All three are recovered from the rustdoc ``span`` (filename + begin/end lines),
# which is the join key from an index item back to its source text — rustdoc-json
# carries no function BODIES, so the value has to be read out of the source.
#
# ONLY STATIC LITERALS ARE RECORDED. A default that is a non-literal EXPRESSION
# (a ``const`` reference, a function call, arithmetic, a ``format!``) is not a
# static value: it is NOT evaluated and NOT guessed — the param simply carries no
# ``default``, exactly as if it had none. The one deliberate exception is an
# ``unwrap_or_else(|| … .unwrap_or(<literal>))`` env-override chain, whose
# TERMINAL literal is the value a caller gets with the env unset — that is the
# static default the reference records as a plain literal (``AgentServer.port``:
# reference ``port: int = 3000``, Rust reads ``$PORT`` then falls back to 3000).
# ---------------------------------------------------------------------------

# A Rust literal we are willing to record as a static default value, with the
# ergonomic conversion suffixes (``"x".to_string()`` is still the literal "x")
# and the numeric type suffixes (``3600u64``) stripped.
_RUST_LITERAL_RE = re.compile(
    r"""^\s*(?:
          "(?P<s>(?:[^"\\]|\\.)*)"
            (?:\s*\.\s*(?:to_string|to_owned|into|to_vec)\s*\(\s*\))?
        | (?P<b>true|false)
        | (?P<f>-?\d+\.\d+)(?:_?f(?:32|64))?
        | (?P<i>-?\d+)(?:_?(?:[iu](?:8|16|32|64|128)|usize|isize))?
        )\s*$""",
    re.X,
)

_STR_ESCAPES = (
    ("\\\\", "\\"),
    ('\\"', '"'),
    ("\\n", "\n"),
    ("\\t", "\t"),
    ("\\r", "\r"),
    ("\\0", "\0"),
)


def parse_rust_literal(text: str):
    """``(found, value)`` for a Rust literal expression.

    Returns ``(False, None)`` for anything that is not a bare literal — a const,
    a call, arithmetic. Callers MUST NOT substitute a guess for a ``False``.
    ``None`` (the Rust ``None``) is a real default value and returns
    ``(True, None)``; that is why the flag is separate from the value.
    """
    t = (text or "").strip()
    if t == "None":
        return (True, None)
    m = _RUST_LITERAL_RE.match(t)
    if not m:
        return (False, None)
    if m.group("s") is not None:
        s = m.group("s")
        # Longest-first so ``\\n`` (an escaped backslash then n) is not eaten by
        # the ``\n`` rule.
        for esc, real in _STR_ESCAPES:
            s = s.replace(esc, real)
        return (True, s)
    if m.group("b") is not None:
        return (True, m.group("b") == "true")
    if m.group("f") is not None:
        return (True, float(m.group("f")))
    if m.group("i") is not None:
        return (True, int(m.group("i")))
    return (False, None)


def _split_top_level(text: str) -> list[str]:
    """Split on commas that are not nested inside brackets or a string literal."""
    out: list[str] = []
    depth = 0
    cur = ""
    in_str = False
    esc = False
    for ch in text:
        if in_str:
            cur += ch
            if esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == '"':
                in_str = False
            continue
        if ch == '"':
            in_str = True
            cur += ch
            continue
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur)
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur)
    return out


def _strip_line_comments(text: str) -> str:
    """Drop ``//`` comments, respecting string literals (a ``//`` inside a URL
    literal is not a comment — the tokenizer-desync trap, AGENT_RULES L20)."""
    out = []
    in_str = False
    esc = False
    i = 0
    while i < len(text):
        ch = text[i]
        if in_str:
            out.append(ch)
            if esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == '"':
                in_str = False
            i += 1
            continue
        if ch == '"':
            in_str = True
            out.append(ch)
            i += 1
            continue
        if ch == "/" and text[i : i + 2] == "//":
            while i < len(text) and text[i] != "\n":
                i += 1
            continue
        out.append(ch)
        i += 1
    return "".join(out)


class _SourceReader:
    """Read a rustdoc item's source text via its ``span`` (filename + lines).

    rustdoc-json records ``has_body`` but never the body itself, so every default
    below is read out of the real source file the span points at.

    Spans can legitimately point OUTSIDE this repo — rustdoc records the defining
    file for re-exported dependency items, which lives under the cargo registry
    and may not be present. Those degrade to "no defaults found", never to a
    fabricated value. A span pointing INTO this repo's own ``src/`` is different:
    an unreadable one there means we silently record "no default" for parameters
    that HAVE defaults, and the run still exits 0. That is a fault, so it aborts.
    """

    def __init__(self, root: Path):
        self.root = root
        self._cache: dict[str, list[str]] = {}

    def _lines(self, filename: str) -> list[str]:
        if filename not in self._cache:
            p = self.root / filename
            try:
                self._cache[filename] = p.read_text(encoding="utf-8").splitlines()
            except (OSError, UnicodeDecodeError) as exc:
                if self._is_in_repo(p):
                    raise SystemExit(
                        f"enumerate_signatures: cannot read this repo's source file "
                        f"{p}: {exc}\nIts parameter defaults would be silently recorded "
                        "as absent while the run still exited 0."
                    ) from exc
                self._cache[filename] = []
        return self._cache[filename]

    @staticmethod
    def _is_in_repo(p: Path) -> bool:
        """True when ``p`` is one of this repo's own sources (vs a cargo-registry
        path rustdoc recorded for a re-exported dependency item)."""
        try:
            p.resolve().relative_to((PORT_ROOT / "src").resolve())
        except (ValueError, OSError):
            return False
        return True

    def text(self, item: dict) -> str | None:
        span = item.get("span") or {}
        filename = span.get("filename")
        begin = span.get("begin")
        end = span.get("end")
        if not filename or not begin or not end:
            return None
        lines = self._lines(filename)
        if not lines:
            return None
        return "\n".join(lines[begin[0] - 1 : end[0]])


def _body_of(text: str) -> str:
    """The ``{ … }`` body of a fn's source text (brace-balanced, string-aware)."""
    start = None
    depth = 0
    in_str = False
    esc = False
    for i, ch in enumerate(text):
        if in_str:
            if esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == '"':
                in_str = False
            continue
        if ch == '"':
            in_str = True
            continue
        if ch == "{":
            if depth == 0:
                start = i
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0 and start is not None:
                return text[start + 1 : i]
    return ""


def _struct_literal_fields(body: str, struct_name: str) -> dict:
    """``{field: value}`` for the literal-valued fields of a ``Name { … }`` /
    ``Self { … }`` struct literal inside ``body``. Non-literal initializers are
    omitted (never guessed)."""

    m = re.search(r"\b(?:" + re.escape(struct_name) + r"|Self)\s*\{", body)
    if not m:
        return {}
    i = m.end()
    depth = 1
    in_str = False
    esc = False
    start = i
    while i < len(body) and depth:
        ch = body[i]
        if in_str:
            if esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == '"':
                in_str = False
        else:
            if ch == '"':
                in_str = True
            elif ch in "{([":
                depth += 1
            elif ch in "})]":
                depth -= 1
                if depth == 0:
                    break
        i += 1
    inner = body[start:i]
    out: dict = {}
    for part in _split_top_level(inner):
        part = _strip_line_comments(part).strip()
        if ":" not in part:
            continue
        key, _, val = part.partition(":")
        key = key.strip()
        if not re.fullmatch(r"\w+", key):
            continue
        found, value = parse_rust_literal(val)
        if found:
            out[key] = value
    return out


def _balanced_arg(text: str, open_paren_end: int) -> str:
    """The text between an already-consumed ``(`` and its matching ``)``."""
    i = open_paren_end
    depth = 1
    in_str = False
    esc = False
    start = i
    while i < len(text) and depth:
        ch = text[i]
        if in_str:
            if esc:
                esc = False
            elif ch == "\\":
                esc = True
            elif ch == '"':
                in_str = False
        else:
            if ch == '"':
                in_str = True
            elif ch in "([{":
                depth += 1
            elif ch in ")]}":
                depth -= 1
                if depth == 0:
                    break
        i += 1
    return text[start:i]


def _none_branch_default(clean_body: str, expr: str):
    """``(found, value)`` for the value ``expr`` takes when it is ``None``.

    ``expr`` is a param name (``host``) or a field access (``options.host``).
    Recognises the ways this port spells "use the default when unset":

        let <x> = expr.unwrap_or(<literal>);
        let <x> = expr.unwrap_or_else(|| <literal>);
        let <x> = expr.map_or_else(|| <literal>, |v| …);   # 1st closure = None arm
        let <x> = expr.map_or(<literal>, |v| …);           # 1st arg     = None arm

    THE ``let`` BINDING IS REQUIRED, and it is not cosmetic. An ``unwrap_or``
    used INLINE inside a condition is a validation guard, not a default:
    ``if body.unwrap_or("").is_empty() { return Err(…) }``
    (relay/client.rs, ``send_message_blocking``) tests emptiness and then ERRORS
    — a caller who omits ``body`` gets a failure, not ``""``. Recording ``""``
    there would be a confident wrong value. Requiring the binding keeps only the
    form where the unwrapped value is what the rest of the body actually uses.

    Returns ``(False, None)`` when the None-arm is not a static literal — the
    caller must then record NO default rather than guess one.
    """
    pat = re.escape(expr).replace(r"\.", r"\s*\.\s*")
    for m in re.finditer(
        r"\blet\s+(?:mut\s+)?\w+\s*(?::[^=;]+)?=\s*"
        + pat
        + r"\s*\.\s*(?P<op>unwrap_or_else|unwrap_or|map_or_else|map_or)\s*\(",
        clean_body,
    ):
        arg = _balanced_arg(clean_body, m.end())
        op = m.group("op")
        if op in ("map_or_else", "map_or"):
            parts = _split_top_level(arg)
            if not parts:
                continue
            arg = parts[0]
        # Strip a closure header so ``|| "x".to_string()`` reads as the literal.
        arg = re.sub(r"^\s*\|\s*\|", "", arg, count=1).strip()
        # ONLY a bare literal None-arm counts. An ENV-CONSULTING chain
        # (``|| env::var("PORT").ok()…unwrap_or(3000)``) is deliberately NOT
        # reduced to its terminal literal: the value depends on the process
        # environment, so it is not a static default. The reference agrees — for
        # exactly this construct (``port if port is not None else
        # int(os.environ.get("PORT", 3000))``, swml_service.py:133) it records the
        # DECLARED ``None``, not the resolved 3000. Reducing the chain made
        # SWMLService.port read 3000 against the oracle's None: a confident wrong
        # value, which is worse than a missing one.
        found, value = parse_rust_literal(arg)
        if found:
            # First occurrence wins: it is the binding the rest of the body reads.
            return (True, value)
    return (False, None)


def extract_defaults(index: dict, reader: _SourceReader) -> tuple[dict, dict]:
    """Recover the port's real parameter defaults from source.

    Returns ``(fn_defaults, options_defaults)``:
      * ``fn_defaults``: ``{rustdoc_item_id: {param_name: value}}`` — mechanisms
        A and B, keyed by the item whose PARAMS carry the default.
      * ``options_defaults``: ``{options_struct_name: {field: value}}`` —
        mechanism C.
    A param absent from these maps has no recoverable default and stays required.
    """

    fn_defaults: dict = {}
    options_defaults: dict = {}
    # Mechanism D's findings, merged over C after the walk (see mechanism D).
    consumed_defaults: dict = {}

    # Index public fns by (filename, name) so a delegation target can be found.
    fns_by_file_name: dict[tuple[str, str], list] = {}
    for iid, item in index.items():
        if "function" not in (item.get("inner") or {}):
            continue
        span = item.get("span") or {}
        fname = span.get("filename")
        if not fname or not item.get("name"):
            continue
        fns_by_file_name.setdefault((fname, item["name"]), []).append((str(iid), item))

    for iid, item in index.items():
        fn = (item.get("inner") or {}).get("function")
        if not fn:
            continue
        sig = fn.get("sig") or {}
        inputs = sig.get("inputs") or []
        text = reader.text(item)
        if not text:
            continue
        body = _body_of(text)
        if not body:
            continue

        # ---- Mechanism A: zero-arg sibling constructor delegating literals ----
        # Only a PUBLIC, genuinely zero-arg fn whose entire body is one
        # ``Self::<target>(<args>)`` expression, and only when EVERY arg is a
        # literal (a partly-literal delegation tells us nothing positionally
        # reliable about which param got which value).
        if not inputs and item.get("visibility") == "public":
            expr = _strip_line_comments(body).strip().rstrip(";").strip()
            m = re.fullmatch(r"Self::(\w+)\s*\((?P<args>.*)\)", expr, re.S)
            if m:
                args = [a.strip() for a in _split_top_level(m.group("args"))]
                parsed = [parse_rust_literal(a) for a in args]
                if args and all(found for found, _ in parsed):
                    span = item.get("span") or {}
                    targets = fns_by_file_name.get(
                        (span.get("filename"), m.group(1)), []
                    )
                    for tid, titem in targets:
                        tsig = (titem.get("inner") or {}).get("function", {}).get(
                            "sig"
                        ) or {}
                        tinputs = [
                            n
                            for n, _ in (tsig.get("inputs") or [])
                            if isinstance(n, str) and n != "self"
                        ]
                        if len(tinputs) != len(args):
                            continue  # not the overload this delegates to
                        fn_defaults.setdefault(tid, {}).update(
                            {n: v for n, (_, v) in zip(tinputs, parsed, strict=False)}
                        )

        clean = _strip_line_comments(body)

        # ---- Mechanism B: Option<T> param + <param>.unwrap_or(<literal>) ----
        opt_params = [
            n
            for n, t in inputs
            if isinstance(n, str)
            and n != "self"
            and isinstance(t, dict)
            and "resolved_path" in t
            and str(t["resolved_path"].get("path", "")).split("::")[-1] == "Option"
        ]
        for pname in opt_params:
            found, value = _none_branch_default(clean, pname)
            if found:
                fn_defaults.setdefault(str(iid), {}).setdefault(pname, value)

        # ---- Mechanism D (collected here, APPLIED after the loop) ----
        # An options struct can leave a field ``None`` and have the constructor
        # that CONSUMES it supply the real default:
        #   ``Service::new(options: ServiceOptions)`` →
        #   ``options.host.unwrap_or_else(|| "0.0.0.0".to_string())``.
        # The struct-literal initializer (mechanism C) sees only that ``None``, so
        # without this the field reads as "defaults to None" when a caller who
        # sets nothing actually gets ``"0.0.0.0"``. Attribute the resolved value
        # to the OPTIONS STRUCT's field, where mechanism C would have put it — the
        # consuming ctor's resolution IS that field's effective default. Held in a
        # separate map and merged AFTER the walk so it deterministically WINS over
        # C's ``None``; doing it inline would make the result depend on rustdoc's
        # index iteration order.
        for pname, ptype in inputs:
            if not isinstance(pname, str) or pname == "self":
                continue
            if not (isinstance(ptype, dict) and "resolved_path" in ptype):
                continue
            struct_name = str(ptype["resolved_path"].get("path", "")).split("::")[-1]
            if struct_name not in _OPTIONS_CONSTRUCTS:
                continue
            for fm in re.finditer(
                re.escape(pname)
                + r"\s*\.\s*(\w+)\s*\.\s*(?:unwrap_or(?:_else)?|map_or_else|map_or)\s*\(",
                clean,
            ):
                fname = fm.group(1)
                found, value = _none_branch_default(clean, f"{pname}.{fname}")
                if found:
                    consumed_defaults.setdefault(struct_name, {}).setdefault(
                        fname, value
                    )

        # ---- Mechanism C: options-struct field initializers ----
        # Require the EXPLICIT ``<Struct> { … }`` literal form. A bare ``Self { … }``
        # inside ``impl <Struct>`` is the same thing, but the ``impl`` header sits
        # OUTSIDE this item's span, so there is no way to tell from the span alone
        # which struct ``Self`` names — attributing it by guess would be exactly
        # the fabrication this whole pass refuses to do.
        if item.get("name") in ("new", "default"):
            for struct_name in _OPTIONS_CONSTRUCTS:
                if not re.search(r"\b" + re.escape(struct_name) + r"\s*\{", body):
                    continue
                fields = _struct_literal_fields(body, struct_name)
                if fields:
                    options_defaults.setdefault(struct_name, {}).update(fields)
                break

    # Mechanism D wins over C: a field C read as ``None`` but which the consuming
    # constructor resolves to a real literal defaults to that literal, because
    # that is what a caller who sets nothing actually gets.
    for struct_name, fields in consumed_defaults.items():
        options_defaults.setdefault(struct_name, {}).update(fields)

    return fn_defaults, options_defaults


# ---------------------------------------------------------------------------
# PARAMETER OPTIONALITY (``required``)
#
# Rust has no omittable arguments: EVERY parameter must appear at the callsite,
# so "can the caller leave it out?" is never answerable from arity alone. What
# Rust does have is a way to MODEL ABSENCE in the type — ``Option<T>``. Passing
# ``None`` IS the don't-supply-it call, and the body then takes its no-value
# branch (``unwrap_or``, ``if let Some``, ``let … else``). That is exactly the
# capability the reference spells ``x: T | None = None``, so an ``Option<T>``
# param is ``required: false`` and a bare ``T`` param is ``required: true``.
#
# Emitting ``required: true`` for every param (the previous behaviour, from
# "Rust has no defaults") was not a conservative choice — it was a WRONG one for
# the 105 params the port models as ``Option<T>``: it reported a capability loss
# the port does not have.
#
# THE ONE EXCEPTION, and why it is a table and not a rule. The reference has 8
# params (of 1,187 it types ``optional<…>``) that are optional-TYPED but still
# positionally REQUIRED — the caller must pass something, and that something may
# be ``None``:
#
#     def merge(self, override: RequestOptions | None) -> RequestOptions
#     def resolve(client_default: RequestOptions | None,
#                 per_request: RequestOptions | None) -> _EffectiveOptions
#
# Rust spells those ``Option<&RequestOptions>`` — byte-identically to a genuinely
# omittable param, because Rust has ONE spelling for both and the bodies are the
# same shape (``client_default.cloned().unwrap_or_default()`` is a fallback
# either way). No property of the Rust source separates them, so no rule can:
# the distinction lives only in the reference's declaration. Rather than pretend
# to derive it, the divergence is NAMED here, per symbol, so it is auditable and
# so a NEW one cannot appear silently — an unlisted ``Option<T>`` is optional.
#
# This is not oracle-copying: the general answer is measured from the Rust type,
# and only this closed, enumerated set of reference-side quirks is reconciled —
# the same shape as PARAM_RECONCILE and RETURN_TYPE_OVERRIDE above.
#
# The 5 remaining reference optional-typed-required params (AIChatError.code,
# AgentBase.on_summary.summary, validate_request.params_or_raw_body,
# SignalWireRestError.status_code, SipEndpoints.create.calling_handler_resource_id)
# are NOT ``Option<T>`` in this port, so they never reach this table.
# ---------------------------------------------------------------------------

# ``{context: {native_param_name}}`` — ``Option<T>`` params the REFERENCE
# declares positionally required (optional-typed, no default). Verified against
# python_signatures.json 2026-07-27: exactly 8 reference params are
# ``optional<…>`` with ``required: true``, and these 3 are the ones this port
# spells ``Option<T>``.
OPTIONAL_TYPED_BUT_REQUIRED: dict[str, set[str]] = {
    "signalwire.rest._request_options.RequestOptions.merge": {"override_opts"},
    "signalwire.rest._request_options.resolve": {"client_default", "per_request"},
}


def _is_optional_slot(t, context: str, name: str) -> bool:
    """True when this param's Rust type MODELS ABSENCE (``Option<T>``).

    ``&Option<T>`` / ``&mut Option<T>`` count too — the borrow is invisible to
    the reference's type model (the same collapse ``translate_rust_type`` does).
    """
    if name in OPTIONAL_TYPED_BUT_REQUIRED.get(context, ()):
        return False
    while isinstance(t, dict) and "borrowed_ref" in t:
        t = t["borrowed_ref"].get("type")
    if not (isinstance(t, dict) and "resolved_path" in t):
        return False
    return str(t["resolved_path"].get("path", "")).split("::")[-1] == "Option"


def build_signature(
    fn: dict, paths: dict, aliases: dict, context: str, defaults: dict | None = None
) -> dict:
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
        p = {
            "name": reconcile.get("name", name),
            "type": reconcile.get("type", canon),
            # A bare Rust param is REQUIRED — the caller must pass a value and
            # there is no default to record. An ``Option<T>`` param is the port's
            # way of MODELLING ABSENCE (see _is_optional_slot), so it is not.
            "required": not _is_optional_slot(t, context, name),
        }
        # Key the recovered default off the NATIVE Rust param name — that is the
        # name the source-side extraction saw — not the post-reconcile emitted
        # name, which may have been renamed to the Python spelling.
        if defaults and name in defaults:
            p["default"] = defaults[name]
            p["required"] = False
        params_out.append(p)
    # Constructors have no Rust receiver but Python's canonical signature
    # includes ``self`` first. Synthesize it so __init__ shapes line up.
    if is_ctor and not is_method:
        params_out.insert(0, {"name": "self", "kind": "self"})

    output = sig.get("output")
    return_canon = (
        translate_rust_type(output, paths, aliases, f"{context}[->]")
        if output
        else "void"
    )
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


# ---------------------------------------------------------------------------
# AI Chat SIGNATURE synthesis (signalwire.ai_chat.client). The Rust client
# realizes each reference method through an idiom the raw rustdoc signature does
# not match the Python kwargs of:
#   * construction is `AIChatClient::builder()` (folded onto `__init__` for the
#     NAME) -- the builder fn takes zero args, so the rustdoc `__init__` has an
#     empty param list rather than the reference (project, token, space, url)
#     kwargs;
#   * `create_conversation` / `chat` / `summarize` collapse their optional kwargs
#     into a single typed options-object (`CreateOptions` / `ChatOptions` /
#     `SummarizeOptions`);
#   * `ChatLog` / `ChatResponse` / `ConversationInfo` are plain public-field
#     structs (the reference records a dataclass auto-`__init__` from the fields);
#   * the error family folds into one `AIChatError` struct (the reference records
#     a base `__init__(code, message)` on `AIChatError`).
# Rather than document these as signature omissions, synthesize the
# reference-shaped signatures here (the emitter/enumerator carries the idiom --
# Rule 2) and MERGE them OVER the rustdoc output, exactly as
# build_generated_signatures does for the generated REST layer. The Rust wire is
# identical (AI-CHAT gate verified); only the STATIC call shape differs, and that
# difference is the options-object / builder / struct-literal idiom.
# (The reference no longer records a `session: aiohttp.ClientSession` DI param on
# __init__ -- it was dropped upstream -- so __init__ folds to the natural
# (project, token, space, url) with no name-projection.)
_AI_CHAT_MODULE = "signalwire.ai_chat.client"
_S = {"name": "self", "kind": "self"}


def build_ai_chat_signatures() -> dict:
    """Reference-shaped signatures for the ai_chat.client classes, keyed to merge
    OVER the rustdoc-derived shapes. Mirrors the Python oracle exactly so the diff
    compares EQUAL with no ai_chat signature omissions."""

    def kw(name, type_, required=False, default=None):
        # Mirror the reference param shape EXACTLY: plain positional-or-keyword
        # (no explicit "kind"), "default" present only for non-required params.
        d = {"name": name, "type": type_, "required": required}
        if not required:
            d["default"] = default
        return d

    def pos(name, type_, required=True, default=None):
        d = {"name": name, "type": type_, "required": required}
        if not required:
            d["default"] = default
        return d

    client_methods = {
        # project/token/space/url map onto the builder setters. The oracle no
        # longer records a `session` DI param (dropped upstream), so __init__
        # folds naturally to (project, token, space, url) with no name-projection.
        "__init__": {
            "params": [
                _S,
                kw("project", "optional<string>", False, None),
                kw("token", "optional<string>", False, None),
                kw("space", "optional<string>", False, None),
                kw("url", "optional<string>", False, None),
            ],
            "returns": "void",
        },
        "close": {"params": [_S], "returns": "void"},
        "create_conversation": {
            "params": [
                _S,
                pos("conversation_id", "string", True),
                pos("config_url", "string", True),
                kw("user_message", "optional<string>", False, None),
                kw("timeout", "optional<int>", False, None),
                kw("user_metadata", "optional<dict<string,any>>", False, None),
                kw("reinit", "bool", False, False),
            ],
            "returns": "class:signalwire.ai_chat.client.ConversationInfo",
        },
        "chat": {
            "params": [
                _S,
                pos("conversation_id", "string", True),
                pos("message", "string", True),
                kw("role", "string", False, "user"),
                kw("config_url", "optional<string>", False, None),
                kw("user_metadata", "optional<dict<string,any>>", False, None),
                kw("timeout", "optional<int>", False, None),
                kw("reinit", "bool", False, False),
            ],
            "returns": "class:signalwire.ai_chat.client.ChatResponse",
        },
        "end": {
            "params": [_S, pos("conversation_id", "string", True)],
            "returns": "bool",
        },
        "delete": {
            "params": [_S, pos("conversation_id", "string", True)],
            "returns": "bool",
        },
        "log": {
            "params": [_S, pos("conversation_id", "string", True)],
            "returns": "class:signalwire.ai_chat.client.ChatLog",
        },
        "summarize": {
            "params": [
                _S,
                pos("conversation_id", "string", True),
                kw("summary_prompt", "optional<string>", False, None),
            ],
            "returns": "string",
        },
    }

    # Dataclass result-model __init__s (auto-ctor from public struct fields) plus
    # the per-field 0-param read accessors the reference dataclass records for each
    # public field. Rust exposes these as bare `pub` struct fields (a field is not a
    # `pub fn`, so rustdoc misses them); synthesize the field-read accessor shape
    # here (self-only, returns the field type) exactly as the oracle records it —
    # CLASS B field-emit via the enumerator (Rule 2), no signature omission needed.
    def acc(ret):
        return {"params": [_S], "returns": ret}

    chatlog_init = {
        "params": [
            _S,
            kw("messages", "list<dict<string,any>>", False, "list()"),
            kw("call_timeline", "list<dict<string,any>>", False, "list()"),
        ],
        "returns": "void",
    }
    chatlog_methods = {
        "__init__": chatlog_init,
        "messages": acc("list<dict<string,any>>"),
        "call_timeline": acc("list<dict<string,any>>"),
    }
    chatresponse_init = {
        "params": [
            _S,
            pos("text", "string", True),
            pos("conversation_id", "string", True),
            kw("user_event", "optional<dict<string,any>>", False, None),
        ],
        "returns": "void",
    }
    chatresponse_methods = {
        "__init__": chatresponse_init,
        "text": acc("string"),
        "conversation_id": acc("string"),
        "user_event": acc("optional<dict<string,any>>"),
    }
    conversationinfo_init = {
        "params": [
            _S,
            pos("id", "string", True),
            pos("status", "string", True),
            kw("initial_message", "optional<string>", False, None),
        ],
        "returns": "void",
    }
    conversationinfo_methods = {
        "__init__": conversationinfo_init,
        "id": acc("string"),
        "status": acc("string"),
        "initial_message": acc("optional<string>"),
    }
    # The error family folds to one AIChatError struct; the reference records the
    # base __init__(code, message). `code` is optional in the oracle but marked
    # required=True (a positional with an Optional type), so mirror that exactly.
    aichaterror_init = {
        "params": [
            _S,
            pos("code", "optional<int>", True),
            pos("message", "string", True),
        ],
        "returns": "void",
    }

    return {
        _AI_CHAT_MODULE: {
            "classes": {
                "AIChatClient": {"methods": dict(sorted(client_methods.items()))},
                "ChatLog": {"methods": dict(sorted(chatlog_methods.items()))},
                "ChatResponse": {"methods": dict(sorted(chatresponse_methods.items()))},
                "ConversationInfo": {
                    "methods": dict(sorted(conversationinfo_methods.items()))
                },
                "AIChatError": {"methods": {"__init__": aichaterror_init}},
            }
        }
    }


def run_dump() -> dict:
    cp = subprocess.run(
        [
            "cargo",
            "+nightly",
            "rustdoc",
            "--lib",
            "--",
            "-Z",
            "unstable-options",
            "--output-format",
            "json",
        ],
        cwd=PORT_ROOT,
        capture_output=True,
        text=True,
        timeout=600,
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
    # Strict is the DEFAULT (see the failure branch below); --no-strict is an
    # ad-hoc local-debugging escape hatch that no gate may use.
    parser.add_argument("--strict", action=argparse.BooleanOptionalAction, default=True)
    args = parser.parse_args()

    aliases = load_aliases()
    if args.raw and args.raw.is_file():
        rust_doc = json.loads(args.raw.read_text(encoding="utf-8"))
    else:
        rust_doc = run_dump()

    canonical, failures = collect(rust_doc, aliases)
    if failures:
        print(
            f"enumerate_signatures: {len(failures)} translation failure(s)",
            file=sys.stderr,
        )
        for f in failures[:30]:
            print(f"  - {f}", file=sys.stderr)
        if len(failures) > 30:
            print(f"  ... ({len(failures) - 30} more)", file=sys.stderr)
        # FAIL LOUD BY DEFAULT. A translation failure means a type this port
        # really exposes could not be rendered into the canonical inventory, so
        # the artifact written below is SHORT — and it is exactly what the
        # SIGNATURES/DRIFT gates then compare the port against. Behind the old
        # opt-in `--strict` (which NO gate passed) that produced a rc=0 "success"
        # whose output blamed the port for omissions it never had. Same defect
        # php shipped. `--no-strict` keeps the tolerant mode for ad-hoc local
        # debugging only; run-ci.sh must never pass it.
        if args.strict:
            return 1

    args.out.write_text(
        json.dumps(canonical, indent=2, sort_keys=False) + "\n", encoding="utf-8"
    )
    n_mods = len(canonical["modules"])
    n_methods = sum(
        sum(len(c["methods"]) for c in m.get("classes", {}).values())
        for m in canonical["modules"].values()
    )
    print(
        f"enumerate_signatures: wrote {args.out} ({n_mods} modules, {n_methods} methods)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
