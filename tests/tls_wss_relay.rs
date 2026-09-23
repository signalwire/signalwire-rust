// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! TLS capability test (quadrant 1 of 3): prove the RELAY client performs a
//! *real*, verified `wss://` handshake.
//!
//! Drives the real `signalwire::relay::Client::connect()` against the shared
//! `mock_relay` started in `--tls` mode (so the WebSocket endpoint is `wss://`
//! backed by the porting-sdk self-signed test CA). The client trusts that CA
//! via `SIGNALWIRE_RELAY_CA_FILE` (the SDK builds a rustls `RootCertStore` from
//! it — see `src/relay/client.rs::tls`). The server-issued protocol string can
//! only come back over a genuinely-completed TLS + RELAY handshake.
//!
//! Negative control: the same `wss://` endpoint, dialed *without* the test CA
//! (falling back to the webpki Mozilla roots), must be rejected — proving the
//! cert is actually verified, not skipped. No `danger_accept_invalid_certs`.

#[path = "common/mod.rs"]
mod common;

use std::sync::Arc;

use common::tls_support;
use serde_json::Value;
use signalwire::relay::Client as RelayClient;

#[test]
fn tls_relay_client_wss_connect_and_auth() {
    let Some(ca) = tls_support::ca_file() else {
        eprintln!("skip: porting-sdk/test_harness/tls not adjacent");
        return;
    };
    // Serialize against any other binary touching the dedicated TLS mock-relay.
    let _lock = tls_support::RelayTlsLock::acquire();

    let Some((_mock, http_url)) = tls_support::spawn_tls_mock_relay() else {
        eprintln!("skip: could not start `python -m mock_relay --tls`");
        return;
    };

    // Point the real RELAY client at the wss:// endpoint and trust the test CA.
    // SAFETY: this binary declares exactly ONE `#[test]`, so no sibling thread can
    // observe or clobber the process env while it runs, and `RelayTlsLock` (held
    // above) serializes it against other BINARIES for the shared TLS mock-relay.
    // Note the process env is per-process, so that flock is not what protects these
    // vars — the single-test-per-binary property is. It is NOT `--test-threads=1`:
    // the TEST gate runs `cargo test --tests` fully parallel (only the REST-COVERAGE
    // and RELAY-mock gates pass that flag). If a SECOND test is ever added here it
    // must take a file-local `ENV_LOCK` first, the way
    // `tests/tls_no_silent_downgrade.rs` and `src/server/tls.rs` do.
    unsafe {
        std::env::set_var("SIGNALWIRE_RELAY_SCHEME", "wss");
        std::env::set_var(
            "SIGNALWIRE_RELAY_HOST",
            format!("127.0.0.1:{}", tls_support::TLS_RELAY_WS_PORT),
        );
        std::env::set_var("SIGNALWIRE_RELAY_CA_FILE", &ca);
    }

    let client = Arc::new(RelayClient::new(
        "test_proj",
        "test_tok",
        &format!("127.0.0.1:{}", tls_support::TLS_RELAY_WS_PORT),
    ));
    {
        let mut ctx = client.contexts.lock().unwrap();
        ctx.push("default".to_string());
    }

    client
        .connect()
        .expect("relay Client::connect() over wss:// should succeed with the test CA trusted");

    // Behavioral proof: the mock only issues a `signalwire_<uuid>` protocol
    // string on a successful credential exchange — which requires a completed
    // TLS session. An empty value means the connect round-trip never happened.
    let proto = client
        .protocol
        .lock()
        .unwrap()
        .clone()
        .expect("RelayProtocol empty after WSS Authenticate; server value missing");
    assert!(
        proto.starts_with("signalwire_"),
        "unexpected protocol shape over WSS: {proto:?}"
    );
    assert!(client.is_connected());

    // Wire proof: the mock journaled the inbound signalwire.connect frame on
    // the same (TLS) WebSocket. Journal is read over the plain-HTTP control
    // plane (mock_relay keeps the control plane HTTP even in --tls).
    let recv = journal_recv_methods(&http_url);
    assert!(
        recv.iter().any(|m| m == "signalwire.connect"),
        "mock journal has no recv signalwire.connect frame over the WSS connection; saw {recv:?}"
    );

    client.disconnect();

    // Negative control: the same wss:// endpoint dialed WITHOUT the test CA
    // (default webpki roots only) must be rejected, proving real verification.
    unsafe {
        std::env::remove_var("SIGNALWIRE_RELAY_CA_FILE");
    }
    let untrusted = Arc::new(RelayClient::new(
        "test_proj",
        "test_tok",
        &format!("127.0.0.1:{}", tls_support::TLS_RELAY_WS_PORT),
    ));
    let result = untrusted.connect();
    assert!(
        result.is_err(),
        "WSS connect with only webpki roots unexpectedly succeeded against the self-signed CA"
    );
    let err = result.unwrap_err();
    // The typed error is a Transport failure (not auth / timeout / dial),
    // carrying the connect context + the rustls verification cause.
    assert!(
        matches!(err, signalwire::relay::RelayError::Transport { .. }),
        "expected a Transport error, got: {err:?}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("connect")
            || msg.to_lowercase().contains("certif")
            || msg.contains("Tls")
            || msg.to_lowercase().contains("handshake"),
        "untrusted WSS connect failed for an unexpected reason: {msg}"
    );
    eprintln!("untrusted WSS dial correctly rejected: {msg}");

    // Clean up env so it doesn't leak to other tests in the binary.
    unsafe {
        std::env::remove_var("SIGNALWIRE_RELAY_SCHEME");
        std::env::remove_var("SIGNALWIRE_RELAY_HOST");
    }
}

/// Read the mock's journal over the plain-HTTP control plane and return the
/// method names of inbound (SDK->server) frames.
fn journal_recv_methods(http_url: &str) -> Vec<String> {
    let url = format!("{http_url}/__mock__/journal");
    let mut resp = ureq::get(&url)
        .call()
        .unwrap_or_else(|e| panic!("GET {url}: {e}"));
    let body: Value = resp
        .body_mut()
        .read_json()
        .unwrap_or_else(|e| panic!("decode journal: {e}"));
    body.as_array()
        .map(|arr| {
            arr.iter()
                .filter(|e| e.get("direction").and_then(Value::as_str) == Some("recv"))
                .filter_map(|e| e.get("method").and_then(Value::as_str).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}
