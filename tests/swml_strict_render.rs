// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! SWML STRICT-RENDER native test suite (Wave-2 P#5).
//!
//! Ports the python reference's `tests/unit/core/test_swml_strict_render.py`
//! (signalwire-python `wave/2-aplus` 045919a) into Rust's native `#[test]`
//! harness. The strict-render contract: building an SWML document with a
//! MISSHAPEN config, an UNKNOWN verb, or a MISSPELLED/unknown key must FAIL
//! (in Rust: the fail-loud `SWMLService::add_verb` panics, or
//! `ContextBuilder::validate` returns `Err`) — not silently drop/accept it. A
//! VALID build must still succeed.
//!
//! These are the same 18 cases pinned in
//! `porting-sdk/scripts/strict_render_corpus.py` and driven cross-port by
//! `diff_port_strict_render.py`; here they are asserted directly against the Rust
//! API so a regression reds `cargo test` on its own, independent of the
//! cross-port gate.

use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::json;
use signalwire::swaig::FunctionResult;
use signalwire::swml::service::ServiceOptions;
use signalwire::{AgentBase, AgentOptions, SWMLService};

/// `true` if adding `verb`/`config` to a validation-ON `SWMLService` FAILS
/// (the fail-loud `add_verb` panics with a schema/unknown-verb error).
fn verb_raises(verb: &str, config: serde_json::Value) -> bool {
    // Suppress the panic backtrace for the EXPECTED-failure cases.
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let result = catch_unwind(AssertUnwindSafe(|| {
        let mut svc = SWMLService::new(ServiceOptions::new("s").route("/s"));
        svc.add_verb(verb, config);
    }));
    std::panic::set_hook(prev);
    result.is_err()
}

// ── Verb-level: unknown / misspelled verb ──────────────────────────────────

#[test]
fn strict_unknown_verb() {
    assert!(
        verb_raises("foobar", json!({})),
        "unknown verb 'foobar' must raise, not be appended silently"
    );
}

// ── Verb-level: misspelled / unknown config key on a CLOSED verb ────────────

#[test]
fn strict_answer_misspelled_key() {
    assert!(
        verb_raises("answer", json!({"maxduration": 5})),
        "misspelled 'maxduration' (should be max_duration) must raise"
    );
}

#[test]
fn strict_answer_unknown_key() {
    assert!(
        verb_raises("answer", json!({"wibble": 1})),
        "unknown key 'wibble' on a closed verb must raise"
    );
}

#[test]
fn strict_play_misspelled_key() {
    assert!(
        verb_raises("play", json!({"urlz": ["say:hi"]})),
        "misspelled 'urlz' (should be urls) must raise"
    );
}

#[test]
fn strict_play_valid_plus_unknown_key() {
    assert!(
        verb_raises("play", json!({"url": "say:hi", "foo": 1})),
        "a valid key plus an unknown extra key still must raise"
    );
}

#[test]
fn strict_record_misspelled_key() {
    assert!(
        verb_raises("record", json!({"formatt": "wav"})),
        "misspelled 'formatt' (should be format) must raise"
    );
}

// ── Verb-level: wrong-typed config ─────────────────────────────────────────

#[test]
fn strict_answer_wrong_type() {
    assert!(
        verb_raises("answer", json!({"max_duration": "notanumber"})),
        "max_duration must be numeric; a string must raise"
    );
}

// ── The ai verb: unknown/misspelled TOP-LEVEL keys (GAP1) ──────────────────
// The specialized AiVerbHandler validates prompt/SWAIG shape; the schema pass
// (run after the handler) rejects unknown/misspelled top-level ai keys because
// the AIObject schema is closed via `unevaluatedProperties`.

#[test]
fn strict_ai_misspelled_top_key() {
    assert!(
        verb_raises("ai", json!({"prompt": {"text": "hi"}, "temperatur": 0.5})),
        "GAP1: misspelled top-level ai key 'temperatur' must raise"
    );
}

#[test]
fn strict_ai_unknown_top_key() {
    assert!(
        verb_raises("ai", json!({"prompt": {"text": "hi"}, "zzz": 1})),
        "GAP1: unknown top-level ai key 'zzz' must raise"
    );
}

#[test]
fn strict_ai_missing_prompt() {
    assert!(
        verb_raises("ai", json!({"post_prompt": {"text": "bye"}})),
        "the ai verb requires a prompt; omitting it must raise"
    );
}

// ── Good documents must still render (regression guard) ────────────────────

#[test]
fn strict_answer_ok() {
    assert!(
        !verb_raises("answer", json!({"max_duration": 5})),
        "a valid answer verb must render"
    );
}

#[test]
fn strict_play_ok() {
    assert!(
        !verb_raises("play", json!({"url": "say:hi"})),
        "a valid play verb must render"
    );
}

#[test]
fn strict_ai_ok() {
    assert!(
        !verb_raises("ai", json!({"prompt": {"text": "hi"}})),
        "a valid ai verb must render"
    );
}

#[test]
fn strict_ai_params_open_ok() {
    // ai.params is the DELIBERATE open door for LLM tuning; a key inside it is
    // not a misspelling and must render (AIParams is open in the schema).
    assert!(
        !verb_raises(
            "ai",
            json!({"prompt": {"text": "hi"}, "params": {"some_future_param": 1}}),
        ),
        "ai.params extras must render (open door)"
    );
}

// ── Contexts-level: dangling step-function reference (GAP2 / r5 F3) ─────────

fn noop_tool(agent: &mut AgentBase, name: &str) {
    agent.define_tool(
        name,
        "corpus tool",
        json!({}),
        Box::new(|_args, _raw| FunctionResult::with_response("ok")),
        false,
    );
}

#[test]
fn strict_dangling_step_function() {
    let mut agent = AgentBase::new(AgentOptions::new("a").route("/a"));
    noop_tool(&mut agent, "order_status");
    {
        let cb = agent.define_contexts();
        let st = cb.add_context("default").add_step("help");
        st.set_text("help");
        st.set_functions(json!(["order_status", "get_datetime"]));
    }
    agent.refresh_context_tools();
    assert!(
        agent.define_contexts().validate().is_err(),
        "GAP2/F3: set_functions references 'get_datetime' (not a registered tool \
         nor a reserved native) — dangling ref must raise"
    );
}

#[test]
fn strict_registered_step_function_ok() {
    let mut agent = AgentBase::new(AgentOptions::new("a").route("/a"));
    noop_tool(&mut agent, "order_status");
    {
        let cb = agent.define_contexts();
        let st = cb.add_context("default").add_step("help");
        st.set_text("help");
        st.set_functions(json!(["order_status"]));
    }
    agent.refresh_context_tools();
    assert!(
        agent.define_contexts().validate().is_ok(),
        "a step referencing a registered tool must render"
    );
}

#[test]
fn strict_reserved_native_function_ok() {
    let mut agent = AgentBase::new(AgentOptions::new("a").route("/a"));
    {
        let cb = agent.define_contexts();
        let st = cb.add_context("default").add_step("help");
        st.set_text("help");
        st.set_functions(json!(["next_step", "change_context"]));
    }
    assert!(
        agent.define_contexts().validate().is_ok(),
        "reserved native tools (next_step/change_context) are not dangling"
    );
}

// ── Contexts-level: dangling valid_contexts reference (pins existing check) ─

#[test]
fn strict_dangling_valid_context() {
    let mut agent = AgentBase::new(AgentOptions::new("a").route("/a"));
    {
        let cb = agent.define_contexts();
        let st = cb.add_context("default").add_step("help");
        st.set_text("help");
        st.set_valid_contexts(vec!["nowhere"]);
    }
    assert!(
        agent.define_contexts().validate().is_err(),
        "valid_contexts references an undefined context — must raise"
    );
}
