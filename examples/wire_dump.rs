// Copyright (c) 2025 SignalWire
// SPDX-License-Identifier: MIT
//
//! `wire_dump` — the Rust port's WIRE-CRYPTO dump program for the cross-port
//! wire differ (porting-sdk/scripts/diff_port_wire.py).
//!
//! It runs the shared `wire_crypto` corpus against the Rust SDK's native
//! `security` module (`SessionManager` tokens, webhook-signature validation,
//! redact/filter helpers) and prints ONE JSON object mapping
//!
//!     case-id -> observable-artifact
//!
//! to stdout. The differ runs this program, canonicalizes both sides, and
//! byte-compares each entry against the Python oracle. Only stdout carries the
//! JSON object; nothing else is printed there.
//!
//! The corpus sentinels (`__ORACLE_FORMAT_TOKEN__`, `__TAMPERED_TOKEN__`,
//! `__ORACLE_SIG__`) are materialized here from the fixed per-case SECRET
//! exactly as the oracle materializes them, so the interop/tamper cases are
//! reproducible.
//!
//! Run from the signalwire-rust repo root:
//!
//!     cargo run --quiet --example wire_dump

use std::collections::{BTreeMap, HashMap};

use hmac::{Hmac, KeyInit, Mac};
use serde_json::{Value, json};
use sha1::Sha1;
use sha2::Sha256;
use signalwire::security::{
    SessionManager, filter_sensitive_headers, redact_url, validate_webhook_signature,
};

/// SECRET mirrors `wire_crypto_corpus.SECRET` (`"a" * 64`).
const SECRET: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Fixed far-future expiry + fixed 16-hex nonce so the oracle token is
/// deterministic (mirrors `diff_port_wire._oracle_token`).
const ORACLE_EXPIRY: u64 = 9_999_999_999;
const ORACLE_NONCE: &str = "0123456789abcdef";

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// hex(HMAC-SHA256(SECRET, msg)) — the token signature algorithm.
fn hmac_sha256_hex(secret: &str, msg: &str) -> String {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(msg.as_bytes());
    hex(&mac.finalize().into_bytes())
}

/// hex(HMAC-SHA1(key, url+body)) — SignalWire webhook Scheme A.
fn oracle_sig(url: &str, body: &str, key: &str) -> String {
    let mut mac =
        Hmac::<Sha1>::new_from_slice(key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(url.as_bytes());
    mac.update(body.as_bytes());
    hex(&mac.finalize().into_bytes())
}

/// Build a token in the SDK wire format
/// (`base64url(call_id.fn.expiry.nonce.sig)`) from the fixed SECRET — the Rust
/// mirror of `diff_port_wire._oracle_token`.
fn oracle_token(call_id: &str, function_name: &str) -> String {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE;
    let msg = format!("{call_id}:{function_name}:{ORACLE_EXPIRY}:{ORACLE_NONCE}");
    let sig = hmac_sha256_hex(SECRET, &msg);
    let raw = format!("{call_id}.{function_name}.{ORACLE_EXPIRY}.{ORACLE_NONCE}.{sig}");
    URL_SAFE.encode(raw.as_bytes())
}

/// Flip one signature character — the Rust mirror of
/// `diff_port_wire._tampered_token`.
fn tampered_token() -> String {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE;
    let tok = oracle_token("c", "f");
    let raw = String::from_utf8(URL_SAFE.decode(&tok).expect("oracle token is valid base64"))
        .expect("oracle token is valid UTF-8");
    let mut parts: Vec<String> = raw.split('.').map(str::to_string).collect();
    if let Some(last) = parts.last_mut() {
        let first = last.chars().next().unwrap_or('a');
        let flipped = if first == 'f' { 'e' } else { 'f' };
        *last = format!("{flipped}{}", &last[1..]);
    }
    URL_SAFE.encode(parts.join(".").as_bytes())
}

/// Decode a token and report its wire-format shape — the Rust mirror of
/// `diff_port_wire._observe_token_fields`.
fn observe_token_fields(token: &str) -> Value {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let raw =
        String::from_utf8(URL_SAFE_NO_PAD.decode(token).unwrap_or_default()).unwrap_or_default();
    let parts: Vec<&str> = raw.split('.').collect();
    let nonce = parts.get(3).copied().unwrap_or("");
    let nonce_is_hex = parts.len() > 3
        && nonce
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
    json!({
        "n_fields": parts.len(),
        "call_id": parts.first().copied(),
        "function_name": parts.get(1).copied(),
        "nonce_len": nonce.len(),
        "nonce_is_hex": nonce_is_hex,
    })
}

fn main() {
    let mut out: BTreeMap<&str, Value> = BTreeMap::new();

    // token_format: generate a token via the SDK, decode its fields.
    let sm = SessionManager::with_secret(ORACLE_EXPIRY, SECRET);
    out.insert(
        "token_format",
        observe_token_fields(&sm.generate_token("my_func", "call_1")),
    );

    // token_nonce_distinct: two generations must differ (random nonce).
    let n1 = sm.generate_token("f", "c");
    let n2 = sm.generate_token("f", "c");
    out.insert("token_nonce_distinct", json!({ "distinct": n1 != n2 }));

    // token_interop: validate an oracle-format token built from SECRET.
    // Rust's validate_token signature is (function_name, call_id, token).
    out.insert(
        "token_interop",
        json!({
            "valid": sm.validate_token("oracle_fn", "oracle_call", &oracle_token("oracle_call", "oracle_fn")),
        }),
    );

    // token_tamper_rejected: a one-byte-flipped signature must fail.
    out.insert(
        "token_tamper_rejected",
        json!({ "valid": sm.validate_token("f", "c", &tampered_token()) }),
    );

    // wire_validate_webhook_signature: correct HMAC-SHA1 -> valid.
    let wh_url = "https://example.com/hook";
    let wh_body = r#"{"event":"call.created"}"#;
    out.insert(
        "wire_validate_webhook_signature",
        json!({
            "valid": validate_webhook_signature(SECRET, &oracle_sig(wh_url, wh_body, SECRET), wh_url, wh_body)
                .unwrap_or(false),
        }),
    );
    // wire_validate_webhook_signature_bad: wrong sig -> invalid.
    let bad_sig = "deadbeef".repeat(8);
    out.insert(
        "wire_validate_webhook_signature_bad",
        json!({ "valid": validate_webhook_signature(SECRET, &bad_sig, wh_url, wh_body).unwrap_or(false) }),
    );

    // wire_redact_url: credentials redacted, structure + query preserved.
    out.insert(
        "wire_redact_url",
        json!({ "redacted": redact_url("https://user:s3cr3t@api.signalwire.com/path?token=abc") }),
    );

    // wire_filter_sensitive_headers: authorization + x-api-key dropped,
    // content-type kept.
    let mut headers: HashMap<String, String> = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer x".to_string());
    headers.insert("X-Api-Key".to_string(), "y".to_string());
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    let filtered: BTreeMap<String, String> =
        filter_sensitive_headers(&headers).into_iter().collect();
    out.insert(
        "wire_filter_sensitive_headers",
        json!({ "filtered": filtered }),
    );

    println!(
        "{}",
        serde_json::to_string(&out).expect("serialize wire dump")
    );
}
