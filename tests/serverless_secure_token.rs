//! `secure = true` must be enforced on EVERY transport, not just the built-in
//! HTTP server.
//!
//! A tool registered `secure = true` REQUIRES a valid `__token`. Absent,
//! forged, or unvalidatable — including a missing `call_id`, since a token can
//! only be checked against one — must all REFUSE. An insecure tool runs ungated
//! in every one of those cases; a fix that refuses everything is not a fix.
//!
//! The refusal is a 200 + `FunctionResult` body, never an HTTP error status:
//! the engine has no handling for a SWAIG refusal status, so the tool reports
//! that it cannot execute and the model relays it.
//!
//! The credential rides the QUERY STRING and the `call_id` rides the POST BODY —
//! the identical split on all five transports. Each serverless host hands its
//! query over in a different shape, so each shape is driven here.

use std::collections::HashMap;

use base64::Engine as _;
use serde_json::{Value, json};
use signalwire::agent::{AgentBase, AgentOptions};
use signalwire::serverless::Adapter;
use signalwire::swaig::FunctionResult;

const USER: &str = "u";
const PASSWORD: &str = "p";
const CALL_ID: &str = "c1";
const REFUSAL: &str = "security token for this function is invalid";

fn agent() -> AgentBase {
    let mut a = AgentBase::new(
        AgentOptions::new("demo")
            .route("/")
            .basic_auth(USER, PASSWORD),
    );
    a.define_tool(
        "secure_tool",
        "secure",
        json!({}),
        Box::new(|_a, _r| FunctionResult::with_response("ran")),
        true,
    );
    a.define_tool(
        "insecure_tool",
        "insecure",
        json!({}),
        Box::new(|_a, _r| FunctionResult::with_response("ran")),
        false,
    );
    a
}

fn auth() -> String {
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{USER}:{PASSWORD}"))
    )
}

/// The SWAIG POST body. The `call_id` rides HERE, never the query string.
fn body(function: &str, call_id: Option<&str>) -> String {
    let mut b = json!({"function": function, "argument": {"parsed": [{}]}});
    if let Some(c) = call_id {
        b["call_id"] = json!(c);
    }
    b.to_string()
}

/// What the caller supplies as the credential.
#[derive(Clone, Copy)]
enum Token {
    Valid,
    Forged,
    Absent,
}

fn token_value(a: &AgentBase, function: &str, call_id: &str, t: Token) -> Option<String> {
    match t {
        Token::Valid => Some(a.create_tool_token(function, call_id)),
        Token::Forged => Some("garbage_token".to_string()),
        Token::Absent => None,
    }
}

// ── the five transports, each reduced to {status, body-as-text} ──────────

fn via_http(a: &AgentBase, function: &str, call_id: Option<&str>, t: Token) -> (u16, String) {
    let tok = token_value(a, function, call_id.unwrap_or(CALL_ID), t);
    let path = tok.map_or_else(
        || "/swaig".to_string(),
        |tok| format!("/swaig?__token={tok}"),
    );
    let mut h = HashMap::new();
    h.insert("Authorization".to_string(), auth());
    let (status, _, out) = a.handle_request("POST", &path, &h, Some(&body(function, call_id)));
    (status, out)
}

/// Lambda, HTTP API v2 shape: the PARSED `queryStringParameters` mapping.
fn via_lambda_params(
    a: &AgentBase,
    function: &str,
    call_id: Option<&str>,
    t: Token,
) -> (u16, String) {
    let mut event = json!({
        "rawPath": "/swaig",
        "headers": {"authorization": auth()},
        "body": body(function, call_id),
        "requestContext": {"http": {"method": "POST"}},
    });
    if let Some(tok) = token_value(a, function, call_id.unwrap_or(CALL_ID), t) {
        event["queryStringParameters"] = json!({"__token": tok});
    }
    reduce(&Adapter::handle_lambda(a, &event), "statusCode")
}

/// Lambda, the RAW `rawQueryString` shape.
fn via_lambda_raw(a: &AgentBase, function: &str, call_id: Option<&str>, t: Token) -> (u16, String) {
    let mut event = json!({
        "rawPath": "/swaig",
        "headers": {"authorization": auth()},
        "body": body(function, call_id),
        "requestContext": {"http": {"method": "POST"}},
    });
    if let Some(tok) = token_value(a, function, call_id.unwrap_or(CALL_ID), t) {
        event["rawQueryString"] = json!(format!("__token={tok}"));
    }
    reduce(&Adapter::handle_lambda(a, &event), "statusCode")
}

/// GCF (Flask): the parsed `args` mapping.
fn via_gcf(a: &AgentBase, function: &str, call_id: Option<&str>, t: Token) -> (u16, String) {
    let mut request = json!({
        "method": "POST",
        "path": "/swaig",
        "headers": {"Authorization": auth()},
        "body": body(function, call_id),
    });
    if let Some(tok) = token_value(a, function, call_id.unwrap_or(CALL_ID), t) {
        request["args"] = json!({"__token": tok});
    }
    reduce(&Adapter::handle_gcf(a, &request), "status")
}

/// Azure: the parsed `params` mapping.
fn via_azure(a: &AgentBase, function: &str, call_id: Option<&str>, t: Token) -> (u16, String) {
    let mut request = json!({
        "method": "POST",
        "url": "https://fn.azurewebsites.net/swaig",
        "headers": {"Authorization": auth()},
        "body": body(function, call_id),
    });
    if let Some(tok) = token_value(a, function, call_id.unwrap_or(CALL_ID), t) {
        request["params"] = json!({"__token": tok});
    }
    reduce(&Adapter::handle_azure(a, &request), "status")
}

/// CGI: the `QUERY_STRING` environment variable.
fn via_cgi(a: &AgentBase, function: &str, call_id: Option<&str>, t: Token) -> (u16, String) {
    let mut env = HashMap::new();
    env.insert("REQUEST_METHOD".to_string(), "POST".to_string());
    env.insert("PATH_INFO".to_string(), "/swaig".to_string());
    env.insert("CONTENT_TYPE".to_string(), "application/json".to_string());
    env.insert("HTTP_AUTHORIZATION".to_string(), auth());
    if let Some(tok) = token_value(a, function, call_id.unwrap_or(CALL_ID), t) {
        env.insert("QUERY_STRING".to_string(), format!("__token={tok}"));
    }
    reduce(
        &Adapter::handle_cgi(a, &env, &body(function, call_id)),
        "status",
    )
}

fn reduce(resp: &Value, status_key: &str) -> (u16, String) {
    let status = u16::try_from(resp[status_key].as_u64().unwrap_or(0)).unwrap_or(0);
    (status, resp["body"].as_str().unwrap_or("").to_string())
}

type Transport = fn(&AgentBase, &str, Option<&str>, Token) -> (u16, String);

/// Every dispatch path a SWAIG call can reach in this crate. `handle_request`
/// is the single funnel — the built-in `serve()` loop and all four
/// `Adapter::handle_*` entry points call it and nothing else — so covering
/// these five covers the crate.
const TRANSPORTS: &[(&str, Transport)] = &[
    ("http", via_http),
    ("lambda/queryStringParameters", via_lambda_params),
    ("lambda/rawQueryString", via_lambda_raw),
    ("gcf", via_gcf),
    ("azure", via_azure),
    ("cgi", via_cgi),
];

// ── the matrix ───────────────────────────────────────────────────────────

#[test]
fn secure_tool_runs_only_with_a_valid_token() {
    for (name, via) in TRANSPORTS {
        let a = agent();
        let (status, out) = via(&a, "secure_tool", Some(CALL_ID), Token::Valid);
        assert_eq!(status, 200, "[{name}] valid token: {out}");
        assert!(
            !out.contains(REFUSAL),
            "[{name}] a VALID token must not be refused: {out}"
        );
        assert!(out.contains("ran"), "[{name}] the handler must run: {out}");
    }
}

#[test]
fn secure_tool_refuses_a_forged_token() {
    for (name, via) in TRANSPORTS {
        let a = agent();
        let (status, out) = via(&a, "secure_tool", Some(CALL_ID), Token::Forged);
        assert_eq!(status, 200, "[{name}] the refusal is in-band, not a status");
        assert!(out.contains(REFUSAL), "[{name}] forged token ran: {out}");
        assert!(!out.contains("ran"), "[{name}] handler must not run: {out}");
    }
}

#[test]
fn secure_tool_refuses_an_absent_token() {
    for (name, via) in TRANSPORTS {
        let a = agent();
        let (status, out) = via(&a, "secure_tool", Some(CALL_ID), Token::Absent);
        assert_eq!(status, 200, "[{name}] the refusal is in-band, not a status");
        assert!(
            out.contains(REFUSAL),
            "[{name}] an ABSENT token must fail CLOSED: {out}"
        );
        assert!(!out.contains("ran"), "[{name}] handler must not run: {out}");
    }
}

#[test]
fn secure_tool_refuses_when_call_id_is_absent() {
    for (name, via) in TRANSPORTS {
        let a = agent();
        // A genuine token, but nothing to validate it against.
        let (status, out) = via(&a, "secure_tool", None, Token::Valid);
        assert_eq!(status, 200, "[{name}] the refusal is in-band, not a status");
        assert!(
            out.contains(REFUSAL),
            "[{name}] a missing call_id is not a bypass: {out}"
        );
        assert!(!out.contains("ran"), "[{name}] handler must not run: {out}");
    }
}

#[test]
fn insecure_tool_runs_ungated_in_every_case() {
    let cases = [
        ("valid", Some(CALL_ID), Token::Valid),
        ("forged", Some(CALL_ID), Token::Forged),
        ("absent", Some(CALL_ID), Token::Absent),
        ("no-call-id", None, Token::Absent),
    ];
    for (name, via) in TRANSPORTS {
        for (case, call_id, t) in cases {
            let a = agent();
            let (status, out) = via(&a, "insecure_tool", call_id, t);
            assert_eq!(status, 200, "[{name}/{case}] {out}");
            assert!(
                !out.contains(REFUSAL),
                "[{name}/{case}] an insecure tool must never be refused: {out}"
            );
            assert!(
                out.contains("ran"),
                "[{name}/{case}] handler must run: {out}"
            );
        }
    }
}

#[test]
fn a_token_for_another_function_or_call_is_refused() {
    // A genuinely-minted token is not a skeleton key: it is bound to the
    // (function, call_id) pair it was minted for.
    let a = agent();
    let wrong_call = a.create_tool_token("secure_tool", "some_other_call");
    let (status, out) = via_http_with_token(&a, "secure_tool", CALL_ID, &wrong_call);
    assert_eq!(status, 200);
    assert!(
        out.contains(REFUSAL),
        "a token minted for another call must be refused: {out}"
    );

    let wrong_fn = a.create_tool_token("insecure_tool", CALL_ID);
    let (_, out2) = via_http_with_token(&a, "secure_tool", CALL_ID, &wrong_fn);
    assert!(
        out2.contains(REFUSAL),
        "a token minted for another function must be refused: {out2}"
    );
}

fn via_http_with_token(a: &AgentBase, function: &str, call_id: &str, tok: &str) -> (u16, String) {
    let mut h = HashMap::new();
    h.insert("Authorization".to_string(), auth());
    let (status, _, out) = a.handle_request(
        "POST",
        &format!("/swaig?__token={tok}"),
        &h,
        Some(&body(function, Some(call_id))),
    );
    (status, out)
}

#[test]
fn the_legacy_bare_token_spelling_is_also_accepted() {
    // The HTTP path has always read `__token` then fallen back to `token`;
    // the serverless envelopes must not diverge from that.
    let a = agent();
    let tok = a.create_tool_token("secure_tool", CALL_ID);
    let event = json!({
        "rawPath": "/swaig",
        "headers": {"authorization": auth()},
        "queryStringParameters": {"token": tok},
        "body": body("secure_tool", Some(CALL_ID)),
        "requestContext": {"http": {"method": "POST"}},
    });
    let (status, out) = reduce(&Adapter::handle_lambda(&a, &event), "statusCode");
    assert_eq!(status, 200);
    assert!(
        !out.contains(REFUSAL),
        "the `token` spelling must work: {out}"
    );
    assert!(out.contains("ran"), "the handler must run: {out}");
}
