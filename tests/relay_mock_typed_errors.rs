//! Typed-error integration tests for the RELAY client surface.
//!
//! These drive the REAL `signalwire::relay::Client` against the shared
//! `mock_relay` server and assert that genuine failures surface as the right
//! [`RelayError`] *variant* (not a stringly-typed message). This is the
//! behavioural proof for the idiom-pass item "typed error enums replacing
//! `Result<_, String>`": the failure modes are now a closed, inspectable set,
//! mirroring the REST layer's `SignalWireRestError`.
//!
//! No mocks of the transport — every error here is produced by the real mock
//! server rejecting a real frame, or by the client's own real validation.

#[path = "common/mod.rs"]
mod common;

use std::sync::Arc;
use std::time::Duration;

use common::relay_mocktest;
use serde_json::json;
use signalwire::relay::{Client as RelayClient, RelayError};

/// Build a client pointed at the mock but with caller-chosen credentials, so a
/// test can drive the auth-rejection path. Mirrors `connected_client` minus the
/// "connect must succeed" assertion. The mock-redirect env vars are set once
/// per process (same values for every test), so this is parallel-safe.
fn client_with_creds(project: &str, token: &str) -> Arc<RelayClient> {
    let h = relay_mocktest::harness();
    relay_mocktest::ensure_redirect();
    Arc::new(RelayClient::new(project, token, &h.relay_host))
}

// ---------------------------------------------------------------------------
// RelayError::Auth — the mock rejects a connect with a missing project.
// ---------------------------------------------------------------------------

#[test]
fn test_connect_with_empty_project_yields_auth_variant() {
    let _g = relay_mocktest::begin();
    // Empty project → mock's auth issuer returns a JSON-RPC error
    // ("project missing"). The client must surface that as RelayError::Auth.
    let client = client_with_creds("", "test_tok");
    let result = client.connect();

    let err = result.expect_err("connect with empty project must fail");
    match &err {
        RelayError::Auth { message } => {
            // The server's reason is preserved verbatim in the variant.
            assert!(
                message.contains("project") || message.contains("missing"),
                "auth message should echo the server reason, got: {message:?}"
            );
        }
        other => panic!("expected RelayError::Auth, got {other:?}"),
    }
    // Display is actionable and names the failure mode.
    assert!(err.to_string().contains("auth error"));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// RelayError::Rpc — the mock returns an error for an unknown method.
// ---------------------------------------------------------------------------

#[test]
fn test_execute_unknown_method_yields_rpc_variant() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    // `bogus.method` is not handled by the mock → it replies with a
    // JSON-RPC error (ERROR_UNKNOWN_METHOD). The client must surface that as
    // RelayError::Rpc carrying the method we sent.
    let result = client.execute_blocking("bogus.method", json!({"x": 1}));
    let err = result.expect_err("unknown method must fail");
    match &err {
        RelayError::Rpc {
            method,
            message,
            code,
        } => {
            assert_eq!(method, "bogus.method", "the failing method is carried");
            assert!(!message.is_empty(), "server message is preserved");
            // The mock's JSON-RPC `error.code` reaches the caller.
            assert!(
                code.is_some(),
                "the JSON-RPC error code is carried, not discarded"
            );
        }
        other => panic!("expected RelayError::Rpc, got {other:?}"),
    }
    assert!(err.to_string().contains("bogus.method"));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// RelayError::InvalidArgument — client-side validation (no body, no media).
// ---------------------------------------------------------------------------

#[test]
fn test_send_message_without_body_or_media_yields_invalid_argument() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    // Neither body nor media → the client rejects before hitting the wire.
    // `Arc<Message>` is not Debug, so match instead of `expect_err`.
    let result =
        client.send_message_blocking("+15551112222", "+15553334444", None, None, None, None);
    match result {
        Ok(_) => panic!("send with no body/media must fail"),
        Err(err) => {
            assert!(
                matches!(err, RelayError::InvalidArgument { .. }),
                "expected RelayError::InvalidArgument, got {err:?}"
            );
            assert!(err.to_string().contains("body or media"));
        }
    }

    // Cross-check: this validation never reached the server (no messaging.send
    // frame was journaled), proving it's a real client-side guard.
    let sent = relay_mocktest::journal_recv(Some("messaging.send"));
    assert!(
        sent.is_empty(),
        "invalid-arg path must not emit a wire frame"
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// RelayError::DialFailed — a dial that never gets an answer times out.
// ---------------------------------------------------------------------------

#[test]
fn test_dial_without_answer_yields_dial_failed_variant() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);

    // Don't arm a dial answer; the mock's calling.dial returns its
    // acknowledgement but no `calling.call.dial` answer event ever arrives,
    // so the SDK's short dial deadline elapses → RelayError::DialFailed.
    let devices = json!([[{"type": "phone", "params": {"to_number": "+15550000000"}}]]);
    // `Arc<Call>` is not Debug, so match instead of `expect_err`.
    let result = client.dial_blocking(
        devices,
        Some("tag-noanswer"),
        None,
        Duration::from_millis(300),
    );
    match result {
        Ok(_) => panic!("dial with no answer must fail"),
        Err(RelayError::DialFailed { reason }) => {
            assert!(
                reason.contains("tag-noanswer") || reason.contains("timed out"),
                "dial-failed reason should be actionable, got: {reason:?}"
            );
        }
        Err(other) => panic!("expected RelayError::DialFailed, got {other:?}"),
    }
    client.disconnect();
}

// ---------------------------------------------------------------------------
// Positive control — a successful call still returns Ok (the typed Err channel
// did not break the happy path).
// ---------------------------------------------------------------------------

#[test]
fn test_successful_send_still_returns_ok() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let msg = client
        .send_message_blocking(
            "+15551112222",
            "+15553334444",
            Some("typed-ok"),
            None,
            None,
            None,
        )
        .expect("a valid send must still succeed");
    assert!(msg.message_id().is_some());
    client.disconnect();
}
