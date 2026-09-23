// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `state_dump` — the Rust port's STATE dump program for the cross-port state
//! differ (porting-sdk/scripts/diff_port_state.py).
//!
//! For each `state_corpus` case it builds the target object, applies the mutation
//! chain via the Rust SDK's native API, reads the observable state through the
//! public accessor / rendered representation, and prints ONE JSON object mapping
//!
//!     case-id -> observed-state
//!
//! to stdout. The differ canonicalizes both sides and byte-compares against the
//! Python oracle. Only stdout carries the JSON object.
//!
//! Run from the signalwire-rust repo root:
//!
//!     cargo run --quiet --example state_dump

use std::collections::BTreeMap;

use serde_json::{Map, Value, json};
use signalwire::agent::AgentBase;
use signalwire::agent::AgentOptions;
use signalwire::prefabs::{InfoGathererAgent, InfoGathererOptions};
use signalwire::server::AgentServer;
use signalwire::skills::skill_base::SkillBase;
use signalwire::skills::skill_registry::SkillRegistry;
use signalwire::swml::handler::{SwmlVerbHandler, VerbHandlerRegistry};
use signalwire::swml::service::{Service, ServiceOptions};

/// A minimal custom verb handler — the Rust analog of the corpus's throwaway
/// `__register_verb__` handler.
#[derive(Clone)]
struct GreetVerbHandler;

impl SwmlVerbHandler for GreetVerbHandler {
    fn get_verb_name(&self) -> String {
        "greet".to_string()
    }
    fn validate_config(&self, _config: &Value) -> (bool, Vec<String>) {
        (true, Vec::new())
    }
    fn build_config(&self, args: &Map<String, Value>) -> Value {
        Value::Object(args.clone())
    }
    fn clone_box(&self) -> Box<dyn SwmlVerbHandler> {
        Box::new(self.clone())
    }
}

/// A minimal `SkillBase` stub — enough for `SkillRegistry::register_skill`'s
/// factory to produce something (the registry keys on the registered name).
struct StubSkill {
    name: String,
    description: String,
    params: Map<String, Value>,
}

impl SkillBase for StubSkill {
    fn name(&self) -> &str {
        &self.name
    }
    fn description(&self) -> &str {
        &self.description
    }
    fn setup(&mut self) -> bool {
        true
    }
    fn register_tools(&self, _agent: &mut AgentBase) {}
    fn params(&self) -> &Map<String, Value> {
        &self.params
    }
}

fn demo_agent() -> AgentBase {
    AgentBase::new(AgentOptions::new("demo").route("/demo"))
}

fn demo_service() -> Service {
    Service::new(ServiceOptions::new("svc").route("/svc"))
}

/// Builtin skill names auto-registered in the Rust global `SkillRegistry`. The
/// corpus builds a FRESH registry in Python (only the custom names present);
/// Rust's registry is a global singleton pre-seeded with builtins, so the
/// observable "what the chain added" is the registered set MINUS these.
const BUILTIN_SKILL_NAMES: &[&str] = &[
    "api_ninjas_trivia",
    "claude_skills",
    "custom_skills",
    "datasphere",
    "datasphere_serverless",
    "datetime",
    "google_maps",
    "info_gatherer",
    "joke",
    "math",
    "mcp_gateway",
    "native_vector_search",
    "play_background_file",
    "spider",
    "swml_transfer",
    "weather_api",
    "web_search",
    "wikipedia_search",
];

fn main() {
    let mut out: BTreeMap<&str, Value> = BTreeMap::new();

    // ---- global_data: set MERGES into the accumulated global data ----
    {
        let mut a = demo_agent();
        a.set_global_data(json!({"company": "SignalWire", "tier": "gold"}));
        out.insert("state_set_global_data", a.get_global_data());
    }
    {
        let mut a = demo_agent();
        a.update_global_data(json!({"k1": "v1"}));
        a.update_global_data(json!({"k2": "v2"}));
        out.insert("state_update_global_data", a.get_global_data());
    }
    {
        // MERGE semantics: overlapping key wins, sibling survives.
        let mut a = demo_agent();
        a.set_global_data(json!({"a": 1, "b": 2}));
        a.set_global_data(json!({"b": 99, "c": 3}));
        out.insert("state_global_data_merge", a.get_global_data());
    }

    // ---- sip-username registration on AgentBase (lowercased set) ----
    {
        let mut a = demo_agent();
        a.register_sip_username("Bob", "");
        a.register_sip_username("alice", "");
        out.insert("state_register_sip_username", json!(a.sip_usernames()));
    }
    {
        // dedup + case-fold: "Bob","BOB","bob" collapse to one.
        let mut a = demo_agent();
        a.register_sip_username("Bob", "");
        a.register_sip_username("BOB", "");
        a.register_sip_username("bob", "");
        out.insert(
            "state_register_sip_username_dedup",
            json!(a.sip_usernames()),
        );
    }

    // ---- AgentServer sip-username mapping (username -> route) + lookup ----
    {
        let mut s = AgentServer::new(None, None);
        s.setup_sip_routing();
        s.register_sip_username("Bob", "/agent");
        s.register_sip_username("sales", "/sales");
        let mapping: BTreeMap<String, String> = s
            .sip_username_mapping()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let lookup = |u: &str| -> Value {
            s.sip_username_mapping()
                .get(&u.to_lowercase())
                .map_or(Value::Null, |r| json!(r))
        };
        out.insert(
            "server_sip_username_mapping",
            json!({
                "mapping": mapping,
                "lookup_bob": lookup("bob"),
                "lookup_BOB": lookup("BOB"),
                "lookup_missing": lookup("nope"),
            }),
        );
    }
    {
        // unregister removes the agent route from the registry.
        let mut s = AgentServer::new(None, None);
        s.register(
            AgentBase::new(AgentOptions::new("agent").route("/agent")),
            Some("/agent"),
        )
        .expect("register /agent");
        s.register(
            AgentBase::new(AgentOptions::new("other").route("/other")),
            Some("/other"),
        )
        .expect("register /other");
        s.unregister("/agent");
        out.insert("server_unregister", json!(s.get_agents()));
    }

    // ---- routing-callback registration on SWMLService (path-normalized) ----
    {
        let mut svc = demo_service();
        svc.register_routing_callback(|_body, _headers| None, Some("/sip/"));
        svc.register_routing_callback(|_body, _headers| None, Some("voice"));
        out.insert(
            "state_register_routing_callback",
            json!(svc.routing_callback_paths()),
        );
    }

    // ---- verb-handler registration (registry ships with "ai") ----
    {
        let mut reg = VerbHandlerRegistry::new();
        reg.register_handler(Box::new(GreetVerbHandler));
        out.insert(
            "state_register_verb_handler",
            json!({
                "verbs": reg.handler_names(),
                "has_greet": reg.has_handler("greet"),
                "has_ai": reg.has_handler("ai"),
                "has_missing": reg.has_handler("nope"),
            }),
        );
    }

    // ---- skill registration (global registry; observe the delta) ----
    {
        let factory = |name: &'static str| {
            let f: signalwire::skills::skill_registry::SkillFactory =
                Box::new(move |params: Map<String, Value>| -> Box<dyn SkillBase> {
                    Box::new(StubSkill {
                        name: name.to_string(),
                        description: "corpus stub skill".to_string(),
                        params,
                    })
                });
            f
        };
        SkillRegistry::register_skill("custom_alpha", factory("custom_alpha"));
        SkillRegistry::register_skill("custom_beta", factory("custom_beta"));
        SkillRegistry::register_skill("custom_alpha", factory("custom_alpha")); // idempotent
        let added: Vec<String> = SkillRegistry::list_skills()
            .into_iter()
            .filter(|n| !BUILTIN_SKILL_NAMES.contains(&n.as_str()))
            .collect();
        out.insert("state_register_skill", json!(added));
    }

    // ---- InfoGatherer.submit_answer: records answer + advances index ----
    {
        let ig = InfoGathererAgent::new(
            InfoGathererOptions::new()
                .name("demo")
                .route("/demo")
                .questions(questions()),
        );
        out.insert(
            "infogatherer_submit_answer_first",
            submit_answer_delta(
                &ig,
                &json_map(json!({"answer": "Alice"})),
                &json_map(json!({"global_data": {
                    "questions": questions(),
                    "question_index": 0,
                    "answers": [],
                }})),
            ),
        );
    }
    {
        let ig = InfoGathererAgent::new(
            InfoGathererOptions::new()
                .name("demo")
                .route("/demo")
                .questions(questions()),
        );
        out.insert(
            "infogatherer_submit_answer_last",
            submit_answer_delta(
                &ig,
                &json_map(json!({"answer": "a@b.com"})),
                &json_map(json!({"global_data": {
                    "questions": questions(),
                    "question_index": 1,
                    "answers": [{"key_name": "name", "answer": "Alice"}],
                }})),
            ),
        );
    }

    // ---- contexts/steps navigation (valid_steps rendered per step) ----
    {
        let mut a = demo_agent();
        let cb = a.define_contexts();
        let ctx = cb.add_context("default");
        ctx.add_step("greet")
            .set_text("Greet the caller.")
            .set_valid_steps(vec!["collect"]);
        ctx.add_step("collect")
            .set_text("Collect their info.")
            .set_valid_steps(vec!["greet"]);
        out.insert("state_contexts_navigation", contexts_nav(&cb.to_value()));
    }

    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize state dump")
    );
}

/// The two corpus questions.
fn questions() -> Vec<Value> {
    vec![
        json!({"key_name": "name", "question_text": "What is your name?"}),
        json!({"key_name": "email", "question_text": "What is your email?"}),
    ]
}

fn json_map(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

/// Drive `submit_answer` and reduce to the observable delta (mirrors
/// `diff_port_state._observe` "`submit_answer_delta`"): the `set_global_data`
/// action's `question_index` + `answers`, plus a `done` flag derived from the
/// completion message.
fn submit_answer_delta(
    ig: &InfoGathererAgent,
    args: &Map<String, Value>,
    raw_data: &Map<String, Value>,
) -> Value {
    let result = ig.submit_answer(args, raw_data);
    let value = result.to_value();
    // Find the set_global_data action.
    let gd = value
        .get("action")
        .and_then(Value::as_array)
        .and_then(|actions| {
            actions
                .iter()
                .find_map(|a| a.get("set_global_data").cloned())
        })
        .unwrap_or(Value::Null);
    let response = value.get("response").and_then(Value::as_str).unwrap_or("");
    json!({
        "question_index": gd.get("question_index").cloned().unwrap_or(Value::Null),
        "answers": gd.get("answers").cloned().unwrap_or(Value::Null),
        "done": response.contains("All questions have been answered"),
    })
}

/// Reduce a rendered contexts doc to per-context `{name, valid_steps}`.
fn contexts_nav(rendered: &Value) -> Value {
    let mut nav = Map::new();
    if let Some(obj) = rendered.as_object() {
        for (cname, cdoc) in obj {
            let steps: Vec<Value> = cdoc
                .get("steps")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .map(|s| {
                            json!({
                                "name": s.get("name").cloned().unwrap_or(Value::Null),
                                "valid_steps": s.get("valid_steps").cloned().unwrap_or(Value::Null),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            nav.insert(cname.clone(), Value::Array(steps));
        }
    }
    Value::Object(nav)
}
