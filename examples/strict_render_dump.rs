// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `strict_render_dump` — the Rust port's SWML STRICT-RENDER dump program for
//! the cross-port strict-render differ
//! (`porting-sdk/scripts/diff_port_strict_render.py`).
//!
//! The strict-render contract: building an SWML document with a MISSHAPEN
//! config, an UNKNOWN verb, or a MISSPELLED/unknown key must FAIL (in Rust:
//! panic from the fail-loud `add_verb`, or `Err` from
//! `ContextBuilder::validate`) — not silently drop/accept it. A VALID build
//! must still succeed.
//!
//! For each `strict_render_corpus` case this program builds the case in the
//! Rust idiom, maps a build FAILURE (a caught panic from the fail-loud
//! `SWMLService::add_verb`, or an `Err` from `ContextBuilder::validate`) to
//! `"raised"` and a clean build to `"ok"`, and emits ONE JSON object mapping
//!
//!     case-id -> "raised" | "ok"
//!
//! to stdout (JSON only). The differ compares each outcome against the python
//! oracle; a planted-bad ("raise") case reported "ok" reds the gate.
//!
//! Run from the signalwire-rust repo root:
//!
//!     cargo run --quiet --example strict_render_dump 2>/dev/null
//!
//! The corpus is pinned in the differ; this program hard-codes the same 18
//! cases in the Rust idiom (there is no shared corpus loader for compiled
//! ports — each port's dump interprets the corpus against its own API, exactly
//! as the `swml_dump` / `doc_wire_dump` programs do).

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::{Value, json};
use signalwire::swaig::FunctionResult;
use signalwire::swml::service::ServiceOptions;
use signalwire::{AgentBase, AgentOptions, SWMLService};

/// Run one verb-level (`SWMLService`) case: construct a service with validation
/// ON, run the `add_verb` chain, and return `"raised"` if any call fails
/// (panics — `add_verb` is fail-loud, mirroring Python's
/// `SchemaValidationError`) or `"ok"` if the whole chain succeeds.
fn run_service_case(build: impl FnOnce(&mut SWMLService)) -> &'static str {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut svc = SWMLService::new(ServiceOptions::new("s").route("/s"));
        build(&mut svc);
    }));
    if result.is_ok() { "ok" } else { "raised" }
}

/// Run one contexts-level (`AgentBase`) case: build the agent + contexts, then
/// validate. Returns `"raised"` if validation fails (`Err`) or the build panics,
/// `"ok"` if the document validates clean.
fn run_agent_case(build: impl FnOnce() -> Result<(), ()>) -> &'static str {
    match catch_unwind(AssertUnwindSafe(build)) {
        Ok(Ok(())) => "ok",
        _ => "raised",
    }
}

/// A no-op SWAIG handler for `define_tool` (the corpus only cares that the
/// tool NAME is registered, not what it returns).
fn noop_tool(agent: &mut AgentBase, name: &str) {
    agent.define_tool(
        name,
        "corpus tool",
        json!({}),
        Box::new(|_args, _raw| FunctionResult::with_response("ok")),
        false,
    );
}

fn main() {
    // Silence the default panic hook so a caught fail-loud `add_verb` panic
    // (an EXPECTED "raised" outcome) does not print a backtrace to stderr and
    // muddy the run. stdout stays JSON-only regardless.
    std::panic::set_hook(Box::new(|_| {}));

    let mut out: BTreeMap<&str, &str> = BTreeMap::new();

    // ================================================================
    // Verb-level strict render (SWMLService, validation ON)
    // ================================================================
    out.insert(
        "strict_unknown_verb",
        run_service_case(|s| {
            s.add_verb("foobar", json!({}));
        }),
    );
    out.insert(
        "strict_answer_misspelled_key",
        run_service_case(|s| {
            s.add_verb("answer", json!({"maxduration": 5}));
        }),
    );
    out.insert(
        "strict_answer_unknown_key",
        run_service_case(|s| {
            s.add_verb("answer", json!({"wibble": 1}));
        }),
    );
    out.insert(
        "strict_play_misspelled_key",
        run_service_case(|s| {
            s.add_verb("play", json!({"urlz": ["say:hi"]}));
        }),
    );
    out.insert(
        "strict_play_valid_plus_unknown_key",
        run_service_case(|s| {
            s.add_verb("play", json!({"url": "say:hi", "foo": 1}));
        }),
    );
    out.insert(
        "strict_record_misspelled_key",
        run_service_case(|s| {
            s.add_verb("record", json!({"formatt": "wav"}));
        }),
    );
    out.insert(
        "strict_answer_wrong_type",
        run_service_case(|s| {
            s.add_verb("answer", json!({"max_duration": "notanumber"}));
        }),
    );
    // ai verb: unknown/misspelled TOP-LEVEL keys must raise (ai.params open).
    out.insert(
        "strict_ai_misspelled_top_key",
        run_service_case(|s| {
            s.add_verb("ai", json!({"prompt": {"text": "hi"}, "temperatur": 0.5}));
        }),
    );
    out.insert(
        "strict_ai_unknown_top_key",
        run_service_case(|s| {
            s.add_verb("ai", json!({"prompt": {"text": "hi"}, "zzz": 1}));
        }),
    );
    out.insert(
        "strict_ai_missing_prompt",
        run_service_case(|s| {
            s.add_verb("ai", json!({"post_prompt": {"text": "bye"}}));
        }),
    );
    // Good documents must still render (regression guard).
    out.insert(
        "strict_answer_ok",
        run_service_case(|s| {
            s.add_verb("answer", json!({"max_duration": 5}));
        }),
    );
    out.insert(
        "strict_play_ok",
        run_service_case(|s| {
            s.add_verb("play", json!({"url": "say:hi"}));
        }),
    );
    out.insert(
        "strict_ai_ok",
        run_service_case(|s| {
            s.add_verb("ai", json!({"prompt": {"text": "hi"}}));
        }),
    );
    out.insert(
        "strict_ai_params_open_ok",
        run_service_case(|s| {
            s.add_verb(
                "ai",
                json!({"prompt": {"text": "hi"}, "params": {"some_future_param": 1}}),
            );
        }),
    );

    // ================================================================
    // Contexts-level strict render (AgentBase; dangling refs)
    // ================================================================
    // dangling step-function reference (GAP2 / r5 F3).
    out.insert(
        "strict_dangling_step_function",
        run_agent_case(|| {
            let mut agent = AgentBase::new(AgentOptions::new("a").route("/a"));
            noop_tool(&mut agent, "order_status");
            {
                let cb = agent.define_contexts();
                let ctx = cb.add_context("default");
                let st = ctx.add_step("help");
                st.set_text("help");
                st.set_functions(json!(["order_status", "get_datetime"]));
            }
            agent.refresh_context_tools();
            agent.define_contexts().validate().map_err(|_| ())
        }),
    );
    out.insert(
        "strict_registered_step_function_ok",
        run_agent_case(|| {
            let mut agent = AgentBase::new(AgentOptions::new("a").route("/a"));
            noop_tool(&mut agent, "order_status");
            {
                let cb = agent.define_contexts();
                let ctx = cb.add_context("default");
                let st = ctx.add_step("help");
                st.set_text("help");
                st.set_functions(json!(["order_status"]));
            }
            agent.refresh_context_tools();
            agent.define_contexts().validate().map_err(|_| ())
        }),
    );
    out.insert(
        "strict_reserved_native_function_ok",
        run_agent_case(|| {
            let mut agent = AgentBase::new(AgentOptions::new("a").route("/a"));
            {
                let cb = agent.define_contexts();
                let ctx = cb.add_context("default");
                let st = ctx.add_step("help");
                st.set_text("help");
                st.set_functions(json!(["next_step", "change_context"]));
            }
            agent.define_contexts().validate().map_err(|_| ())
        }),
    );
    // dangling valid_contexts reference (already enforced; pins it).
    out.insert(
        "strict_dangling_valid_context",
        run_agent_case(|| {
            let mut agent = AgentBase::new(AgentOptions::new("a").route("/a"));
            {
                let cb = agent.define_contexts();
                let ctx = cb.add_context("default");
                let st = ctx.add_step("help");
                st.set_text("help");
                st.set_valid_contexts(vec!["nowhere"]);
            }
            agent.define_contexts().validate().map_err(|_| ())
        }),
    );

    // JSON-only on stdout.
    let obj: serde_json::Map<String, Value> = out
        .into_iter()
        .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
        .collect();
    println!(
        "{}",
        serde_json::to_string(&Value::Object(obj)).expect("serialize strict-render dump")
    );
}
