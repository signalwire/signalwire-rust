// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `secure_default_dump` — the Rust port's SECURE-DEFAULT (A1) Layer-D dump
//! program for the cross-port behavioral differ
//! (porting-sdk `scripts/diff_port_secure_default.py`).
//!
//! The differ drives the python reference through `secure_default_corpus` to
//! build the golden per-fixture classification, then runs THIS program (which
//! embeds the same two fixtures) and structurally compares our classification.
//!
//! The A1 contract: a tool registered WITHOUT an explicit opt-out is SECURE, and
//! the WIRE manifestation of `secure` is the per-tool `__token` the rendered
//! SWAIG webhook carries (python `agent_base.py:1040` mints it whenever
//! `func.secure` and a `call_id` is in play, and `agent_base.py:958` GENERATES a
//! `call_id` when the caller supplied none — so a secure tool ALWAYS renders with
//! a `__token`). A tool registered `secure = false` carries NO `__token`.
//!
//! ## How Rust expresses the "default" case
//!
//! Rust has no default parameter values, so `Service::define_tool` takes
//! `secure: bool` positionally and the fixture cannot literally OMIT it. The
//! default the SDK documents (and that every reference program gets) is
//! `secure = true`, so the default-case fixture passes `true`. The load-bearing
//! half of the classification is `wire_reflects_secure`, which is measured off
//! the ACTUALLY RENDERED document: it is only true when the render really minted
//! a `__token` for the secure tool and really withheld one from the insecure
//! tool. That half cannot be satisfied by construction.
//!
//! Per fixture this dump builds a fresh `AgentBase`, defines the tool, renders
//! the SWML, and reduces to the deterministic pair:
//!   `secure_default_true`  — the tool's declared secure state.
//!   `wire_reflects_secure` — a `__token` is present on the rendered webhook IFF
//!                            the tool is secure.
//! The token VALUE (an HMAC) is nondeterministic and is NOT compared — only its
//! PRESENCE folds into the boolean.
//!
//! Protocol: stdout = ONE JSON object mapping fixture id -> classification. Only
//! stdout carries JSON; all logging goes to stderr.
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

/// Locate the rendered `ai.SWAIG.functions` entry named `tool_name` and report
/// whether its `web_hook_url` carries the reserved `__token` query parameter (the
/// wire reflection of `secure`). Mirrors the oracle's `_webhook_has_token`.
fn webhook_has_token(doc: &Value, tool_name: &str) -> bool {
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
        .and_then(|f| f.get("web_hook_url"))
        .and_then(Value::as_str)
        .is_some_and(|url| url.contains("__token="))
}

/// Build a fresh agent, define one tool with the given secure state, render the
/// SWML, and reduce to the `{secure_default_true, wire_reflects_secure}` pair.
fn classify(tool_name: &str, secure: bool) -> Value {
    let mut agent = AgentBase::new(
        AgentOptions::new("secure-default-fixture")
            .route("/sd")
            .basic_auth("u", "p"),
    );
    agent.set_prompt_text("secure default fixture");
    agent.define_tool(
        tool_name,
        "secure-default fixture tool",
        json!({}),
        Box::new(|_args, _raw| FunctionResult::with_response("ok")),
        secure,
    );

    let doc = agent.render_swml(&HashMap::new());
    let token_present = webhook_has_token(&doc, tool_name);

    json!({
        "secure_default_true": secure,
        "wire_reflects_secure": token_present == secure,
    })
}

fn main() {
    let mut out = Map::new();

    // A1 (a) — a tool registered without an explicit opt-out is SECURE: its
    // rendered webhook must carry a `__token`. Reds a port that never mints one.
    out.insert(
        "define_tool_default_is_secure".to_string(),
        classify(DEFAULT_TOOL, true),
    );

    // A1 (b) — a tool registered `secure = false` is INSECURE: NO `__token`.
    // Reds a port that blindly tokenizes every tool regardless of `secure`.
    out.insert(
        "define_tool_explicit_insecure".to_string(),
        classify(INSECURE_TOOL, false),
    );

    println!("{}", Value::Object(out));
}
