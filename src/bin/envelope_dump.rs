// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

// envelope-dump — cross-port REST error-envelope comparison program.
//
// It runs the shared error-envelope corpus (mirrored natively below, kept in
// lockstep with its canonical source `porting-sdk/scripts/envelope_corpus.py`)
// through the Rust SDK's REST `signalwire::rest::RestClient` and prints ONE
// JSON object mapping corpus-id -> artifact to stdout, where each artifact is
// the shared cross-port reduction:
//
//     { "raised": bool, "error_kind": "typed"|"bare:<Type>"|null,
//       "status_code": int|null, "body_error_code": string|null,
//       "request_count": int }
//
// A reference comparison is built by running the same corpus against the
// Python reference client, then byte-comparing each artifact this program
// emits against Python's (`diff_port_envelope.py --port rust`).
//
// Each case is exercised against an in-process `tiny_http` mock that honors the
// case's scenario (status / response body / Retry-After header / delay). A case
// flagged `transport: true` instead points the client at a DEAD port (a free
// port we bind then immediately release, so nothing is listening) — the
// connection-refused path. A correct client raises its TYPED transport error
// (the `SignalWireRestError` family with `is_transport() == true`,
// `status_code() == 0`), which this program reports as `error_kind: "typed"`
// with `status_code: null` and `request_count: 0`.
//
// RequestOptions envelope (plan 4.2): a case may carry `request_options`
// (retries / retry_backoff / timeout) — passed as the client-default
// `RequestOptions` — and `scenario_repeat` (arm the failure status on the FIRST
// N attempts, then let the mock return its default success). The retry-armed
// cases prove the opt-in retry COUNT (request_count == retries + 1 when the
// failure repeats every attempt; == retries + 1 when a retry succeeds), with
// the idempotency asymmetry (POST/PATCH retry only 429/503, never 500/502/504).
//
// Run from the signalwire-rust repo root:
//
//     cargo run --quiet --bin envelope-dump

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::Duration;

use serde_json::{Value, json};

use signalwire::rest::{RequestOptions, RestClient, SignalWireRestError};

/// The armed mock override for a case (`None` => a synthesized default success).
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

/// The `RequestOptions` a case passes to the client (plan 4.2). Absent => the
/// port's default (retries 0). `retry_backoff` is 0 in the retry cases so the
/// differ never waits on wall-clock.
#[derive(Default)]
struct ReqOpts {
    retries: Option<u32>,
    retry_backoff: Option<f64>,
    timeout: Option<f64>,
}

/// The REST verb a case issues.
struct CallSpec {
    method: &'static str,
    path: &'static str,
    /// POST/PATCH body, if any.
    body: Option<Value>,
}

/// One corpus case — the Rust-native mirror of the shared error-envelope
/// corpus. Keep the id set and armed scenarios in lockstep with the Python
/// source; each artifact is compared against Python's for the same id.
struct EnvCase {
    id: &'static str,
    call: CallSpec,
    scenario: Option<Scenario>,
    /// Arm the failure `scenario` on the first N attempts (FIFO), then the mock
    /// returns its default success — so a retry-armed case can either recover
    /// (repeat < attempts) or exhaust (repeat >= attempts).
    scenario_repeat: u32,
    request_options: Option<ReqOpts>,
    transport: bool,
}

/// The GET list route every non-POST case targets.
const GET_CALL: CallSpec = CallSpec {
    method: "GET",
    path: "/api/fabric/addresses",
    body: None,
};

fn get_case(
    id: &'static str,
    scenario: Option<Scenario>,
    transport: bool,
    request_options: Option<ReqOpts>,
    scenario_repeat: u32,
) -> EnvCase {
    EnvCase {
        id,
        call: GET_CALL,
        scenario,
        scenario_repeat,
        request_options,
        transport,
    }
}

fn corpus() -> Vec<EnvCase> {
    vec![
        // 200 success baseline: no scenario -> a synthesized default success.
        get_case("envelope_200_success", None, false, None, 1),
        // 404 with a well-formed errors[] envelope.
        get_case(
            "envelope_404_typed",
            Some(Scenario {
                status: 404,
                response: Some(
                    json!({"errors": [{"code": "NOT_FOUND", "message": "no such address"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            false,
            None,
            1,
        ),
        // 429 + Retry-After with DEFAULT options: pinned NO retry (count 1).
        get_case(
            "envelope_429_retry_after",
            Some(Scenario {
                status: 429,
                response: Some(
                    json!({"errors": [{"code": "RATE_LIMITED", "message": "slow down"}]}),
                ),
                raw_body: None,
                retry: Some("2"),
                delay_ms: 0,
            }),
            false,
            None,
            1,
        ),
        // 503 service-unavailable with DEFAULT options: no retry (count 1).
        get_case(
            "envelope_503_unavailable",
            Some(Scenario {
                status: 503,
                response: Some(
                    json!({"errors": [{"code": "UNAVAILABLE", "message": "maintenance"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            false,
            None,
            1,
        ),
        // 500 with a NON-JSON body: still typed, body_error_code null.
        get_case(
            "envelope_500_malformed_body",
            Some(Scenario {
                status: 500,
                response: None,
                raw_body: Some("not-json-at-all <garbage"),
                retry: None,
                delay_ms: 0,
            }),
            false,
            None,
            1,
        ),
        // 200 whose body carries errors[]: 2xx == success, nothing raised.
        get_case(
            "envelope_200_with_error_body",
            Some(Scenario {
                status: 200,
                response: Some(
                    json!({"errors": [{"code": "SOFT_FAIL", "message": "ignored on 2xx"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            false,
            None,
            1,
        ),
        // 200ms-delayed 503: the delay path still yields one typed 503.
        get_case(
            "envelope_503_delayed",
            Some(Scenario {
                status: 503,
                response: Some(
                    json!({"errors": [{"code": "UNAVAILABLE", "message": "slow-fail"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 200,
            }),
            false,
            None,
            1,
        ),
        // connection refused (dead port): typed transport error, status null, count 0.
        get_case("envelope_transport_refused", None, true, None, 1),
        // ==============================================================
        // RequestOptions envelope — opt-in retry (plan 4.2). retry_backoff=0.
        // ==============================================================
        // GET 503 with retries=1, armed ONCE: retried into the default 200.
        get_case(
            "envelope_get_retry_once_succeeds",
            Some(Scenario {
                status: 503,
                response: Some(
                    json!({"errors": [{"code": "UNAVAILABLE", "message": "transient"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            false,
            Some(ReqOpts {
                retries: Some(1),
                retry_backoff: Some(0.0),
                timeout: None,
            }),
            1,
        ),
        // GET 503 armed on BOTH attempts with retries=1: exhausted -> 503, count 2.
        get_case(
            "envelope_get_retry_exhausted",
            Some(Scenario {
                status: 503,
                response: Some(json!({"errors": [{"code": "UNAVAILABLE", "message": "down"}]})),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            false,
            Some(ReqOpts {
                retries: Some(1),
                retry_backoff: Some(0.0),
                timeout: None,
            }),
            2,
        ),
        // POST 500 with retries=2: NOT retried (idempotency safety), count 1.
        EnvCase {
            id: "envelope_post_500_not_retried",
            call: CallSpec {
                method: "POST",
                path: "/api/relay/rest/addresses",
                body: Some(json!({"label": "x"})),
            },
            scenario: Some(Scenario {
                status: 500,
                response: Some(json!({"errors": [{"code": "SERVER_ERROR", "message": "boom"}]})),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            scenario_repeat: 1,
            request_options: Some(ReqOpts {
                retries: Some(2),
                retry_backoff: Some(0.0),
                timeout: None,
            }),
            transport: false,
        },
        // POST 503 with retries=1, armed ONCE: retried into the default success, count 2.
        EnvCase {
            id: "envelope_post_503_retried",
            call: CallSpec {
                method: "POST",
                path: "/api/relay/rest/addresses",
                body: Some(json!({"label": "x"})),
            },
            scenario: Some(Scenario {
                status: 503,
                response: Some(
                    json!({"errors": [{"code": "UNAVAILABLE", "message": "throttled"}]}),
                ),
                raw_body: None,
                retry: None,
                delay_ms: 0,
            }),
            scenario_repeat: 1,
            request_options: Some(ReqOpts {
                retries: Some(1),
                retry_backoff: Some(0.0),
                timeout: None,
            }),
            transport: false,
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

/// Build the `RestClient`'s client-default `RequestOptions` for a case, if any.
fn build_request_options(opts: Option<&ReqOpts>) -> Option<RequestOptions> {
    opts.map(|o| {
        let mut ro = RequestOptions::new();
        if let Some(r) = o.retries {
            ro = ro.retries(r);
        }
        if let Some(b) = o.retry_backoff {
            ro = ro.retry_backoff(b);
        }
        if let Some(t) = o.timeout {
            ro = ro.timeout(t);
        }
        ro
    })
}

/// Issue the case's REST verb against the client and return the raw result.
fn issue_call(client: &RestClient, call: &CallSpec) -> Result<Value, SignalWireRestError> {
    match call.method {
        "POST" => {
            let body = call.body.clone().unwrap_or_else(|| json!({}));
            // Drive the raw POST at the exact path so the mock's create route is hit.
            client.http().post(call.path, &body)
        }
        _ => client.http().get(call.path, &HashMap::new()),
    }
}

/// Exercise one corpus case and return its artifact. Stands up an in-process
/// `tiny_http` mock honoring the scenario (or, for a transport case, points the
/// client at a dead port), makes the request, and reduces the outcome.
fn run_case(case: &EnvCase) -> Artifact {
    let mut art = Artifact::default();
    let request_options = build_request_options(case.request_options.as_ref());

    if case.transport {
        // Dead port: nothing listening -> connection refused. request_count
        // stays 0 (no mock is even started for this case).
        let dead_port = free_dead_port();
        let client = RestClient::with_base_url_and_options(
            "envelope_proj",
            "envelope_tok",
            &format!("http://127.0.0.1:{dead_port}"),
            request_options,
        )
        .expect("construct client");
        let result = issue_call(&client, &case.call);
        reduce_result(&mut art, result);
        return art;
    }

    let hits = Arc::new(AtomicI32::new(0));
    let hits_clone = Arc::clone(&hits);
    let scenario = case.scenario.as_ref();
    let scenario_present = scenario.is_some();
    let call_path = case.call.path.to_string();

    // Build the fixed FAILURE response bytes/status/headers up front. The
    // failure scenario is served on the first `scenario_repeat` requests; after
    // that the mock returns a synthesized default success (200 with a small
    // JSON body) so a retry can recover.
    let status: u16 = scenario.map_or(200, |s| s.status);
    let retry: Option<&'static str> = scenario.and_then(|s| s.retry);
    let delay_ms: u64 = scenario.map_or(0, |s| s.delay_ms);
    let scenario_repeat: i32 = i32::try_from(case.scenario_repeat).unwrap_or(1);
    // Whether the armed scenario is itself a success (2xx) — then every request
    // gets it (there is no "recover to default" for a success scenario).
    let scenario_is_success = (200..300).contains(&status);
    let fail_body: String = match scenario {
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
    let default_success_body = r#"{"data":[]}"#.to_string();

    let server = tiny_http::Server::http("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = server.server_addr().to_ip().unwrap().port();
    let base_url = format!("http://127.0.0.1:{port}");

    let handle = std::thread::spawn(move || {
        // Serve requests until the client stops making them. Each request over
        // the FIRST `scenario_repeat` gets the armed failure; after that a
        // synthesized default success (so a retry can recover). A success
        // scenario (2xx) is served on every request. `recv_timeout` lets the
        // thread exit on its own once the client has returned and no further
        // request arrives — no dummy connection needed.
        loop {
            let Ok(Some(mut req)) = server.recv_timeout(Duration::from_millis(400)) else {
                break; // idle (or error): the client is done making requests
            };
            if req.url() == call_path {
                hits_clone.fetch_add(1, Ordering::SeqCst);
            }
            // Drain the request body (unused, but required by tiny_http).
            let mut discard = String::new();
            let _ = req.as_reader().read_to_string(&mut discard);

            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }

            let this_hit = hits_clone.load(Ordering::SeqCst); // 1-based hit index
            let serve_failure =
                scenario_present && (scenario_is_success || this_hit <= scenario_repeat);
            let (resp_status, resp_body) = if serve_failure {
                (status, fail_body.clone())
            } else {
                (200u16, default_success_body.clone())
            };

            let mut response =
                tiny_http::Response::from_string(resp_body).with_status_code(resp_status);
            if let Ok(header) =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
            {
                response = response.with_header(header);
            }
            if serve_failure
                && let Some(retry_val) = retry
                && let Ok(header) =
                    tiny_http::Header::from_bytes(&b"Retry-After"[..], retry_val.as_bytes())
            {
                response = response.with_header(header);
            }
            let _ = req.respond(response);
        }
    });

    let client = RestClient::with_base_url_and_options(
        "envelope_proj",
        "envelope_tok",
        &base_url,
        request_options,
    )
    .expect("construct client");
    let result = issue_call(&client, &case.call);
    reduce_result(&mut art, result);

    let _ = handle.join();
    art.request_count = hits.load(Ordering::SeqCst);
    art
}

/// Fill the `raised`/`error_kind`/`status_code`/`body_error_code` fields from
/// the request result. The Rust client returns `Result<Value,
/// SignalWireRestError>` — every failure the client raises IS the typed family
/// (there is no bare error path), so `error_kind` is always "typed" on `Err`.
fn reduce_result(art: &mut Artifact, result: Result<Value, SignalWireRestError>) {
    match result {
        Ok(_) => {}
        Err(e) => {
            art.raised = true;
            art.error_kind = Some("typed".to_string());
            if e.is_transport() {
                // Transport failure: no HTTP status -> status_code null (Rust's
                // is_transport()==true / status_code()==0 maps to Python's None);
                // no errors[] to decode -> body_error_code null.
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
