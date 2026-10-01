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

/// Scope access to the process-global CA env vars.
///
/// The SDK reads the CA bundle path off the PROCESS environment
/// (`rest/http_client.rs::custom_ca_tls_config`, `relay/client.rs::CA_FILE_ENV`)
/// with no per-client override, so `SIGNALWIRE_{RELAY,REST}_CA_FILE` is genuinely
/// ONE shared resource for every test in this binary. The four tests below split
/// into two pairs: one SETS a CA and asserts the connect is refused, the other
/// asserts plain transport still works with NO CA configured — so a `set_var` in
/// one thread is read by the other's connect and the control panics on the very
/// var whose absence is its entire premise.
///
/// The comment this replaces claimed "integration tests run single-threaded
/// (`--test-threads=1`)". That is FALSE: `scripts/run-tests.sh` and run-ci's TEST
/// gate both run `cargo test --tests` with cargo's default parallelism (the two
/// `--test-threads=1` invocations in run-ci.sh belong to the REST-COVERAGE and
/// RELAY-mock gates, which name specific `--test` binaries). Ordering alone cannot
/// fix it either — the control already cleared the var before connecting and still
/// lost the race.
///
/// Isolation therefore comes from SCOPING the shared resource, never from removing
/// concurrency: the lock is file-local and held only across the env mutation plus
/// the connect it configures. Same pattern as `src/server/tls.rs`'s `ENV_LOCK`.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    let env_guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: ENV_LOCK held across the mutation and the connect it configures.
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

    // SAFETY: ENV_LOCK still held.
    unsafe {
        std::env::remove_var("SIGNALWIRE_RELAY_CA_FILE");
        std::env::remove_var("SIGNALWIRE_RELAY_SCHEME");
        std::env::remove_var("SIGNALWIRE_RELAY_HOST");
    }
    drop(env_guard);
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
    let env_guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: ENV_LOCK held across the mutation and the connect it configures.
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
    // SAFETY: ENV_LOCK still held.
    unsafe {
        std::env::remove_var("SIGNALWIRE_RELAY_SCHEME");
        std::env::remove_var("SIGNALWIRE_RELAY_HOST");
    }
    drop(env_guard);
    if connected {
        client.disconnect();
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

    let env_guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: ENV_LOCK held across the mutation and the request it configures.
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
    // SAFETY: ENV_LOCK still held.
    unsafe {
        std::env::remove_var("SIGNALWIRE_REST_CA_FILE");
    }
    drop(env_guard);

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
    let env_guard = ENV_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // SAFETY: ENV_LOCK held across the mutation and the request it configures.
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
    drop(env_guard);
    assert!(
        body.as_object().is_some_and(|o| o.contains_key("data")),
        "unexpected mock response: {body:?}"
    );
}
