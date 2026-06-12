// Mock-backed integration tests translated from
// signalwire-python/tests/unit/relay/test_connect_mock.py.
//
// Drives the real `signalwire::relay::Client::connect()` against the
// shared `mock_relay` WebSocket server. Each test asserts both:
//
//   1. Behavioral state on the SDK after connect (protocol set, etc.).
//   2. The journal entry the mock recorded for the SDK's outbound
//      `signalwire.connect` frame.

#[path = "common/mod.rs"]
mod common;

use serde_json::Value;
use signalwire::relay::Client as RelayClient;
use std::sync::Arc;

use common::relay_mocktest;

// ---------------------------------------------------------------------------
// Connect — happy path
// ---------------------------------------------------------------------------

#[test]
fn test_connect_returns_protocol_string() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    // The mock issues `signalwire_<uuid>` protocol strings.
    let proto = client
        .protocol
        .lock()
        .unwrap()
        .clone()
        .expect("protocol should be set after connect");
    assert!(
        proto.starts_with("signalwire_"),
        "unexpected protocol shape: {proto:?}"
    );
    assert!(client.is_connected());
    client.disconnect();
}

#[test]
fn test_connect_journal_records_signalwire_connect() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let connects = relay_mocktest::journal_recv(Some("signalwire.connect"));
    assert_eq!(
        connects.len(),
        1,
        "expected exactly 1 signalwire.connect frame, got {}: {:?}",
        connects.len(),
        connects.iter().map(|e| &e.frame).collect::<Vec<_>>()
    );
    client.disconnect();
}

#[test]
fn test_connect_journal_carries_project_and_token() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let entry = relay_mocktest::journal_recv(Some("signalwire.connect"))
        .into_iter()
        .next()
        .expect("expected one connect frame");
    let auth = &entry.frame["params"]["authentication"];
    assert_eq!(
        auth.get("project").and_then(Value::as_str),
        Some("test_proj")
    );
    assert_eq!(auth.get("token").and_then(Value::as_str), Some("test_tok"));
    client.disconnect();
}

#[test]
fn test_connect_journal_carries_contexts() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let entry = relay_mocktest::journal_recv(Some("signalwire.connect"))
        .into_iter()
        .next()
        .expect("expected one connect frame");
    let ctxs = entry.frame["params"]["contexts"]
        .as_array()
        .expect("contexts should be array");
    let names: Vec<&str> = ctxs.iter().filter_map(Value::as_str).collect();
    assert_eq!(names, vec!["default"]);
    client.disconnect();
}

#[test]
fn test_connect_journal_carries_agent_and_version() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let entry = relay_mocktest::journal_recv(Some("signalwire.connect"))
        .into_iter()
        .next()
        .expect("expected one connect frame");
    let p = &entry.frame["params"];
    let agent = p.get("agent").and_then(Value::as_str).unwrap_or("");
    assert!(
        agent.contains("signalwire-agents-rust"),
        "unexpected agent: {agent:?}"
    );
    let version = &p["version"];
    assert!(
        version.is_object(),
        "version should be an object, got {version:?}"
    );
    assert_eq!(version.get("major").and_then(Value::as_u64), Some(2));
    assert_eq!(version.get("minor").and_then(Value::as_u64), Some(0));
    client.disconnect();
}

#[test]
fn test_connect_journal_event_acks_true() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let entry = relay_mocktest::journal_recv(Some("signalwire.connect"))
        .into_iter()
        .next()
        .expect("expected one connect frame");
    let event_acks = entry.frame["params"]
        .get("event_acks")
        .and_then(Value::as_bool);
    assert_eq!(
        event_acks,
        Some(true),
        "event_acks should be true, got {event_acks:?}"
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Reconnect with protocol → session_restored
// ---------------------------------------------------------------------------

#[test]
fn test_reconnect_with_protocol_string_includes_protocol_in_frame() {
    let _g = relay_mocktest::begin();
    let h = relay_mocktest::harness();
    unsafe {
        std::env::set_var("SIGNALWIRE_RELAY_SCHEME", "ws");
        std::env::set_var("SIGNALWIRE_RELAY_HOST", &h.relay_host);
    }

    // First connect — capture the issued protocol.
    let c1 = Arc::new(RelayClient::new("p", "t", &h.relay_host));
    {
        let mut ctx = c1.contexts.lock().unwrap();
        ctx.push("c1".to_string());
    }
    c1.connect().expect("first connect");
    let issued = c1
        .protocol
        .lock()
        .unwrap()
        .clone()
        .expect("expected issued protocol on first connect");
    c1.disconnect();

    // Second connect — pre-seed the protocol so it goes on the wire.
    let c2 = Arc::new(RelayClient::new("p", "t", &h.relay_host));
    {
        let mut ctx = c2.contexts.lock().unwrap();
        ctx.push("c1".to_string());
    }
    *c2.protocol.lock().unwrap() = Some(issued.clone());
    // The Client has no built-in "send protocol on connect" — until we
    // wire that in, drive it manually by setting protocol on the
    // outbound frame. Since the existing handshake doesn't add it,
    // we'll feed the protocol via authorization_state-style direct
    // injection: insert the protocol field in the next connect frame.
    // Expedient approach: send a second signalwire.connect frame
    // ourselves with the protocol field. But the Client's connect()
    // sends its own; the mock should still see it — we need to
    // augment connect to include `protocol` if pre-set.
    c2.connect().expect("second connect");
    c2.disconnect();

    // The journal should now have a connect frame whose params.protocol
    // matches the issued value (we'll add support in the SDK below).
    let connects = relay_mocktest::journal_recv(Some("signalwire.connect"));
    let has_resume = connects.iter().any(|e| {
        e.frame["params"]
            .get("protocol")
            .and_then(Value::as_str)
            == Some(issued.as_str())
    });
    assert!(
        has_resume,
        "no resume connect carried protocol={:?}; saw protocols={:?}",
        issued,
        connects
            .iter()
            .map(|e| e.frame["params"].get("protocol").and_then(Value::as_str))
            .collect::<Vec<_>>()
    );

    unsafe {
        std::env::remove_var("SIGNALWIRE_RELAY_SCHEME");
        std::env::remove_var("SIGNALWIRE_RELAY_HOST");
    }
}

#[test]
fn test_reconnect_with_protocol_preserves_protocol_value() {
    let _g = relay_mocktest::begin();
    let h = relay_mocktest::harness();
    unsafe {
        std::env::set_var("SIGNALWIRE_RELAY_SCHEME", "ws");
        std::env::set_var("SIGNALWIRE_RELAY_HOST", &h.relay_host);
    }
    let c1 = Arc::new(RelayClient::new("p", "t", &h.relay_host));
    c1.connect().expect("first connect");
    let issued = c1
        .protocol
        .lock()
        .unwrap()
        .clone()
        .expect("expected issued protocol");
    c1.disconnect();

    let c2 = Arc::new(RelayClient::new("p", "t", &h.relay_host));
    *c2.protocol.lock().unwrap() = Some(issued.clone());
    c2.connect().expect("second connect");
    // The mock returns the same protocol on resume.
    let after = c2
        .protocol
        .lock()
        .unwrap()
        .clone()
        .expect("protocol after resume");
    assert_eq!(after, issued);
    c2.disconnect();

    unsafe {
        std::env::remove_var("SIGNALWIRE_RELAY_SCHEME");
        std::env::remove_var("SIGNALWIRE_RELAY_HOST");
    }
}

// ---------------------------------------------------------------------------
// Auth failure paths
// ---------------------------------------------------------------------------

#[test]
fn test_unauthenticated_raw_connect_rejected_by_mock() {
    let _g = relay_mocktest::begin();
    // Bypass the SDK to send a connect with empty creds. We connect
    // directly with tungstenite, no SDK involvement.
    let h = relay_mocktest::harness();
    let url = h.ws_url.clone();

    let (mut sock, _resp) = tungstenite::connect(&url).expect("WS connect");
    let req_id = "raw-empty-cred-1";
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "id": req_id,
        "method": "signalwire.connect",
        "params": {
            "version": {"major": 2, "minor": 0, "revision": 0},
            "agent": "raw-test/1.0",
            "authentication": {"project": "", "token": ""},
        },
    });
    sock.send(tungstenite::Message::Text(frame.to_string().into()))
        .expect("send connect");
    // Read the reply.
    let resp = sock.read().expect("read reply");
    let txt = match resp {
        tungstenite::Message::Text(t) => t,
        other => panic!("expected text reply, got {other:?}"),
    };
    let parsed: Value = serde_json::from_str(&txt).expect("parse reply");
    let err = parsed
        .get("error")
        .expect("expected error from mock for empty creds");
    let code = err
        .get("data")
        .and_then(|d| d.get("signalwire_error_code"))
        .and_then(Value::as_str);
    assert_eq!(code, Some("AUTH_REQUIRED"));
    let _ = sock.close(None);
}

// ---------------------------------------------------------------------------
// JWT path
// ---------------------------------------------------------------------------

// Note: the Rust SDK's RelayClient::new takes (project, token, host); JWT
// support is a follow-up surface. For now, exercise the JWT path by
// driving the wire directly.
#[test]
fn test_jwt_only_connect_accepted_by_mock() {
    let _g = relay_mocktest::begin();
    let h = relay_mocktest::harness();
    let url = h.ws_url.clone();
    let (mut sock, _resp) = tungstenite::connect(&url).expect("WS connect");
    let frame = serde_json::json!({
        "jsonrpc": "2.0",
        "id": "jwt-1",
        "method": "signalwire.connect",
        "params": {
            "version": {"major": 2, "minor": 0, "revision": 0},
            "agent": "raw-jwt-test/1.0",
            "authentication": {"jwt_token": "fake-jwt-eyJ.AaaA.BbB"},
        },
    });
    sock.send(tungstenite::Message::Text(frame.to_string().into()))
        .expect("send jwt connect");
    let resp = sock.read().expect("read reply");
    let txt = match resp {
        tungstenite::Message::Text(t) => t,
        other => panic!("expected text reply, got {other:?}"),
    };
    let parsed: Value = serde_json::from_str(&txt).expect("parse reply");
    assert!(parsed.get("result").is_some());
    assert!(
        parsed["result"]["protocol"]
            .as_str()
            .map(|s| s.starts_with("signalwire_"))
            .unwrap_or(false)
    );
    let _ = sock.close(None);
}
