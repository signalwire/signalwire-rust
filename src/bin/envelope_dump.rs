// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! `envelope-dump` is the Rust port's ENVELOPE-DUMP program for the cross-port
//! REST error-envelope differ (`porting-sdk/scripts/diff_port_envelope.py`).
//!
//! It runs the shared error-envelope corpus (`porting-sdk/scripts/envelope_corpus.py`
//! — the single source of truth, mirrored natively below) through the Rust SDK's
//! REST [`signalwire::rest::RestClient`] and prints ONE JSON object mapping
//! corpus-id -> artifact to stdout, where each artifact is the shared
//! cross-port reduction:
//!
//! ```text
//! { "raised": bool, "error_kind": "typed"|"bare:<Type>"|null,
//!   "status_code": int|null, "body_error_code": string|null,
//!   "request_count": int }
//! ```
//!
//! The differ builds the golden oracle by running the same corpus against the
//! Python reference client, then byte-compares each artifact this program emits
//! against Python's. See the differ's module docstring for the contract.
//!
//! Each case is exercised against an in-process `tiny_http` mock that honors the
//! case's scenario (status / response body / Retry-After header / delay). A case
//! flagged `transport: true` instead points the client at a DEAD port (a free
//! port we bind then immediately release, so nothing is listening) — the
//! connection-refused path. A correct client raises its TYPED transport error
//! (the `SignalWireRestError` family with `is_transport() == true`,
//! `status_code() == 0`), which this program reports as `error_kind: "typed"`
//! with `status_code: null` and `request_count: 0`.
//!
//! Run from the signalwire-rust repo root:
//!
//! ```text
//! cargo run --quiet --bin envelope-dump
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use signalwire::rest::{RestClient, SignalWireRestError};

/// The armed mock override for a case (`None` => a synthesized 200 list body).
struct Scenario {
    status: u16,
    /// JSON-encodable response body (mutually exclusive with `raw_body`).
    response: Option<Value>,
    /// A deliberately non-JSON raw body (set instead of `response`).
    raw_body: Option<&'static str>,
    /// `Retry-After` header value, if any.
    retry: Option<&'static str>,
    delay_ms: u64,
}

/// One corpus case — the Rust-native mirror of
/// `porting-sdk/scripts/envelope_corpus.CORPUS`. Keep the id set and armed
/// scenarios in lockstep with the Python source; the differ compares each
/// artifact against Python's oracle for the same id.
struct EnvCase {
    id: &'static str,
    scenario: Option<Scenario>,
    transport: bool,
}

/// The path every case targets (`envelope_corpus.CALL` — a GET list route).
const CALL_PATH: &str = "/api/fabric/addresses";

fn corpus() -> Vec<EnvCase> {
    vec![
        // 200 success baseline: no scenario -> a synthesized 200 list body.
        EnvCase {
            id: "envelope_200_success",
            scenario: None,
            transport: false,
        },
        // 404 with a well-formed errors[] envelope.
        EnvCase {
            id: "envelope_404_typed",
            scenario: Some(Scenario {
                status: 404,
                response: Some(
                    json!({"errors": [{"code": "NOT_FOUND", "message": "no such address"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            transport: false,
        },
        // 429 + Retry-After: pinned NO retry.
        EnvCase {
            id: "envelope_429_retry_after",
            scenario: Some(Scenario {
                status: 429,
                response: Some(
                    json!({"errors": [{"code": "RATE_LIMITED", "message": "slow down"}]}),
                ),
                raw_body: None,
                retry: Some("2"),
                delay_ms: 0,
            }),
            transport: false,
        },
        // 503 service-unavailable: no retry.
        EnvCase {
            id: "envelope_503_unavailable",
            scenario: Some(Scenario {
                status: 503,
                response: Some(
                    json!({"errors": [{"code": "UNAVAILABLE", "message": "maintenance"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            transport: false,
        },
        // 500 with a NON-JSON body: still typed, body_error_code null.
        EnvCase {
            id: "envelope_500_malformed_body",
            scenario: Some(Scenario {
                status: 500,
                response: None,
                raw_body: Some("not-json-at-all <garbage"),
                retry: None,
                delay_ms: 0,
            }),
            transport: false,
        },
        // 200 whose body carries errors[]: 2xx == success, nothing raised.
        EnvCase {
            id: "envelope_200_with_error_body",
            scenario: Some(Scenario {
                status: 200,
                response: Some(
                    json!({"errors": [{"code": "SOFT_FAIL", "message": "ignored on 2xx"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            transport: false,
        },
        // 200ms-delayed 503: the delay path still yields one typed 503.
        EnvCase {
            id: "envelope_503_delayed",
            scenario: Some(Scenario {
                status: 503,
                response: Some(
                    json!({"errors": [{"code": "UNAVAILABLE", "message": "slow-fail"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 200,
            }),
            transport: false,
        },
        // connection refused (dead port): typed transport error, status null, count 0.
        EnvCase {
            id: "envelope_transport_refused",
            scenario: None,
            transport: true,
        },
    ]
}

/// The shared cross-port reduction the differ byte-compares.
#[derive(serde::Serialize, Default)]
struct Artifact {
    raised: bool,
    error_kind: Option<String>,
    status_code: Option<u16>,
    body_error_code: Option<String>,
    request_count: i32,
}

/// Bind a loopback port then immediately release it, so nothing listens there.
fn free_dead_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().expect("local_addr").port();
    drop(listener);
    port
}

/// Mirrors the differ's `_decode_body_error_code`: parse a JSON body and pull
/// `errors[0].code`, else `None`.
fn decode_body_error_code(body: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(body).ok()?;
    let errors = parsed.get("errors")?.as_array()?;
    let first = errors.first()?;
    first
        .get("code")?
        .as_str()
        .map(std::string::ToString::to_string)
}

/// Exercise one corpus case and return its artifact. Stands up an in-process
/// `tiny_http` mock honoring the scenario (or, for a transport case, points the
/// client at a dead port), makes the request, and reduces the outcome.
fn run_case(case: &EnvCase) -> Artifact {
    let mut art = Artifact::default();

    if case.transport {
        // Dead port: nothing listening -> connection refused. request_count
        // stays 0 (no mock is even started for this case).
        let dead_port = free_dead_port();
        let client = RestClient::with_base_url(
            "envelope_proj",
            "envelope_tok",
            &format!("http://127.0.0.1:{dead_port}"),
        )
        .expect("construct client");
        let result = client.fabric().addresses().list(&HashMap::new());
        reduce_result(&mut art, result);
        return art;
    }

    let hits = Arc::new(AtomicI32::new(0));
    let hits_clone = Arc::clone(&hits);
    let scenario = case.scenario.as_ref();

    // Build the fixed response bytes/status/headers up front (Scenario borrows
    // 'static data only, so this closure can be 'static + Send).
    let status: u16 = scenario.map_or(200, |s| s.status);
    let retry: Option<&'static str> = scenario.and_then(|s| s.retry);
    let delay_ms: u64 = scenario.map_or(0, |s| s.delay_ms);
    let body_bytes: String = match scenario {
        None => r#"{"data":[]}"#.to_string(),
        Some(s) => {
            if let Some(raw) = s.raw_body {
                raw.to_string()
            } else if let Some(resp) = &s.response {
                serde_json::to_string(resp).unwrap()
            } else {
                String::new()
            }
        }
    };

    let server = tiny_http::Server::http("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = server.server_addr().to_ip().unwrap().port();
    let base_url = format!("http://127.0.0.1:{port}");

    let handle = std::thread::spawn(move || {
        // Exactly one request is expected per case; serve it then exit.
        if let Ok(mut req) = server.recv() {
            if req.url() == CALL_PATH {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            }
            // Drain the request body (unused, but required by tiny_http).
            let mut discard = String::new();
            let _ = req.as_reader().read_to_string(&mut discard);

            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }

            let mut response =
                tiny_http::Response::from_string(body_bytes.clone()).with_status_code(status);
            if let Ok(header) =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
            {
                response = response.with_header(header);
            }
            if let Some(retry_val) = retry
                && let Ok(header) =
                    tiny_http::Header::from_bytes(&b"Retry-After"[..], retry_val.as_bytes())
            {
                response = response.with_header(header);
            }
            let _ = req.respond(response);
        }
    });

    let client = RestClient::with_base_url("envelope_proj", "envelope_tok", &base_url)
        .expect("construct client");
    let result = client.fabric().addresses().list(&HashMap::new());
    reduce_result(&mut art, result);

    let _ = handle.join();
    art.request_count = hits.load(Ordering::SeqCst);
    art
}

/// Fill the `raised`/`error_kind`/`status_code`/`body_error_code` fields from the
/// request result. The Rust client returns `Result<Value, SignalWireRestError>`
/// — every failure the client raises IS the typed family (there is no bare
/// error path in the current client), so `error_kind` is always "typed" on
/// `Err`. Kept as an explicit match (not `unwrap_or`) so a future change that
/// introduces a bare-error leak would be visible in this reduction.
fn reduce_result(art: &mut Artifact, result: Result<Value, SignalWireRestError>) {
    match result {
        Ok(_) => {}
        Err(e) => {
            art.raised = true;
            art.error_kind = Some("typed".to_string());
            if e.is_transport() {
                // Transport failure: no HTTP status -> status_code null (Rust's
                // is_transport()==true / status_code()==0 maps to the Python
                // reference's None); body carries the transport message with no
                // errors[] to decode -> body_error_code null.
                art.status_code = None;
                art.body_error_code = None;
            } else {
                art.status_code = Some(e.status_code());
                art.body_error_code = decode_body_error_code(e.response_body());
            }
        }
    }
}

fn main() {
    let mut out: HashMap<&'static str, Artifact> = HashMap::new();
    for case in &corpus() {
        out.insert(case.id, run_case(case));
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
