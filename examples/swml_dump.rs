// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `swml_dump` — the Rust port's SWML dump program for the cross-port SWML
//! differ (porting-sdk/scripts/diff_port_swml.py).
//!
//! For each `swml_corpus` case it builds an `AgentBase`, applies the setter
//! chain, renders the SWML document, and extracts the observed dotted path
//! (e.g. `ai.prompt.pom`) — emitting ONE JSON object mapping
//!
//!     case-id -> extracted-fragment
//!
//! to stdout. The differ canonicalizes both sides and byte-compares against the
//! Python oracle. Only stdout carries the JSON object.
//!
//! Run from the signalwire-rust repo root:
//!
//!     cargo run --quiet --example swml_dump

use std::collections::{BTreeMap, HashMap};

use serde_json::{Value, json};
use signalwire::swaig::FunctionResult;
use signalwire::{AgentBase, AgentOptions};

/// A demo agent ("demo" at "/demo") with POM enabled (the default) so
/// `prompt_add_section` renders into `ai.prompt.pom`, matching the oracle.
fn new_agent() -> AgentBase {
    AgentBase::new(AgentOptions::new("demo").route("/demo").use_pom(true))
}

/// Render the SWML doc with no request headers.
fn render(agent: &AgentBase) -> Value {
    agent.render_swml(&HashMap::new())
}

/// Walk a dotted path into a rendered SWML doc. `ai.prompt` means: find the
/// `ai` verb in `sections.main`, wrap it as `{"ai": ...}`, then index — the
/// Rust mirror of `diff_port_swml._extract`.
fn extract(doc: &Value, path: &str) -> Value {
    let ai = doc
        .get("sections")
        .and_then(|s| s.get("main"))
        .and_then(Value::as_array)
        .and_then(|main| main.iter().find_map(|sec| sec.get("ai")));

    let mut node = ai.map_or_else(|| doc.clone(), |ai| json!({ "ai": ai }));
    for part in path.split('.') {
        node = node.get(part).cloned().unwrap_or(Value::Null);
    }
    node
}

/// From a rendered `ai.SWAIG.functions` array, pick the entry whose
/// `function` equals `name` and return its parameters schema (mirrors the
/// oracle's `swaig_fn`/`field: "parameters"` filter in
/// `diff_port_swml.build_oracle`).
///
/// This port natively serializes the SWAIG parameter schema under the key
/// `argument` (the `purpose`/`argument` wire-idiom documented in
/// porting-sdk/CONFORMANCE_AUDIT.md §3.1 — Java/PHP/Rust/.NET), while the
/// Python oracle + spec use `parameters`. The observed artifact is the SCHEMA
/// VALUE, not the key it hangs under; per `PORTING_GUIDE` (a consumer accepts
/// `parameters|argument` interchangeably) we read `parameters` first and fall
/// back to `argument`, so the byte-compared value is the flat schema either
/// way.
fn swaig_fn_parameters(functions: &Value, name: &str) -> Value {
    functions
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|f| f.get("function").and_then(Value::as_str) == Some(name))
        })
        .and_then(|f| f.get("parameters").or_else(|| f.get("argument")).cloned())
        .unwrap_or(Value::Null)
}

/// Reduce a map fragment to the listed keys (mirrors the oracle's `pick`).
fn pick(frag: &Value, keys: &[&str]) -> Value {
    let Some(obj) = frag.as_object() else {
        return frag.clone();
    };
    let mut out = serde_json::Map::new();
    for &k in keys {
        out.insert(k.to_string(), obj.get(k).cloned().unwrap_or(Value::Null));
    }
    Value::Object(out)
}

fn main() {
    let mut out: BTreeMap<&str, Value> = BTreeMap::new();

    // swml_set_prompt_llm_params: two set_prompt_llm_params calls MERGE.
    {
        let mut a = new_agent();
        a.set_prompt_llm_params(json!({"temperature": 0.5}));
        a.set_prompt_llm_params(json!({"top_p": 0.9}));
        out.insert(
            "swml_set_prompt_llm_params",
            pick(
                &extract(&render(&a), "ai.prompt"),
                &["temperature", "top_p"],
            ),
        );
    }

    // swml_set_post_prompt_llm_params: establish a post-prompt, then merge.
    {
        let mut a = new_agent();
        a.set_post_prompt("Summarize the call.");
        a.set_post_prompt_llm_params(json!({"temperature": 0.3}));
        a.set_post_prompt_llm_params(json!({"top_p": 0.8}));
        out.insert(
            "swml_set_post_prompt_llm_params",
            pick(
                &extract(&render(&a), "ai.post_prompt"),
                &["temperature", "top_p"],
            ),
        );
    }

    // swml_add_language: engine/model/voice carried into ai.languages. Rust's
    // builder idiom takes the core three args then attaches engine/model via
    // the fluent set_language_* setters.
    {
        let mut a = new_agent();
        a.add_language("English", "en-US", "rime.spore")
            .set_language_engine("rime")
            .set_language_model("mistv2");
        out.insert("swml_add_language", extract(&render(&a), "ai.languages"));
    }

    // swml_add_pattern_hint: structured hint into ai.hints. Rust seeds the
    // entry from `pattern`, then refines hint/replace/ignore_case.
    {
        let mut a = new_agent();
        a.add_pattern_hint("signal wire")
            .set_pattern_hint_hint("SignalWire")
            .set_pattern_hint_replace("SignalWire")
            .set_pattern_hint_ignore_case(true);
        out.insert("swml_add_pattern_hint", extract(&render(&a), "ai.hints"));
    }

    // swml_add_hint: a plain string hint.
    {
        let mut a = new_agent();
        a.add_hint("SignalWire");
        out.insert("swml_add_hint", extract(&render(&a), "ai.hints"));
    }

    // swml_prompt_add_section: POM sections render into ai.prompt.pom.
    {
        let mut a = new_agent();
        a.prompt_add_section("Role", "You are a helpful assistant.", vec![]);
        a.prompt_add_section("Rules", "", vec!["Be concise", "Be accurate"]);
        out.insert(
            "swml_prompt_add_section",
            extract(&render(&a), "ai.prompt.pom"),
        );
    }

    // swml_add_pronunciation: renders into ai.pronounce.
    {
        let mut a = new_agent();
        a.add_pronunciation("SW", "SignalWire", Some(true));
        out.insert(
            "swml_add_pronunciation",
            extract(&render(&a), "ai.pronounce"),
        );
    }

    // swml_define_tool_complete_schema: define_tool with a COMPLETE
    // {type, properties, required} schema must PASS THROUGH to
    // ai.SWAIG.functions[lookup].parameters as that schema FLAT (not
    // double-wrapped). Mirrors Python's `_ensure_parameter_structure`, which
    // returns the schema unchanged when it already has type+properties.
    {
        let mut a = new_agent();
        a.define_tool(
            "lookup",
            "Look up a thing",
            json!({
                "type": "object",
                "properties": {"q": {"type": "string"}},
                "required": ["q"],
            }),
            Box::new(|_args, _raw| FunctionResult::with_response("ok")),
            false,
        );
        let functions = extract(&render(&a), "ai.SWAIG.functions");
        out.insert(
            "swml_define_tool_complete_schema",
            swaig_fn_parameters(&functions, "lookup"),
        );
    }

    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize swml dump")
    );
}
