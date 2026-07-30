// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! TLS capability test (quadrant 4): prove a client CONFIGURED for TLS never
//! silently completes a PLAINTEXT session.
//!
//! The three sibling `tls_*` tests prove the positive direction — a `wss://` /
//! `https://` client really does verify the peer certificate, with a negative
//! control showing an untrusted CA is rejected. This one covers the opposite
//! failure mode: a user who asked for encryption, got plaintext, and was never
//! told. Pointing `SIGNALWIRE_RELAY_CA_FILE` at a CA bundle is that request —
//! it means "verify the peer against this CA" and is meaningless without TLS.
//!
//! Driven behaviourally against the shared plain-mode `mock_relay` (a real
//! `ws://` listener), not by inspecting flags.

#[path = "common/mod.rs"]
mod common;

use std::sync::Arc;

use common::{mocktest, relay_mocktest, tls_support};
use signalwire::relay::Client as RelayClient;
use signalwire::rest::RestClient;

/// A CA bundle is configured (the caller asked for a verified TLS peer) but the
/// scheme resolves to plain `ws://`. The connect must FAIL rather than quietly
/// establishing an unencrypted session.
#[test]
fn relay_ca_configured_refuses_plaintext_scheme() {
    let Some(ca) = tls_support::ca_file() else {
        eprintln!("skip: porting-sdk/test_harness/tls not adjacent");
        return;
    };
    let h = relay_mocktest::harness();

    // SAFETY: integration tests run single-threaded (`--test-threads=1`).
    unsafe {
        std::env::set_var("SIGNALWIRE_RELAY_SCHEME", "ws");
        std::env::set_var("SIGNALWIRE_RELAY_HOST", format!("127.0.0.1:{}", h.ws_port));
        std::env::set_var("SIGNALWIRE_RELAY_CA_FILE", &ca);
    }

    let client = Arc::new(RelayClient::new(
        "test_proj",
        "test_tok",
        &format!("127.0.0.1:{}", h.ws_port),
    ));
    let result = client.connect();

    unsafe {
        std::env::remove_var("SIGNALWIRE_RELAY_CA_FILE");
        std::env::remove_var("SIGNALWIRE_RELAY_SCHEME");
        std::env::remove_var("SIGNALWIRE_RELAY_HOST");
    }
    if client.is_connected() {
        client.disconnect();
    }

    assert!(
        result.is_err(),
        "a CA-configured client completed a PLAINTEXT ws:// session — silent \
         TLS downgrade: the caller asked for a verified peer and got none"
    );
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("SIGNALWIRE_RELAY_CA_FILE"),
        "downgrade refusal must name the setting that was ignored; got: {msg}"
    );
}

/// Control: with no CA configured, plain `ws://` is the audit/mock transport and
/// must keep working. The refusal above must be scoped to "TLS was requested",
/// not a blanket ban on `ws://`.
#[test]
fn relay_plain_ws_still_connects_without_ca() {
    let h = relay_mocktest::harness();
    unsafe {
        std::env::remove_var("SIGNALWIRE_RELAY_CA_FILE");
        std::env::set_var("SIGNALWIRE_RELAY_SCHEME", "ws");
        std::env::set_var("SIGNALWIRE_RELAY_HOST", format!("127.0.0.1:{}", h.ws_port));
    }
    let client = Arc::new(RelayClient::new(
        "test_proj",
        "test_tok",
        &format!("127.0.0.1:{}", h.ws_port),
    ));
    let result = client.connect();
    let connected = client.is_connected();
    if connected {
        client.disconnect();
    }
    unsafe {
        std::env::remove_var("SIGNALWIRE_RELAY_SCHEME");
        std::env::remove_var("SIGNALWIRE_RELAY_HOST");
    }
    assert!(
        result.is_ok(),
        "plain ws:// with no CA configured must still connect: {result:?}"
    );
    assert!(connected, "plain ws:// client reported not connected");
}

/// REST twin of the RELAY case: `SIGNALWIRE_REST_CA_FILE` is set (the caller
/// asked for a verified TLS peer) but the base URL is plain `http://`. The
/// request must be refused, not sent in the clear.
#[test]
fn rest_ca_configured_refuses_plaintext_base_url() {
    let Some(ca) = tls_support::ca_file() else {
        eprintln!("skip: porting-sdk/test_harness/tls not adjacent");
        return;
    };
    let h = mocktest::harness();
    let base_url = format!("http://127.0.0.1:{}", h.port);

    // SAFETY: integration tests run single-threaded (`--test-threads=1`).
    unsafe {
        std::env::set_var("SIGNALWIRE_REST_CA_FILE", &ca);
    }
    let built = RestClient::with_base_url("test_proj", "test_tok", &base_url);
    let sent = built.as_ref().ok().map(|client| {
        client
            .fabric()
            .addresses()
            .list(&std::collections::HashMap::new(), None)
    });
    unsafe {
        std::env::remove_var("SIGNALWIRE_REST_CA_FILE");
    }

    match sent {
        // Refused at construction — also acceptable, and the message must say why.
        None => {
            let msg = match built {
                Err(e) => e.to_string(),
                Ok(_) => unreachable!("sent is None only when construction failed"),
            };
            assert!(
                msg.contains("SIGNALWIRE_REST_CA_FILE"),
                "construction refusal must name the ignored setting; got: {msg}"
            );
        }
        Some(Ok(body)) => panic!(
            "a CA-configured REST client completed a PLAINTEXT http:// request — \
             silent TLS downgrade. Response: {body:?}"
        ),
        Some(Err(e)) => {
            let msg = e.to_string();
            assert!(
                msg.contains("SIGNALWIRE_REST_CA_FILE"),
                "downgrade refusal must name the ignored setting; got: {msg}"
            );
        }
    }
}

/// Control: with no CA configured, plain `http://` against the mock is the
/// audit transport and must keep working.
#[test]
fn rest_plain_http_still_works_without_ca() {
    let h = mocktest::harness();
    let base_url = format!("http://127.0.0.1:{}", h.port);
    unsafe {
        std::env::remove_var("SIGNALWIRE_REST_CA_FILE");
    }
    let client = RestClient::with_base_url("test_proj", "test_tok", &base_url)
        .expect("RestClient::with_base_url");
    let body = client
        .fabric()
        .addresses()
        .list(&std::collections::HashMap::new(), None)
        .expect("plain http:// with no CA configured must still work");
    assert!(
        body.as_object().is_some_and(|o| o.contains_key("data")),
        "unexpected mock response: {body:?}"
    );
}
