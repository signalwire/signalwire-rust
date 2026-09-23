#!/usr/bin/env python3
"""Generate the SignalWire REST namespace resource layer for signalwire-rust.

This is the RUST realization of porting-sdk/REST_GENERATOR_RULES.md — the
language-neutral contract of the REST resource generator (bases,
x-sdk-resource markup, path composition, command-dispatch, set_methods,
cross-spec client-tree placement, fail-loud invariants). It mirrors the proven
php/go/ts generators; only the EMITTER (Rust) differs.

Inputs (resolved from $PORTING_SDK or the adjacent ../porting-sdk):
    rest-apis/<ns>/openapi.yaml       (+ x-sdk-* markup)
    rest-apis/x-sdk-bases.yaml        (shared base method-sets)
    rest-apis/fabric/x-sdk-bases.yaml (FabricResource)

Outputs: one Rust module file per generated namespace under
    src/rest/namespaces/generated/<ns>_resources_generated.rs
plus the client-tree file
    src/rest/namespaces/generated/client_tree_generated.rs
and a mod.rs re-exporting them. The hand BASES stay hand-written
(src/rest/{crud_resource, http_client, ...}); the generator emits ONLY the
per-resource structs, their constructors (§4 base paths baked in), their
declared/command/set methods, and the container tree.

RUST IDIOM (PORT_PHILOSOPHY_RUST.md, SESSION_CHANGESET_FOR_PORTS L13):
Rust is static-typed with NO defaults, so the named-param set of an
operation/command/set method is realized as a REQUEST STRUCT + a fluent BUILDER
(`XRequest::new(required...).opt(v).build()`) carrying an ``extras:
serde_json::Map`` open door — the aws-sdk-rust / API-Guidelines options-builder
idiom. Distinct i64/f64 (Rust has no numeric monotype). Struct/type names =
x-sdk-resource.name VERBATIM (the Python oracle canonical names — AiAgents,
SipEndpoints, VideoRooms, …), so the rust adapter (enumerate_surface.py) projects
each generated struct onto the same
signalwire.rest.namespaces.<ns>_resources_generated.<Name> oracle module.

A spec field whose name is a Rust keyword (type/ref/match/…) or otherwise not a
legal field ident is emitted as a raw identifier (``r#type``) — never dropped;
reported at the end of a full run.

Usage:
    python3 scripts/generate_rest.py                 # write into the repo tree
    python3 scripts/generate_rest.py --check         # GEN-FRESH: fail if stale
    python3 scripts/generate_rest.py --out DIR       # scratch: emit into DIR
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover
    sys.stderr.write("generate_rest.py requires PyYAML (pip install pyyaml)\n")
    raise


# REST namespace DISCOVERY (mirrors the Python reference's spec-dir scan in
# porting-sdk/scripts/generate_python_rest_types.py, and the php/java ports — no
# hardcoded membership list).
#
# The two namespace SETS are derived by scanning rest-apis/<ns>/openapi.yaml:
#   * RESOURCE namespaces (the former SPEC_DIRS): a spec dir with at least one
#     non-excluded, named ``x-sdk-resource`` block — the specs that emit generated
#     resource modules + client-tree containers. (``projects`` — the /api/projects
#     project-management API — carries an ``x-sdk-resource`` block and so IS a
#     resource namespace, emitting the flat ``Projects`` CrudResource; distinct
#     from the singular ``project`` token namespace. ``swml-webhooks`` is a
#     types-only webhook-payload spec with no resources.)
#   * TYPE namespaces (the former TYPE_NS): every RESOURCE namespace PLUS the
#     types-only specs — a spec with components.schemas but no ``servers`` block
#     (the webhook-payload specs, e.g. ``swml-webhooks``). BROADER than the
#     resource set: swml-webhooks has no x-sdk-resource yet still emits DTOs.
#
# The membership of both sets is fully discovered. The one fact the scan cannot
# derive from the spec dir + markup is the curated cross-namespace ORDER (which
# drives the client-tree container + flat-resource accessor order — not
# alphabetical, not derivable from any spec field). It is kept as a small explicit
# table (like ATTR_OVERRIDE); discovery fails LOUD if a scanned namespace is
# missing from it, so a new resource spec dir is picked up automatically and only
# needs an order placement. The module/key leaf is ``<spec dir>`` with ``-`` -> ``_``
# (``relay-rest`` -> ``relay_rest``), derived via snake_of, not tabulated.
_NS_ORDER = (
    "relay-rest",
    "fabric",
    "calling",
    "video",
    "datasphere",
    "logs",
    "message",
    "messages",
    "voice",
    "fax",
    "project",
    "projects",
    "chat",
    "pubsub",
    "swml-webhooks",
)


def _spec_docs(psdk: Path) -> dict[str, dict]:
    """Scan rest-apis/ once: {spec_dir: parsed openapi doc} for every dir with an
    openapi.yaml (sorted). Cached on the function for the process lifetime."""
    cache = getattr(_spec_docs, "_cache", None)
    if cache is None:
        cache = {}
        for d in sorted((psdk / "rest-apis").iterdir()):
            y = d / "openapi.yaml"
            if y.is_file():
                cache[d.name] = yaml.safe_load(y.read_text()) or {}
        _spec_docs._cache = cache  # type: ignore[attr-defined]
    return cache


def _has_resource(doc: dict) -> bool:
    for item in (doc.get("paths") or {}).values():
        if not isinstance(item, dict):
            continue
        r = item.get("x-sdk-resource")
        if r and not r.get("exclude") and r.get("name"):
            return True
    return False


def _order_key(ns: str) -> int:
    if ns not in _NS_ORDER:
        raise SystemExit(
            f"generate_rest.py: spec dir {ns!r} has no entry in _NS_ORDER — add it "
            f"to the curated cross-namespace order (the scan cannot derive order)"
        )
    return _NS_ORDER.index(ns)


def discover_spec_dirs(psdk: Path) -> list[str]:
    """RESOURCE namespaces (former SPEC_DIRS): spec dirs carrying x-sdk-resource
    markup, in the curated cross-namespace order."""
    dirs = [ns for ns, doc in _spec_docs(psdk).items() if _has_resource(doc)]
    return sorted(dirs, key=_order_key)


def discover_type_ns(psdk: Path) -> list[tuple[str, str]]:
    """TYPE namespaces (former TYPE_NS): RESOURCE namespaces PLUS types-only specs
    (components.schemas but no servers block). Returns (spec_dir, ns_key) in the
    curated order — ns_key = spec dir with '-' -> '_'."""
    out: list[tuple[str, str]] = []
    for ns, doc in _spec_docs(psdk).items():
        has_schemas = bool(((doc.get("components") or {}).get("schemas")) or {})
        if not has_schemas:
            continue
        is_types_only = not doc.get("servers")
        if _has_resource(doc) or is_types_only:
            out.append((ns, snake_of(ns)))
    return sorted(out, key=lambda t: _order_key(t[0]))


# Rust reserved words (2015+2018+2021+2024 keywords, incl. reserved). A spec
# field or path arg colliding gets a raw identifier ``r#<word>`` (except the few
# that are not valid even as raw: crate/self/super/Self — none occur as wire
# field names, but guard anyway → trailing underscore).
RUST_KEYWORDS = {
    "as",
    "break",
    "const",
    "continue",
    "crate",
    "dyn",
    "else",
    "enum",
    "extern",
    "false",
    "fn",
    "for",
    "if",
    "impl",
    "in",
    "let",
    "loop",
    "match",
    "mod",
    "move",
    "mut",
    "pub",
    "ref",
    "return",
    "self",
    "Self",
    "static",
    "struct",
    "super",
    "trait",
    "true",
    "type",
    "unsafe",
    "use",
    "where",
    "while",
    "async",
    "await",
    "gen",
    "abstract",
    "become",
    "box",
    "do",
    "final",
    "macro",
    "override",
    "priv",
    "typeof",
    "unsized",
    "virtual",
    "yield",
    "try",
    "union",
}
# Raw identifiers r#kw are legal for every keyword EXCEPT these.
RUST_NO_RAW = {"crate", "self", "Self", "super"}


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
        "generate_rest.py: porting-sdk not found (set $PORTING_SDK or clone adjacent)"
    )


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


# ---------------------------------------------------------------------------
# SDK-surface policy overlay (the single source; NOT wire truth).
# ---------------------------------------------------------------------------
# rest-apis/x-sdk-overlay.yaml is the ONE authoritative place that says which spec
# fields the SDKs hide (dropped from the surface) or deprecate (emitted-but-flagged).
# It is a policy overlay, not markup in the (often vendored) specs, so the same field
# is governed once and applied wherever it surfaces (schema.json AIParams + the
# calling/fabric REST projections). Matching is by (field name, containing SPEC schema
# name — the $defs / components.schemas key), NOT the Rust type name we later emit.
_overlay_cache: dict[str, set[tuple[str, str | None]]] | None = None


def _load_overlay(psdk: Path | None = None) -> dict[str, set[tuple[str, str | None]]]:
    global _overlay_cache
    if _overlay_cache is None:
        base = psdk if psdk is not None else resolve_porting_sdk()
        path = base / "rest-apis" / "x-sdk-overlay.yaml"

        def rules(key: str, data: dict) -> set[tuple[str, str | None]]:
            out: set[tuple[str, str | None]] = set()
            for entry in data.get(key) or []:
                if isinstance(entry, dict) and entry.get("field"):
                    out.add((entry["field"], entry.get("scope")))
            return out

        data = {}
        if path.is_file():
            data = yaml.safe_load(path.read_text()) or {}
        _overlay_cache = {
            "hidden": rules("hidden", data),
            "deprecated": rules("deprecated", data),
        }
    return _overlay_cache


def _overlay_match(
    rules: set[tuple[str, str | None]], field: str, schema_name: str | None
) -> bool:
    # A rule matches when its field equals `field` AND (it is unscoped OR its scope
    # equals the containing SPEC schema name). `schema_name` is the schema's name as it
    # appears in the spec (the $defs / components.schemas key) — NOT the Rust type name
    # this generator later emits — so the scope value is identical across all ports.
    for rf, scope in rules:
        if rf == field and (scope is None or scope == schema_name):
            return True
    return False


def overlay_hidden(field: str, schema_name: str | None = None) -> bool:
    return _overlay_match(_load_overlay()["hidden"], field, schema_name)


def overlay_deprecated(field: str, schema_name: str | None = None) -> bool:
    return _overlay_match(_load_overlay()["deprecated"], field, schema_name)


# ---------------------------------------------------------------------------
# Base loading (x-sdk-bases; §2) — validate + flatten to method-sets.
# ---------------------------------------------------------------------------


def load_bases(psdk: Path) -> dict[str, list[str]]:
    raw = yaml.safe_load((psdk / "rest-apis" / "x-sdk-bases.yaml").read_text())
    bases = dict(raw.get("x-sdk-bases") or {})
    fab = psdk / "rest-apis" / "fabric" / "x-sdk-bases.yaml"
    if fab.is_file():
        bases.update(yaml.safe_load(fab.read_text()).get("x-sdk-bases") or {})

    def resolve(name: str, seen: set[str]) -> list[str]:
        if name in seen:
            raise SystemExit(f"x-sdk-bases: cyclic extends at {name}")
        if name not in bases:
            raise SystemExit(f"x-sdk-bases: undefined base {name!r}")
        seen = seen | {name}
        methods: list[str] = []
        ext = bases[name].get("extends")
        if ext:
            methods.extend(resolve(ext, seen))
        methods.extend(list((bases[name].get("methods") or {}).keys()))
        return methods

    return {name: resolve(name, set()) for name in bases}


# ---------------------------------------------------------------------------
# Spec model.
# ---------------------------------------------------------------------------


class Spec:
    def __init__(self, name: str, doc: dict):
        self.name = name
        self.doc = doc
        self.server_path = _url_path(doc["servers"][0]["url"])
        if self.server_path != "/" and self.server_path.endswith("/"):
            raise SystemExit(
                f"{name}: servers[0].url path {self.server_path!r} has a trailing slash"
            )
        self.namespace_attr = (doc.get("x-sdk-namespace") or {}).get("attr") or ""
        self.ops: dict[str, tuple[str, str, bool]] = {}
        self.op_body: dict[str, dict] = {}
        for path, item in (doc.get("paths") or {}).items():
            for verb in ("get", "post", "put", "patch", "delete"):
                o = item.get(verb)
                if o and o.get("operationId"):
                    self.ops[o["operationId"]] = (
                        verb,
                        path,
                        bool(o.get("requestBody")),
                    )
                    body = o.get("requestBody") or {}
                    content = body.get("content") or {}
                    media = content.get("application/json") or (
                        next(iter(content.values())) if content else {}
                    )
                    self.op_body[o["operationId"]] = (media or {}).get("schema") or {}
        self.schemas = ((doc.get("components") or {}).get("schemas")) or {}

    def resources(self) -> list[tuple[str, dict]]:
        out = []
        for path, item in (self.doc.get("paths") or {}).items():
            r = item.get("x-sdk-resource")
            if r and not r.get("exclude") and r.get("name"):
                out.append((path, r))
        return out


def _url_path(url: str) -> str:
    if "://" in url:
        url = url.split("://", 1)[1]
    i = url.find("/")
    return url[i:] if i >= 0 else "/"


def load_spec(psdk: Path, ns: str) -> Spec:
    return Spec(
        ns, yaml.safe_load((psdk / "rest-apis" / ns / "openapi.yaml").read_text())
    )


# ---------------------------------------------------------------------------
# Path composition (§4).
# ---------------------------------------------------------------------------


def join_path(a: str, b: str) -> str:
    if not b:
        return a
    return a.rstrip("/") + "/" + b.lstrip("/")


def collection_segment(anchor: str, markup: dict) -> str:
    if "collection" in markup:
        return markup["collection"]
    p = anchor
    i = p.find("/{")
    if i >= 0:
        p = p[:i]
    return p


def base_path(spec: Spec, anchor: str, markup: dict) -> str:
    return join_path(spec.server_path, collection_segment(anchor, markup))


def relative_tail(spec: Spec, anchor: str, markup: dict, op_path: str):
    coll = collection_segment(anchor, markup)
    full = join_path(spec.server_path, coll)
    absp = join_path(spec.server_path, op_path)
    if coll and absp.startswith(full + "/"):
        return ([s for s in absp[len(full) + 1 :].split("/") if s], False)
    if coll and absp == full:
        return ([], False)
    return ([s for s in absp.lstrip("/").split("/") if s], True)


# ---------------------------------------------------------------------------
# Naming.
# ---------------------------------------------------------------------------


def snake_of(s: str) -> str:
    """Normalize an already-snake-ish string (fold '-'/'.' to '_')."""
    return re.sub(r"[^A-Za-z0-9_]", "_", s)


def snake(s: str) -> str:
    """PascalCase / mixed → snake_case (for file names). Insert ``_`` before an
    interior capital run boundary, fold non-idents, lower-case, dedup ``_``."""
    s = re.sub(r"[^A-Za-z0-9]+", "_", s)
    # AB C → A_B C ; camelCase → camel_Case ; then lower.
    s = re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", s)
    s = re.sub(r"(?<=[A-Z])(?=[A-Z][a-z])", "_", s)
    s = re.sub(r"_+", "_", s).strip("_").lower()
    return s or "schema"


# Reserved-word field/arg renames encountered during a run (for the report).
_RESERVED_RENAMES: set[tuple[str, str]] = set()


def field_ident(field: str) -> str:
    """The Rust identifier for a wire field / path-param name. A Rust keyword
    becomes a raw identifier ``r#kw`` (a genuine rename recorded for the report,
    NOT an omission); a keyword with no legal raw form gets a trailing ``_``. A
    non-identifier rune folds to ``_``."""
    ident = snake_of(field)
    if not ident:
        ident = "field"
    if ident[0].isdigit():
        ident = "_" + ident
    if ident in RUST_KEYWORDS:
        if ident in RUST_NO_RAW:
            _RESERVED_RENAMES.add((field, ident + "_"))
            return ident + "_"
        _RESERVED_RENAMES.add((field, "r#" + ident))
        return "r#" + ident
    return ident


def rs_str(s: str) -> str:
    return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'


def setter_ident(wire: str) -> str:
    """The builder-setter method name for an optional field: the field's Rust
    identifier, kept raw-escaped if it collides with a keyword (a `loop` field
    yields `r#loop` — a bare `pub fn loop()` is a parse error)."""
    return field_ident(wire)


PARAM_ARG_NAME = {
    "id": "id",
    "queue_id": "queue_id",
    "NumberGroupId": "group_id",
    "documentId": "document_id",
    "chunkId": "chunk_id",
    "mfa_request_id": "request_id",
    "e164_number": "e164",
    "fabric_subscriber_id": "subscriber_id",
    "ai_agent_id": "id",
    "cxml_webhook_id": "id",
    "swml_webhook_id": "id",
    "token_id": "token_id",
    "room_id": "room_id",
    "resource_id": "resource_id",
    "sip_endpoint_id": "sip_endpoint_id",
    "membership_id": "membership_id",
}


def arg_for(brace: str) -> str:
    return PARAM_ARG_NAME.get(brace, snake_of(brace) or "id")


# ---------------------------------------------------------------------------
# Base mapping (§2).
# ---------------------------------------------------------------------------

# ``paginate`` is provided ONLY by the pure read base (ReadResource): the Python
# oracle records ``paginate`` as an OWN method on the read-only leaf resources
# (FaxLogs / MessageLogs / VideoRoomSessions / VoiceLogs / FabricAddresses) that
# subclass ReadResource directly, but NOT on the CRUD/Fabric resources (there it
# is inherited-and-collapsed, excused by the diff's crud_base handling). So keep
# ``paginate`` out of the CRUD/Fabric provides.
BASE_PROVIDES = {
    "CrudResource": {"list", "create", "get", "update", "delete"},
    "FabricResource": {"list", "create", "get", "update", "delete", "list_addresses"},
    "ReadResource": {"list", "get", "paginate"},
    "BaseResource": set(),
}


# ---------------------------------------------------------------------------
# Command-dispatch (§6).
# ---------------------------------------------------------------------------


def command_method_name(cmd: str) -> str:
    s = cmd[len("calling.") :] if cmd.startswith("calling.") else cmd
    return snake_of(s)


def discriminator_mapping(spec: Spec, schema_name: str) -> dict[str, str]:
    sch = spec.schemas.get(schema_name)
    if sch is None:
        raise SystemExit(
            f"command-dispatch request {schema_name!r} not in components.schemas"
        )
    mapping = (sch.get("discriminator") or {}).get("mapping")
    if not mapping:
        raise SystemExit(
            f"command-dispatch request {schema_name!r} has no discriminator.mapping"
        )
    return dict(mapping)


# ---------------------------------------------------------------------------
# Typed inputs (§5) — schema → Rust native type.
# ---------------------------------------------------------------------------
#
# The generated operation/command/set methods take a REQUEST STRUCT + BUILDER
# (Rust's named idiom, PORT_PHILOSOPHY_RUST). Required fields → constructor
# args; optional fields → builder setters storing Option<T>; plus an ``extras:
# serde_json::Map<String, Value>`` open door (the cross-port ``extras`` escape
# hatch). ``.build()`` assembles the body Value, omitting unset optionals
# (matching the reference's "omit unset optionals" wire behavior).
#
# Rust HAS distinct i64/f64 (no numeric monotype) — integer→i64, number→f64.


def resolve_schema(spec: Spec, schema: dict | None, seen=None) -> dict:
    if not schema:
        return {}
    if seen is None:
        seen = set()
    ref = schema.get("$ref")
    if ref:
        leaf = ref.rsplit("/", 1)[-1]
        if leaf in seen:
            return {}
        seen.add(leaf)
        return resolve_schema(spec, spec.schemas.get(leaf), seen)
    allof = schema.get("allOf")
    if (
        allof
        and len(allof) == 1
        and not schema.get("properties")
        and not schema.get("type")
    ):
        return resolve_schema(spec, allof[0], seen)
    return schema


def _json_type(schema: dict) -> str | None:
    t = schema.get("type")
    if isinstance(t, list):
        non_null = [x for x in t if x != "null"]
        return non_null[0] if non_null else None
    return t


# JSON scalar → (Rust owned type, `serde_json::json!` / into-Value builder).
_SCALAR_RUST = {
    "string": "String",
    "integer": "i64",
    "number": "f64",
    "boolean": "bool",
}


def rust_field_type(spec: Spec, schema: dict) -> str:
    """The Rust owned type for a body field (unwrapped — the struct wraps optionals
    in Option<T> itself). Scalars stay distinct (i64/f64); array/object/$ref/union
    → serde_json::Value (an open JSON value, the dynamic-bag idiom)."""
    resolved = resolve_schema(spec, schema)
    jt = _json_type(resolved)
    return _SCALAR_RUST.get(jt, "serde_json::Value")


def object_body_fields(spec: Spec, body_schema: dict) -> list[tuple[str, dict, bool]]:
    resolved = resolve_schema(spec, body_schema)
    props: dict[str, dict] = {}
    required: set[str] = set(resolved.get("required") or [])
    for name, psc in (resolved.get("properties") or {}).items():
        props.setdefault(name, psc)
    for br in resolved.get("allOf") or []:
        rb = resolve_schema(spec, br)
        required |= set(rb.get("required") or [])
        for name, psc in (rb.get("properties") or {}).items():
            props.setdefault(name, psc)
    return [(name, psc, name in required) for name, psc in props.items()]


def command_param_fields(
    spec: Spec, command_schema: dict
) -> tuple[list[tuple[str, dict, bool]], bool]:
    """§6 union-flatten: union of all variants' fields, required only if EVERY
    variant requires it. ``has_id`` = command schema declares an ``id``."""
    cs = resolve_schema(spec, command_schema)
    has_id = "id" in (cs.get("properties") or {})
    params_schema = (cs.get("properties") or {}).get("params")
    if params_schema is None:
        return [], has_id
    ps = resolve_schema(spec, params_schema)
    variants: list[dict] = []
    for comb in ("anyOf", "oneOf"):
        if comb in ps:
            variants = [resolve_schema(spec, v) for v in ps[comb]]
            break
    if not variants:
        variants = [ps]
    all_props: dict[str, dict] = {}
    req_sets: list[set[str]] = []
    for v in variants:
        req_sets.append(set(v.get("required") or []))
        for name, psc in (v.get("properties") or {}).items():
            all_props.setdefault(name, psc)
    req_all = set.intersection(*req_sets) if req_sets else set()
    return [(name, psc, name in req_all) for name, psc in all_props.items()], has_id


def is_object_body(spec: Spec, body_schema: dict) -> bool:
    if not body_schema:
        return False
    if "anyOf" in body_schema or "oneOf" in body_schema:
        return False
    resolved = resolve_schema(spec, body_schema)
    if "anyOf" in resolved or "oneOf" in resolved:
        return False
    if resolved.get("properties") or resolved.get("allOf"):
        return True
    return _json_type(resolved) == "object"


def ordered_fields(fields):
    req = [f for f in fields if f[2]]
    opt = [f for f in fields if not f[2]]
    return req + opt


def schema_fields(spec: Spec, schema: dict, seen=None) -> set[str]:
    if schema is None:
        return set()
    if seen is None:
        seen = set()
    ref = schema.get("$ref")
    if ref:
        leaf = ref.rsplit("/", 1)[-1]
        if leaf in seen:
            return set()
        seen.add(leaf)
        return schema_fields(spec, spec.schemas.get(leaf), seen)
    out = set(((schema.get("properties")) or {}).keys())
    for comb in ("allOf", "anyOf", "oneOf"):
        for br in schema.get(comb) or []:
            out |= schema_fields(spec, br, seen)
    return out


def update_request_fields(
    spec: Spec, anchor: str, markup: dict
) -> tuple[set[str], dict[str, dict]]:
    coll = collection_segment(anchor, markup)
    want_verb = "put" if markup.get("update_method") == "PUT" else "patch"
    for path, item in (spec.doc.get("paths") or {}).items():
        if not path.startswith(coll + "/{"):
            continue
        if path.count("/{") != 1 or not path.endswith("}"):
            continue
        op = item.get(want_verb) or item.get("put") or item.get("patch")
        if not op:
            continue
        content = (op.get("requestBody") or {}).get("content") or {}
        for media in content.values():
            sch = media.get("schema")
            if sch:
                fields = schema_fields(spec, sch)
                fschemas = {name: psc for name, psc, _ in object_body_fields(spec, sch)}
                return fields, fschemas
    return set(), {}


# ---------------------------------------------------------------------------
# Rust emitters.
# ---------------------------------------------------------------------------

GEN_BANNER = """// Code generated by scripts/generate_rest.py; DO NOT EDIT.
//
// Regenerate with:
//   python3 scripts/generate_rest.py
//
// {desc}
"""


def gen_imports(body: str) -> str:
    """Emit ONLY the `use` lines the assembled module body actually references,
    so the generated file carries no unused imports (each namespace uses a
    different subset of the bases / std types)."""
    lines: list[str] = []
    if re.search(r"\bHashMap\b", body):
        lines.append("use std::collections::HashMap;")
        lines.append("")
    serde_json = [n for n in ("Map", "Value") if re.search(rf"\b{n}\b", body)]
    if serde_json:
        one = (
            serde_json[0] if len(serde_json) == 1 else "{" + ", ".join(serde_json) + "}"
        )
        lines.append(f"use serde_json::{one};")
        lines.append("")
    if re.search(r"\bSignalWireRestError\b", body):
        lines.append("use crate::rest::error::SignalWireRestError;")
    if re.search(r"\bRequestOptions\b", body):
        lines.append("use crate::rest::request_options::RequestOptions;")
    bases = [
        b
        for b in ("BaseResource", "CrudResource", "FabricResource", "ReadResource")
        if re.search(rf"\b{b}\b", body)
    ]
    if bases:
        lines.append(f"use crate::rest::generated_bases::{{{', '.join(bases)}}};")
    if re.search(r"\bHttpClient\b", body):
        lines.append("use crate::rest::http_client::HttpClient;")
    if re.search(r"\bPaginatedIterator\b", body):
        lines.append("use crate::rest::pagination::PaginatedIterator;")
    return "\n".join(lines) + ("\n" if lines else "")


def method_call_path(spec: Spec, anchor: str, markup: dict, op_path: str):
    """Return (id_arg_names, rust_path_expr, sibling)."""
    segs, sibling = relative_tail(spec, anchor, markup, op_path)
    id_args: list[str] = []
    pieces: list[str] = []
    for s in segs:
        if s.startswith("{") and s.endswith("}"):
            arg = arg_for(s[1:-1])
            while arg in id_args:
                arg += "2"
            id_args.append(arg)
            pieces.append(arg)  # a variable ref
        else:
            pieces.append(rs_str(s))  # a literal
    if sibling:
        full = join_path(spec.server_path, op_path.lstrip("/"))
        expr = abs_rust_path(full, id_args)
    elif not pieces:
        # base_path() already returns &str — pass it directly (no borrow/owned).
        expr = "self.base_path()"
    else:
        # self.path(&[<pieces>]) returns an owned String; each piece is a &str
        # literal or &str var (already &str, no conversion). Borrow the String.
        expr = "&self.path(&[" + ", ".join(pieces) + "])"
    return id_args, expr, sibling


def abs_rust_path(full: str, id_args: list[str]) -> str:
    """A `format!`-style Rust expression for a sibling absolute path,
    substituting {brace} with the positional id_args in order."""
    out_fmt = []
    any_arg = False
    ai = 0
    i = 0
    while i < len(full):
        if full[i] == "{":
            j = full.find("}", i)
            if ai < len(id_args):
                # inline captured identifier `{ident}` (uninlined_format_args):
                # id_args are simple snake_case path-param identifiers.
                out_fmt.append("{" + id_args[ai] + "}")
                any_arg = True
                ai += 1
            i = j + 1
            continue
        out_fmt.append(full[i])
        i += 1
    fmt_str = "".join(out_fmt).replace('"', '\\"')
    if any_arg:
        # format!(...) yields an owned String; borrow it for the &str-taking client.
        return f'&format!("{fmt_str}")'
    # A constant path with no substitutions is already a &str literal.
    return f'"{fmt_str}"'


def _request_struct_name(cls: str, method_rs: str) -> str:
    """PascalCase request-struct name for a method's named params (Rust idiom)."""
    parts = [p for p in method_rs.split("_") if p]
    pm = "".join(w[:1].upper() + w[1:] for w in parts)
    return f"{cls}{pm}Request"


def emit_request_struct(
    struct_name: str,
    spec: Spec,
    leading: list[tuple[str, str]],
    fields: list[tuple[str, dict, bool]],
    wire_container: str,
) -> tuple[str, str]:
    """Emit a request struct + fluent builder + build() -> Value.

    ``leading`` = [(rust_ident, "String")] required leading positional args
    (a call_id for command methods; NONE for object bodies — the id there is a
    method arg, not a body field). ``wire_container`` = "body" | "params" — the
    JSON object the fields go into. Returns (struct_source, ctor_call_hint)."""
    req = [(n, s, r) for (n, s, r) in ordered_fields(fields) if r]
    opt = [(n, s, r) for (n, s, r) in ordered_fields(fields) if not r]

    lines: list[str] = []
    lines.append(
        "/// Named request parameters for the generated method (Rust options-builder"
    )
    lines.append(
        "/// idiom — required fields in `new`, optionals via setters, `extras` open door)."
    )
    lines.append("#[derive(Debug, Clone, Default)]")
    lines.append(f"pub struct {struct_name} {{")
    for ident, ty in leading:
        lines.append(f"    {ident}: {ty},")
    for wire, sch, _ in req:
        ident = field_ident(wire)
        lines.append(f"    {ident}: {rust_field_type(spec, sch)},")
    for wire, sch, _ in opt:
        ident = field_ident(wire)
        lines.append(f"    {ident}: Option<{rust_field_type(spec, sch)}>,")
    lines.append("    extras: Map<String, Value>,")
    lines.append("}")
    lines.append("")
    lines.append(f"impl {struct_name} {{")

    # new(required...)
    new_params = []
    for ident, ty in leading:
        new_params.append(
            f"{ident}: impl Into<{ty}>" if ty == "String" else f"{ident}: {ty}"
        )
    for wire, sch, _ in req:
        ident = field_ident(wire)
        ty = rust_field_type(spec, sch)
        new_params.append(
            f"{ident}: impl Into<{ty}>" if ty == "String" else f"{ident}: {ty}"
        )
    lines.append("    /// Construct the request with its required fields.")
    if len(new_params) > 7:
        # required-param count is spec-mandated (all required create fields); the
        # arg count mirrors the wire contract, so allow it narrowly on this fn.
        lines.append("    #[allow(clippy::too_many_arguments)]")
    lines.append(f"    pub fn new({', '.join(new_params)}) -> Self {{")
    lines.append(f"        {struct_name} {{")
    for ident, ty in leading:
        # field-init shorthand when no conversion is needed (redundant_field_names)
        lines.append(
            f"            {ident}: {ident}.into(),"
            if ty == "String"
            else f"            {ident},"
        )
    for wire, sch, _ in req:
        ident = field_ident(wire)
        ty = rust_field_type(spec, sch)
        lines.append(
            f"            {ident}: {ident}.into(),"
            if ty == "String"
            else f"            {ident},"
        )
    lines.append("            ..Default::default()")
    lines.append("        }")
    lines.append("    }")

    # optional setters
    for wire, sch, _ in opt:
        ident = field_ident(wire)
        ty = rust_field_type(spec, sch)
        setter = setter_ident(wire)
        arg = "value"
        if ty == "String":
            lines.append(f"    /// Set the optional `{wire}` field.")
            lines.append("    #[must_use]")
            lines.append(
                f"    pub fn {setter}(mut self, {arg}: impl Into<{ty}>) -> Self {{"
            )
            lines.append(f"        self.{ident} = Some({arg}.into());")
        else:
            lines.append(f"    /// Set the optional `{wire}` field.")
            lines.append("    #[must_use]")
            lines.append(f"    pub fn {setter}(mut self, {arg}: {ty}) -> Self {{")
            lines.append(f"        self.{ident} = Some({arg});")
        lines.append("        self")
        lines.append("    }")

    # extras door
    lines.append("    /// Add a forward-compat field the spec does not yet name.")
    lines.append("    #[must_use]")
    lines.append(
        "    pub fn extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {"
    )
    lines.append("        self.extras.insert(key.into(), value.into());")
    lines.append("        self")
    lines.append("    }")

    # build() -> the wire Value object for the fields (leading args excluded —
    # they are path/id args carried separately by the emitting method).
    lines.append(
        f"    /// Assemble the `{wire_container}` JSON object (unset optionals omitted)."
    )
    lines.append("    #[must_use]")
    lines.append("    pub fn build(self) -> Value {")
    lines.append("        let mut obj = Map::new();")
    for wire, sch, _ in req:
        ident = field_ident(wire)
        ty = rust_field_type(spec, sch)
        conv = (
            (f"Value::from(self.{ident})")
            if ty in ("String", "i64", "f64", "bool")
            else (f"self.{ident}")
        )
        lines.append(f"        obj.insert({rs_str(wire)}.to_string(), {conv});")
    for wire, sch, _ in opt:
        ident = field_ident(wire)
        ty = rust_field_type(spec, sch)
        conv = ("Value::from(v)") if ty in ("String", "i64", "f64", "bool") else "v"
        lines.append(f"        if let Some(v) = self.{ident} {{")
        lines.append(f"            obj.insert({rs_str(wire)}.to_string(), {conv});")
        lines.append("        }")
    lines.append("        for (k, v) in self.extras {")
    lines.append("            obj.insert(k, v);")
    lines.append("        }")
    lines.append("        Value::Object(obj)")
    lines.append("    }")
    # leading-arg accessors (command call_id) — the method needs them out of the struct.
    for ident, ty in leading:
        lines.append(f"    fn take_{ident}(&self) -> {ty} {{ self.{ident}.clone() }}")
    lines.append("}")
    return "\n".join(lines), struct_name


# accumulate request structs to emit once per module (dedup by name).
def emit_operation_method(
    spec: Spec,
    anchor: str,
    markup: dict,
    base: str,
    method_snake: str,
    op_id: str,
    structs: dict[str, str],
) -> str:
    if op_id not in spec.ops:
        raise SystemExit(f"{markup['name']}.{method_snake}: op {op_id!r} not in spec")
    verb, op_path, has_body = spec.ops[op_id]
    id_args, path_expr, _sibling = method_call_path(spec, anchor, markup, op_path)
    name = snake_of(method_snake)
    cls = markup["name"]
    id_params = [f"{a}: &str" for a in id_args]

    lines: list[str] = []
    write_verb = verb in ("post", "put", "patch")
    verb_fn = {"post": "post", "put": "put", "patch": "patch"}.get(verb, verb)

    def _body(expr: str) -> str:
        # `post`/`put`/`patch` all take an OPTIONAL body (`Option<&Value>`)
        # because the reference defaults each to `body=None`. A generated
        # operation always HAS a body to send, so it wraps in `Some(..)`.
        return f"Some({expr})" if verb in ("post", "put", "patch") else expr

    # `post` carries the reference's QUERY-params argument between the body and
    # the options (`_base.py` `post(path, body, params, request_options)`); a
    # generated operation sends its inputs in the body, so it passes `None`.
    # `put`/`patch` have no such argument.
    post_params_fwd = "None, " if verb == "post" else ""

    # Every generated method carries a trailing ``request_options:
    # Option<RequestOptions>`` (plan 4.2 / PY-9), forwarded to the client's
    # ``*_with_options`` variant (transport-only; NEVER serialized into the body).
    ro_param = "request_options: Option<RequestOptions>"
    ro_fwd = "request_options.as_ref()"
    if write_verb and has_body:
        body_schema = spec.op_body.get(op_id) or {}
        if is_object_body(spec, body_schema):
            fields = object_body_fields(spec, body_schema)
            sname = _request_struct_name(cls, name)
            src, _ = emit_request_struct(sname, spec, [], fields, "body")
            structs[sname] = src
            params = [*id_params, f"request: {sname}", ro_param]
            lines.append(
                f"    /// `{verb.upper()} {op_path}` (generated operation method)."
            )
            lines.append("    ///")
            lines.append("    /// # Errors")
            lines.append(
                "    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx"
            )
            lines.append("    /// status, or an unparseable response body.")
            lines.append(
                f"    pub fn {name}(&self, {', '.join(params)}) -> Result<Value, SignalWireRestError> {{"
            )
            lines.append(
                f"        self.client().{verb_fn}_with_options({path_expr}, {_body('&request.build()')}, {post_params_fwd}{ro_fwd})"
            )
            lines.append("    }")
        else:
            # §5.2 union body → a single positional body: Value.
            params = [*id_params, "body: &Value", ro_param]
            lines.append(
                f"    /// `{verb.upper()} {op_path}` (generated operation method; union body)."
            )
            lines.append("    ///")
            lines.append("    /// # Errors")
            lines.append(
                "    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx"
            )
            lines.append("    /// status, or an unparseable response body.")
            lines.append(
                f"    pub fn {name}(&self, {', '.join(params)}) -> Result<Value, SignalWireRestError> {{"
            )
            lines.append(
                f"        self.client().{verb_fn}_with_options({path_expr}, {_body('body')}, {post_params_fwd}{ro_fwd})"
            )
            lines.append("    }")
    elif write_verb:
        params = [*id_params, ro_param]
        lines.append(
            f"    /// `{verb.upper()} {op_path}` (generated operation method; no body)."
        )
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append(
            "    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status."
        )
        lines.append(
            f"    pub fn {name}(&self, {', '.join(params)}) -> Result<Value, SignalWireRestError> {{"
        )
        lines.append(
            f"        self.client().{verb_fn}_with_options({path_expr}, {_body('&Value::Object(Map::new())')}, {post_params_fwd}{ro_fwd})"
        )
        lines.append("    }")
    elif verb == "get":
        # §5.3 GET query door — a trailing params map + request_options.
        params = [*id_params, "params: &HashMap<String, String>", ro_param]
        lines.append(
            f"    /// `GET {op_path}` (generated operation method; query params)."
        )
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append(
            "    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status."
        )
        lines.append(
            f"    pub fn {name}(&self, {', '.join(params)}) -> Result<Value, SignalWireRestError> {{"
        )
        lines.append(
            f"        self.client().get_with_options({path_expr}, Some(params), {ro_fwd})"
        )
        lines.append("    }")
    else:  # delete
        params = [*id_params, ro_param]
        lines.append(f"    /// `DELETE {op_path}` (generated operation method).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append(
            "    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status."
        )
        lines.append(
            f"    pub fn {name}(&self, {', '.join(params)}) -> Result<Value, SignalWireRestError> {{"
        )
        lines.append(
            f"        self.client().delete_with_options({path_expr}, {ro_fwd})"
        )
        lines.append("    }")
    return "\n".join(lines)


def emit_set_method(
    spec: Spec,
    markup: dict,
    sm_name: str,
    sm: dict,
    update_fields: set[str],
    field_schemas: dict[str, dict],
    structs: dict[str, str],
) -> str:
    handler = sm.get("handler")
    if not handler:
        raise SystemExit(f"{markup['name']}.{sm_name}: set_method missing handler")
    cls = markup["name"]
    name = snake_of(sm_name)
    args = sm.get("args") or {}
    # Validate bound fields (fail loud, §7/§9).
    bound: list[tuple[str, str, dict, bool]] = []  # (arg_name, field, schema, required)
    for arg_name, arg in args.items():
        field = arg.get("field")
        if not field:
            raise SystemExit(
                f"{markup['name']}.{sm_name}: arg {arg_name!r} missing field"
            )
        if field not in update_fields:
            raise SystemExit(
                f"{markup['name']}.{sm_name}: arg field {field!r} not in update request schema"
            )
        bound.append(
            (arg_name, field, field_schemas.get(field, {}), bool(arg.get("required")))
        )

    # Emit a request struct keyed by arg-name (bound to update field on build).
    sname = _request_struct_name(cls, name)
    # Build synthetic field list using arg names as wire keys mapped to update fields.
    req = [(a, s, True) for (a, f, s, r) in bound if r]
    opt = [(a, s, False) for (a, f, s, r) in bound if not r]
    field_map = {a: f for (a, f, s, r) in bound}

    lines: list[str] = []
    # struct with arg-named fields; build() maps to update-field wire keys + handler.
    slines: list[str] = []
    slines.append(
        "/// Named request parameters for a generated `set_*` wrapper (binds args to"
    )
    slines.append(
        "/// update-request fields + a fixed `call_handler`; Rust options-builder idiom)."
    )
    slines.append("#[derive(Debug, Clone, Default)]")
    slines.append(f"pub struct {sname} {{")
    for a, s, _ in req:
        slines.append(f"    {field_ident(a)}: {rust_field_type(spec, s)},")
    for a, s, _ in opt:
        slines.append(f"    {field_ident(a)}: Option<{rust_field_type(spec, s)}>,")
    slines.append("    extras: Map<String, Value>,")
    slines.append("}")
    slines.append("")
    slines.append(f"impl {sname} {{")
    new_params = []
    for a, s, _ in req:
        ty = rust_field_type(spec, s)
        new_params.append(
            f"{field_ident(a)}: impl Into<{ty}>"
            if ty == "String"
            else f"{field_ident(a)}: {ty}"
        )
    if len(new_params) > 7:
        slines.append("    #[allow(clippy::too_many_arguments)]")
    slines.append(f"    pub fn new({', '.join(new_params)}) -> Self {{")
    slines.append(f"        {sname} {{")
    for a, s, _ in req:
        ty = rust_field_type(spec, s)
        ident = field_ident(a)
        slines.append(
            f"            {ident}: {ident}.into(),"
            if ty == "String"
            else f"            {ident},"
        )
    slines.append("            ..Default::default()")
    slines.append("        }")
    slines.append("    }")
    for a, s, _ in opt:
        ident = field_ident(a)
        ty = rust_field_type(spec, s)
        setter = setter_ident(a)
        slines.append("    #[must_use]")
        if ty == "String":
            slines.append(
                f"    pub fn {setter}(mut self, value: impl Into<{ty}>) -> Self {{"
            )
            slines.append(f"        self.{ident} = Some(value.into());")
        else:
            slines.append(f"    pub fn {setter}(mut self, value: {ty}) -> Self {{")
            slines.append(f"        self.{ident} = Some(value);")
        slines.append("        self")
        slines.append("    }")
    slines.append("    #[must_use]")
    slines.append(
        "    pub fn extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {"
    )
    slines.append("        self.extras.insert(key.into(), value.into());")
    slines.append("        self")
    slines.append("    }")
    slines.append("    #[must_use]")
    slines.append("    pub fn build(self) -> Value {")
    slines.append("        let mut obj = Map::new();")
    slines.append(
        f'        obj.insert("call_handler".to_string(), Value::from({rs_str(handler)}));'
    )
    for a, s, _ in req:
        ident = field_ident(a)
        ty = rust_field_type(spec, s)
        conv = (
            (f"Value::from(self.{ident})")
            if ty in ("String", "i64", "f64", "bool")
            else (f"self.{ident}")
        )
        slines.append(
            f"        obj.insert({rs_str(field_map[a])}.to_string(), {conv});"
        )
    for a, s, _ in opt:
        ident = field_ident(a)
        ty = rust_field_type(spec, s)
        conv = ("Value::from(v)") if ty in ("String", "i64", "f64", "bool") else "v"
        slines.append(f"        if let Some(v) = self.{ident} {{")
        slines.append(
            f"            obj.insert({rs_str(field_map[a])}.to_string(), {conv});"
        )
        slines.append("        }")
    slines.append("        for (k, v) in self.extras { obj.insert(k, v); }")
    slines.append("        Value::Object(obj)")
    slines.append("    }")
    slines.append("}")
    structs[sname] = "\n".join(slines)

    lines.append(
        f"    /// `set_{sm_name}` — update wrapper binding a fixed `call_handler` (§7)."
    )
    lines.append("    ///")
    lines.append("    /// # Errors")
    lines.append(
        "    /// Returns [`SignalWireRestError`] on transport failure or a non-2xx status."
    )
    lines.append(
        f"    pub fn {name}(&self, resource_id: &str, request: {sname}, request_options: Option<RequestOptions>) -> Result<Value, SignalWireRestError> {{"
    )
    lines.append("        self.update(resource_id, &request.build(), request_options)")
    lines.append("    }")
    return "\n".join(lines)


def emit_command_dispatch(
    spec: Spec, anchor: str, markup: dict, structs: dict[str, str]
) -> str:
    name = markup["name"]
    request = markup.get("request")
    if not request:
        raise SystemExit(f"{name}: command-dispatch requires request")
    mapping = discriminator_mapping(spec, request)
    commands = list(mapping.keys())
    op = spec.ops.get("call-commands")
    if op:
        base = join_path(spec.server_path, op[1].lstrip("/"))
    else:
        base = join_path(spec.server_path, anchor.lstrip("/"))

    lines: list[str] = []
    lines.append(
        f"/// `{name}` — command-dispatch resource ({spec.name} spec). Each method POSTs"
    )
    lines.append(f"/// `{{command, params, id?}}` to `{base}`.")
    lines.append(f"pub struct {name}<'a> {{")
    lines.append("    client: &'a HttpClient,")
    lines.append("}")
    lines.append("")
    lines.append(f"impl<'a> {name}<'a> {{")
    lines.append(f"    const BASE_PATH: &'static str = {rs_str(base)};")
    lines.append("")
    lines.append("    #[must_use]")
    lines.append("    pub fn new(client: &'a HttpClient) -> Self {")
    lines.append(f"        {name} {{ client }}")
    lines.append("    }")
    lines.append("")
    lines.append("    #[must_use]")
    lines.append("    pub fn base_path(&self) -> &str {")
    lines.append("        Self::BASE_PATH")
    lines.append("    }")
    lines.append("")
    lines.append(
        "    fn execute(&self, command: &str, call_id: Option<&str>, params: Value,"
    )
    lines.append("        request_options: Option<RequestOptions>)")
    lines.append("        -> Result<Value, SignalWireRestError> {")
    lines.append("        let mut body = Map::new();")
    lines.append('        body.insert("command".to_string(), Value::from(command));')
    lines.append('        body.insert("params".to_string(), params);')
    lines.append("        if let Some(id) = call_id {")
    lines.append('            body.insert("id".to_string(), Value::from(id));')
    lines.append("        }")
    lines.append(
        "        // request_options is transport-only — forwarded to the HTTP layer, never"
    )
    lines.append("        // serialized into the command body.")
    lines.append(
        "        self.client.post_with_options(Self::BASE_PATH, Some(&Value::Object(body)), None, request_options.as_ref())"
    )
    lines.append("    }")

    for cmd in commands:
        mname = command_method_name(cmd)
        cmd_leaf = mapping[cmd].rsplit("/", 1)[-1] if mapping.get(cmd) else ""
        cmd_schema = spec.schemas.get(cmd_leaf, {})
        fields, with_id = command_param_fields(spec, cmd_schema)
        sname = _request_struct_name(name, mname)
        leading: list[
            tuple[str, str]
        ] = []  # call_id handled as a method arg, not struct field
        src, _ = emit_request_struct(sname, spec, leading, fields, "params")
        structs[sname] = src
        id_param = "call_id: &str, " if with_id else ""
        call_arg = "Some(call_id)" if with_id else "None"
        lines.append("")
        lines.append(f"    /// `{cmd}` — generated command method.")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append(
            "    /// Returns [`SignalWireRestError`] on transport failure or a non-2xx status."
        )
        lines.append(
            f"    pub fn {mname}(&self, {id_param}request: {sname}, request_options: Option<RequestOptions>) -> Result<Value, SignalWireRestError> {{"
        )
        lines.append(
            f"        self.execute({rs_str(cmd)}, {call_arg}, request.build(), request_options)"
        )
        lines.append("    }")
    lines.append("}")
    return "\n".join(lines)


def emit_resource(
    spec: Spec, anchor: str, markup: dict, structs: dict[str, str]
) -> str:
    name = markup["name"]
    base = markup["base"]
    if markup.get("kind") == "command-dispatch":
        return emit_command_dispatch(spec, anchor, markup, structs)
    if base not in BASE_PROVIDES:
        raise SystemExit(f"{name}: unknown base {base!r}")

    # §9: write-capable bases require update_method matching the spec verb.
    if base in ("CrudResource", "FabricResource"):
        upd = markup.get("update_method")
        if not upd:
            raise SystemExit(f"{name}: {base} requires update_method")
        item = spec.doc["paths"][anchor]
        spec_verb = (
            "PUT" if item.get("put") else ("PATCH" if item.get("patch") else None)
        )
        if spec_verb and upd != spec_verb:
            raise SystemExit(
                f"{name}: update_method {upd} != spec update verb {spec_verb}"
            )

    bp = base_path(spec, anchor, markup)
    upd = markup.get("update_method", "PATCH")

    lines: list[str] = []
    lines.append(f"/// `{name}` resource for the SignalWire `{spec.name}` REST API.")
    lines.append("///")
    lines.append(f"/// Composes [`{base}`] (its base path is baked in) and adds the")
    lines.append("/// resource's own methods.")
    lines.append(f"pub struct {name}<'a> {{")
    lines.append(f"    base: {base}<'a>,")
    lines.append("}")
    lines.append("")
    lines.append(f"impl<'a> {name}<'a> {{")
    lines.append("    /// Construct the resource; its base path (§4) is baked in.")
    lines.append("    #[must_use]")
    lines.append("    pub fn new(client: &'a HttpClient) -> Self {")
    if base in ("CrudResource", "FabricResource"):
        lines.append(
            f"        {name} {{ base: {base}::new(client, {rs_str(bp)}, {rs_str(upd)}) }}"
        )
    else:
        lines.append(f"        {name} {{ base: {base}::new(client, {rs_str(bp)}) }}")
    lines.append("    }")
    lines.append("")
    # `base_path` is part of the resource surface (the oracle records it), so it is
    # always emitted. The private `client`/`path` helpers exist only for declared
    # operation methods; a pure-CRUD resource (all methods delegated to the base)
    # never references them, so they are emitted below ONLY if the body uses them.
    lines.append("    #[must_use]")
    lines.append("    pub fn base_path(&self) -> &str {")
    lines.append("        self.base.base_path()")
    lines.append("    }")
    # marker replaced after the body is built with whichever helpers it references
    helper_marker = "@@GEN_PRIVATE_HELPERS@@"
    lines.append(helper_marker)

    provided = BASE_PROVIDES[base]
    declared = markup.get("methods") or {}

    # A resource may re-declare `list_addresses` with a SIBLING path (fabric
    # singular resources: /resources/call_flow/{id}/addresses, sibling to the
    # /resources/call_flows collection). In Python/php the declared override
    # SHADOWS the base method (same name). Rust inherent methods can't be
    # overloaded, so when the resource emits its own `list_addresses` override we
    # must NOT also emit the base-delegation `list_addresses` (they'd collide).
    override_list_addresses = False
    if "list_addresses" in provided and "list_addresses" in declared:
        la_op = (declared["list_addresses"] or {}).get("op")
        if la_op and la_op in spec.ops:
            _, la_path, _ = spec.ops[la_op]
            _, la_sibling = relative_tail(spec, anchor, markup, la_path)
            override_list_addresses = la_sibling

    # Re-expose base CRUD/read methods by delegation (so the resource surface
    # carries them, matching the oracle which records them on the resource).
    # Each carries the trailing keyword-only ``request_options`` (plan 4.2 /
    # PY-9), forwarded to the base's ``*_with_options`` variant (transport-only —
    # never serialized into the body). The Rust ``Option<RequestOptions>`` is the
    # named idiom for the reference's ``*, request_options=None``.
    if "list" in provided:
        lines.append("")
        lines.append("    /// `list` (delegated to the base; GET base path).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append(
            "    pub fn list(&self, params: &HashMap<String, String>, request_options: Option<RequestOptions>) -> Result<Value, SignalWireRestError> {"
        )
        lines.append(
            "        self.base.list_with_options(params, request_options.as_ref())"
        )
        lines.append("    }")
    if "get" in provided:
        lines.append("")
        lines.append("    /// `get` (delegated to the base; GET base/{id}).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append(
            "    pub fn get(&self, id: &str, request_options: Option<RequestOptions>) -> Result<Value, SignalWireRestError> {"
        )
        lines.append("        self.base.get_with_options(id, request_options.as_ref())")
        lines.append("    }")
    if "paginate" in provided:
        lines.append("")
        lines.append(
            "    /// `paginate` (delegated to the base): iterate every item across all"
        )
        lines.append("    /// pages, following the response's `links.next` cursor.")
        lines.append("    #[must_use]")
        lines.append(
            "    pub fn paginate(&self, request_options: Option<RequestOptions>, params: &HashMap<String, String>) -> PaginatedIterator<'a> {"
        )
        lines.append("        self.base.paginate(request_options, params)")
        lines.append("    }")
    if "create" in provided:
        lines.append("")
        lines.append("    /// `create` (delegated to the base; POST base path).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append(
            "    pub fn create(&self, data: &Value, request_options: Option<RequestOptions>) -> Result<Value, SignalWireRestError> {"
        )
        lines.append(
            "        self.base.create_with_options(data, request_options.as_ref())"
        )
        lines.append("    }")
    if "update" in provided:
        lines.append("")
        lines.append("    /// `update` (delegated to the base; PUT/PATCH base/{id}).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append(
            "    pub fn update(&self, id: &str, data: &Value, request_options: Option<RequestOptions>) -> Result<Value, SignalWireRestError> {"
        )
        lines.append(
            "        self.base.update_with_options(id, data, request_options.as_ref())"
        )
        lines.append("    }")
    if "delete" in provided:
        lines.append("")
        lines.append("    /// `delete` (delegated to the base; DELETE base/{id}).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append(
            "    pub fn delete(&self, id: &str, request_options: Option<RequestOptions>) -> Result<Value, SignalWireRestError> {"
        )
        lines.append(
            "        self.base.delete_with_options(id, request_options.as_ref())"
        )
        lines.append("    }")
    if "list_addresses" in provided and not override_list_addresses:
        lines.append("")
        lines.append(
            "    /// `list_addresses` (delegated to the Fabric base; GET base/{id}/addresses)."
        )
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append(
            "    pub fn list_addresses(&self, id: &str, params: &HashMap<String, String>, request_options: Option<RequestOptions>) -> Result<Value, SignalWireRestError> {"
        )
        lines.append(
            "        self.base.list_addresses_with_options(id, params, request_options.as_ref())"
        )
        lines.append("    }")

    for method_snake, spec_ref in declared.items():
        op_id = spec_ref.get("op")
        if not op_id:
            raise SystemExit(f"{name}.{method_snake}: method markup missing op")
        if method_snake in provided:
            if method_snake == "list_addresses":
                _verb, op_path, _ = spec.ops[op_id]
                _, sibling = relative_tail(spec, anchor, markup, op_path)
                if not sibling:
                    continue
                # sibling override — fall through and emit
            else:
                continue
        lines.append("")
        lines.append(
            emit_operation_method(
                spec, anchor, markup, base, method_snake, op_id, structs
            )
        )

    set_methods = markup.get("set_methods") or {}
    if set_methods:
        if base not in ("CrudResource", "FabricResource"):
            raise SystemExit(f"{name}: set_methods require a CRUD base, got {base}")
        upd_fields, upd_field_schemas = update_request_fields(spec, anchor, markup)
        for sm_name, sm in set_methods.items():
            lines.append("")
            lines.append(
                emit_set_method(
                    spec, markup, sm_name, sm, upd_fields, upd_field_schemas, structs
                )
            )

    lines.append("}")
    src = "\n".join(lines)

    # Emit the private client()/path() helpers only if a declared method uses them
    # (a pure-CRUD resource delegates everything to the base and references neither,
    # so emitting them would trip dead_code).
    body_after_marker = src.split(helper_marker, 1)[1]
    helpers: list[str] = []
    if re.search(r"\bself\.client\(\)", body_after_marker):
        helpers += [
            "",
            "    fn client(&self) -> &HttpClient {",
            "        self.base.client()",
            "    }",
        ]
    if re.search(r"\bself\.path\(", body_after_marker):
        helpers += [
            "",
            "    fn path(&self, parts: &[&str]) -> String {",
            "        self.base.path(parts)",
            "    }",
        ]
    return src.replace(helper_marker, "\n".join(helpers))


# ---------------------------------------------------------------------------
# Client tree (§8).
# ---------------------------------------------------------------------------

CONTAINERS = {
    "fabric": ("FabricNamespace", "fabric"),
    "video": ("VideoNamespace", "video"),
    "logs": ("LogsNamespace", "logs"),
    "registry": ("RegistryNamespace", "registry"),
    "project": ("ProjectNamespace", "project"),
    "datasphere": ("DatasphereNamespace", "datasphere"),
}

ATTR_OVERRIDE = {
    "GenericResources": "resources",
    "FabricAddresses": "addresses",
    "FabricTokens": "tokens",
    "DatasphereDocuments": "documents",
    "ProjectTokens": "tokens",
    "PubSub": "pubsub",
    "MessageLogs": "messages",
    "VoiceLogs": "voice",
    "FaxLogs": "fax",
    "ConferenceLogs": "conferences",
}


def container_accessor(markup: dict, name: str, container: str) -> str:
    if markup.get("attr"):
        return snake_of(markup["attr"])
    if name in ATTR_OVERRIDE:
        return snake_of(ATTR_OVERRIDE[name])
    lead = container[:1].upper() + container[1:]
    stem = name[len(lead) :] if name.startswith(lead) else name
    return _pascal_to_snake(stem) if stem else _pascal_to_snake(name)


def _pascal_to_snake(s: str) -> str:
    return re.sub(r"(?<!^)(?=[A-Z])", "_", s).lower()


def flat_accessor(name: str) -> str:
    if name in ATTR_OVERRIDE:
        return snake_of(ATTR_OVERRIDE[name])
    return _pascal_to_snake(name)


def resolve_placement(specs):
    placed = []
    for spec in specs:
        for anchor, markup in spec.resources():
            container = markup.get("namespace") or spec.namespace_attr or ""
            placed.append((spec, anchor, markup, container))
    return placed


# Which generated module a resource struct lives in (for the client-tree `use`).
# The leaf is the spec dir with '-' -> '_' (relay-rest -> relay_rest), derived —
# not tabulated.
def _res_module(spec: Spec) -> str:
    return f"{snake_of(spec.name)}_resources_generated"


def emit_client_tree(placed) -> str:
    """Emit the generated client-tree: one container struct per namespace group +
    a `GeneratedResourceTree` the hand RestClient composes (lazy accessor per flat
    resource + per container). Base paths per §4, placement per §8."""
    flats = []  # (accessor, struct, module)
    containers: dict[str, list[tuple[str, str, str]]] = {}
    corder: list[str] = []
    for spec, _anchor, markup, container in placed:
        name = markup["name"]
        module = _res_module(spec)
        if not container:
            flats.append((flat_accessor(name), name, module))
        else:
            if container not in containers:
                containers[container] = []
                corder.append(container)
            acc = container_accessor(markup, name, container)
            containers[container].append((acc, name, module))

    lines: list[str] = []
    lines.append("// Code generated by scripts/generate_rest.py; DO NOT EDIT.")
    lines.append("//")
    lines.append("//")
    lines.append("")
    lines.append("use crate::rest::http_client::HttpClient;")
    # imports
    imports: dict[str, set[str]] = {}
    for _, struct, module in flats:
        imports.setdefault(module, set()).add(struct)
    for c in corder:
        for _, struct, module in containers[c]:
            imports.setdefault(module, set()).add(struct)
    for module in sorted(imports):
        names = ", ".join(sorted(imports[module]))
        lines.append(f"use super::{module}::{{{names}}};")
    lines.append("")

    # container structs
    for c in corder:
        clsname, acc = CONTAINERS[c]
        members = containers[c]
        lines.append(
            f"/// `{clsname}` — generated container grouping the {c} namespace resources (§8)."
        )
        lines.append(f"pub struct {clsname}<'a> {{")
        lines.append("    client: &'a HttpClient,")
        lines.append("}")
        lines.append("")
        lines.append(f"impl<'a> {clsname}<'a> {{")
        lines.append("    #[must_use]")
        lines.append("    pub fn new(client: &'a HttpClient) -> Self {")
        lines.append(f"        {clsname} {{ client }}")
        lines.append("    }")
        for accessor, struct, _ in members:
            lines.append("")
            lines.append(f"    /// Access the `{struct}` resource.")
            lines.append("    #[must_use]")
            lines.append(f"    pub fn {accessor}(&self) -> {struct}<'a> {{")
            lines.append(f"        {struct}::new(self.client)")
            lines.append("    }")
        lines.append("}")
        lines.append("")

    # the resource tree
    lines.append(
        "/// `GeneratedResourceTree` — generated lazy accessors for every flat REST"
    )
    lines.append(
        "/// resource plus the namespace containers (§8). The hand `RestClient` composes"
    )
    lines.append(
        "/// this; each accessor constructs the resource with the client's `HttpClient`"
    )
    lines.append("/// (base paths baked in per §4).")
    lines.append("pub struct GeneratedResourceTree<'a> {")
    lines.append("    client: &'a HttpClient,")
    lines.append("}")
    lines.append("")
    lines.append("impl<'a> GeneratedResourceTree<'a> {")
    lines.append("    #[must_use]")
    lines.append("    pub fn new(client: &'a HttpClient) -> Self {")
    lines.append("        GeneratedResourceTree { client }")
    lines.append("    }")
    for accessor, struct, _ in flats:
        lines.append("")
        lines.append(f"    /// Access the flat `{struct}` resource.")
        lines.append("    #[must_use]")
        lines.append(f"    pub fn {accessor}(&self) -> {struct}<'a> {{")
        lines.append(f"        {struct}::new(self.client)")
        lines.append("    }")
    for c in corder:
        clsname, acc = CONTAINERS[c]
        lines.append("")
        lines.append(f"    /// Access the `{c}` namespace container.")
        lines.append("    #[must_use]")
        lines.append(f"    pub fn {acc}(&self) -> {clsname}<'a> {{")
        lines.append(f"        {clsname}::new(self.client)")
        lines.append("    }")
    lines.append("}")
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Generated TYPE surface (SESSION_CHANGESET §D/§H/§I) — the READ-side wire types.
#
# Shared by the REST ``<ns>_types_generated`` emitter (below) AND the sibling
# generators (generate_swml_verbs.py / generate_relay_protocol.py /
# generate_swaig_payloads.py) so they never diverge. RUST IDIOM: one method-less
# ``#[derive(Serialize, Deserialize)]`` struct per OBJECT schema; every field is
# ``Option<T>`` with ``#[serde(rename)]`` on the snake wire key and
# ``skip_serializing_if`` (omit-unset, matching the reference). Scalars stay
# distinct (String/i64/f64/bool); array/object/$ref/union → ``serde_json::Value``
# (the open-JSON idiom — a read-side payload is forward-compatible). An
# ``x-sdk-enum`` public enum → a Rust ``enum`` with ``#[serde(rename)]`` variants
# (the closed-set idiom). Scalar/array/union/oneOf/anyOf/allOf ALIASES are NOT
# surfaced (the reference's enumerator drops module-level scalar TypeAlias /
# inline union) — emit nothing, matching the oracle surface EXACTLY (0/0).
#
# A method-less struct records the bare class name on the SURFACE (fields are not
# methods in Rust) and is DROPPED by the signature enumerator (no impl methods) —
# matching the reference, whose SIGNATURE oracle records these method-less
# (relay-protocol / swaig-actions / all REST wire types are NOT in it). The three
# read-side payload modules the reference DOES record with per-class-typed-field
# zero-arg accessors (swml_verbs / post_prompt / swaig_request) are handled via
# the gen-payload SIGNATURE SIDECAR (build_gen_payload_sidecar) the sig enumerator
# unfolds — Rust structs carry no accessor methods, so we synthesize the accessor
# members there (each ``any``-return, self-only → excused / class-typed-folded by
# the diff tool's gen-payload rules).
# ---------------------------------------------------------------------------

# The TYPE namespace set (former hardcoded TYPE_NS) is discovered by
# discover_type_ns(psdk): every RESOURCE namespace PLUS the types-only specs
# (components.schemas but no servers block — e.g. swml-webhooks). See the
# discovery block near the top of this module.

TYPES_HEADER = """// Code generated by scripts/{gen}; DO NOT EDIT.
//
// {desc}
//
// Read-side wire types (open shapes) — method-less serde structs / closed-set
// enums. Regenerate with: python3 scripts/{gen}
//
// Two narrow lint allows, both grounded in the generated wire shape:
//   * non_camel_case_types — a few wire schema keys carry dotted names
//     (``Types.StatusCodes.StatusCode400``); the type identifier folds the dots
//     to underscores (``Types_StatusCodes_StatusCode400``) and must stay verbatim
//     so it matches the wire schema key, which the naming lint would rewrite.
//   * clippy::doc_markdown — the generated doc comments echo raw wire schema key
//     names in prose; backticking every one mechanically is not meaningful here.
#![allow(non_camel_case_types, clippy::doc_markdown)]

use serde::{{Deserialize, Serialize}};
"""


def type_name(raw: str) -> str:
    """Sanitise a components/schemas key to a valid Rust type identifier, folding
    every non-identifier rune to ``_`` — matching the go/ts/php/ruby/python
    ref_name so the LEAF the surface diff compares is the identical token across
    ports (``Types.StatusCodes.StatusCode400`` -> ``Types_StatusCodes_StatusCode400``).
    Rust type names must start with a letter/underscore; every wire schema name
    already begins with an uppercase letter."""
    s = re.sub(r"[^A-Za-z0-9_]", "_", raw).lstrip("_")
    if not s:
        return "Schema"
    if s[0].isdigit():
        return "Schema_" + s
    if not s[0].isupper():
        s = s[0].upper() + s[1:]
    return s


def _type_schema_type(node: dict):
    t = node.get("type")
    if isinstance(t, list):
        return next((x for x in t if x != "null"), None)
    return t


def is_object_schema(node: dict) -> bool:
    """Mirror the reference is_object test: type:object (or no type but non-empty
    properties) AND not a oneOf/anyOf/allOf combinator AND properties non-empty."""
    if any(k in node for k in ("oneOf", "anyOf", "allOf")):
        return False
    props = node.get("properties")
    t = _type_schema_type(node)
    return (
        (t == "object" or (t is None and props))
        and isinstance(props, dict)
        and len(props) > 0
    )


def _wire_owned_type(psc: dict) -> str:
    """The Rust owned field type for a read-side wire field. Scalars stay distinct
    (String/i64/f64/bool); array/object/$ref/union/enum → ``serde_json::Value``
    (open JSON — forward-compatible read payload)."""
    if not isinstance(psc, dict):
        return "serde_json::Value"
    if any(k in psc for k in ("$ref", "allOf", "oneOf", "anyOf")):
        return "serde_json::Value"
    t = _type_schema_type(psc)
    if t == "string" and not psc.get("enum"):
        return "String"
    if t == "integer":
        return "i64"
    if t == "number":
        return "f64"
    if t == "boolean":
        return "bool"
    return "serde_json::Value"


def _struct_field_ident(wire: str, used: set) -> str:
    """A unique idiomatic snake_case Rust struct-field identifier for a wire key
    (camelCase/PascalCase wire keys → snake_case, so the generated source is
    non_snake_case-clean; the true wire key is preserved on ``#[serde(rename)]``).
    A Rust keyword → raw identifier ``r#kw``. Dedup on collision."""
    ident = snake(wire)
    # snake() lower-cases; a keyword result is escaped as a raw identifier.
    if ident in RUST_KEYWORDS:
        ident = ident + "_" if ident in RUST_NO_RAW else "r#" + ident
    if not ident or ident[0].isdigit():
        ident = "_" + ident
    base = ident
    n = 2
    while ident in used:
        ident = base + "_" + str(n)
        n += 1
    used.add(ident)
    return ident


def emit_methodless_struct(
    rs_name: str,
    properties: dict,
    source_desc: str,
    gen: str,
    schema_name: str | None = None,
) -> str:
    """Emit one method-less serde struct for an OBJECT schema (shared by the REST
    wire-type emitter and the swml-verbs / relay-protocol / swaig payload
    generators so they never diverge). Every field is ``Option<T>`` with a
    ``#[serde(rename)]`` snake wire key + ``skip_serializing_if``. No ``impl`` —
    the surface records the bare struct name; the signature enumerator drops it
    (method-less).

    ``schema_name`` is the field's containing SPEC schema name (the $defs /
    components.schemas key) — passed to the x-sdk-overlay check so hidden fields are
    dropped from the SDK surface (still on the wire) and deprecated fields carry a
    ``#[deprecated]`` marker. It is the SPEC name, NOT ``rs_name`` (the emitted type)."""
    lines: list[str] = []
    lines.append(f"/// `{rs_name}` — generated read-side wire type ({source_desc}).")
    lines.append("///")
    lines.append("/// Method-less serde DTO: each field maps a snake wire key (via")
    lines.append(
        "/// `#[serde(rename)]`) to its owned Rust type; unset fields are omitted."
    )
    lines.append("#[derive(Debug, Clone, Default, Serialize, Deserialize)]")
    lines.append(f"pub struct {rs_name} {{")
    used: set = set()
    for wire_key, psc in properties.items():
        # SDK-surface policy comes from the single overlay (rest-apis/x-sdk-overlay.yaml),
        # matched by (wire_key, SPEC schema name) — NOT the emitted Rust type name.
        if overlay_hidden(wire_key, schema_name):
            # hidden: drop from the SDK surface entirely (still on the wire).
            continue
        ty = _wire_owned_type(psc if isinstance(psc, dict) else {})
        ident = _struct_field_ident(wire_key, used)
        # serde's default field name is the ident text (minus a raw ``r#`` prefix).
        # Emit an explicit ``rename`` whenever the wire key differs so the wire
        # contract is preserved verbatim regardless of the snake_case ident.
        serde_ident = ident[2:] if ident.startswith("r#") else ident
        attrs = ["default", 'skip_serializing_if = "Option::is_none"']
        if serde_ident != wire_key:
            attrs.insert(0, f"rename = {rs_str(wire_key)}")
        lines.append(f"    #[serde({', '.join(attrs)})]")
        if overlay_deprecated(wire_key, schema_name):
            # deprecated: still emitted (back-compat), flagged for tooling + docs.
            lines.append(
                f'    #[deprecated(note = "{wire_key}: deprecated per x-sdk-overlay")]'
            )
        lines.append(f"    pub {ident}: Option<{ty}>,")
    lines.append("}")
    return "\n".join(lines)


def _enum_variant_name(value: str, used: set) -> str:
    """PascalCase variant name for a closed-set wire value (deduped)."""
    parts = re.split(r"[^A-Za-z0-9]+", value)
    name = "".join(w[:1].upper() + w[1:] for w in parts if w)
    if not name:
        name = "Value"
    if name[0].isdigit():
        name = "V" + name
    base = name
    n = 2
    while name in used:
        name = base + str(n)
        n += 1
    used.add(name)
    return name


def emit_type_enum(rs_name: str, values: list, source_desc: str, gen: str) -> str:
    """Emit a Rust ``enum`` (closed set) for an x-sdk-enum public enum: one
    ``#[serde(rename)]`` variant per wire value. Method-less → records the bare
    type name on the surface, dropped by the signature enumerator (matching the
    reference, which records the public enum method-less)."""
    lines: list[str] = []
    lines.append(f"/// `{rs_name}` — generated public closed-set ({source_desc}).")
    lines.append("///")
    lines.append(
        "/// Each variant serialises to its wire string via `#[serde(rename)]`."
    )
    lines.append("#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]")
    lines.append(f"pub enum {rs_name} {{")
    used: set = set()
    for v in values:
        if not isinstance(v, str) or v == "":
            continue
        vname = _enum_variant_name(v, used)
        lines.append(f"    #[serde(rename = {rs_str(v)})]")
        lines.append(f"    {vname},")
    lines.append("}")
    return "\n".join(lines)


def _load_types_schemas(psdk: Path, spec_dir: str) -> dict:
    """Load a spec's components/schemas WITHOUT the full Spec model (swml-webhooks
    has no servers block, so Spec() would reject it). Ordered by yaml declaration."""
    doc = yaml.safe_load((psdk / "rest-apis" / spec_dir / "openapi.yaml").read_text())
    return ((doc.get("components") or {}).get("schemas")) or {}


def build_type_module(psdk: Path, spec_dir: str, ns_key: str) -> str:
    """Emit the whole ``<ns>_types_generated.rs`` module: one method-less struct per
    object schema + one enum per x-sdk-enum, in yaml declaration order. First-seen
    wins within the namespace (names are unique inside one spec's schemas)."""
    schemas = _load_types_schemas(psdk, spec_dir)
    blocks: list[str] = []
    emitted: set = set()
    for raw_name, node in schemas.items():
        if not isinstance(node, dict):
            continue
        xe = node.get("x-sdk-enum")
        if xe:
            rs_name = type_name(xe)
            if rs_name in emitted:
                continue
            emitted.add(rs_name)
            blocks.append(
                emit_type_enum(
                    rs_name,
                    list(node.get("enum") or []),
                    f"{spec_dir!r} REST API, schema {raw_name!r}",
                    "generate_rest.py",
                )
            )
            continue
        if is_object_schema(node):
            rs_name = type_name(raw_name)
            if rs_name in emitted:
                continue
            emitted.add(rs_name)
            blocks.append(
                emit_methodless_struct(
                    rs_name,
                    node.get("properties") or {},
                    f"{spec_dir!r} REST API, schema {raw_name!r}",
                    "generate_rest.py",
                    schema_name=raw_name,
                )
            )
    desc = (
        f"Generated REST wire types for the {ns_key!r} namespace (components/schemas)."
    )
    src = TYPES_HEADER.format(gen="generate_rest.py", desc=desc) + "\n"
    for b in blocks:
        src += "\n" + b + "\n"
    return src


def emit_types(psdk: Path, outs: dict) -> None:
    """Emit every ``types/<ns>_types_generated.rs`` module into ``outs`` (keys
    relative to the generated dir)."""
    for spec_dir, ns_key in discover_type_ns(psdk):
        outs[f"types/{ns_key}_types_generated.rs"] = build_type_module(
            psdk, spec_dir, ns_key
        )


# ---------------------------------------------------------------------------
# Gen-payload SIGNATURE sidecar (§D3, read side). The three read-side payload
# modules the reference records WITH zero-arg accessors per class-typed field
# (swml_verbs / post_prompt / swaig_request) have no accessor methods on the Rust
# struct, so the sibling generators contribute their per-class field lists here
# and enumerate_signatures.py synthesizes an ``any``-return, self-only accessor
# per field — routed to the oracle module. The diff tool's gen-payload fold +
# _is_port_state_accessor excuse make these compare EQUAL to the reference
# (class-typed fields fold; scalar fields excuse as port-side state). Written by
# each generator via GenPayloadSidecar.add + flushed to the JSON alongside the .rs.
# ---------------------------------------------------------------------------


def gen_payload_accessors(
    properties: dict, schema_name: str | None = None
) -> list[str]:
    """The accessor member names for a read-side payload struct: the wire field
    identifier per property (deduped, keyword→r#kw stripped to a bare method name).
    Matches the reference's recorded accessor names (wire field verbatim where a
    valid ident; the reference records ``SWAIG`` as ``SWAIG``).

    Overlay-hidden fields are dropped (their struct field is not emitted, so no
    accessor is synthesized), matched by (wire_key, SPEC schema name)."""
    out: list[str] = []
    used: set = set()
    for wire_key in properties:
        if overlay_hidden(wire_key, schema_name):
            continue
        # Accessor NAME = wire key folded to a valid ident (NOT r#-escaped: an
        # accessor is a synthesized symbol name in the sidecar, and the oracle
        # records the wire field name verbatim, e.g. ``SWAIG``). Fold only
        # non-idents; keep case (SWAIG stays SWAIG).
        m = re.sub(r"[^A-Za-z0-9_]", "_", wire_key)
        if not m:
            m = "field"
        if m[0].isdigit():
            m = "_" + m
        while m in used:
            m += "_"
        used.add(m)
        out.append(m)
    return out


# ---------------------------------------------------------------------------
# Signature sidecar (L10) — the adapter-consumed model of each generated
# method's EXPLODED named params + kinds, so enumerate_signatures.py can
# reclassify the single ``request: XRequest`` rustdoc param back into the
# reference's keyword/positional/var_keyword shape (drift compares count+kind).
#
# Also carries, per resource, the SURFACE drop-set — the base-delegated methods
# the runtime keeps (behavioral completeness) but the Python oracle does NOT
# record on the class (silently-inherited, never re-declared). The surface +
# signature adapters subtract these so the projection lands 1:1 on the oracle.
# ---------------------------------------------------------------------------


# The oracle's per-resource method set = declared methods + set_methods
# + (create, update) for a CRUD/Fabric base. Every OTHER base-provided method
# (list/get/delete[/list_addresses]) is inherited-not-redeclared → dropped from
# the surface. ``base_path`` is a Rust accessor with no Python analogue → always
# dropped.
def surface_drop_set(
    base: str, declared: list[str], set_methods: list[str]
) -> set[str]:
    prov = BASE_PROVIDES.get(base, set())
    keep = set(declared) | set(set_methods)
    if base in ("CrudResource", "FabricResource"):
        keep |= {"create", "update"}
    drop = prov - keep
    drop.add("base_path")
    return drop


def _param(name: str, kind: str, required: bool, ptype: str = "any") -> dict:
    # ``ptype`` is the canonical type the adapter emits. Path-id / string args are
    # genuinely ``&str`` in Rust → "string"; body/command keyword fields carry the
    # open "any" (the drift gate compares count+kind on those — L10). A loose
    # CRUD body (create/update ``data``) is also "any".
    return {"name": name, "kind": kind, "required": required, "type": ptype}


# The per-request options envelope (plan 4.2 / tracker PY-9). The Python
# reference records a trailing keyword-only ``request_options: RequestOptions |
# None = None`` on EVERY generated REST method (operation/command/set/paginate +
# the CRUD create/update overrides); the enumerator drops the ``**params`` /
# ``**kwargs`` var_keyword tail, so ``request_options`` is the last param the
# oracle records. Mirror that EXACTLY: a keyword, optional, carrying the concrete
# RequestOptions class type (a bare ``any`` here would read as untyped-laziness
# and FAIL drift — the reference types it concretely). The Rust type is projected
# to the reference module ``signalwire.rest._request_options`` by the enumerator's
# FREE_FN module rename, so the canonical type token is identical across ports.
_REQUEST_OPTIONS_TYPE = (
    "optional<class:signalwire.rest._request_options.RequestOptions>"
)


def _request_options_param() -> dict:
    return _param("request_options", "keyword", False, _REQUEST_OPTIONS_TYPE)


def _with_request_options(params: list[dict]) -> list[dict]:
    """Insert the trailing keyword-only ``request_options`` param BEFORE any
    trailing var_keyword (``params``/``kwargs``) — matching the reference's
    ``(..., *, request_options=None, **params)`` ordering, where the oracle drops
    the var_keyword tail so ``request_options`` becomes the last recorded param.
    A remaining port-side var_keyword is an optional extra the drift tool ignores
    (overlap-prefix compare), so ``request_options`` still aligns at the oracle's
    slot."""
    tail = params[-1:] if params and params[-1].get("kind") == "var_keyword" else []
    head = params[: len(params) - len(tail)]
    return [*head, _request_options_param(), *tail]


# JSON scalar → the oracle's canonical *param-type* token (the reference records
# ``string``/``int``/``float``/``bool`` — NOT ``integer``/``number``). This is the
# SAME schema the generator already types the struct field from (rust_field_type /
# _wire_owned_type map the identical scalar set to String/i64/f64/bool); here we
# spell it in the reference's canonical vocabulary so the enumerated param compares
# EQUAL to the oracle under diff_port_signatures.types_compatible.
_SCALAR_CANON = {
    "string": "string",
    "integer": "int",
    "number": "float",
    "boolean": "bool",
}


def _named_ref_leaf(schema: dict) -> str | None:
    """The components/schemas NAME a field points at through a bare ``$ref`` or a
    single-element ``allOf`` (the OpenAPI idiom for "this field IS <NamedSchema>,
    plus a description"). None for an inline/anonymous field. This is the name the
    reference generates its ``<ns>_types_generated.<Name>`` type under."""
    if not isinstance(schema, dict):
        return None
    ref = schema.get("$ref")
    if ref:
        return ref.rsplit("/", 1)[-1]
    allof = schema.get("allOf")
    if (
        allof
        and len(allof) == 1
        and not schema.get("properties")
        and not schema.get("type")
    ):
        return _named_ref_leaf(allof[0])
    return None


def _body_field_canon_type(spec: Spec, schema: dict) -> str:
    """Canonical param-type token for an exploded body field, matching the oracle.

    A field that IS a NAMED spec schema (via ``$ref`` / single ``allOf``) which the
    reference materialises as a generated type — an object schema OR a string-``enum``
    — carries that generated type ref
    (``class:signalwire.rest.namespaces.<ns>_types_generated.<Name>``, which the diff
    normalises to ``gen:<Name>`` by leaf, the cross-port contract go/java/etc. also
    record). Scalars carry their concrete token (``string``/``int``/``float``/``bool``,
    the same schema→type decision ``rust_field_type`` makes for the struct field); an
    array carries ``list<inner>``; an anonymous object / union / untyped field carries
    the open ``dict<string,any>``. Every branch is non-``any`` (passes the typed-surface
    param gate) and compatible with what the reference records for that wire field (the
    port serialises them through one ``serde_json::Value`` on the wire — wire-neutral)."""
    if not isinstance(schema, dict):
        return "dict<string,any>"
    # A field that IS a NAMED spec schema (via ``$ref`` / single ``allOf``) → the
    # reference materialises it as a generated type keyed by the RAW schema name
    # (``Encryption``, ``UsedForType``, the string-format aliases ``uuid``/``jwt``/
    # ``docid``, …). Record the matching ``class:...<ns>_types_generated.<rawname>``
    # ref, which the diff normalises to ``gen:<rawname>`` by leaf — the same
    # cross-port token go/java/etc. record. (Raw name, NOT ``type_name`` — the oracle
    # preserves the lowercase alias spellings ``uuid``/``jwt``/``docid``.)
    leaf = _named_ref_leaf(schema)
    if leaf and isinstance(spec.schemas.get(leaf), dict):
        return f"class:signalwire.rest.namespaces.{spec.name}_types_generated.{leaf}"
    resolved = resolve_schema(spec, schema)
    jt = _json_type(resolved) if isinstance(resolved, dict) else None
    if jt in _SCALAR_CANON:
        return _SCALAR_CANON[jt]
    if jt == "array":
        items = resolved.get("items") if isinstance(resolved, dict) else None
        inner = (
            _body_field_canon_type(spec, items) if isinstance(items, dict) else "any"
        )
        return f"list<{inner}>"
    # anonymous object / oneOf / anyOf / union / untyped → open JSON object.
    return "dict<string,any>"


def _body_field_params(
    spec: Spec, fields, kind_for_fields: str, tail_extra_name: str, tail_kwargs: bool
) -> list[dict]:
    """Exploded params for an object/command body: each field → kind_for_fields
    (``keyword``) carrying the field's CONCRETE type (threaded from its schema so the
    param compares equal to the oracle, not a bare ``any``); then the
    ``extras``/``extra`` OPEN door + optional ``kwargs`` tail, mirroring the oracle.
    The ``extras``/``extra`` door carries the OPEN ``dict<string,any>`` (the reference's
    ``optional<dict<string,any>>``, the cross-port extras SIGNAL — an open dict, NOT a
    typed field) and the ``kwargs`` tail stays var_keyword. Required-first
    (``ordered_fields``)."""
    out: list[dict] = []
    for wire, _sch, req in ordered_fields(fields):
        out.append(
            _param(
                field_ident(wire),
                kind_for_fields,
                bool(req),
                _body_field_canon_type(spec, _sch),
            )
        )
    out.append(_param(tail_extra_name, kind_for_fields, False, "dict<string,any>"))
    if tail_kwargs:
        out.append(_param("kwargs", "var_keyword", False))
    return out


def sidecar_operation_method(
    spec: Spec, anchor: str, markup: dict, base: str, method_snake: str, op_id: str
) -> list[dict] | None:
    """Exploded param model for a declared operation method (mirrors
    emit_operation_method's branches)."""
    if op_id not in spec.ops:
        return None
    verb, op_path, has_body = spec.ops[op_id]
    id_args, _expr, _sib = method_call_path(spec, anchor, markup, op_path)
    params: list[dict] = [_param(a, "positional", True, "string") for a in id_args]
    write_verb = verb in ("post", "put", "patch")
    if write_verb and has_body:
        body_schema = spec.op_body.get(op_id) or {}
        if is_object_body(spec, body_schema):
            fields = object_body_fields(spec, body_schema)
            params += _body_field_params(spec, fields, "keyword", "extras", True)
        else:
            # union body → a single ``body`` param (L10 watch-out: do NOT explode). A
            # NAMED union schema ($ref) carries its generated-type ref (the oracle records
            # ``class:...<Name>`` / ``gen:<Name>``); an anonymous union stays open.
            btype = _body_field_canon_type(spec, body_schema)
            if btype == "dict<string,any>":
                btype = "any"  # anonymous union body → the open value (unchanged)
            params.append(_param("body", "positional", True, btype))
    elif write_verb:
        pass  # no body
    elif verb == "get":
        params.append(_param("params", "var_keyword", False))
    # delete → just the id positionals
    return _with_request_options(params)


def sidecar_set_method(
    spec: Spec,
    markup: dict,
    sm_name: str,
    sm: dict,
    update_fields: set[str],
    field_schemas: dict[str, dict],
) -> list[dict]:
    """Exploded param model for a set_* wrapper: leading resource_id positional,
    the bound args (required→positional-req / optional→positional), trailing
    ``extra`` var_keyword — matching the oracle (e.g. set_call_flow:
    resource_id, flow_id, version?, **extra)."""
    params: list[dict] = [_param("resource_id", "positional", True, "string")]
    args = sm.get("args") or {}
    for arg_name, arg in args.items():
        req = bool(arg.get("required"))
        # Thread the BOUND update-field's concrete schema type (matching the oracle:
        # set_call_flow.flow_id is bound to ``call_flow_id`` → the named ``uuid`` schema
        # → ``gen:uuid``, NOT a bare ``string``). Falls back to ``string`` when the
        # field schema is absent/plain-string.
        fld = arg.get("field")
        fsch = field_schemas.get(fld) if fld else None
        ptype = (
            _body_field_canon_type(spec, fsch) if isinstance(fsch, dict) else "string"
        )
        params.append(_param(field_ident(arg_name), "positional", req, ptype))
    params.append(_param("extra", "var_keyword", False))
    return _with_request_options(params)


def sidecar_command_method(
    spec: Spec, mapping_leaf: str, cmd_schema: dict, with_id: bool
) -> list[dict]:
    fields, _has_id = command_param_fields(spec, cmd_schema)
    params: list[dict] = []
    if with_id:
        params.append(_param("call_id", "positional", True, "string"))
    params += _body_field_params(spec, fields, "keyword", "extras", False)
    return _with_request_options(params)


def sidecar_for_resource(spec: Spec, anchor: str, markup: dict) -> dict:
    """Return {method_name: [param,...]} for one resource's EMITTED methods
    (declared/command/set + the create/update CRUD-write overrides), matching
    the oracle's exploded shape. __init__ is added by the adapter."""
    markup["name"]
    methods: dict[str, list[dict]] = {}
    if markup.get("kind") == "command-dispatch":
        request = markup.get("request")
        mapping = discriminator_mapping(spec, request)
        for cmd, ref in mapping.items():
            mname = command_method_name(cmd)
            cmd_leaf = ref.rsplit("/", 1)[-1] if ref else ""
            cmd_schema = spec.schemas.get(cmd_leaf, {})
            _fields, with_id = command_param_fields(spec, cmd_schema)
            methods[mname] = sidecar_command_method(spec, cmd_leaf, cmd_schema, with_id)
        return methods

    base = markup["base"]
    provided = BASE_PROVIDES.get(base, set())
    declared = markup.get("methods") or {}

    # CRUD-write overrides (create/update) the oracle records as exploded typed
    # bodies. Rust delegates them to the base as create(&Value)/update(id,&Value),
    # but the ORACLE explodes them from the create/update request schemas. We keep
    # them a single loose body param on the port (base delegation) — the oracle
    # records the exploded set, so these show as documented loose-body residual
    # (item H types_generated closes them). Emit the port's actual shape:
    #   create(data)              → [ body ]           (positional loose)
    #   update(id, data)          → [ id, body ]       (positional loose)
    # so drift is param-count/type only where the oracle explodes, NOT a
    # kind-mismatch on the id.
    if base in ("CrudResource", "FabricResource"):
        methods["create"] = _with_request_options([_param("data", "positional", True)])
        methods["update"] = _with_request_options(
            [
                _param("id", "positional", True, "string"),
                _param("data", "positional", True),
            ]
        )

    # Declared operation methods (may override list_addresses with a sibling path).
    for m_snake, ref in declared.items():
        op_id = ref.get("op")
        if not op_id:
            continue
        if m_snake in provided and m_snake != "list_addresses":
            continue
        if m_snake == "list_addresses" and m_snake in provided:
            # only a SIBLING override is emitted (base delegation otherwise)
            _verb, op_path, _ = spec.ops.get(op_id, (None, None, None))
            if op_path is None:
                continue
            _, sibling = relative_tail(spec, anchor, markup, op_path)
            if not sibling:
                continue
        p = sidecar_operation_method(spec, anchor, markup, base, m_snake, op_id)
        if p is not None:
            methods[m_snake] = p

    # set_methods.
    set_methods = markup.get("set_methods") or {}
    if set_methods:
        upd_fields, upd_field_schemas = update_request_fields(spec, anchor, markup)
        for sm_name, sm in set_methods.items():
            methods[snake_of(sm_name)] = sidecar_set_method(
                spec, markup, sm_name, sm, upd_fields, upd_field_schemas
            )
    return methods


def build_sidecar(specs) -> dict:
    """The full signature/surface sidecar the rust adapters consume."""
    resources: dict[str, dict] = {}
    containers: dict[str, dict] = {}
    for spec in specs:
        module = _res_module(spec)
        for anchor, markup in spec.resources():
            name = markup["name"]
            base = (
                "command-dispatch"
                if markup.get("kind") == "command-dispatch"
                else markup.get("base")
            )
            declared = list((markup.get("methods") or {}).keys())
            setm = list((markup.get("set_methods") or {}).keys())
            drop = (
                sorted(surface_drop_set(base, declared, setm))
                if base != "command-dispatch"
                else ["base_path"]
            )
            resources[name] = {
                "module": f"signalwire.rest.namespaces.{module}",
                "class": name,
                "base": base,
                "surface_drop": drop,
                "methods": sidecar_for_resource(spec, anchor, markup),
            }
    # Container structs → the _client_tree_generated oracle module. The SIGNATURE
    # oracle records each sub-resource accessor (``fabric.ai_agents()`` etc.)
    # returning its resource class; the SURFACE oracle records only __init__
    # (accessors are property-like there). We carry the accessors here (name +
    # return class) so the signature adapter emits them; the surface adapter
    # drops every non-__init__ (its ``*accessors*`` rule). GeneratedResourceTree
    # is port-internal glue — suppressed entirely.
    placed = resolve_placement(specs)
    # Map each generated resource NAME → its oracle module (for accessor returns).
    res_module: dict[str, str] = {}
    for spec in specs:
        module = _res_module(spec)
        for _anchor, markup in spec.resources():
            res_module[markup["name"]] = f"signalwire.rest.namespaces.{module}"
    for _spec, _anchor, markup, container in placed:
        if container and container in CONTAINERS:
            clsname, _acc = CONTAINERS[container]
            entry = containers.setdefault(
                clsname,
                {
                    "module": "signalwire.rest.namespaces._client_tree_generated",
                    "class": clsname,
                    "accessors": {},
                },
            )
            rname = markup["name"]
            acc = container_accessor(markup, rname, container)
            entry["accessors"][acc] = {
                "returns": f"class:{res_module.get(rname, '')}.{rname}",
            }
    return {
        "version": "1",
        "note": (
            "adapter sidecar for the generated REST layer — exploded param "
            "model (kinds) + surface drop-sets; consumed by "
            "enumerate_signatures.py / enumerate_surface.py"
        ),
        "suppress_structs": ["GeneratedResourceTree"],
        "resources": resources,
        "containers": containers,
    }


# ---------------------------------------------------------------------------
# Driver.
# ---------------------------------------------------------------------------


def _rustfmt(src: str) -> str:
    """Format Rust source with the pinned stable rustfmt so the generated files
    are byte-identical to what the FMT gate expects (rustfmt is idempotent, so a
    formatted file re-formats to itself). Falls back to the raw source if
    rustfmt is unavailable (GEN-FRESH stays internally consistent either way)."""
    import subprocess

    try:
        cp = subprocess.run(
            ["rustfmt", "+stable", "--edition", "2024", "--emit", "stdout"],
            input=src,
            capture_output=True,
            text=True,
            check=False,
        )
        if cp.returncode == 0 and cp.stdout:
            return cp.stdout
        # `rustfmt +stable` form may not be accepted directly; retry via rustup.
        cp = subprocess.run(
            ["rustfmt", "--edition", "2024", "--emit", "stdout"],
            input=src,
            capture_output=True,
            text=True,
            check=False,
        )
        if cp.returncode == 0 and cp.stdout:
            return cp.stdout
    except FileNotFoundError:
        pass
    return src


def build_outputs(psdk: Path) -> dict[str, str]:
    load_bases(psdk)  # validate x-sdk-bases (fail loud)
    _RESERVED_RENAMES.clear()
    specs = [load_spec(psdk, ns) for ns in discover_spec_dirs(psdk)]
    outs: dict[str, str] = {}
    mod_names: list[str] = []

    for spec in specs:
        structs: dict[str, str] = {}
        bodies: list[str] = []
        for anchor, markup in spec.resources():
            bodies.append(emit_resource(spec, anchor, markup, structs))
        module = _res_module(spec)
        mod_names.append(module)
        # assemble the body first so imports can be computed from actual usage
        body_src = ""
        # request structs first (referenced by the impls)
        for sname in sorted(structs):
            body_src += "\n" + structs[sname] + "\n"
        for body in bodies:
            body_src += "\n" + body + "\n"
        src = GEN_BANNER.format(
            desc=f"Generated REST resources for the {spec.name!r} namespace."
        )
        src += "\n" + gen_imports(body_src)
        src += body_src
        outs[module + ".rs"] = src

    placed = resolve_placement(specs)
    outs["client_tree_generated.rs"] = emit_client_tree(placed)
    mod_names.append("client_tree_generated")

    # §D/§H/§I: the READ-side <ns>_types_generated wire-type modules (one file per
    # namespace, under types/). Emitted into outs["types/<ns>_types_generated.rs"].
    emit_types(psdk, outs)
    type_mod_names = sorted(
        fn[len("types/") : -len(".rs")]
        for fn in outs
        if fn.startswith("types/") and fn.endswith(".rs")
    )
    type_mod_lines = [
        "// Code generated by scripts/generate_rest.py; DO NOT EDIT.",
        "//",
        "// AUTO-GENERATED index for the generated REST wire-type modules (§H/§I).",
        "",
    ]
    type_mod_lines.extend(f"pub mod {m};" for m in type_mod_names)
    outs["types/mod.rs"] = "\n".join(type_mod_lines) + "\n"

    # mod.rs re-exporting each generated module (+ the types index submodule).
    mod_lines = [
        "// Code generated by scripts/generate_rest.py; DO NOT EDIT.",
        "//",
        "// AUTO-GENERATED module index for the generated REST resource layer.",
        "",
    ]
    mod_lines.extend(f"pub mod {m};" for m in mod_names)
    mod_lines.append("pub mod types;")
    outs["mod.rs"] = "\n".join(mod_lines) + "\n"

    # Adapter sidecar (JSON, L10) — written alongside the generated modules so a
    # regen keeps it in lockstep and GEN-FRESH gates it too.
    import json as _json

    outs["rest_signatures.json"] = (
        _json.dumps(build_sidecar(specs), indent=2, sort_keys=True) + "\n"
    )

    # Format the generated Rust with the pinned rustfmt so the emitted files are
    # byte-identical to what the FMT gate produces (otherwise `cargo fmt --all`
    # would rewrite them and GEN-FRESH would then read them as stale).
    for fn in list(outs):
        if fn.endswith(".rs"):
            outs[fn] = _rustfmt(outs[fn])
    return outs


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--check", action="store_true", help="GEN-FRESH: exit non-zero if stale"
    )
    ap.add_argument("--out", default="", help="scratch: emit into this dir")
    ap.add_argument(
        "--report-renames",
        action="store_true",
        help="print reserved-word field/arg renames encountered",
    )
    args = ap.parse_args(argv)

    psdk = resolve_porting_sdk()
    outs = build_outputs(psdk)

    if args.out:
        out_dir = Path(args.out)
    else:
        out_dir = repo_root() / "src" / "rest" / "namespaces" / "generated"

    if args.check:
        stale = []
        for fn, src in outs.items():
            p = out_dir / fn
            if not p.is_file() or p.read_text() != src:
                stale.append(str(p))
        expected = set(outs.keys())
        if out_dir.is_dir():
            for p in sorted(out_dir.rglob("*.rs")):
                rel = p.relative_to(out_dir).as_posix()
                if rel not in expected:
                    stale.append(f"{p} (leftover — not in generator output)")
        if stale:
            sys.stderr.write(
                f"GEN-FRESH FAIL: {len(stale)} generated REST file(s) stale:\n"
            )
            for s in stale:
                sys.stderr.write(f"  - {s}\n")
            return 1
        print("GEN-FRESH: generated REST files match the canonical specs.")
        return 0

    out_dir.mkdir(parents=True, exist_ok=True)
    for fn, src in outs.items():
        p = out_dir / fn
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(src)
    print(f"generated {len(outs)} REST file(s) into {out_dir}")
    if args.report_renames and _RESERVED_RENAMES:
        print("reserved-word field/arg renames:")
        for wire, ident in sorted(_RESERVED_RENAMES):
            print(f"  {wire} -> {ident}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
