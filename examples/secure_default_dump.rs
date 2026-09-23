// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `secure_default_dump` — the Rust port's SECURE-DEFAULT (A1) Layer-D dump
//! program for the cross-port behavioral differ
//! (porting-sdk `scripts/diff_port_secure_default.py`).
//!
//! Defines a default (secure) tool + an explicit `secure = false` tool on ONE
//! agent, renders the SWML, and emits per fixture the RENDERED WIRE PAYLOAD for
//! the differ to classify:
//!
//! ```text
//!   {"<fixture id>": {"secure_default_true": bool, "rendered": {<functions[] entry>}}}
//! ```
//!
//! * `secure_default_true` — the SDK-recorded secure flag for that tool.
//! * `rendered` — that tool's own `SWAIG.functions[]` entry, VERBATIM, with every
//!   token VALUE replaced by the corpus placeholder `<TOKEN>` (the values are
//!   HMACs and vary per run; the KEY PATH is the whole contract and is preserved
//!   exactly). A tool with NO entry — or an entry with no `web_hook_url` — emits
//!   exactly that; the absence IS the signal.
//!
//! This program deliberately makes NO judgement about whether the render is
//! correct. The previous version emitted a self-computed `wire_reflects_secure`
//! boolean, which made the gate vacuous: the differ never saw the wire, so it
//! could not see WHICH key a port had classified on, nor that an INSECURE tool
//! was being handed its own unauthenticated per-tool callback. The differ now
//! sees the keys and decides.
//!
//! ## How Rust expresses the "default" case
//!
//! Rust has no default parameter values, so `AgentBase::define_tool` takes
//! `secure: bool` positionally and the fixture cannot literally OMIT it. The
//! default the SDK documents (and that every reference program gets) is
//! `secure = true`, so the default-case fixture passes `true`. The load-bearing
//! half of the comparison is the RENDERED payload, which cannot be satisfied by
//! construction.
//!
//! Protocol: stdout = ONE JSON object mapping fixture id -> payload. Only stdout
//! carries JSON; all logging goes to stderr.
//!
//! Run from the repo root: `cargo run --quiet --example secure_default_dump`.

use std::collections::HashMap;

use serde_json::{Map, Value, json};
use signalwire::swaig::FunctionResult;
use signalwire::{AgentBase, AgentOptions};

/// Mirrors `secure_default_corpus.CORPUS[0].tool_name`.
const DEFAULT_TOOL: &str = "sd_default_secure";
/// Mirrors `secure_default_corpus.CORPUS[1].tool_name`.
const INSECURE_TOOL: &str = "sd_explicit_insecure";
/// Mirrors `secure_default_corpus.TOKEN_PLACEHOLDER`.
const TOKEN_PLACEHOLDER: &str = "<TOKEN>";

/// The rendered `ai.SWAIG.functions[]` entry named `tool_name`, if the render
/// emitted one.
fn rendered_entry<'a>(doc: &'a Value, tool_name: &str) -> Option<&'a Value> {
    doc.get("sections")
        .and_then(|s| s.get("main"))
        .and_then(Value::as_array)
        .and_then(|main| main.iter().find_map(|sec| sec.get("ai")))
        .and_then(|ai| ai.get("SWAIG"))
        .and_then(|swaig| swaig.get("functions"))
        .and_then(Value::as_array)
        .and_then(|fns| {
            fns.iter()
                .find(|f| f.get("function").and_then(Value::as_str) == Some(tool_name))
        })
}

/// Replace the VALUE of every token-suffixed query parameter in a URL with the
/// placeholder, preserving every key and the rest of the URL exactly.
fn redact_url_tokens(url: &str) -> String {
    let Some(q) = url.find('?') else {
        return url.to_string();
    };
    let (base, query) = url.split_at(q + 1);
    let redacted: Vec<String> = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((k, _)) if k.to_lowercase().ends_with("token") => {
                format!("{k}={TOKEN_PLACEHOLDER}")
            }
            _ => pair.to_string(),
        })
        .collect();
    format!("{base}{}", redacted.join("&"))
}

/// Normalize one rendered entry: replace every nondeterministic token VALUE (an
/// HMAC) with the placeholder while preserving every KEY and key path exactly —
/// both a token-suffixed field and a token-suffixed query parameter on a URL
/// value. Mirrors `diff_port_secure_default.redact_entry`, so the differ's
/// re-application of it is a no-op.
fn redact(entry: Option<&Value>) -> Value {
    let Some(Value::Object(obj)) = entry else {
        return json!({});
    };
    let mut out = Map::new();
    for (k, v) in obj {
        match v.as_str() {
            Some(_) if k.to_lowercase().ends_with("token") => {
                out.insert(k.clone(), json!(TOKEN_PLACEHOLDER));
            }
            Some(s) if s.contains("://") || s.starts_with('/') => {
                out.insert(k.clone(), json!(redact_url_tokens(s)));
            }
            _ => {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    Value::Object(out)
}

/// Emit one fixture: the SDK-recorded secure flag plus the redacted rendered
/// entry. NO classification — the differ does that.
fn emit(doc: &Value, tool_name: &str, secure_default_true: bool) -> Value {
    json!({
        "secure_default_true": secure_default_true,
        "rendered": redact(rendered_entry(doc, tool_name)),
    })
}

fn main() {
    let mut agent = AgentBase::new(
        AgentOptions::new("secure-default-fixture")
            .route("/sd")
            .basic_auth("u", "p"),
    );
    agent.set_prompt_text("secure default fixture");

    // A1 (a) — the DEFAULT case: secure. Its rendered entry must carry its own
    // web_hook_url with a `__token` query param.
    agent.define_tool(
        DEFAULT_TOOL,
        "secure-default fixture tool",
        json!({}),
        Box::new(|_args, _raw| FunctionResult::with_response("ok")),
        true,
    );
    // A1 (b) — explicit `secure = false`: NO per-tool web_hook_url at all; it
    // falls back to the shared `SWAIG.defaults.web_hook_url`.
    agent.define_tool(
        INSECURE_TOOL,
        "secure-default fixture tool",
        json!({}),
        Box::new(|_args, _raw| FunctionResult::with_response("ok")),
        false,
    );

    let doc = agent.render_swml(&HashMap::new());

    let mut out = Map::new();
    out.insert(
        "define_tool_default_is_secure".to_string(),
        emit(&doc, DEFAULT_TOOL, true),
    );
    out.insert(
        "define_tool_explicit_insecure".to_string(),
        emit(&doc, INSECURE_TOOL, false),
    );

    println!("{}", Value::Object(out));
}
