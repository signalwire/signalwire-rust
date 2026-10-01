//! `ChatGateway` / `HandoffRouter`: what a browser holding a publishable key
//! can and cannot do. Ported from signalwire-python
//! `tests/unit/ai_chat/test_gateway.py` and `test_handoff.py`, against an
//! in-process stub chat service (a real HTTP server on a free port).

use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value, json};

use crate::ai_chat::client::AIChatClient;
use crate::ai_chat::gateway::{
    ChatGateway, ChatGatewayOptions, GatewayRejection, MAX_MESSAGE_BYTES, MAX_REQUEST_BODY_BYTES,
    MAX_USER_METADATA_BYTES,
};
use crate::ai_chat::handoff::{HandoffRouter, HandoffRouterOptions};

const CONFIG_URL: &str = "https://agent.example.com/swml";
const KEY: &str = "pk_test_key";

/// The stub chat service: records the JSON-RPC bodies the gateway forwarded.
struct Service {
    url: String,
    seen: Arc<Mutex<Vec<Value>>>,
    server: Arc<tiny_http::Server>,
}

impl Service {
    fn start() -> Self {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").expect("bind"));
        let port = server.server_addr().to_ip().expect("ip").port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let (s, srv) = (Arc::clone(&seen), Arc::clone(&server));
        std::thread::spawn(move || {
            for mut req in srv.incoming_requests() {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                let body: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                s.lock().unwrap().push(body.clone());
                let result = match body["method"].as_str().unwrap_or("") {
                    "chat" => json!({"response": "hi there"}),
                    "create_conversation" => {
                        json!({"status": "created", "initial_message": "Hi, I am Sigmond."})
                    }
                    "chat_log" => json!({"chat_log": [
                        {"role": "system", "content": "secret prompt"},
                        {"role": "user", "content": "hi", "timestamp": 1_700_000_000_000_000_i64},
                        {"role": "assistant", "content": "hi there"},
                    ]}),
                    _ => json!({"status": "ended"}),
                };
                // Keepalive padding before the body, as the service sends on a slow turn.
                let text = format!(
                    "   \n{}",
                    json!({"jsonrpc": "2.0", "result": result, "id": body["id"]})
                );
                let _ = req.respond(tiny_http::Response::from_string(text));
            }
        });
        Service {
            url: format!("http://127.0.0.1:{port}/"),
            seen,
            server,
        }
    }

    fn seen(&self) -> Vec<Value> {
        self.seen.lock().unwrap().clone()
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        self.server.unblock();
    }
}

fn client(url: &str) -> Arc<AIChatClient> {
    Arc::new(
        AIChatClient::builder()
            .project("p")
            .token("t")
            .url(url)
            .build()
            .expect("client"),
    )
}

fn gateway_with(
    svc: &Service,
    f: impl FnOnce(ChatGatewayOptions) -> ChatGatewayOptions,
) -> ChatGateway {
    ChatGateway::new(f(ChatGatewayOptions::new(CONFIG_URL)
        .key(KEY)
        .allowed_origins(vec!["https://shop.example.com".to_string()])
        .client(client(&svc.url))
        .secret(b"s3cret".to_vec())))
    .expect("gateway")
}

fn obj(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap()
}

fn prepare(
    gw: &ChatGateway,
    body: Value,
) -> Result<crate::ai_chat::gateway::PreparedCall, GatewayRejection> {
    gw.prepare(&obj(body), None, Some(KEY))
}

// ── Handles ──────────────────────────────────────────────────────────

#[test]
fn a_handle_round_trips_and_cannot_be_forged() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let h = gw.mint_handle(Some("conv-1"));
    assert_eq!(gw.read_handle(&h).unwrap(), "conv-1");
    // Tampering with the payload breaks the signature.
    let (_, sig) = h.split_once('.').unwrap();
    let forged = format!(
        "{}.{sig}",
        base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            "conv-2:9999999999",
        )
    );
    assert_eq!(gw.read_handle(&forged).unwrap_err().status, 403);
    // A handle from another gateway (another secret) is refused.
    let other = gateway_with(&svc, |o| o.secret(b"other".to_vec()));
    assert_eq!(
        gw.read_handle(&other.mint_handle(None)).unwrap_err().reason,
        "invalid handle"
    );
}

#[test]
fn an_expired_handle_is_refused_and_garbage_does_not_leak_why() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o.handle_ttl(-1));
    let h = gw.mint_handle(None);
    assert_eq!(gw.read_handle(&h).unwrap_err().reason, "expired handle");
    for junk in ["", "nodot", "!!!.!!!", "a.b"] {
        let err = gw.read_handle(junk).unwrap_err();
        assert!(matches!(err.status, 400 | 403), "{junk}: {err}");
    }
}

// ── Guards ───────────────────────────────────────────────────────────

#[test]
fn origins_localhost_listed_unlisted_and_missing() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    for ok in [
        "http://localhost:3000",
        "http://127.0.0.1:8080",
        "http://app.localhost",
        "https://shop.example.com",
        "https://shop.example.com/",
    ] {
        assert!(gw.check_origin(Some(ok)).is_ok(), "{ok}");
    }
    assert_eq!(
        gw.check_origin(Some("https://evil.example"))
            .unwrap_err()
            .status,
        403
    );
    assert!(gw.check_origin(None).is_ok());
}

#[test]
fn the_key_is_required() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    for bad in [None, Some(""), Some("pk_wrong")] {
        let err = gw
            .prepare(&obj(json!({"message": "hi"})), None, bad)
            .unwrap_err();
        assert_eq!(err.status, 401);
    }
}

// ── prepare ──────────────────────────────────────────────────────────

#[test]
fn config_url_and_conversation_are_the_gateways_not_the_browsers() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let (method, params, minted) = prepare(
        &gw,
        json!({"message": "hi", "config_url": "https://evil", "id": "theirs"}),
    )
    .unwrap();
    assert_eq!(method, "chat");
    assert_eq!(params["config_url"], CONFIG_URL);
    let minted = minted.expect("first chat mints");
    assert_eq!(params["id"], json!(gw.read_handle(&minted).unwrap()));
    // A later turn reuses the handle and mints nothing.
    let (_, p2, m2) = prepare(&gw, json!({"message": "again", "handle": minted})).unwrap();
    assert!(m2.is_none());
    assert_eq!(p2["id"], params["id"]);
}

#[test]
fn method_rules() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    assert_eq!(
        prepare(&gw, json!({"method": "chat_log"}))
            .unwrap_err()
            .status,
        400
    );
    assert_eq!(
        prepare(&gw, json!({"method": "end"})).unwrap_err().reason,
        "end requires a handle"
    );
    assert_eq!(
        prepare(&gw, json!({"method": "log"})).unwrap_err().reason,
        "log requires a handle"
    );
    let h = gw.mint_handle(Some("c"));
    let (m, p, _) = prepare(&gw, json!({"method": "end", "handle": h})).unwrap();
    assert_eq!(
        (m.as_str(), p["id"].as_str()),
        ("end_conversation", Some("c"))
    );
    let (m, _, _) = prepare(&gw, json!({"method": "log", "handle": h})).unwrap();
    assert_eq!(m, "chat_log");
    assert_eq!(
        prepare(&gw, json!({"message": "   "})).unwrap_err().reason,
        "message is required"
    );
    let (m, p, minted) = prepare(&gw, json!({"method": "start"})).unwrap();
    assert_eq!(m, "create_conversation");
    assert!(minted.is_some() && p.get("message").is_none());
}

#[test]
fn caps_bound_minting_and_turns_per_conversation() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o.max_new_conversations(2).max_turns(2));
    prepare(&gw, json!({"message": "a"})).unwrap();
    let (_, _, h) = prepare(&gw, json!({"message": "b"})).unwrap();
    assert_eq!(
        prepare(&gw, json!({"message": "c"})).unwrap_err().status,
        429
    );
    let h = h.unwrap();
    prepare(&gw, json!({"message": "2", "handle": h})).unwrap();
    assert_eq!(
        prepare(&gw, json!({"message": "3", "handle": h}))
            .unwrap_err()
            .status,
        429
    );
    // Another conversation is not stopped by this one's cap.
    let other = gw.mint_handle(Some("other"));
    assert!(prepare(&gw, json!({"message": "x", "handle": other})).is_ok());
}

#[test]
fn page_context_rules() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o.max_new_conversations(1));
    let meta = json!({"page": {"url": "https://shop.example.com/x"}});
    let (_, p, _) = prepare(&gw, json!({"method": "start", "user_meta_data": meta})).unwrap();
    assert_eq!(p["user_meta_data"], meta);
    // It cannot displace what the gateway owns.
    assert_eq!(p["config_url"], CONFIG_URL);
    assert_eq!(
        prepare(&gw, json!({"user_meta_data": "x", "message": "m"}))
            .unwrap_err()
            .status,
        400
    );
    let big = "x".repeat(MAX_USER_METADATA_BYTES + 1);
    let gw2 = gateway_with(&svc, |o| o.max_new_conversations(1));
    assert_eq!(
        prepare(&gw2, json!({"user_meta_data": {"b": big}, "message": "m"}))
            .unwrap_err()
            .status,
        413
    );
    // Rejected before a conversation was charged: the one slot is still free.
    assert!(prepare(&gw2, json!({"message": "m"})).is_ok());
}

#[test]
fn the_message_limit_counts_utf8_bytes_and_charges_nothing() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o.max_new_conversations(1));
    assert!(
        prepare(&gw, json!({"message": "x".repeat(MAX_MESSAGE_BYTES + 1)}))
            .unwrap_err()
            .status
            == 413
    );
    let multibyte = "é".repeat(MAX_MESSAGE_BYTES / 2 + 1); // over in bytes, under in chars
    assert_eq!(
        prepare(&gw, json!({"message": multibyte}))
            .unwrap_err()
            .status,
        413
    );
    // Nothing was minted by the refusals.
    assert!(prepare(&gw, json!({"message": "x".repeat(MAX_MESSAGE_BYTES)})).is_ok());
}

// ── Transcript helpers ───────────────────────────────────────────────

#[test]
fn the_transcript_hides_everything_but_the_dialogue_in_seconds() {
    let msgs = vec![
        json!({"role": "system", "content": "secret"}),
        json!({"role": "tool", "content": "x"}),
        json!({"role": "user", "content": "hi", "timestamp": 2_000_000}),
        json!({"role": "assistant", "content": "  "}),
        json!("junk"),
    ];
    assert_eq!(
        ChatGateway::visible_messages(Some(&msgs)),
        vec![json!({"role": "user", "content": "hi", "timestamp": 2.0})]
    );
    let dated = vec![
        json!({"role": "system", "timestamp": 5_000_000}),
        json!({"role": "user", "timestamp": 3_000_000}),
    ];
    assert_eq!(ChatGateway::last_activity(Some(&dated)), Some(5.0));
    assert_eq!(
        ChatGateway::last_activity(Some(&[json!({"role": "user"})])),
        None
    );
    assert_eq!(ChatGateway::last_activity(None), None);
}

#[test]
fn effective_timeout_is_always_a_number() {
    let svc = Service::start();
    assert_eq!(gateway_with(&svc, |o| o).effective_timeout(), 3600);
    assert_eq!(
        gateway_with(&svc, |o| o.conversation_timeout(90)).effective_timeout(),
        90
    );
}

// ── Over HTTP (mounted on an agent) ──────────────────────────────────

fn agent_with(gw: &ChatGateway, handoff: Option<&HandoffRouter>) -> crate::agent::AgentBase {
    let mut agent = crate::agent::AgentBase::new(crate::agent::AgentOptions::new("t"));
    agent.mount(gw.router(), Some("/chat"), None);
    if let Some(h) = handoff {
        agent.mount(h.router(), Some("/chat"), None);
    }
    agent
}

fn post(
    agent: &crate::agent::AgentBase,
    path: &str,
    body: &Value,
    key: Option<&str>,
    origin: Option<&str>,
) -> (u16, HashMap<String, String>, Value) {
    let mut headers = HashMap::new();
    if let Some(k) = key {
        headers.insert("Authorization".to_string(), format!("Bearer {k}"));
    }
    if let Some(o) = origin {
        headers.insert("Origin".to_string(), o.to_string());
    }
    let (status, h, text) = agent.handle_request("POST", path, &headers, Some(&body.to_string()));
    (
        status,
        h,
        serde_json::from_str(&text).unwrap_or(Value::String(text)),
    )
}

#[test]
fn a_full_exchange_over_http() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o.conversation_timeout(600));
    let agent = agent_with(&gw, None);
    let (status, headers, body) = post(
        &agent,
        "/chat",
        &json!({"method": "start"}),
        Some(KEY),
        Some("https://shop.example.com"),
    );
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["greeting"], "Hi, I am Sigmond.");
    assert_eq!(body["timeout"], 600);
    let handle = headers["X-Chat-Handle"].clone();
    assert_eq!(
        headers["Access-Control-Allow-Origin"],
        "https://shop.example.com"
    );
    // The configured timeout went upstream on the create.
    assert_eq!(svc.seen()[0]["params"]["conversation_timeout"], 600);
    // A chat streams the service body through (keepalive padding and all).
    let (status, _, body) = post(
        &agent,
        "/chat",
        &json!({"message": "hi", "handle": handle}),
        Some(KEY),
        None,
    );
    assert_eq!(status, 200);
    assert_eq!(body["result"]["response"], "hi there");
    // The log is the visible transcript only.
    let (_, _, body) = post(
        &agent,
        "/chat",
        &json!({"method": "log", "handle": handle}),
        Some(KEY),
        None,
    );
    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
    assert!(!body.to_string().contains("secret prompt"));
    assert_eq!(body["last_activity"], 1_700_000_000.0);
    let (_, _, body) = post(
        &agent,
        "/chat",
        &json!({"method": "end", "handle": handle}),
        Some(KEY),
        None,
    );
    assert_eq!(body["status"], "ended");
}

#[test]
fn the_chat_relay_keeps_the_keepalive_padding() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let router = gw.router();
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), format!("Bearer {KEY}"));
    let (status, _, mut reader) =
        router("POST", "/", &headers, &json!({"message": "hi"}).to_string()).unwrap();
    assert_eq!(status, 200);
    let mut text = String::new();
    reader.read_to_string(&mut text).unwrap();
    assert!(
        text.starts_with("   \n"),
        "padding relayed, not swallowed: {text:?}"
    );
}

#[test]
fn http_refusals() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let agent = agent_with(&gw, None);
    assert_eq!(
        post(
            &agent,
            "/chat",
            &json!({"message": "hi"}),
            Some("bad"),
            None
        )
        .0,
        401
    );
    assert_eq!(
        post(
            &agent,
            "/chat",
            &json!({"message": "hi"}),
            Some(KEY),
            Some("https://evil.example")
        )
        .0,
        403
    );
    let big = json!({"message": "x".repeat(MAX_REQUEST_BODY_BYTES)});
    assert_eq!(post(&agent, "/chat", &big, Some(KEY), None).0, 413);
    assert_eq!(post(&agent, "/chat", &json!([1]), Some(KEY), None).0, 400);
    assert!(svc.seen().is_empty(), "nothing reached the service");
    // Preflight answers a listed origin with the allow headers.
    let mut h = HashMap::new();
    h.insert("Origin".to_string(), "https://shop.example.com".to_string());
    let (status, headers, _) = agent.handle_request("OPTIONS", "/chat", &h, None);
    assert_eq!(status, 204);
    assert_eq!(headers["Access-Control-Allow-Methods"], "POST, OPTIONS");
}

// ── Handoff ──────────────────────────────────────────────────────────

fn handoff_with(
    gw: &ChatGateway,
    f: impl FnOnce(HandoffRouterOptions) -> HandoffRouterOptions,
) -> HandoffRouter {
    HandoffRouter::new(f(HandoffRouterOptions::new(gw.clone())))
}

#[test]
fn a_nonce_redeems_once_ending_the_call_and_waiting_for_capture() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let order = Arc::new(Mutex::new(Vec::<String>::new()));
    let (o1, o2) = (Arc::clone(&order), Arc::clone(&order));
    let h = handoff_with(&gw, |o| {
        o.end_call(Arc::new(move |call| {
            o1.lock().unwrap().push(format!("end:{call}"));
        }))
        .capture_leg(Arc::new(move |conv, medium| {
            o2.lock().unwrap().push(format!("capture:{conv}:{medium}"));
            true
        }))
    });
    h.register("n1", "conv", Some("call-1"));
    let handle = h.redeem("n1").expect("redeemed");
    assert_eq!(gw.read_handle(&handle).unwrap(), "conv.1");
    assert_eq!(
        *order.lock().unwrap(),
        vec!["end:call-1", "capture:conv:voice"]
    );
    // Single use; unknown answers the same.
    assert!(h.redeem("n1").is_none());
    assert!(h.redeem("nope").is_none());
    // A redeemed nonce cannot be registered again.
    h.register("n1", "conv", Some("call-1"));
    assert!(h.redeem("n1").is_none());
}

#[test]
fn the_first_registration_stands() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let h = handoff_with(&gw, |o| o);
    h.register("n", "first", Some("call-1"));
    h.register("n", "second", Some("call-2"));
    assert_eq!(gw.read_handle(&h.redeem("n").unwrap()).unwrap(), "first.1");
}

#[test]
fn next_conversation_id_increments_a_numeric_suffix() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let h = handoff_with(&gw, |o| o);
    for (conv, want) in [("root", "root.1"), ("root.2", "root.3"), ("a.b", "a.b.1")] {
        h.register(conv, conv, Some("c"));
        assert_eq!(gw.read_handle(&h.redeem(conv).unwrap()).unwrap(), want);
    }
}

#[test]
fn a_capture_that_overruns_does_not_hold_up_the_handoff() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let h = handoff_with(&gw, |o| {
        o.capture_timeout(0.05).capture_leg(Arc::new(|_, _| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            true
        }))
    });
    h.register("n", "c", Some("call"));
    let start = std::time::Instant::now();
    assert!(h.redeem("n").is_some());
    assert!(start.elapsed() < std::time::Duration::from_millis(400));
}

#[test]
fn say_is_repeatable_capped_and_never_consumes_the_nonce() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let sent = Arc::new(AtomicUsize::new(0));
    let s = Arc::clone(&sent);
    let h = handoff_with(&gw, |o| {
        o.max_messages_per_call(2)
            .send_message(Arc::new(move |call, text| {
                assert_eq!((call, text), ("call-1", "hello"));
                s.fetch_add(1, Ordering::SeqCst);
                true
            }))
    });
    h.register("n", "c", Some("call-1"));
    assert!(h.say("n", "  hello  "));
    assert!(h.say("n", "hello"));
    assert!(!h.say("n", "hello"), "capped");
    assert_eq!(sent.load(Ordering::SeqCst), 2);
    assert!(!h.say("n", "   "));
    assert!(!h.say("n", &"x".repeat(MAX_MESSAGE_BYTES + 1)));
    // Typing did not consume the nonce.
    assert!(h.redeem("n").is_some());
    // A failed delivery gives its slot back.
    let h2 = handoff_with(&gw, |o| {
        o.max_messages_per_call(1)
            .send_message(Arc::new(|_, _| panic!("down")))
    });
    h2.register("m", "c", Some("call"));
    assert!(!h2.say("m", "x"));
    assert!(!h2.say("m", "x"), "slot returned, delivery still failing");
}

#[test]
fn escalate_needs_a_valid_handle_and_waits_for_capture() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let captured = Arc::new(Mutex::new(Vec::new()));
    let c = Arc::clone(&captured);
    let h = handoff_with(&gw, |o| {
        o.capture_leg(Arc::new(move |conv, medium| {
            c.lock()
                .unwrap()
                .push((conv.to_string(), medium.to_string()));
            true
        }))
    });
    assert!(h.escalate(&gw.mint_handle(Some("conv"))));
    assert_eq!(
        *captured.lock().unwrap(),
        vec![("conv".to_string(), "chat".to_string())]
    );
    assert!(!h.escalate("garbage"));
}

#[test]
fn handoff_routes_over_http() {
    let svc = Service::start();
    let gw = gateway_with(&svc, |o| o);
    let h = handoff_with(&gw, |o| o.send_message(Arc::new(|_, _| true)));
    let agent = agent_with(&gw, Some(&h));
    h.register("n", "conv", Some("call"));
    assert_eq!(
        post(
            &agent,
            "/chat/say",
            &json!({"nonce": "n", "text": "hi"}),
            None,
            None
        )
        .0,
        200
    );
    assert_eq!(
        post(
            &agent,
            "/chat/say",
            &json!({"nonce": "n", "text": "x".repeat(MAX_MESSAGE_BYTES + 1)}),
            None,
            None
        )
        .0,
        413
    );
    let (status, _, body) = post(&agent, "/chat/handoff", &json!({"nonce": "n"}), None, None);
    assert_eq!(status, 200);
    assert_eq!(
        gw.read_handle(body["handle"].as_str().unwrap()).unwrap(),
        "conv.1"
    );
    assert_eq!(
        post(&agent, "/chat/handoff", &json!({"nonce": "n"}), None, None).0,
        404
    );
    assert_eq!(
        post(&agent, "/chat/escalate", &json!({}), None, None).0,
        400
    );
    assert_eq!(
        post(
            &agent,
            "/chat/say",
            &json!({"nonce": "n", "text": "hi"}),
            None,
            Some("https://evil.example")
        )
        .0,
        403
    );
    // The gateway's own route is still reachable beside the handoff routes.
    assert_eq!(
        post(&agent, "/chat", &json!({"message": "hi"}), Some(KEY), None).0,
        200
    );
}
