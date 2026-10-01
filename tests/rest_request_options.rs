// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Mock-backed integration tests for the `RequestOptions` envelope (plan 4.2).
//!
//! Mirrors signalwire-python `tests/unit/rest/test_request_options.py`: the
//! opt-in retry policy (retry-into-200, retries-exhausted, POST-no-retry-500,
//! POST-retry-503), the per-request-over-client-default override, timeout, and
//! cooperative abort — all exercised over the SHARED `mock_signalwire` server
//! (no transport mock). The mock's scenario store consumes overrides FIFO, so
//! arming a failure status N times makes the first N requests fail and the
//! rest fall through to the mock's default success — exactly what a retry loop
//! needs to prove its attempt COUNT on the wire.

#[path = "common/mod.rs"]
mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::json;
use signalwire::rest::RequestOptions;

// The endpoint ids the mock keys scenarios on (Spectral operationIds).
const GET_ENDPOINT: &str = "fabric.list_fabric_addresses";
const GET_PATH: &str = "/api/fabric/addresses";
const CREATE_ENDPOINT: &str = "relay-rest.create_address";
const CREATE_PATH: &str = "/api/relay/rest/addresses";

/// Count how many times THIS test's client hit `path` (per-test journal scope).
fn hits(path: &str) -> usize {
    common::mocktest::journal_all()
        .into_iter()
        .filter(|e| e.path == path)
        .count()
}

// ---------------------------------------------------------------------------
// GET retry-into-200: a single armed 503 is retried into the mock default 200.
// ---------------------------------------------------------------------------
#[test]
fn test_get_retry_once_succeeds() {
    let _g = common::mocktest::begin();
    let opts = RequestOptions::new().retries(1).retry_backoff(0.0);
    let c = common::mocktest::client_with_options(opts);

    // Arm the 503 ONCE; the retry falls through to the mock's synthesized 200.
    common::mocktest::scenario_set(
        GET_ENDPOINT,
        503,
        json!({"errors": [{"code": "UNAVAILABLE", "message": "transient"}]}),
    );

    let body = c
        .http()
        .get(GET_PATH, None)
        .expect("retry into 200 succeeds");
    assert!(body.is_object());
    // attempt + 1 retry == 2 requests on the wire.
    assert_eq!(hits(GET_PATH), 2, "expected 2 requests (1 retry)");
}

// ---------------------------------------------------------------------------
// GET retries exhausted: 503 armed on BOTH attempts -> typed 503, count 2.
// ---------------------------------------------------------------------------
#[test]
fn test_get_retry_exhausted() {
    let _g = common::mocktest::begin();
    let opts = RequestOptions::new().retries(1).retry_backoff(0.0);
    let c = common::mocktest::client_with_options(opts);

    // Arm 503 TWICE so every attempt (retries+1 == 2) sees the failure.
    let body = json!({"errors": [{"code": "UNAVAILABLE", "message": "down"}]});
    common::mocktest::scenario_set(GET_ENDPOINT, 503, body.clone());
    common::mocktest::scenario_set(GET_ENDPOINT, 503, body);

    let err = c
        .http()
        .get(GET_PATH, None)
        .expect_err("retries exhausted -> typed 503");
    assert_eq!(err.status_code(), 503);
    assert!(!err.is_transport());
    assert_eq!(hits(GET_PATH), 2, "expected retries+1 == 2 requests");
}

// ---------------------------------------------------------------------------
// POST 500 is NOT retried (duplicate-side-effect safety): count 1 despite
// retries=2 armed.
// ---------------------------------------------------------------------------
#[test]
fn test_post_500_not_retried() {
    let _g = common::mocktest::begin();
    let opts = RequestOptions::new().retries(2).retry_backoff(0.0);
    let c = common::mocktest::client_with_options(opts);

    common::mocktest::scenario_set(
        CREATE_ENDPOINT,
        500,
        json!({"errors": [{"code": "SERVER_ERROR", "message": "boom"}]}),
    );

    let err = c
        .http()
        .post(CREATE_PATH, Some(&json!({"label": "x"})), None)
        .expect_err("POST 500 -> typed 500, no retry");
    assert_eq!(err.status_code(), 500);
    // A non-idempotent method must NOT retry 500 -> exactly 1 request.
    assert_eq!(hits(CREATE_PATH), 1, "POST must not retry a 500");
}

// ---------------------------------------------------------------------------
// POST 503 IS retried (a throttle is safe — the request was not processed):
// a single armed 503 retried into the mock default success, count 2.
// ---------------------------------------------------------------------------
#[test]
fn test_post_503_retried() {
    let _g = common::mocktest::begin();
    let opts = RequestOptions::new().retries(1).retry_backoff(0.0);
    let c = common::mocktest::client_with_options(opts);

    // Arm 503 ONCE; the retry falls through to the mock's default success.
    common::mocktest::scenario_set(
        CREATE_ENDPOINT,
        503,
        json!({"errors": [{"code": "UNAVAILABLE", "message": "throttled"}]}),
    );

    let body = c
        .http()
        .post(CREATE_PATH, Some(&json!({"label": "x"})), None)
        .expect("POST 503 retried into success");
    assert!(body.is_object());
    assert_eq!(
        hits(CREATE_PATH),
        2,
        "POST 503 must retry once -> 2 requests"
    );
}

// ---------------------------------------------------------------------------
// Per-request override beats the client default: a client default of retries=0
// is overridden per-call to retries=1, which retries a single armed 503 into
// the mock default 200.
// ---------------------------------------------------------------------------
#[test]
fn test_per_request_override_beats_client_default() {
    let _g = common::mocktest::begin();
    // Client default: NO retry.
    let c = common::mocktest::client_with_options(RequestOptions::new().retries(0));

    common::mocktest::scenario_set(
        GET_ENDPOINT,
        503,
        json!({"errors": [{"code": "UNAVAILABLE", "message": "transient"}]}),
    );

    // Per-request override opts INTO one retry (backoff 0 so no wall-clock wait).
    let per = RequestOptions::new().retries(1).retry_backoff(0.0);
    let body = c
        .http()
        .get_with_options(GET_PATH, None, Some(&per))
        .expect("per-request override retries into 200");
    assert!(body.is_object());
    assert_eq!(hits(GET_PATH), 2, "per-request retries=1 -> 2 requests");
}

// ---------------------------------------------------------------------------
// Client default of retries=0 (the built-in) does NOT retry: a single armed
// 503 surfaces immediately, count 1.
// ---------------------------------------------------------------------------
#[test]
fn test_no_retry_by_default() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client(); // no options -> built-in defaults

    common::mocktest::scenario_set(
        GET_ENDPOINT,
        503,
        json!({"errors": [{"code": "UNAVAILABLE", "message": "once"}]}),
    );

    let err = c
        .http()
        .get(GET_PATH, None)
        .expect_err("no retry by default -> typed 503 on first response");
    assert_eq!(err.status_code(), 503);
    assert_eq!(hits(GET_PATH), 1, "default is no retry -> 1 request");
}

// ---------------------------------------------------------------------------
// Abort signal: a signal already set before the call raises the TYPED transport
// error before any request reaches the mock (count 0).
// ---------------------------------------------------------------------------
#[test]
fn test_abort_signal_cancels_before_send() {
    let _g = common::mocktest::begin();
    let signal = Arc::new(AtomicBool::new(true)); // already cancelled
    let opts = RequestOptions::new().abort_signal(signal);
    let c = common::mocktest::client_with_options(opts);

    let err = c
        .http()
        .get(GET_PATH, None)
        .expect_err("a set abort_signal cancels before the send");
    assert!(
        err.is_transport(),
        "cancellation is a typed transport error"
    );
    assert_eq!(err.status_code(), 0);
    assert_eq!(hits(GET_PATH), 0, "nothing should reach the mock");
}

// ---------------------------------------------------------------------------
// Abort signal set BETWEEN attempts: retries armed, but a signal flipped after
// the first failing attempt stops the retry before the second send. The mock
// sees exactly one request (the pre-second-attempt abort check fires).
// ---------------------------------------------------------------------------
#[test]
fn test_abort_signal_stops_retry_between_attempts() {
    let _g = common::mocktest::begin();
    let signal = Arc::new(AtomicBool::new(false));
    let sig_for_flip = Arc::clone(&signal);
    // Flip the signal on another thread shortly after the call begins, so the
    // pre-second-attempt abort check catches it. retry_backoff gives a small
    // window; keep it tiny so the test stays fast but the flip lands in time.
    let opts = RequestOptions::new()
        .retries(3)
        .retry_backoff(0.05)
        .abort_signal(signal);
    let c = common::mocktest::client_with_options(opts);

    // Arm 503 on every attempt so, absent the abort, it would retry 4x.
    let body = json!({"errors": [{"code": "UNAVAILABLE", "message": "down"}]});
    for _ in 0..4 {
        common::mocktest::scenario_set(GET_ENDPOINT, 503, body.clone());
    }

    let flip = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(10));
        sig_for_flip.store(true, Ordering::SeqCst);
    });

    let err = c
        .http()
        .get(GET_PATH, None)
        .expect_err("abort between attempts -> typed transport error");
    let _ = flip.join();
    assert!(err.is_transport(), "abort surfaces as a transport error");
    // Exactly one request reached the mock: the first attempt failed 503, then
    // the pre-second-attempt abort check fired before any further send.
    assert_eq!(
        hits(GET_PATH),
        1,
        "abort between attempts stops the retry after the first request"
    );
}

// ---------------------------------------------------------------------------
// Timeout: a tiny per-attempt timeout against a deliberately slow mock response
// surfaces the TYPED transport error (status 0), not a bare error. The mock's
// `delay_ms` scenario field drives a slow response.
// ---------------------------------------------------------------------------
#[test]
fn test_timeout_surfaces_typed_transport_error() {
    let _g = common::mocktest::begin();
    // 100ms per-attempt deadline; no retry.
    let opts = RequestOptions::new().timeout(0.1).retries(0);
    let c = common::mocktest::client_with_options(opts);

    // Arm a 2000ms-delayed response so the 100ms deadline trips first. The
    // scenario helper only sets status+body; the mock's delay field is set via
    // the raw scenario endpoint, so use it directly here.
    common::mocktest::scenario_set_delayed(GET_ENDPOINT, 200, json!({"data": []}), 2000);

    let err = c
        .http()
        .get(GET_PATH, None)
        .expect_err("a per-attempt timeout surfaces the typed transport error");
    assert!(
        err.is_transport(),
        "timeout must be a typed transport error, got: {err}"
    );
    assert_eq!(err.status_code(), 0);
}
