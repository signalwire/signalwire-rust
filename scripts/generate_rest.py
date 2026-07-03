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


# The 12 real REST spec directories (registry has no own dir — its resources
# live inside relay-rest via namespace: registry; swml-webhooks is types-only).
SPEC_DIRS = [
    "relay-rest", "fabric", "calling", "video", "datasphere",
    "logs", "message", "voice", "fax", "project", "chat", "pubsub",
]

# Spec-dir -> the oracle <ns>_resources_generated leaf (the module name callers
# and the adapter key on). "relay-rest" -> "relay_rest".
NS_LEAF = {
    "relay-rest": "relay_rest", "fabric": "fabric", "calling": "calling",
    "video": "video", "datasphere": "datasphere", "logs": "logs",
    "message": "message", "voice": "voice", "fax": "fax",
    "project": "project", "chat": "chat", "pubsub": "pubsub",
}

# Rust reserved words (2015+2018+2021+2024 keywords, incl. reserved). A spec
# field or path arg colliding gets a raw identifier ``r#<word>`` (except the few
# that are not valid even as raw: crate/self/super/Self — none occur as wire
# field names, but guard anyway → trailing underscore).
RUST_KEYWORDS = {
    "as", "break", "const", "continue", "crate", "dyn", "else", "enum",
    "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
    "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
    "where", "while", "async", "await", "gen", "abstract", "become", "box",
    "do", "final", "macro", "override", "priv", "typeof", "unsized",
    "virtual", "yield", "try", "union",
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
    raise SystemExit("generate_rest.py: porting-sdk not found (set $PORTING_SDK or clone adjacent)")


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


# ---------------------------------------------------------------------------
# Base loading (x-sdk-bases; §2) — validate + flatten to method-sets.
# ---------------------------------------------------------------------------

def load_bases(psdk: Path) -> dict[str, list[str]]:
    raw = yaml.safe_load((psdk / "rest-apis" / "x-sdk-bases.yaml").read_text())
    bases = dict(raw.get("x-sdk-bases") or {})
    fab = psdk / "rest-apis" / "fabric" / "x-sdk-bases.yaml"
    if fab.is_file():
        bases.update((yaml.safe_load(fab.read_text()).get("x-sdk-bases") or {}))

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
            raise SystemExit(f"{name}: servers[0].url path {self.server_path!r} has a trailing slash")
        self.namespace_attr = (doc.get("x-sdk-namespace") or {}).get("attr") or ""
        self.ops: dict[str, tuple[str, str, bool]] = {}
        self.op_body: dict[str, dict] = {}
        for path, item in (doc.get("paths") or {}).items():
            for verb in ("get", "post", "put", "patch", "delete"):
                o = item.get(verb)
                if o and o.get("operationId"):
                    self.ops[o["operationId"]] = (verb, path, bool(o.get("requestBody")))
                    body = o.get("requestBody") or {}
                    content = body.get("content") or {}
                    media = content.get("application/json") or (next(iter(content.values())) if content else {})
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
    return Spec(ns, yaml.safe_load((psdk / "rest-apis" / ns / "openapi.yaml").read_text()))


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
        return ([s for s in absp[len(full) + 1:].split("/") if s], False)
    if coll and absp == full:
        return ([], False)
    return ([s for s in absp.lstrip("/").split("/") if s], True)


# ---------------------------------------------------------------------------
# Naming.
# ---------------------------------------------------------------------------

def snake_of(s: str) -> str:
    """Normalize an already-snake-ish string (fold '-'/'.' to '_')."""
    return re.sub(r"[^A-Za-z0-9_]", "_", s)


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
    "id": "id", "queue_id": "queue_id", "NumberGroupId": "group_id",
    "documentId": "document_id", "chunkId": "chunk_id", "mfa_request_id": "request_id",
    "e164_number": "e164", "fabric_subscriber_id": "subscriber_id",
    "ai_agent_id": "id", "cxml_webhook_id": "id", "swml_webhook_id": "id",
    "token_id": "token_id", "room_id": "room_id", "resource_id": "resource_id",
    "sip_endpoint_id": "sip_endpoint_id", "membership_id": "membership_id",
}


def arg_for(brace: str) -> str:
    return PARAM_ARG_NAME.get(brace, snake_of(brace) or "id")


# ---------------------------------------------------------------------------
# Base mapping (§2).
# ---------------------------------------------------------------------------

BASE_PROVIDES = {
    "CrudResource": {"list", "create", "get", "update", "delete"},
    "FabricResource": {"list", "create", "get", "update", "delete", "list_addresses"},
    "ReadResource": {"list", "get"},
    "BaseResource": set(),
}


# ---------------------------------------------------------------------------
# Command-dispatch (§6).
# ---------------------------------------------------------------------------

def command_method_name(cmd: str) -> str:
    s = cmd[len("calling."):] if cmd.startswith("calling.") else cmd
    return snake_of(s)


def discriminator_mapping(spec: Spec, schema_name: str) -> dict[str, str]:
    sch = spec.schemas.get(schema_name)
    if sch is None:
        raise SystemExit(f"command-dispatch request {schema_name!r} not in components.schemas")
    mapping = (sch.get("discriminator") or {}).get("mapping")
    if not mapping:
        raise SystemExit(f"command-dispatch request {schema_name!r} has no discriminator.mapping")
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
    if allof and len(allof) == 1 and not schema.get("properties") and not schema.get("type"):
        return resolve_schema(spec, allof[0], seen)
    return schema


def _json_type(schema: dict) -> str | None:
    t = schema.get("type")
    if isinstance(t, list):
        non_null = [x for x in t if x != "null"]
        return non_null[0] if non_null else None
    return t


# JSON scalar → (Rust owned type, `serde_json::json!` / into-Value builder).
_SCALAR_RUST = {"string": "String", "integer": "i64", "number": "f64", "boolean": "bool"}


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


def command_param_fields(spec: Spec, command_schema: dict) -> tuple[list[tuple[str, dict, bool]], bool]:
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


def update_request_fields(spec: Spec, anchor: str, markup: dict) -> tuple[set[str], dict[str, dict]]:
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

GEN_HEADER = """// Code generated by scripts/generate_rest.py; DO NOT EDIT.
//
// AUTO-GENERATED from porting-sdk/rest-apis/ (x-sdk-* markup) — regenerate with:
//   python3 scripts/generate_rest.py
//
// {desc}

// Generated code is not hand-tuned for every pedantic clippy lint; the LINT
// gate governs SOURCE STYLE (parity-neutral), so the generated layer allows the
// pedantic lints its emission shape naturally trips. The hand base resources
// (generated_bases.rs) and the rest of the crate stay under the strict gate.
// ``dead_code``: a resource's private client()/path() helpers are used only by
// its DECLARED operation methods; a pure-CRUD resource (all methods delegated to
// the base) doesn't call them, so they read as dead in that file.
#![allow(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    clippy::redundant_field_names,
    clippy::useless_asref,
    clippy::unnecessary_to_owned,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    dead_code,
    unused_imports
)]

use std::collections::HashMap;

use serde_json::{{Map, Value}};

use crate::rest::error::SignalWireRestError;
use crate::rest::generated_bases::{{
    BaseResource, CrudResource, FabricResource, ReadResource,
}};
use crate::rest::http_client::HttpClient;
"""


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
        expr = "self.base_path().to_string()"
    else:
        # self.path(&[<pieces>]) — each piece is either a &str literal or a var.
        joined = ", ".join(pieces if False else pieces)
        expr = "self.path(&[" + ", ".join(
            (p if p.startswith('"') else p + ".as_ref()") for p in pieces
        ) + "])"
    return id_args, expr, sibling


def abs_rust_path(full: str, id_args: list[str]) -> str:
    """A `format!`-style Rust expression for a sibling absolute path,
    substituting {brace} with the positional id_args in order."""
    out_fmt = []
    fmt_args = []
    ai = 0
    i = 0
    while i < len(full):
        if full[i] == "{":
            j = full.find("}", i)
            if ai < len(id_args):
                out_fmt.append("{}")
                fmt_args.append(id_args[ai])
                ai += 1
            i = j + 1
            continue
        out_fmt.append(full[i])
        i += 1
    fmt_str = "".join(out_fmt).replace('"', '\\"')
    if fmt_args:
        return f'format!("{fmt_str}", ' + ", ".join(fmt_args) + ")"
    return f'"{fmt_str}".to_string()'


def _request_struct_name(cls: str, method_rs: str) -> str:
    """PascalCase request-struct name for a method's named params (Rust idiom)."""
    parts = [p for p in method_rs.split("_") if p]
    pm = "".join(w[:1].upper() + w[1:] for w in parts)
    return f"{cls}{pm}Request"


def emit_request_struct(struct_name: str, spec: Spec,
                        leading: list[tuple[str, str]],
                        fields: list[tuple[str, dict, bool]],
                        wire_container: str) -> tuple[str, str]:
    """Emit a request struct + fluent builder + build() -> Value.

    ``leading`` = [(rust_ident, "String")] required leading positional args
    (a call_id for command methods; NONE for object bodies — the id there is a
    method arg, not a body field). ``wire_container`` = "body" | "params" — the
    JSON object the fields go into. Returns (struct_source, ctor_call_hint)."""
    req = [(n, s, r) for (n, s, r) in ordered_fields(fields) if r]
    opt = [(n, s, r) for (n, s, r) in ordered_fields(fields) if not r]

    lines: list[str] = []
    lines.append("/// Named request parameters for the generated method (Rust options-builder")
    lines.append("/// idiom — required fields in `new`, optionals via setters, `extras` open door).")
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
        new_params.append(f"{ident}: impl Into<{ty}>" if ty == "String" else f"{ident}: {ty}")
    for wire, sch, _ in req:
        ident = field_ident(wire)
        ty = rust_field_type(spec, sch)
        new_params.append(f"{ident}: impl Into<{ty}>" if ty == "String" else f"{ident}: {ty}")
    lines.append("    /// Construct the request with its required fields.")
    lines.append(f"    pub fn new({', '.join(new_params)}) -> Self {{")
    lines.append(f"        {struct_name} {{")
    for ident, ty in leading:
        lines.append(f"            {ident}: {ident}{'.into()' if ty == 'String' else ''},")
    for wire, sch, _ in req:
        ident = field_ident(wire)
        ty = rust_field_type(spec, sch)
        lines.append(f"            {ident}: {ident}{'.into()' if ty == 'String' else ''},")
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
            lines.append(f"    pub fn {setter}(mut self, {arg}: impl Into<{ty}>) -> Self {{")
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
    lines.append("    pub fn extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {")
    lines.append("        self.extras.insert(key.into(), value.into());")
    lines.append("        self")
    lines.append("    }")

    # build() -> the wire Value object for the fields (leading args excluded —
    # they are path/id args carried separately by the emitting method).
    lines.append(f"    /// Assemble the `{wire_container}` JSON object (unset optionals omitted).")
    lines.append("    #[must_use]")
    lines.append("    pub fn build(self) -> Value {")
    lines.append("        let mut obj = Map::new();")
    for wire, sch, _ in req:
        ident = field_ident(wire)
        ty = rust_field_type(spec, sch)
        conv = ("Value::from(self.%s)" % ident) if ty in ("String", "i64", "f64", "bool") else ("self.%s" % ident)
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
def emit_operation_method(spec: Spec, anchor: str, markup: dict, base: str,
                          method_snake: str, op_id: str,
                          structs: dict[str, str]) -> str:
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

    if write_verb and has_body:
        body_schema = spec.op_body.get(op_id) or {}
        if is_object_body(spec, body_schema):
            fields = object_body_fields(spec, body_schema)
            sname = _request_struct_name(cls, name)
            src, _ = emit_request_struct(sname, spec, [], fields, "body")
            structs[sname] = src
            params = id_params + [f"request: {sname}"]
            lines.append(f"    /// `{verb.upper()} {op_path}` (generated operation method).")
            lines.append("    ///")
            lines.append("    /// # Errors")
            lines.append("    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx")
            lines.append("    /// status, or an unparseable response body.")
            lines.append(f"    pub fn {name}(&self, {', '.join(params)}) -> Result<Value, SignalWireRestError> {{")
            lines.append(f"        self.client().{verb_fn}(&{path_expr}, &request.build())")
            lines.append("    }")
        else:
            # §5.2 union body → a single positional body: Value.
            params = id_params + ["body: &Value"]
            lines.append(f"    /// `{verb.upper()} {op_path}` (generated operation method; union body).")
            lines.append("    ///")
            lines.append("    /// # Errors")
            lines.append("    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx")
            lines.append("    /// status, or an unparseable response body.")
            lines.append(f"    pub fn {name}(&self, {', '.join(params)}) -> Result<Value, SignalWireRestError> {{")
            lines.append(f"        self.client().{verb_fn}(&{path_expr}, body)")
            lines.append("    }")
    elif write_verb:
        params = id_params
        sig = ("&self, " + ", ".join(params)) if params else "&self"
        lines.append(f"    /// `{verb.upper()} {op_path}` (generated operation method; no body).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status.")
        lines.append(f"    pub fn {name}({sig}) -> Result<Value, SignalWireRestError> {{")
        lines.append(f"        self.client().{verb_fn}(&{path_expr}, &Value::Object(Map::new()))")
        lines.append("    }")
    elif verb == "get":
        # §5.3 GET query door — a trailing params map.
        params = id_params + ["params: &HashMap<String, String>"]
        lines.append(f"    /// `GET {op_path}` (generated operation method; query params).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status.")
        lines.append(f"    pub fn {name}(&self, {', '.join(params)}) -> Result<Value, SignalWireRestError> {{")
        lines.append(f"        self.client().get(&{path_expr}, params)")
        lines.append("    }")
    else:  # delete
        params = id_params
        sig = ("&self, " + ", ".join(params)) if params else "&self"
        lines.append(f"    /// `DELETE {op_path}` (generated operation method).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status.")
        lines.append(f"    pub fn {name}({sig}) -> Result<Value, SignalWireRestError> {{")
        lines.append(f"        self.client().delete(&{path_expr})")
        lines.append("    }")
    return "\n".join(lines)


def emit_set_method(spec: Spec, markup: dict, sm_name: str, sm: dict,
                    update_fields: set[str], field_schemas: dict[str, dict],
                    structs: dict[str, str]) -> str:
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
            raise SystemExit(f"{markup['name']}.{sm_name}: arg {arg_name!r} missing field")
        if field not in update_fields:
            raise SystemExit(
                f"{markup['name']}.{sm_name}: arg field {field!r} not in update request schema")
        bound.append((arg_name, field, field_schemas.get(field, {}), bool(arg.get("required"))))

    # Emit a request struct keyed by arg-name (bound to update field on build).
    sname = _request_struct_name(cls, name)
    # Build synthetic field list using arg names as wire keys mapped to update fields.
    req = [(a, s, True) for (a, f, s, r) in bound if r]
    opt = [(a, s, False) for (a, f, s, r) in bound if not r]
    field_map = {a: f for (a, f, s, r) in bound}

    lines: list[str] = []
    # struct with arg-named fields; build() maps to update-field wire keys + handler.
    slines: list[str] = []
    slines.append("/// Named request parameters for a generated set_* wrapper (binds args to")
    slines.append("/// update-request fields + a fixed call_handler; Rust options-builder idiom).")
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
        new_params.append(f"{field_ident(a)}: impl Into<{ty}>" if ty == "String" else f"{field_ident(a)}: {ty}")
    slines.append(f"    pub fn new({', '.join(new_params)}) -> Self {{")
    slines.append(f"        {sname} {{")
    for a, s, _ in req:
        ty = rust_field_type(spec, s)
        slines.append(f"            {field_ident(a)}: {field_ident(a)}{'.into()' if ty == 'String' else ''},")
    slines.append("            ..Default::default()")
    slines.append("        }")
    slines.append("    }")
    for a, s, _ in opt:
        ident = field_ident(a)
        ty = rust_field_type(spec, s)
        setter = setter_ident(a)
        slines.append("    #[must_use]")
        if ty == "String":
            slines.append(f"    pub fn {setter}(mut self, value: impl Into<{ty}>) -> Self {{")
            slines.append(f"        self.{ident} = Some(value.into());")
        else:
            slines.append(f"    pub fn {setter}(mut self, value: {ty}) -> Self {{")
            slines.append(f"        self.{ident} = Some(value);")
        slines.append("        self")
        slines.append("    }")
    slines.append("    #[must_use]")
    slines.append("    pub fn extra(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {")
    slines.append("        self.extras.insert(key.into(), value.into());")
    slines.append("        self")
    slines.append("    }")
    slines.append("    #[must_use]")
    slines.append("    pub fn build(self) -> Value {")
    slines.append("        let mut obj = Map::new();")
    slines.append(f"        obj.insert(\"call_handler\".to_string(), Value::from({rs_str(handler)}));")
    for a, s, _ in req:
        ident = field_ident(a)
        ty = rust_field_type(spec, s)
        conv = ("Value::from(self.%s)" % ident) if ty in ("String", "i64", "f64", "bool") else ("self.%s" % ident)
        slines.append(f"        obj.insert({rs_str(field_map[a])}.to_string(), {conv});")
    for a, s, _ in opt:
        ident = field_ident(a)
        ty = rust_field_type(spec, s)
        conv = ("Value::from(v)") if ty in ("String", "i64", "f64", "bool") else "v"
        slines.append(f"        if let Some(v) = self.{ident} {{")
        slines.append(f"            obj.insert({rs_str(field_map[a])}.to_string(), {conv});")
        slines.append("        }")
    slines.append("        for (k, v) in self.extras { obj.insert(k, v); }")
    slines.append("        Value::Object(obj)")
    slines.append("    }")
    slines.append("}")
    structs[sname] = "\n".join(slines)

    lines.append(f"    /// `set_{sm_name}` — update wrapper binding a fixed call_handler (§7).")
    lines.append("    ///")
    lines.append("    /// # Errors")
    lines.append("    /// Returns [`SignalWireRestError`] on transport failure or a non-2xx status.")
    lines.append(f"    pub fn {name}(&self, resource_id: &str, request: {sname}) -> Result<Value, SignalWireRestError> {{")
    lines.append("        self.update(resource_id, &request.build())")
    lines.append("    }")
    return "\n".join(lines)


def emit_command_dispatch(spec: Spec, anchor: str, markup: dict, structs: dict[str, str]) -> str:
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
    lines.append(f"/// `{name}` — command-dispatch resource ({spec.name} spec). Each method POSTs")
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
    lines.append("    fn execute(&self, command: &str, call_id: Option<&str>, params: Value)")
    lines.append("        -> Result<Value, SignalWireRestError> {")
    lines.append("        let mut body = Map::new();")
    lines.append("        body.insert(\"command\".to_string(), Value::from(command));")
    lines.append("        body.insert(\"params\".to_string(), params);")
    lines.append("        if let Some(id) = call_id {")
    lines.append("            body.insert(\"id\".to_string(), Value::from(id));")
    lines.append("        }")
    lines.append("        self.client.post(Self::BASE_PATH, &Value::Object(body))")
    lines.append("    }")

    for cmd in commands:
        mname = command_method_name(cmd)
        cmd_leaf = mapping[cmd].rsplit("/", 1)[-1] if mapping.get(cmd) else ""
        cmd_schema = spec.schemas.get(cmd_leaf, {})
        fields, with_id = command_param_fields(spec, cmd_schema)
        sname = _request_struct_name(name, mname)
        leading: list[tuple[str, str]] = []  # call_id handled as a method arg, not struct field
        src, _ = emit_request_struct(sname, spec, leading, fields, "params")
        structs[sname] = src
        id_param = "call_id: &str, " if with_id else ""
        call_arg = "Some(call_id)" if with_id else "None"
        lines.append("")
        lines.append(f"    /// `{cmd}` — generated command method.")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// Returns [`SignalWireRestError`] on transport failure or a non-2xx status.")
        lines.append(f"    pub fn {mname}(&self, {id_param}request: {sname}) -> Result<Value, SignalWireRestError> {{")
        lines.append(f"        self.execute({rs_str(cmd)}, {call_arg}, request.build())")
        lines.append("    }")
    lines.append("}")
    return "\n".join(lines)


def emit_resource(spec: Spec, anchor: str, markup: dict, structs: dict[str, str]) -> str:
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
        spec_verb = "PUT" if item.get("put") else ("PATCH" if item.get("patch") else None)
        if spec_verb and upd != spec_verb:
            raise SystemExit(f"{name}: update_method {upd} != spec update verb {spec_verb}")

    bp = base_path(spec, anchor, markup)
    upd = markup.get("update_method", "PATCH")

    lines: list[str] = []
    lines.append(f"/// `{name}` — generated from x-sdk-resource {name!r} ({spec.name} spec, base {base}).")
    lines.append("///")
    lines.append(f"/// Composes [`{base}`] (its base-path is baked in per §4) and adds the")
    lines.append("/// resource's declared/set methods.")
    lines.append(f"pub struct {name}<'a> {{")
    lines.append(f"    base: {base}<'a>,")
    lines.append("}")
    lines.append("")
    lines.append(f"impl<'a> {name}<'a> {{")
    lines.append("    /// Construct the resource; its base path (§4) is baked in.")
    lines.append("    #[must_use]")
    lines.append("    pub fn new(client: &'a HttpClient) -> Self {")
    if base in ("CrudResource", "FabricResource"):
        lines.append(f"        {name} {{ base: {base}::new(client, {rs_str(bp)}, {rs_str(upd)}) }}")
    else:
        lines.append(f"        {name} {{ base: {base}::new(client, {rs_str(bp)}) }}")
    lines.append("    }")
    lines.append("")
    # Deref-style base accessors: base_path + client + path (needed by declared methods),
    # and re-expose the base's own CRUD/read methods by delegation.
    lines.append("    #[must_use]")
    lines.append("    pub fn base_path(&self) -> &str {")
    lines.append("        self.base.base_path()")
    lines.append("    }")
    lines.append("")
    lines.append("    fn client(&self) -> &HttpClient {")
    lines.append("        self.base.client()")
    lines.append("    }")
    lines.append("")
    lines.append("    fn path(&self, parts: &[&str]) -> String {")
    lines.append("        self.base.path(parts)")
    lines.append("    }")

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
    if "list" in provided:
        lines.append("")
        lines.append("    /// `list` (delegated to the base; GET base path).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append("    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {")
        lines.append("        self.base.list(params)")
        lines.append("    }")
    if "get" in provided:
        lines.append("")
        lines.append("    /// `get` (delegated to the base; GET base/{id}).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append("    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {")
        lines.append("        self.base.get(id)")
        lines.append("    }")
    if "create" in provided:
        lines.append("")
        lines.append("    /// `create` (delegated to the base; POST base path).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append("    pub fn create(&self, data: &Value) -> Result<Value, SignalWireRestError> {")
        lines.append("        self.base.create(data)")
        lines.append("    }")
    if "update" in provided:
        lines.append("")
        lines.append("    /// `update` (delegated to the base; PUT/PATCH base/{id}).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append("    pub fn update(&self, id: &str, data: &Value) -> Result<Value, SignalWireRestError> {")
        lines.append("        self.base.update(id, data)")
        lines.append("    }")
    if "delete" in provided:
        lines.append("")
        lines.append("    /// `delete` (delegated to the base; DELETE base/{id}).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append("    pub fn delete(&self, id: &str) -> Result<Value, SignalWireRestError> {")
        lines.append("        self.base.delete(id)")
        lines.append("    }")
    if "list_addresses" in provided and not override_list_addresses:
        lines.append("")
        lines.append("    /// `list_addresses` (delegated to the Fabric base; GET base/{id}/addresses).")
        lines.append("    ///")
        lines.append("    /// # Errors")
        lines.append("    /// See the base resource.")
        lines.append("    pub fn list_addresses(&self, id: &str, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {")
        lines.append("        self.base.list_addresses(id, params)")
        lines.append("    }")

    for method_snake, spec_ref in declared.items():
        op_id = spec_ref.get("op")
        if not op_id:
            raise SystemExit(f"{name}.{method_snake}: method markup missing op")
        if method_snake in provided:
            if method_snake == "list_addresses":
                verb, op_path, _ = spec.ops[op_id]
                _, sibling = relative_tail(spec, anchor, markup, op_path)
                if not sibling:
                    continue
                # sibling override — fall through and emit
            else:
                continue
        lines.append("")
        lines.append(emit_operation_method(spec, anchor, markup, base, method_snake, op_id, structs))

    set_methods = markup.get("set_methods") or {}
    if set_methods:
        if base not in ("CrudResource", "FabricResource"):
            raise SystemExit(f"{name}: set_methods require a CRUD base, got {base}")
        upd_fields, upd_field_schemas = update_request_fields(spec, anchor, markup)
        for sm_name, sm in set_methods.items():
            lines.append("")
            lines.append(emit_set_method(spec, markup, sm_name, sm, upd_fields, upd_field_schemas, structs))

    lines.append("}")
    return "\n".join(lines)


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
    "GenericResources": "resources", "FabricAddresses": "addresses",
    "FabricTokens": "tokens", "DatasphereDocuments": "documents",
    "ProjectTokens": "tokens", "PubSub": "pubsub",
    "MessageLogs": "messages", "VoiceLogs": "voice", "FaxLogs": "fax",
    "ConferenceLogs": "conferences",
}


def container_accessor(markup: dict, name: str, container: str) -> str:
    if markup.get("attr"):
        return snake_of(markup["attr"])
    if name in ATTR_OVERRIDE:
        return snake_of(ATTR_OVERRIDE[name])
    lead = container[:1].upper() + container[1:]
    stem = name[len(lead):] if name.startswith(lead) else name
    return _pascal_to_snake(stem) if stem else _pascal_to_snake(name)


def _pascal_to_snake(s: str) -> str:
    out = re.sub(r"(?<!^)(?=[A-Z])", "_", s).lower()
    return out


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
def _res_module(spec: Spec) -> str:
    return f"{NS_LEAF[spec.name]}_resources_generated"


def emit_client_tree(placed) -> str:
    """Emit the generated client-tree: one container struct per namespace group +
    a `GeneratedResourceTree` the hand RestClient composes (lazy accessor per flat
    resource + per container). Base paths per §4, placement per §8."""
    flats = []            # (accessor, struct, module)
    containers: dict[str, list[tuple[str, str, str]]] = {}
    corder: list[str] = []
    for spec, anchor, markup, container in placed:
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
    lines.append("// AUTO-GENERATED from porting-sdk/rest-apis/ placement markup (§8).")
    lines.append("")
    lines.append("#![allow(clippy::module_name_repetitions)]")
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
        lines.append(f"/// `{clsname}` — generated container grouping the {c} namespace resources (§8).")
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
    lines.append("/// `GeneratedResourceTree` — generated lazy accessors for every flat REST")
    lines.append("/// resource plus the namespace containers (§8). The hand `RestClient` composes")
    lines.append("/// this; each accessor constructs the resource with the client's `HttpClient`")
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
def surface_drop_set(base: str, declared: list[str], set_methods: list[str]) -> set[str]:
    prov = BASE_PROVIDES.get(base, set())
    keep = set(declared) | set(set_methods)
    if base in ("CrudResource", "FabricResource"):
        keep |= {"create", "update"}
    drop = (prov - keep)
    drop.add("base_path")
    return drop


def _param(name: str, kind: str, required: bool, ptype: str = "any") -> dict:
    # ``ptype`` is the canonical type the adapter emits. Path-id / string args are
    # genuinely ``&str`` in Rust → "string"; body/command keyword fields carry the
    # open "any" (the drift gate compares count+kind on those — L10). A loose
    # CRUD body (create/update ``data``) is also "any".
    return {"name": name, "kind": kind, "required": required, "type": ptype}


def _body_field_params(spec: Spec, fields, kind_for_fields: str,
                       tail_extra_name: str, tail_kwargs: bool) -> list[dict]:
    """Exploded params for an object/command body: each field → kind_for_fields
    (``keyword``); then the ``extras``/``extra`` door + optional ``kwargs`` tail,
    mirroring the oracle. Required-first ordering (matches ``ordered_fields``)."""
    out: list[dict] = []
    for wire, _sch, req in ordered_fields(fields):
        out.append(_param(field_ident(wire), kind_for_fields, bool(req)))
    out.append(_param(tail_extra_name, kind_for_fields, False))
    if tail_kwargs:
        out.append(_param("kwargs", "var_keyword", False))
    return out


def sidecar_operation_method(spec: Spec, anchor: str, markup: dict, base: str,
                             method_snake: str, op_id: str) -> list[dict] | None:
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
            # union body → a single loose ``body`` param (L10 watch-out: do NOT explode).
            params.append(_param("body", "positional", True))
    elif write_verb:
        pass  # no body
    elif verb == "get":
        params.append(_param("params", "var_keyword", False))
    # delete → just the id positionals
    return params


def sidecar_set_method(spec: Spec, markup: dict, sm_name: str, sm: dict,
                       update_fields: set[str], field_schemas: dict[str, dict]) -> list[dict]:
    """Exploded param model for a set_* wrapper: leading resource_id positional,
    the bound args (required→positional-req / optional→positional), trailing
    ``extra`` var_keyword — matching the oracle (e.g. set_call_flow:
    resource_id, flow_id, version?, **extra)."""
    params: list[dict] = [_param("resource_id", "positional", True, "string")]
    args = sm.get("args") or {}
    bound = []
    for arg_name, arg in args.items():
        bound.append((arg_name, bool(arg.get("required"))))
    for arg_name, req in bound:
        # set_* args are ``impl Into<String>`` in the generated builder → string.
        params.append(_param(field_ident(arg_name), "positional", req, "string"))
    params.append(_param("extra", "var_keyword", False))
    return params


def sidecar_command_method(spec: Spec, mapping_leaf: str, cmd_schema: dict,
                           with_id: bool) -> list[dict]:
    fields, _has_id = command_param_fields(spec, cmd_schema)
    params: list[dict] = []
    if with_id:
        params.append(_param("call_id", "positional", True, "string"))
    params += _body_field_params(spec, fields, "keyword", "extras", False)
    return params


def sidecar_for_resource(spec: Spec, anchor: str, markup: dict) -> dict:
    """Return {method_name: [param,...]} for one resource's EMITTED methods
    (declared/command/set + the create/update CRUD-write overrides), matching
    the oracle's exploded shape. __init__ is added by the adapter."""
    name = markup["name"]
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
        methods["create"] = [_param("data", "positional", True)]
        methods["update"] = [_param("id", "positional", True, "string"),
                             _param("data", "positional", True)]

    # Declared operation methods (may override list_addresses with a sibling path).
    for m_snake, ref in declared.items():
        op_id = ref.get("op")
        if not op_id:
            continue
        if m_snake in provided and m_snake != "list_addresses":
            continue
        if m_snake == "list_addresses" and m_snake in provided:
            # only a SIBLING override is emitted (base delegation otherwise)
            verb, op_path, _ = spec.ops.get(op_id, (None, None, None))
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
                spec, markup, sm_name, sm, upd_fields, upd_field_schemas)
    return methods


def build_sidecar(specs) -> dict:
    """The full signature/surface sidecar the rust adapters consume."""
    resources: dict[str, dict] = {}
    containers: dict[str, dict] = {}
    for spec in specs:
        module = _res_module(spec)
        for anchor, markup in spec.resources():
            name = markup["name"]
            base = "command-dispatch" if markup.get("kind") == "command-dispatch" else markup.get("base")
            declared = list((markup.get("methods") or {}).keys())
            setm = list((markup.get("set_methods") or {}).keys())
            drop = sorted(surface_drop_set(base, declared, setm)) if base != "command-dispatch" else ["base_path"]
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
    for spec, anchor, markup, container in placed:
        if container and container in CONTAINERS:
            clsname, _acc = CONTAINERS[container]
            entry = containers.setdefault(clsname, {
                "module": "signalwire.rest.namespaces._client_tree_generated",
                "class": clsname,
                "accessors": {},
            })
            rname = markup["name"]
            acc = container_accessor(markup, rname, container)
            entry["accessors"][acc] = {
                "returns": f"class:{res_module.get(rname, '')}.{rname}",
            }
    return {
        "version": "1",
        "note": ("adapter sidecar for the generated REST layer — exploded param "
                 "model (kinds) + surface drop-sets; consumed by "
                 "enumerate_signatures.py / enumerate_surface.py"),
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
            input=src, capture_output=True, text=True, check=False,
        )
        if cp.returncode == 0 and cp.stdout:
            return cp.stdout
        # `rustfmt +stable` form may not be accepted directly; retry via rustup.
        cp = subprocess.run(
            ["rustfmt", "--edition", "2024", "--emit", "stdout"],
            input=src, capture_output=True, text=True, check=False,
        )
        if cp.returncode == 0 and cp.stdout:
            return cp.stdout
    except FileNotFoundError:
        pass
    return src


def build_outputs(psdk: Path) -> dict[str, str]:
    load_bases(psdk)  # validate x-sdk-bases (fail loud)
    _RESERVED_RENAMES.clear()
    specs = [load_spec(psdk, ns) for ns in SPEC_DIRS]
    outs: dict[str, str] = {}
    mod_names: list[str] = []

    for spec in specs:
        structs: dict[str, str] = {}
        bodies: list[str] = []
        for anchor, markup in spec.resources():
            bodies.append(emit_resource(spec, anchor, markup, structs))
        module = _res_module(spec)
        mod_names.append(module)
        src = GEN_HEADER.format(
            desc=f"Generated REST resources for the {spec.name!r} namespace.")
        src += "\n"
        # request structs first (referenced by the impls)
        for sname in sorted(structs):
            src += "\n" + structs[sname] + "\n"
        for body in bodies:
            src += "\n" + body + "\n"
        outs[module + ".rs"] = src

    placed = resolve_placement(specs)
    outs["client_tree_generated.rs"] = emit_client_tree(placed)
    mod_names.append("client_tree_generated")

    # mod.rs re-exporting each generated module.
    mod_lines = [
        "// Code generated by scripts/generate_rest.py; DO NOT EDIT.",
        "//",
        "// AUTO-GENERATED module index for the generated REST resource layer.",
        "",
    ]
    for m in mod_names:
        mod_lines.append(f"pub mod {m};")
    outs["mod.rs"] = "\n".join(mod_lines) + "\n"

    # Adapter sidecar (JSON, L10) — written alongside the generated modules so a
    # regen keeps it in lockstep and GEN-FRESH gates it too.
    import json as _json
    outs["rest_signatures.json"] = _json.dumps(build_sidecar(specs), indent=2, sort_keys=True) + "\n"

    # Format the generated Rust with the pinned rustfmt so the emitted files are
    # byte-identical to what the FMT gate produces (otherwise `cargo fmt --all`
    # would rewrite them and GEN-FRESH would then read them as stale).
    for fn in list(outs):
        if fn.endswith(".rs"):
            outs[fn] = _rustfmt(outs[fn])
    return outs


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--check", action="store_true", help="GEN-FRESH: exit non-zero if stale")
    ap.add_argument("--out", default="", help="scratch: emit into this dir")
    ap.add_argument("--report-renames", action="store_true",
                    help="print reserved-word field/arg renames encountered")
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
            sys.stderr.write("GEN-FRESH FAIL: %d generated REST file(s) stale:\n" % len(stale))
            for s in stale:
                sys.stderr.write("  - %s\n" % s)
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
