// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! TLS capability test (quadrant 2 of 3): prove the REST client performs a
//! *real*, verified `https://` request.
//!
//! Drives the real `signalwire::rest::RestClient` (ureq transport) against the
//! shared `mock_signalwire` started in `--tls` mode (HTTPS, backed by the
//! porting-sdk self-signed test CA). The client trusts that CA via
//! `SIGNALWIRE_REST_CA_FILE` (ureq `TlsConfig` + `RootCerts::Specific` — see
//! `src/rest/http_client.rs`). A real JSON `data` array can only come back over
//! a completed, CA-verified TLS session.
//!
//! Negative control: a client built WITHOUT the test CA (webpki Mozilla roots
//! only) must have its GET rejected — proving genuine certificate verification.
//! No `danger_accept_invalid_certs`, no transport mock.

#[path = "common/mod.rs"]
mod common;

use common::tls_support;
use signalwire::rest::RestClient;

#[test]
fn tls_rest_client_https_get() {
    let Some(ca) = tls_support::ca_file() else {
        eprintln!("skip: porting-sdk/test_harness/tls not adjacent");
        return;
    };
    let Some((_mock, base_url)) = tls_support::spawn_tls_mock_signalwire(&ca) else {
        eprintln!("skip: could not start `python -m mock_signalwire --tls`");
        return;
    };

    // Build the REST client with the test CA trusted. UreqTransport::new()
    // reads SIGNALWIRE_REST_CA_FILE at construction, so set it first.
    // SAFETY: integration test runs single-threaded (`--test-threads=1`).
    unsafe {
        std::env::set_var("SIGNALWIRE_REST_CA_FILE", &ca);
    }
    let client = RestClient::with_base_url("test_proj", "test_tok", &base_url)
        .expect("RestClient::with_base_url");

    // GET a spec-backed collection endpoint over HTTPS. A real JSON response
    // with a "data" array can only come back over a CA-verified TLS session.
    let body = client
        .fabric()
        .addresses()
        .list(
            &std::collections::HashMap::from([("page_size".to_string(), "5".to_string())]),
            None,
        )
        .expect("fabric addresses.list over https:// should succeed");
    let obj = body
        .as_object()
        .unwrap_or_else(|| panic!("https response not an object: {body:?}"));
    assert!(
        obj.contains_key("data"),
        "https response missing 'data' key; got {body:?}"
    );
    assert!(obj.get("data").unwrap().is_array());

    // Wire proof: the mock journaled the GET on its (HTTPS) control plane.
    // The control plane is served over HTTPS in --tls mode, so read it with a
    // CA-trusting agent.
    let agent = tls_support::ca_trusting_agent(&ca);
    let last = journal_last(&agent, &base_url);
    assert_eq!(
        last.0, "GET",
        "journal method = {:?}, want GET (path {:?})",
        last.0, last.1
    );
    assert_eq!(
        last.1, "/api/fabric/addresses",
        "journal path = {:?}, want /api/fabric/addresses",
        last.1
    );

    // Negative control: a client WITHOUT the CA must be rejected on HTTPS.
    unsafe {
        std::env::remove_var("SIGNALWIRE_REST_CA_FILE");
    }
    let untrusted = RestClient::with_base_url("test_proj", "test_tok", &base_url)
        .expect("RestClient::with_base_url (untrusted)");
    let result = untrusted
        .fabric()
        .addresses()
        .list(&std::collections::HashMap::new(), None);
    assert!(
        result.is_err(),
        "HTTPS GET with only webpki roots unexpectedly succeeded against the self-signed CA"
    );
    eprintln!(
        "untrusted HTTPS GET correctly rejected: {}",
        result.unwrap_err()
    );
}

/// Read the mock's HTTPS `/__mock__/journal` and return the (method, path) of
/// the most recent entry.
fn journal_last(agent: &ureq::Agent, base_url: &str) -> (String, String) {
    let url = format!("{base_url}/__mock__/journal");
    let mut resp = agent
        .get(&url)
        .call()
        .unwrap_or_else(|e| panic!("GET {url}: {e}"));
    let body: serde_json::Value = resp
        .body_mut()
        .read_json()
        .unwrap_or_else(|e| panic!("decode journal: {e}"));
    let entries = body
        .as_array()
        .unwrap_or_else(|| panic!("journal not an array: {body:?}"));
    let last = entries
        .last()
        .expect("journal empty - HTTPS request did not reach the mock");
    let method = last
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let path = last
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    (method, path)
}
