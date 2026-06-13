// Mock-backed integration tests translated from
// signalwire-python/tests/unit/relay/test_messaging_mock.py.
//
// Drives the real `signalwire::relay::Client::send_message_blocking()`
// against the shared `mock_relay` server, then asserts both:
//
//   1. SDK state on the returned Message (or pushed inbound).
//   2. The journaled `messaging.send` (or `messaging.receive`) wire frame.

#[path = "common/mod.rs"]
mod common;

use serde_json::{Value, json};

use common::relay_mocktest;

const SHORT_WAIT_MS: u64 = 100;

// Helper: poll a closure until it returns true or budget elapses.
fn wait_until<F: Fn() -> bool>(budget_ms: u64, f: F) -> bool {
    use std::time::{Duration, Instant};
    let deadline = Instant::now() + Duration::from_millis(budget_ms);
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

// ---------------------------------------------------------------------------
// send_message — outbound
// ---------------------------------------------------------------------------

#[test]
fn test_send_message_journals_messaging_send() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let tags = vec!["t1".to_string(), "t2".to_string()];
    let msg = client
        .send_message_blocking(
            "+15551112222",
            "+15553334444",
            Some("hello"),
            None,
            Some(&tags),
            None,
        )
        .expect("send_message_blocking");
    assert!(
        msg.message_id().is_some(),
        "message_id should be set by mock"
    );
    assert_eq!(msg.body(), Some("hello".to_string()));
    let entry = relay_mocktest::journal_recv(Some("messaging.send"))
        .into_iter()
        .next()
        .expect("expected one messaging.send frame");
    let p = entry.inner_params();
    assert_eq!(
        p.get("to_number").and_then(Value::as_str),
        Some("+15551112222")
    );
    assert_eq!(
        p.get("from_number").and_then(Value::as_str),
        Some("+15553334444")
    );
    assert_eq!(p.get("body").and_then(Value::as_str), Some("hello"));
    let wire_tags = p
        .get("tags")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert_eq!(wire_tags, vec!["t1".to_string(), "t2".to_string()]);
    client.disconnect();
}

#[test]
fn test_send_message_with_media_only() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let media = vec!["https://media.example/cat.jpg".to_string()];
    let _msg = client
        .send_message_blocking(
            "+15551112222",
            "+15553334444",
            None,
            Some(&media),
            None,
            None,
        )
        .expect("send_message_blocking media-only");
    let entry = relay_mocktest::journal_recv(Some("messaging.send"))
        .into_iter()
        .next()
        .expect("expected one messaging.send frame");
    let p = entry.inner_params();
    let wire_media = p
        .get("media")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    assert_eq!(
        wire_media,
        vec!["https://media.example/cat.jpg".to_string()]
    );
    let body = p.get("body").and_then(Value::as_str);
    assert!(body.is_none() || body == Some(""));
    client.disconnect();
}

#[test]
fn test_send_message_includes_context() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let _msg = client
        .send_message_blocking(
            "+15551112222",
            "+15553334444",
            Some("hi"),
            None,
            None,
            Some("custom-ctx"),
        )
        .expect("send_message_blocking");
    let entry = relay_mocktest::journal_recv(Some("messaging.send"))
        .into_iter()
        .next()
        .expect("expected messaging.send frame");
    assert_eq!(
        entry.inner_params().get("context").and_then(Value::as_str),
        Some("custom-ctx")
    );
    client.disconnect();
}

#[test]
fn test_send_message_returns_initial_state_queued() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let msg = client
        .send_message_blocking("+15551112222", "+15553334444", Some("hi"), None, None, None)
        .expect("send_message_blocking");
    assert_eq!(msg.state(), Some("queued".to_string()));
    assert!(!msg.is_done());
    client.disconnect();
}

#[test]
fn test_send_message_resolves_on_delivered() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let msg = client
        .send_message_blocking("+15551112222", "+15553334444", Some("hi"), None, None, None)
        .expect("send_message_blocking");
    let mid = msg
        .message_id()
        .expect("message_id should be set")
        .to_string();
    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "id": "evt-deliv-1",
        "method": "signalwire.event",
        "params": {
            "event_type": "messaging.state",
            "params": {
                "message_id": mid,
                "state": "delivered",
                "from_number": "+15553334444",
                "to_number": "+15551112222",
                "body": "hi",
            },
        },
    }));
    assert!(
        wait_until(2000, || msg.is_done()),
        "msg did not transition to terminal within 2s; state={:?}",
        msg.state()
    );
    assert_eq!(msg.state(), Some("delivered".to_string()));
    client.disconnect();
}

#[test]
fn test_send_message_resolves_on_undelivered() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let msg = client
        .send_message_blocking("+15551112222", "+15553334444", Some("hi"), None, None, None)
        .expect("send_message_blocking");
    let mid = msg.message_id().unwrap().to_string();
    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "id": "evt-und-1",
        "method": "signalwire.event",
        "params": {
            "event_type": "messaging.state",
            "params": {
                "message_id": mid,
                "state": "undelivered",
                "reason": "carrier_blocked",
            },
        },
    }));
    assert!(wait_until(2000, || msg.is_done()));
    assert_eq!(msg.state(), Some("undelivered".to_string()));
    assert_eq!(msg.reason(), Some("carrier_blocked".to_string()));
    client.disconnect();
}

#[test]
fn test_send_message_resolves_on_failed() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let msg = client
        .send_message_blocking("+15551112222", "+15553334444", Some("hi"), None, None, None)
        .expect("send_message_blocking");
    let mid = msg.message_id().unwrap().to_string();
    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "id": "evt-fail-1",
        "method": "signalwire.event",
        "params": {
            "event_type": "messaging.state",
            "params": {
                "message_id": mid,
                "state": "failed",
                "reason": "spam",
            },
        },
    }));
    assert!(wait_until(2000, || msg.is_done()));
    assert_eq!(msg.state(), Some("failed".to_string()));
    client.disconnect();
}

#[test]
fn test_send_message_intermediate_state_does_not_resolve() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let msg = client
        .send_message_blocking("+15551112222", "+15553334444", Some("hi"), None, None, None)
        .expect("send_message_blocking");
    let mid = msg.message_id().unwrap().to_string();
    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "id": "evt-sent-1",
        "method": "signalwire.event",
        "params": {
            "event_type": "messaging.state",
            "params": {"message_id": mid, "state": "sent"},
        },
    }));
    // Wait for state to reach 'sent', and ensure it stays non-terminal.
    assert!(wait_until(2000, || msg.state() == Some("sent".to_string())));
    assert!(!msg.is_done());
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Inbound messages
// ---------------------------------------------------------------------------

#[test]
fn test_inbound_message_fires_on_message_handler() {
    use std::sync::{Arc, Mutex};
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    let received: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
    let received2 = received.clone();
    client.on_message(move |_event, params| {
        *received2.lock().unwrap() = Some(params.clone());
    });

    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "id": "evt-in-1",
        "method": "signalwire.event",
        "params": {
            "event_type": "messaging.receive",
            "params": {
                "message_id": "in-msg-1",
                "context": "default",
                "direction": "inbound",
                "from_number": "+15551110000",
                "to_number": "+15552220000",
                "body": "hello back",
                "media": [],
                "segments": 1,
                "message_state": "received",
                "tags": ["incoming"],
            },
        },
    }));
    assert!(wait_until(2000, || received.lock().unwrap().is_some()));
    let p = received.lock().unwrap().clone().unwrap();
    assert_eq!(
        p.get("message_id").and_then(Value::as_str),
        Some("in-msg-1")
    );
    assert_eq!(
        p.get("from_number").and_then(Value::as_str),
        Some("+15551110000")
    );
    assert_eq!(p.get("body").and_then(Value::as_str), Some("hello back"));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// State progression — full pipeline
// ---------------------------------------------------------------------------

#[test]
fn test_full_message_state_progression() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let msg = client
        .send_message_blocking(
            "+15551112222",
            "+15553334444",
            Some("full pipeline"),
            None,
            None,
            None,
        )
        .expect("send_message_blocking");
    let mid = msg.message_id().unwrap().to_string();

    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "id": "evt-sent",
        "method": "signalwire.event",
        "params": {
            "event_type": "messaging.state",
            "params": {"message_id": mid, "state": "sent"},
        },
    }));
    assert!(wait_until(2000, || msg.state() == Some("sent".to_string())));

    let mid2 = mid.clone();
    relay_mocktest::push(json!({
        "jsonrpc": "2.0",
        "id": "evt-deliv",
        "method": "signalwire.event",
        "params": {
            "event_type": "messaging.state",
            "params": {"message_id": mid2, "state": "delivered"},
        },
    }));
    assert!(wait_until(2000, || msg.is_done()));
    assert_eq!(msg.state(), Some("delivered".to_string()));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Anti-cheat: hit the journal at least once with a non-trivial assertion
// ---------------------------------------------------------------------------

#[test]
fn test_journal_last_records_messaging_send_method() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let _ = client
        .send_message_blocking(
            "+15551112222",
            "+15553334444",
            Some("anti-cheat"),
            None,
            None,
            None,
        )
        .expect("send_message_blocking");
    // Wait briefly for the journal write — the SDK has already returned
    // by the time we get here, but the mock writes the response after
    // reading the request.
    std::thread::sleep(std::time::Duration::from_millis(SHORT_WAIT_MS));
    // Find the most recent recv frame whose inner method is messaging.send.
    let entries = relay_mocktest::journal_recv(Some("messaging.send"));
    assert!(!entries.is_empty(), "journal should record messaging.send");
    let last = entries.last().unwrap();
    assert_eq!(last.method, "messaging.send");
    assert_eq!(
        last.inner_params().get("body").and_then(Value::as_str),
        Some("anti-cheat")
    );
    client.disconnect();
}
