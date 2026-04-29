// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `skills_audit_harness` — runtime probe for the skills system.
//!
//! Driven by porting-sdk's `audit_skills_dispatch.py`. Reads:
//!   - SKILL_NAME            e.g. `web_search`, `datasphere`
//!   - SKILL_FIXTURE_URL     `http://127.0.0.1:NNNN`
//!   - SKILL_HANDLER_ARGS    JSON dict of args for the skill handler
//!   - per-skill upstream env (e.g. WEB_SEARCH_BASE_URL); the audit
//!     sets these to point the skill at its loopback fixture
//!   - per-skill credential env vars (e.g. GOOGLE_API_KEY)
//!
//! For handler-based skills (`web_search`, `wikipedia_search`,
//! `datasphere`, `spider`) the harness instantiates the skill, registers
//! its tools on a temporary AgentBase, and dispatches the documented
//! tool name with the parsed args. The skill's handler issues real
//! HTTP through ureq (proven by the audit's fixture seeing the request).
//!
//! For DataMap-based skills (`api_ninjas_trivia`, `weather_api`) the
//! SignalWire platform — not the SDK — would normally fetch the
//! configured webhook URL. The harness simulates that platform behavior
//! by extracting the webhook URL from the registered DataMap and
//! issuing the HTTP call itself, satisfying the audit's contract that
//! "the SDK contacted the upstream" via real bytes on the wire.

use serde_json::{json, Map, Value};
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::skills::skill_registry::SkillRegistry;
use std::collections::HashMap;
use std::env;
use std::process;
use std::time::Duration;

fn main() {
    if env::var("SIGNALWIRE_LOG_MODE").is_err() {
        unsafe {
            env::set_var("SIGNALWIRE_LOG_MODE", "off");
        }
    }

    let skill_name = env::var("SKILL_NAME").unwrap_or_else(|_| die("SKILL_NAME required"));
    let args_raw = env::var("SKILL_HANDLER_ARGS").unwrap_or_else(|_| "{}".to_string());
    let args: Value = serde_json::from_str(&args_raw)
        .unwrap_or_else(|e| die(&format!("SKILL_HANDLER_ARGS not JSON: {}", e)));

    // Wire skill-specific construction params from the audit-mandated
    // env vars (mirrors what a deployed agent would read).
    let mut skill_params: Map<String, Value> = Map::new();
    match skill_name.as_str() {
        "web_search" => {
            // Audit sets GOOGLE_API_KEY / GOOGLE_CSE_ID and
            // WEB_SEARCH_BASE_URL. Pull the credentials into params so
            // the skill's setup() succeeds.
            if let Ok(k) = env::var("GOOGLE_API_KEY") {
                skill_params.insert("api_key".to_string(), json!(k));
            }
            if let Ok(c) = env::var("GOOGLE_CSE_ID") {
                skill_params.insert("search_engine_id".to_string(), json!(c));
            }
        }
        "datasphere" => {
            // Audit sets DATASPHERE_TOKEN and DATASPHERE_BASE_URL.
            // We need to plug a synthetic project_id / space_name /
            // document_id so setup() validates — the actual upstream
            // call uses DATASPHERE_BASE_URL not the space.
            skill_params.insert("space_name".to_string(), json!("audit-space"));
            skill_params.insert("project_id".to_string(), json!("audit-project"));
            skill_params.insert("document_id".to_string(), json!("audit-doc"));
            if let Ok(t) = env::var("DATASPHERE_TOKEN") {
                skill_params.insert("token".to_string(), json!(t));
            }
        }
        "weather_api" => {
            if let Ok(k) = env::var("WEATHER_API_KEY") {
                skill_params.insert("api_key".to_string(), json!(k));
            }
        }
        "api_ninjas_trivia" => {
            if let Ok(k) = env::var("API_NINJAS_KEY") {
                skill_params.insert("api_key".to_string(), json!(k));
            }
        }
        _ => {}
    }

    // Build the skill from the registry. We instantiate the skill
    // straight, register its tools on a temporary AgentBase, then drive
    // the tool name we know each skill exposes.
    let factory = SkillRegistry::get_factory(&skill_name)
        .unwrap_or_else(|| die(&format!("skill '{}' not registered", skill_name)));
    let mut skill = factory(skill_params);

    if !skill.setup() {
        die(&format!("skill '{}' setup() returned false", skill_name));
    }

    let mut agent_opts = AgentOptions::new("skills-audit");
    agent_opts.route = Some("/audit".to_string());
    let mut agent = AgentBase::new(agent_opts);
    skill.register_tools(&mut agent);

    let result = match skill_name.as_str() {
        "web_search" => dispatch_handler(&agent, "web_search", &args),
        "wikipedia_search" => dispatch_handler(&agent, "search_wiki", &args),
        "datasphere" => dispatch_handler(&agent, "search_knowledge", &args),
        "spider" => dispatch_handler(&agent, "scrape_url", &args),
        "weather_api" => execute_datamap(&agent, "get_weather", &args),
        "api_ninjas_trivia" => {
            // The audit doesn't pass a `category` arg but the skill
            // requires one — the API Ninjas endpoint accepts a
            // wildcard request without a category, returning a random
            // question. We synthesize an empty category for the URL
            // template so the harness can still issue a real GET.
            let mut effective = args.clone();
            if effective
                .as_object()
                .map(|o| !o.contains_key("category"))
                .unwrap_or(true)
            {
                if let Some(o) = effective.as_object_mut() {
                    o.insert("category".to_string(), json!("general"));
                }
            }
            execute_datamap(&agent, "get_trivia", &effective)
        }
        other => Err(format!("unsupported skill '{}'", other)),
    };

    match result {
        Ok(v) => {
            println!("{}", serde_json::to_string(&v).unwrap_or_default());
            process::exit(0);
        }
        Err(e) => die(&e),
    }
}

/// Dispatch a handler-based tool through the agent's SWAIG dispatcher.
/// The SWAIG dispatcher invokes the handler, which issues a real HTTP
/// request to the configured upstream (the audit fixture).
fn dispatch_handler(agent: &AgentBase, tool_name: &str, args: &Value) -> Result<Value, String> {
    let args_map: Map<String, Value> = args
        .as_object()
        .cloned()
        .unwrap_or_default();
    let raw_data: Map<String, Value> = Map::new();

    let r = agent
        .on_function_call(tool_name, &args_map, &raw_data)
        .ok_or_else(|| format!("handler '{}' not registered or returned None", tool_name))?;
    Ok(r.to_value())
}

/// For DataMap-based tools, extract the webhook URL from the registered
/// DataMap config and execute the GET ourselves. This is what the
/// SignalWire platform does in production; the audit verifies the URL
/// shape and the SDK's parsing.
fn execute_datamap(agent: &AgentBase, tool_name: &str, args: &Value) -> Result<Value, String> {
    let definition = agent
        .tool_definition(tool_name)
        .ok_or_else(|| format!("tool '{}' not registered", tool_name))?;

    let webhook = definition
        .get("data_map")
        .and_then(|d| d.get("webhooks"))
        .and_then(|w| w.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| format!("tool '{}' has no DataMap webhook", tool_name))?;

    let url_template = webhook
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("tool '{}' webhook has no url", tool_name))?;
    let method = webhook
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let extra_headers = webhook
        .get("headers")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let url = expand_template(url_template, args);

    let agent_http: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .build()
        .into();

    let mut headers: HashMap<String, String> = HashMap::new();
    for (k, v) in &extra_headers {
        if let Some(s) = v.as_str() {
            headers.insert(k.clone(), s.to_string());
        }
    }

    let mut response = match method.as_str() {
        "GET" => {
            let mut req = agent_http.get(&url);
            for (k, v) in &headers {
                req = req.header(k, v);
            }
            req.call()
        }
        "POST" => {
            let body = "";
            let mut req = agent_http.post(&url);
            for (k, v) in &headers {
                req = req.header(k, v);
            }
            req.send(body)
        }
        m => return Err(format!("unsupported method '{}' in webhook", m)),
    }
    .map_err(|e| format!("HTTP {} {} failed: {}", method, url, e))?;

    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("body read failed: {}", e))?;

    let parsed: Value = serde_json::from_str(&body).unwrap_or(Value::String(body.clone()));
    Ok(json!({
        "status": status,
        "url": url,
        "body": parsed,
    }))
}

/// Naive template expansion for DataMap webhook URLs:
///   `%{args.foo}`  → string from args["foo"]
///   `${...}`       → left as-is (it's an SWML reference resolved at
///                    runtime by the platform; the audit fixture
///                    accepts whatever path comes through, so we don't
///                    need to expand SWML refs).
fn expand_template(template: &str, args: &Value) -> String {
    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' && chars.peek() == Some(&'{') {
            chars.next(); // consume {
            let mut key = String::new();
            while let Some(&nc) = chars.peek() {
                if nc == '}' {
                    chars.next();
                    break;
                }
                key.push(nc);
                chars.next();
            }
            if let Some(field) = key.strip_prefix("args.") {
                if let Some(val) = args.get(field).and_then(|v| v.as_str()) {
                    out.push_str(val);
                } else if let Some(val) = args.get(field) {
                    out.push_str(&val.to_string());
                }
            } else {
                out.push_str(&format!("%{{{}}}", key));
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn die(msg: &str) -> ! {
    eprintln!("skills_audit_harness: {}", msg);
    process::exit(1);
}
