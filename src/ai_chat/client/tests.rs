// Copyright (c) 2026 SignalWire
//
// This file is part of the SignalWire SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Unit tests for the AI Chat client.
//!
//! Each test spins up a tiny in-process HTTP mock (via `tiny_http`, already a
//! crate dependency) on an OS-assigned free port, drives the async client against
//! it on a current-thread tokio runtime, and asserts BOTH the wire the client
//! sent (method, params, Basic-auth header, no identity leak) and what it decoded.
//! The mock mirrors the shared cross-language `mock_ai_chat` behavior: canned
//! success per method, sentinel-driven errors (`__err_<code>`), and the
//! summarize `{error}` branch
//! (`__summarize_error`).

use std::sync::{Arc, Mutex};
use std::thread;

use base64::Engine as _;
use serde_json::{Value, json};

use super::{AIChatClient, AIChatErrorKind, ChatOptions, CreateOptions, SummarizeOptions};

/// A recorded wire request: the JSON-RPC method + params + Authorization header.
#[derive(Debug, Clone)]
struct Recorded {
    method: String,
    params: Value,
    authorization: Option<String>,
}

/// Identity keys that must never ride in the JSON-RPC params.
const FORBIDDEN_IN_PARAMS: &[&str] = &[
    "project_id",
    "project",
    "token",
    "api_token",
    "space_id",
    "space",
];

/// A running in-process mock: its base URL + the requests it recorded. Dropping
/// it stops the server thread (the `tiny_http::Server` is closed).
struct Mock {
    url: String,
    requests: Arc<Mutex<Vec<Recorded>>>,
    server: Arc<tiny_http::Server>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Mock {
    /// Bind a free port and serve the canned AI-Chat responder on a background
    /// thread. `mock_ai_chat`-equivalent: canned success, sentinel errors,
    /// summarize `{error}` branch.
    fn start() -> Self {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").expect("bind mock"));
        let addr = server.server_addr();
        let port = addr.to_ip().expect("ip addr").port();
        let url = format!("http://127.0.0.1:{port}/api/ai/chat");
        let requests = Arc::new(Mutex::new(Vec::new()));

        let srv = Arc::clone(&server);
        let reqs = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            for mut req in srv.incoming_requests() {
                let auth = req
                    .headers()
                    .iter()
                    .find(|h| h.field.equiv("Authorization"))
                    .map(|h| h.value.as_str().to_string());
                let mut body = String::new();
                let _ = std::io::Read::read_to_string(req.as_reader(), &mut body);
                let payload: Value = serde_json::from_str(&body).unwrap_or(json!({}));
                let method = payload
                    .get("method")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                let params = payload.get("params").cloned().unwrap_or(json!({}));
                let id = payload.get("id").cloned().unwrap_or(Value::Null);

                reqs.lock().unwrap().push(Recorded {
                    method: method.clone(),
                    params: params.clone(),
                    authorization: auth,
                });

                let envelope = responder(&method, &params);
                let mut body_obj = serde_json::Map::new();
                body_obj.insert("jsonrpc".to_string(), json!("2.0"));
                if let Value::Object(m) = envelope {
                    for (k, v) in m {
                        body_obj.insert(k, v);
                    }
                }
                body_obj.insert("id".to_string(), id);
                let text = serde_json::to_string(&Value::Object(body_obj)).unwrap();
                let header =
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap();
                let response = tiny_http::Response::from_string(text).with_header(header);
                let _ = req.respond(response);
            }
        });

        Self {
            url,
            requests,
            server,
            handle: Some(handle),
        }
    }

    fn requests(&self) -> Vec<Recorded> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// The mock responder: `{ "result": {...} }` or `{ "error": {...} }`.
fn responder(method: &str, params: &Value) -> Value {
    let id = params.get("id").and_then(Value::as_str).unwrap_or("");
    if let Some(rest) = id.strip_prefix("__err_")
        && let Ok(code) = rest.parse::<i64>()
    {
        return json!({ "error": { "code": code, "message": "forced error" } });
    }
    if method == "summarize" && id == "__summarize_error" {
        return json!({ "result": { "error": "Failed to generate summary" } });
    }
    let result = match method {
        "create_conversation" => {
            json!({ "status": "created", "id": "conv-1", "initial_message": "hello" })
        }
        "chat" => json!({ "response": "hi there", "user_event": { "event_type": "demo", "n": 1 } }),
        "end_conversation" => json!({ "status": "ended", "id": "conv-1" }),
        "delete" => json!({ "status": "deleted", "id": "conv-1" }),
        "chat_log" => {
            json!({ "chat_log": [{ "role": "user", "content": "m" }], "call_timeline": [{ "t": 1 }] })
        }
        "summarize" => json!({ "summary": "a concise summary" }),
        _ => json!({}),
    };
    json!({ "result": result })
}

/// A current-thread tokio runtime for driving one async body to completion.
fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
        .block_on(fut)
}

/// A client pointed at the mock (deterministic: read-idle disabled).
fn new_client(url: &str) -> AIChatClient {
    AIChatClient::builder()
        .project("proj-1")
        .token("tok-1")
        .url(url)
        .read_idle_timeout_secs(0)
        .build()
        .expect("build client")
}

// ── construction ─────────────────────────────────────────────────────

#[test]
fn requires_a_project() {
    // No project arg, and clear the env so this is deterministic.
    let saved = std::env::var("SIGNALWIRE_PROJECT_ID").ok();
    unsafe {
        std::env::remove_var("SIGNALWIRE_PROJECT_ID");
    }
    let err = AIChatClient::builder().url("http://x").build().unwrap_err();
    assert!(err.message.contains("project is required"), "got: {err}");
    if let Some(v) = saved {
        unsafe {
            std::env::set_var("SIGNALWIRE_PROJECT_ID", v);
        }
    }
}

#[test]
fn builds_the_space_url_when_no_explicit_url() {
    let c = AIChatClient::builder()
        .project("p")
        .token("t")
        .space("myspace")
        .build()
        .unwrap();
    assert_eq!(c.url(), "https://myspace.signalwire.com/api/ai/chat");
}

#[test]
fn uses_an_explicit_url_verbatim() {
    let c = AIChatClient::builder()
        .project("p")
        .token("t")
        .url("http://local/api/ai/chat")
        .build()
        .unwrap();
    assert_eq!(c.url(), "http://local/api/ai/chat");
}

#[test]
fn errors_when_neither_url_nor_space_resolves() {
    // Clear SIGNALWIRE_SPACE so the env can't supply one.
    let saved = std::env::var("SIGNALWIRE_SPACE").ok();
    unsafe {
        std::env::remove_var("SIGNALWIRE_SPACE");
    }
    let err = AIChatClient::builder()
        .project("p")
        .token("t")
        .build()
        .unwrap_err();
    assert!(err.message.contains("No service URL"), "got: {err}");
    if let Some(v) = saved {
        unsafe {
            std::env::set_var("SIGNALWIRE_SPACE", v);
        }
    }
}

// ── wire behavior ────────────────────────────────────────────────────

#[test]
fn sends_basic_auth_and_never_leaks_identity_into_params() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    block_on(async {
        client
            .create_conversation(
                "conv-1",
                CreateOptions::new("http://cfg").timeout(30).reinit(true),
            )
            .await
            .unwrap();
    });
    let reqs = mock.requests();
    let req = &reqs[0];
    let auth = req.authorization.as_deref().expect("authorization header");
    assert!(auth.starts_with("Basic "), "got: {auth}");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(auth.trim_start_matches("Basic "))
        .unwrap();
    assert_eq!(String::from_utf8(decoded).unwrap(), "proj-1:tok-1");
    let params = req.params.as_object().unwrap();
    for key in FORBIDDEN_IN_PARAMS {
        assert!(!params.contains_key(*key), "identity leaked: {key}");
    }
}

#[test]
fn create_conversation_maps_timeout_and_decodes() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    let info = block_on(async {
        client
            .create_conversation(
                "conv-1",
                CreateOptions::new("http://cfg").timeout(30).reinit(true),
            )
            .await
            .unwrap()
    });
    let reqs = mock.requests();
    assert_eq!(reqs[0].method, "create_conversation");
    let params = reqs[0].params.as_object().unwrap();
    assert_eq!(params.get("id"), Some(&json!("conv-1")));
    assert_eq!(params.get("config_url"), Some(&json!("http://cfg")));
    assert_eq!(params.get("conversation_timeout"), Some(&json!(30)));
    assert_eq!(params.get("reinit"), Some(&json!(true)));
    assert_eq!(info.id, "conv-1");
    assert_eq!(info.status, "created");
    assert_eq!(info.initial_message.as_deref(), Some("hello"));
}

#[test]
fn chat_sends_role_user_by_default_and_decodes() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    let reply = block_on(async {
        client
            .chat(
                "conv-1",
                "hello",
                ChatOptions::default().timeout(30).reinit(true),
            )
            .await
            .unwrap()
    });
    let reqs = mock.requests();
    assert_eq!(reqs[0].method, "chat");
    let params = reqs[0].params.as_object().unwrap();
    assert_eq!(params.get("id"), Some(&json!("conv-1")));
    assert_eq!(params.get("message"), Some(&json!("hello")));
    assert_eq!(params.get("role"), Some(&json!("user")));
    assert_eq!(params.get("conversation_timeout"), Some(&json!(30)));
    assert_eq!(params.get("reinit"), Some(&json!(true)));
    assert_eq!(reply.text, "hi there");
    assert_eq!(reply.conversation_id, "conv-1");
    assert_eq!(
        reply.user_event,
        Some(json!({ "event_type": "demo", "n": 1 }))
    );
}

#[test]
fn end_returns_true_on_status_ended() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    let ended = block_on(async { client.end("conv-1").await.unwrap() });
    assert!(ended);
    assert_eq!(mock.requests()[0].method, "end_conversation");
}

#[test]
fn delete_returns_true_on_status_deleted() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    let deleted = block_on(async { client.delete("conv-1").await.unwrap() });
    assert!(deleted);
    assert_eq!(mock.requests()[0].method, "delete");
}

#[test]
fn log_decodes_messages_and_timeline() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    let log = block_on(async { client.log("conv-1").await.unwrap() });
    assert_eq!(mock.requests()[0].method, "chat_log");
    assert_eq!(
        log.messages,
        vec![json!({ "role": "user", "content": "m" })]
    );
    assert_eq!(log.call_timeline, vec![json!({ "t": 1 })]);
}

#[test]
fn summarize_returns_summary_on_the_summary_branch() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    let summary = block_on(async {
        client
            .summarize("conv-1", SummarizeOptions::default())
            .await
            .unwrap()
    });
    assert_eq!(summary, "a concise summary");
}

#[test]
fn summarize_passes_sampling_params_on_the_wire() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    block_on(async {
        client
            .summarize(
                "conv-1",
                SummarizeOptions::default()
                    .summary_prompt("be brief")
                    .temperature(0.2)
                    .max_tokens(64),
            )
            .await
            .unwrap();
    });
    let params = mock.requests()[0].params.as_object().unwrap().clone();
    assert_eq!(params.get("summary_prompt"), Some(&json!("be brief")));
    assert_eq!(params.get("temperature"), Some(&json!(0.2)));
    assert_eq!(params.get("max_tokens"), Some(&json!(64)));
}

// ── summarize one_of {error} branch ──────────────────────────────────

#[test]
fn summarize_error_branch_returns_err_summary_never_empty_string() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    let err = block_on(async {
        client
            .summarize("__summarize_error", SummarizeOptions::default())
            .await
            .unwrap_err()
    });
    assert_eq!(err.kind, AIChatErrorKind::Summary);
    assert_eq!(err.code, None);
    assert_eq!(err.message, "Failed to generate summary");
    assert_eq!(err.kind.name(), "SummaryError");
}

#[test]
fn summarize_does_not_raise_when_both_summary_and_error_present() {
    // summary wins: the failure branch requires error AND NOT summary.
    // Drive a dedicated mock that returns both.
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let port = server.server_addr().to_ip().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/api/ai/chat");
    let srv = Arc::clone(&server);
    let handle = thread::spawn(move || {
        for req in srv.incoming_requests() {
            let body = r#"{"jsonrpc":"2.0","result":{"summary":"s","error":"ignored"},"id":"x"}"#;
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap();
            let _ = req.respond(tiny_http::Response::from_string(body).with_header(header));
        }
    });
    let client = new_client(&url);
    let summary = block_on(async {
        client
            .summarize("conv-1", SummarizeOptions::default())
            .await
            .unwrap()
    });
    assert_eq!(summary, "s");
    server.unblock();
    let _ = handle.join();
}

// ── JSON-RPC error mapping ───────────────────────────────────────────

#[test]
fn maps_error_codes_to_typed_kinds_carrying_the_code() {
    let cases: &[(i64, AIChatErrorKind)] = &[
        (-32001, AIChatErrorKind::ConversationNotFound),
        (-32005, AIChatErrorKind::RateLimit),
        (-32006, AIChatErrorKind::RateLimit),
        (-32007, AIChatErrorKind::ChatInProgress),
        (-32009, AIChatErrorKind::Authentication),
    ];
    let mock = Mock::start();
    let client = new_client(&mock.url);
    for (code, kind) in cases {
        let id = format!("__err_{code}");
        let err = block_on(async {
            client
                .chat(&id, "x", ChatOptions::default())
                .await
                .unwrap_err()
        });
        assert_eq!(err.kind, *kind, "code {code}");
        assert_eq!(err.code, Some(*code), "code {code}");
    }
}

#[test]
fn maps_an_unmapped_code_to_the_base_api_error() {
    let mock = Mock::start();
    let client = new_client(&mock.url);
    let err = block_on(async {
        client
            .chat("__err_-32602", "x", ChatOptions::default())
            .await
            .unwrap_err()
    });
    assert_eq!(err.kind, AIChatErrorKind::Api);
    assert_eq!(err.code, Some(-32602));
    assert_eq!(err.kind.name(), "AIChatError");
}

#[test]
fn raises_api_error_on_a_non_json_body() {
    let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let port = server.server_addr().to_ip().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/api/ai/chat");
    let srv = Arc::clone(&server);
    let handle = thread::spawn(move || {
        for req in srv.incoming_requests() {
            let _ = req.respond(tiny_http::Response::from_string("<html>not json"));
        }
    });
    let client = new_client(&url);
    let err = block_on(async {
        client
            .chat("conv-1", "x", ChatOptions::default())
            .await
            .unwrap_err()
    });
    assert_eq!(err.kind, AIChatErrorKind::Api);
    server.unblock();
    let _ = handle.join();
}
