// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `http_dump` — the Rust port's HTTP dump program for the cross-port HTTP
//! differ (porting-sdk/scripts/diff_port_http.py).
//!
//! For each `http_corpus` case it feeds a synthetic request into the Rust SDK's
//! framework-free dispatch core (`Service::handle_request`,
//! `Service::extract_sip_username`, the framework-free webhook `validate`
//! decision, and the serverless lambda `Adapter`) and prints ONE JSON object
//! mapping
//!
//!     case-id -> reduced-artifact
//!
//! to stdout, reduced to the same shape the Python oracle emits. The differ
//! canonicalizes both sides and byte-compares. Only stdout carries the JSON
//! object.
//!
//! The corpus sentinels (`__AUTH__`/`__AUTH_BAD__` Basic headers, `__SIG__`
//! webhook signature, `__REDIRECT_CB__` routing callback, `__HELLO_HANDLER__`
//! SWAIG handler, `__JSON__:` lambda body prefix) are materialized here as the
//! oracle materializes them, so the interop cases are reproducible.
//!
//! Run from the signalwire-rust repo root:
//!
//!     cargo run --quiet --example http_dump

use std::collections::{BTreeMap, HashMap};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Value, json};
use sha1::Sha1;
use signalwire::security::validate;
use signalwire::serverless::Adapter;
use signalwire::swaig::FunctionResult;
use signalwire::swml::service::{Service, ServiceOptions};
use signalwire::{AgentBase, AgentOptions};

const USER: &str = "user";
const PASSWORD: &str = "pass";
const SIGNING_KEY: &str = "PSK-fixed-signing-key";
const WH_URL: &str = "https://agent.example.com/webhook";
const WH_BODY: &str = r#"{"event":"call.created","id":"abc"}"#;

fn basic_auth(u: &str, p: &str) -> String {
    format!("Basic {}", BASE64.encode(format!("{u}:{p}")))
}

fn webhook_sig(url: &str, body: &str, key: &str) -> String {
    use std::fmt::Write as _;
    let mut mac =
        Hmac::<Sha1>::new_from_slice(key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(url.as_bytes());
    mac.update(body.as_bytes());
    mac.finalize()
        .into_bytes()
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// Reduce a `(status, headers, body)` triple to a comparable artifact — the
/// Rust mirror of `diff_port_http._observe_response`.
fn observe_response(
    status: u16,
    headers: &HashMap<String, String>,
    body: &str,
    full_body: bool,
) -> Value {
    let mut header_keys: Vec<&String> = headers.keys().collect();
    header_keys.sort();
    let mut out = json!({
        "status": status,
        "header_keys": header_keys,
    });
    if let Some(loc) = headers.get("Location") {
        out["location"] = json!(loc);
    }
    if let Some(wa) = headers.get("WWW-Authenticate") {
        out["www_authenticate"] = json!(wa);
    }
    if full_body {
        if body.is_empty() {
            out["body"] = json!("");
        } else {
            out["body"] = serde_json::from_str::<Value>(body).unwrap_or_else(|_| json!(body));
        }
    }
    out
}

fn new_service() -> Service {
    Service::new(
        ServiceOptions::new("demo")
            .route("/swml")
            .basic_auth(USER, PASSWORD),
    )
}

fn main() {
    let mut out: BTreeMap<&str, Value> = BTreeMap::new();

    // ---- handle_request: 200 SWML happy path ----
    {
        let svc = new_service();
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), basic_auth(USER, PASSWORD));
        let (s, h, b) = svc.handle_request("GET", "/swml", &headers, None);
        out.insert(
            "http_handle_request_200_swml",
            observe_response(s, &h, &b, true),
        );
    }
    // ---- handle_request: 401 no auth ----
    {
        let svc = new_service();
        let (s, h, b) = svc.handle_request("GET", "/swml", &HashMap::new(), None);
        out.insert(
            "http_handle_request_401_no_auth",
            observe_response(s, &h, &b, true),
        );
    }
    // ---- handle_request: 401 bad password (status+headers only) ----
    {
        let svc = new_service();
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), basic_auth(USER, "wrong"));
        let (s, h, b) = svc.handle_request("GET", "/swml", &headers, None);
        out.insert(
            "http_handle_request_401_bad_password",
            observe_response(s, &h, &b, false),
        );
    }
    // ---- handle_request: 307 redirect via routing callback ----
    {
        let mut svc = new_service();
        svc.register_routing_callback(redirect_cb, None);
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), basic_auth(USER, PASSWORD));
        let (s, h, b) = svc.handle_request(
            "POST",
            "/swml/sip",
            &headers,
            Some(r#"{"call": {"to": "sip:redirect-me@space"}}"#),
        );
        out.insert(
            "http_handle_request_307_redirect",
            observe_response(s, &h, &b, true),
        );
    }
    // ---- handle_request: callback returns None -> normal 200 SWML ----
    {
        let mut svc = new_service();
        svc.register_routing_callback(redirect_cb, None);
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), basic_auth(USER, PASSWORD));
        let (s, h, b) = svc.handle_request(
            "POST",
            "/swml/sip",
            &headers,
            Some(r#"{"call": {"to": "sip:keep@space"}}"#),
        );
        out.insert(
            "http_handle_request_callback_passthrough_200",
            observe_response(s, &h, &b, true),
        );
    }

    // ---- extract_sip_username: pure extractor ----
    out.insert(
        "http_extract_sip_username_sip",
        extract_username(&json!({"call": {"to": "sip:alice@agents.signalwire.com"}})),
    );
    out.insert(
        "http_extract_sip_username_tel",
        extract_username(&json!({"call": {"to": "tel:+15551234567"}})),
    );
    out.insert(
        "http_extract_sip_username_plain",
        extract_username(&json!({"call": {"to": "support"}})),
    );
    out.insert(
        "http_extract_sip_username_missing",
        extract_username(&json!({"vars": {}})),
    );

    // ---- webhook validate ----
    out.insert(
        "http_webhook_validate_ok",
        webhook_decision(
            "POST",
            WH_URL,
            WH_BODY,
            &[(
                "x-signalwire-signature",
                &webhook_sig(WH_URL, WH_BODY, SIGNING_KEY),
            )],
            SIGNING_KEY,
        ),
    );
    let bad_sig = "deadbeef".repeat(5);
    out.insert(
        "http_webhook_validate_bad_sig",
        webhook_decision(
            "POST",
            WH_URL,
            WH_BODY,
            &[("x-signalwire-signature", &bad_sig)],
            SIGNING_KEY,
        ),
    );
    out.insert(
        "http_webhook_validate_missing_sig",
        webhook_decision("POST", WH_URL, WH_BODY, &[], SIGNING_KEY),
    );
    out.insert(
        "http_webhook_validate_twilio_alias",
        webhook_decision(
            "POST",
            WH_URL,
            WH_BODY,
            &[(
                "x-twilio-signature",
                &webhook_sig(WH_URL, WH_BODY, SIGNING_KEY),
            )],
            SIGNING_KEY,
        ),
    );

    // ---- serverless (lambda) ----
    out.insert("http_serverless_lambda_swaig", serverless_swaig());
    out.insert("http_serverless_lambda_noauth_401", serverless_noauth());

    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize http dump")
    );
}

/// Redirect one specific 'to', else pass through (None).
fn redirect_cb(body: &Value, _headers: &HashMap<String, String>) -> Option<String> {
    let to = body
        .get("call")
        .and_then(|c| c.get("to"))
        .and_then(Value::as_str);
    if to == Some("sip:redirect-me@space") {
        Some("/other-route".to_string())
    } else {
        None
    }
}

fn extract_username(body: &Value) -> Value {
    match Service::extract_sip_username(body) {
        Some(u) => json!({ "username": u }),
        None => json!({ "username": Value::Null }),
    }
}

fn webhook_decision(
    method: &str,
    url: &str,
    body: &str,
    headers: &[(&str, &str)],
    key: &str,
) -> Value {
    let hmap: HashMap<String, String> = headers
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect();
    match validate(method, url, &hmap, body, key) {
        None => json!({"decision": "pass"}),
        Some((status, _, _)) => json!({"decision": "reject", "status": status}),
    }
}

/// Reduce a lambda response to `{status, body}` with the body parsed as JSON
/// — mirroring the oracle's `serverless_result` observer.
fn reduce_lambda(resp: &Value) -> Value {
    let status = resp.get("statusCode").cloned().unwrap_or(Value::Null);
    let body = resp
        .get("body")
        .and_then(Value::as_str)
        .map_or(Value::Null, |s| {
            serde_json::from_str::<Value>(s).unwrap_or_else(|_| json!(s))
        });
    json!({ "status": status, "body": body })
}

/// Drive the lambda adapter for the /swaig dispatch case. The agent is built
/// at route "/" so the event's root-relative "/swaig" path routes correctly.
fn serverless_swaig() -> Value {
    let mut agent = AgentBase::new(
        AgentOptions::new("demo")
            .route("/")
            .basic_auth(USER, PASSWORD),
    );
    agent.define_tool(
        "say_hello",
        "greet",
        json!({}),
        Box::new(|_args, _raw| FunctionResult::with_response("hello there")),
        false,
    );
    let event = json!({
        "rawPath": "/swaig",
        "headers": {
            "authorization": basic_auth(USER, PASSWORD),
            "content-type": "application/json",
        },
        "body": r#"{"function":"say_hello","argument":{"parsed":[{}]},"call_id":"c1"}"#,
        "requestContext": {"http": {"method": "POST"}},
    });
    reduce_lambda(&Adapter::handle_lambda(&agent, &event))
}

fn serverless_noauth() -> Value {
    let agent = AgentBase::new(
        AgentOptions::new("demo")
            .route("/")
            .basic_auth(USER, PASSWORD),
    );
    let event = json!({
        "rawPath": "/",
        "headers": {},
        "requestContext": {"http": {"method": "GET"}},
    });
    reduce_lambda(&Adapter::handle_lambda(&agent, &event))
}
