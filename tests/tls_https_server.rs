// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! TLS capability test (quadrant 3 of 3): prove the SDK's *own* webhook / SWML
//! server serves a real, verified `https://` endpoint.
//!
//! Starts a `signalwire::swml::Service` whose `run()` binds with `tiny_http`'s
//! `ssl-rustls` HTTPS listener — selected through the `SWML_SSL_ENABLED` /
//! `SWML_SSL_CERT_PATH` / `SWML_SSL_KEY_PATH` env vars (mirroring Python's
//! `SecurityConfig` / uvicorn `ssl_*`). The leaf cert is the shared porting-sdk
//! self-signed `server.crt` (SAN localhost/127.0.0.1). A rustls client
//! (`ureq`) that trusts the test CA then reaches the unauthenticated `/health`
//! route over `https://` and asserts a real `{"status":"healthy"}` body.
//!
//! Negative control: a client that does NOT trust the test CA (webpki Mozilla
//! roots only) must be rejected — proving the server presents a cert that is
//! actually verified. No `danger_accept_invalid_certs`.

#[path = "common/mod.rs"]
mod common;

use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use common::tls_support;
use serde_json::Value;
use signalwire::swml::service::{Service, ServiceOptions};

/// Serialize this test against itself across binaries — only one HTTPS server
/// should own the chosen port at a time. (The test picks an ephemeral port, so
/// collisions are unlikely, but the mutex also guards the process-global
/// `SWML_SSL_*` env vars set below.)
static SERVER_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn tls_sdk_server_serves_verified_https() {
    let _g = SERVER_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let Some(certs) = tls_support::certs_dir() else {
        eprintln!("skip: porting-sdk/test_harness/tls not adjacent");
        return;
    };
    let cert_path = certs.join("server.crt");
    let key_path = certs.join("server.key");
    let ca_path = certs.join("ca.crt");

    let port = free_tcp_port();

    // Configure the SDK server for HTTPS via the documented SWML_SSL_* env
    // contract (mirrors Python). `run()` reads these at bind time.
    // SAFETY: integration test is single-threaded and holds SERVER_TEST_LOCK.
    unsafe {
        std::env::set_var("SWML_SSL_ENABLED", "true");
        std::env::set_var("SWML_SSL_CERT_PATH", &cert_path);
        std::env::set_var("SWML_SSL_KEY_PATH", &key_path);
    }

    // Run the SDK's blocking HTTPS server on a background thread. The thread is
    // abandoned at process exit (test binary ends); that is fine for a test.
    std::thread::spawn(move || {
        let svc = Service::new(ServiceOptions {
            name: "tls-cap-test".to_string(),
            route: None,
            host: Some("127.0.0.1".to_string()),
            port: Some(port),
            basic_auth_user: None,
            basic_auth_password: None,
        });
        svc.run();
    });

    let base_url = format!("https://127.0.0.1:{port}");
    // A rustls client (ureq) that trusts the test CA.
    let agent = tls_support::ca_trusting_agent(&ca_path);

    // Poll /health until the TLS listener is up, then assert a real response.
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match agent.get(&format!("{base_url}/health")).call() {
            Ok(mut resp) => {
                assert_eq!(resp.status().as_u16(), 200, "https /health status != 200");
                let body: Value = resp
                    .body_mut()
                    .read_json()
                    .expect("decode /health json over https");
                assert_eq!(
                    body.get("status").and_then(Value::as_str),
                    Some("healthy"),
                    "https /health body = {body:?}, want status=healthy"
                );
                break;
            }
            Err(e) => {
                assert!(
                    Instant::now() <= deadline,
                    "SDK https server /health never reachable: {e}"
                );
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    // Negative control: a client that does NOT trust the test CA must be
    // rejected, proving the server's cert is genuinely verified.
    let untrusted: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .into();
    let result = untrusted.get(&format!("{base_url}/health")).call();
    assert!(
        result.is_err(),
        "https /health with default (webpki) trust store unexpectedly succeeded against the self-signed CA"
    );
    eprintln!(
        "untrusted client correctly rejected by SDK https server: {}",
        result.unwrap_err()
    );

    // Clean up env so it does not leak.
    unsafe {
        std::env::remove_var("SWML_SSL_ENABLED");
        std::env::remove_var("SWML_SSL_CERT_PATH");
        std::env::remove_var("SWML_SSL_KEY_PATH");
    }
}

/// Ask the OS for an unused loopback TCP port.
fn free_tcp_port() -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    l.local_addr().expect("local_addr").port()
}
