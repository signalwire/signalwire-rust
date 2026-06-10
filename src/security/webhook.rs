//! Webhook signature validation for SignalWire-signed HTTP requests.
//!
//! Implements both schemes from `porting-sdk/webhooks.md`:
//!
//! - **Scheme A** — RELAY/SWML/JSON: `lowercase_hex(HMAC-SHA1(key, url + raw_body))`
//! - **Scheme B** — Compat/cXML form: `base64(HMAC-SHA1(key, url + sortedFormParams))`
//!   with optional `bodySHA256` query-param fallback for JSON-on-compat-surface.
//!
//! Public API mirrors Python's `signalwire.core.security.webhook_validator`:
//!
//! - [`validate_webhook_signature`] — combined validator (tries A, then B).
//! - [`validate_request`] — legacy `@signalwire/compatibility-api` drop-in
//!   that accepts either a raw body string or pre-parsed form params.
//!
//! All comparisons go through [`subtle::ConstantTimeEq`] so the secret
//! is not leaked through repeated requests. Inputs are taken as
//! `&str` / borrowed strings — no allocation hidden behind the API.
//!
//! ```
//! use signalwire::security::webhook::validate_webhook_signature;
//!
//! let key = "PSKtest1234567890abcdef";
//! let url = "https://example.ngrok.io/webhook";
//! let raw_body = r#"{"event":"call.state","params":{"call_id":"abc-123","state":"answered"}}"#;
//! let sig = "c3c08c1fefaf9ee198a100d5906765a6f394bf0f";
//! assert!(validate_webhook_signature(key, sig, url, raw_body).unwrap());
//! ```
//!
//! Copyright (c) 2025 SignalWire. Licensed under the MIT License.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use url::Url;

type HmacSha1 = Hmac<Sha1>;

/// Errors returned by the webhook validator.
///
/// `validate_webhook_signature` and `validate_request` return
/// `Result<bool, WebhookError>` so callers can distinguish a legitimate
/// "signature did not match" (`Ok(false)`) from a programming-error
/// "you forgot the signing key" (`Err(WebhookError::MissingSigningKey)`).
/// Per the spec, missing/empty *signature header* and malformed
/// signatures return `Ok(false)` — those are not errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookError {
    /// Caller passed an empty `signing_key`. This is a programming
    /// error (the key is mandatory configuration), not a validation
    /// failure — Python raises `ValueError` here, Node throws.
    MissingSigningKey,
}

impl std::fmt::Display for WebhookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebhookError::MissingSigningKey => write!(f, "signing_key is required"),
        }
    }
}

impl std::error::Error for WebhookError {}

/// Argument shape for [`validate_request`]. Either a raw body string
/// (which will run Scheme A first, then Scheme B with the form-parsed
/// body) or a pre-parsed form-param list (which goes straight to
/// Scheme B).
///
/// Repeated keys in the `Params` variant are supported by emitting a
/// `Vec<String>` per key in submission order.
#[derive(Debug, Clone)]
pub enum ParamsOrBody {
    /// Raw HTTP request body (UTF-8). The validator will sort form
    /// params if it parses cleanly; for JSON bodies this branch
    /// degrades to the empty-params Scheme B fallback.
    Body(String),
    /// Pre-parsed form params, list-of-(key, list-of-values) so
    /// repeated keys keep their submission order.
    Params(Vec<(String, Vec<String>)>),
}

// ---------------------------------------------------------------------------
//  Internal helpers
// ---------------------------------------------------------------------------

/// Scheme-A digest: lowercase hex of HMAC-SHA1.
fn hex_hmac_sha1(key: &str, message: &str) -> String {
    let mut mac = HmacSha1::new_from_slice(key.as_bytes())
        .expect("HMAC-SHA1 accepts any key length");
    mac.update(message.as_bytes());
    let digest = mac.finalize().into_bytes();
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest.iter() {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

/// Scheme-B digest: standard base64 of HMAC-SHA1.
fn b64_hmac_sha1(key: &str, message: &str) -> String {
    let mut mac = HmacSha1::new_from_slice(key.as_bytes())
        .expect("HMAC-SHA1 accepts any key length");
    mac.update(message.as_bytes());
    let digest = mac.finalize().into_bytes();
    BASE64_STANDARD.encode(digest)
}

/// Constant-time string compare. Uses `subtle::ConstantTimeEq`. Returns
/// `false` if the lengths differ (length itself is non-secret —
/// matches every other port's behavior).
fn safe_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Concatenate form params per Scheme B rules:
///
/// - Sort by key, ASCII ascending.
/// - For repeated keys: keep original submission order, emit
///   `key + value` once per occurrence.
/// - Stable sort preserves order within repeated keys.
fn sorted_concat_params(params: &[(String, Vec<String>)]) -> String {
    if params.is_empty() {
        return String::new();
    }
    // Flatten: each (k, [v1, v2]) -> [(k,v1), (k,v2)] in submission order.
    let mut flat: Vec<(&str, &str)> = Vec::with_capacity(params.len());
    for (k, vs) in params {
        for v in vs {
            flat.push((k.as_str(), v.as_str()));
        }
    }
    // Stable sort by key only.
    flat.sort_by(|a, b| a.0.cmp(b.0));
    let mut out = String::new();
    for (k, v) in flat {
        out.push_str(k);
        out.push_str(v);
    }
    out
}

/// Parse an `application/x-www-form-urlencoded` body into ordered
/// `(key, [values])` tuples. Repeated keys produce a single entry whose
/// value-vector preserves submission order.
fn parse_form_body(raw_body: &str) -> Vec<(String, Vec<String>)> {
    if raw_body.is_empty() {
        return Vec::new();
    }
    let mut order: Vec<String> = Vec::new();
    let mut map: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut insertion: Vec<(String, Vec<String>)> = Vec::new();

    // Use url::form_urlencoded for percent-decoding; preserve original order.
    for (k, v) in url::form_urlencoded::parse(raw_body.as_bytes()) {
        let k = k.into_owned();
        let v = v.into_owned();
        if let Some(pos) = order.iter().position(|x| x == &k) {
            // Append to the existing entry.
            if let Some((_, vs)) = insertion.get_mut(pos) {
                vs.push(v.clone());
            }
            map.entry(k).or_default().push(v);
        } else {
            order.push(k.clone());
            insertion.push((k.clone(), vec![v.clone()]));
            map.entry(k).or_default().push(v);
        }
    }
    insertion
}

/// Return the URL variants to try for Scheme B port normalization.
///
/// SignalWire's backend signs some requests with the standard port
/// included in the URL (`:443` for https, `:80` for http) and some
/// without. The validator must try **both** forms.
///
/// We operate at the string level rather than going through `url::Url`
/// because `Url::parse("https://x:443/")` normalises away the
/// standard port — and the wire signature is over the literal URL bytes.
///
/// Rules:
/// - If the URL has no port and scheme is http/https: emit input AND with-port.
/// - If the URL has the scheme's standard port literal: emit input AND without-port.
/// - Otherwise (non-standard explicit port, or non-http(s) scheme): emit input only.
fn candidate_urls(url: &str) -> Vec<String> {
    // Find scheme and the host portion in the raw string. We need the
    // start index of the host (after `scheme://`) so we can splice
    // `:<port>` in/out without disturbing the rest of the URL.
    let scheme_sep = match url.find("://") {
        Some(i) => i,
        None => return vec![url.to_string()],
    };
    let scheme = &url[..scheme_sep];
    let standard_port = match scheme {
        "http" => "80",
        "https" => "443",
        _ => return vec![url.to_string()],
    };

    let host_start = scheme_sep + 3;
    // Host ends at the first '/', '?', '#', or end of string (whichever
    // comes first). A `:` inside `[ipv6]` should not be confused with a
    // port separator — handle that by skipping over `[...]` blocks.
    let rest = &url[host_start..];
    let mut host_end = rest.len();
    let mut in_v6 = false;
    for (i, ch) in rest.char_indices() {
        match ch {
            '[' => in_v6 = true,
            ']' => in_v6 = false,
            '/' | '?' | '#' if !in_v6 => {
                host_end = i;
                break;
            }
            _ => {}
        }
    }
    let authority = &rest[..host_end];
    let tail = &rest[host_end..];

    // Find the port colon — must come after the closing `]` of an
    // IPv6 host, or after a normal host.
    let port_colon = if authority.starts_with('[') {
        match authority.find(']') {
            Some(close) => {
                if close + 1 < authority.len()
                    && authority.as_bytes().get(close + 1) == Some(&b':')
                {
                    Some(close + 1)
                } else {
                    None
                }
            }
            None => None,
        }
    } else {
        authority.rfind(':')
    };

    let (host_part, current_port): (&str, Option<&str>) = match port_colon {
        Some(idx) => (&authority[..idx], Some(&authority[idx + 1..])),
        None => (authority, None),
    };

    let mut candidates = vec![url.to_string()];

    match current_port {
        None => {
            // No port → also try with the standard port.
            let with_port = format!("{}://{}:{}{}", scheme, host_part, standard_port, tail);
            if with_port != url {
                candidates.push(with_port);
            }
        }
        Some(p) if p == standard_port => {
            // Standard port present → also try without it.
            let without_port = format!("{}://{}{}", scheme, host_part, tail);
            if without_port != url {
                candidates.push(without_port);
            }
        }
        _ => { /* non-standard explicit port — only try as-is */ }
    }
    candidates
}

/// If the URL has `?bodySHA256=<hex>`, verify `sha256_hex(raw_body) == bodySHA256`.
/// Returns `true` when the param is absent (no constraint) or matches.
fn check_body_sha256(url: &str, raw_body: &str) -> bool {
    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(_) => return true, // unparseable URL — let Scheme B fail naturally elsewhere
    };
    let expected = parsed
        .query_pairs()
        .find(|(k, _)| k == "bodySHA256")
        .map(|(_, v)| v.into_owned());
    let expected = match expected {
        Some(e) => e,
        None => return true,
    };
    let mut hasher = Sha256::new();
    hasher.update(raw_body.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest.iter() {
        hex.push_str(&format!("{:02x}", b));
    }
    safe_eq(&hex, &expected)
}

// ---------------------------------------------------------------------------
//  Public API
// ---------------------------------------------------------------------------

/// Validate a SignalWire webhook signature against both schemes.
///
/// # Arguments
/// * `signing_key` — the customer's Signing Key from the SignalWire
///   Dashboard. Empty / missing returns [`WebhookError::MissingSigningKey`]
///   (this is a programming error, not a validation failure).
/// * `signature` — the `X-SignalWire-Signature` (or `X-Twilio-Signature`)
///   header value. Empty returns `Ok(false)` without raising.
/// * `url` — the full URL SignalWire POSTed to (scheme, host, optional
///   port, path, query). Must match exactly what the platform saw.
/// * `raw_body` — the raw UTF-8 request body bytes, **before** any JSON
///   or form parser consumes them. Re-serialization breaks the signature.
///
/// # Returns
/// * `Ok(true)` if the signature matches Scheme A (hex JSON) or
///   Scheme B (base64 form, with port-normalization variants and
///   optional `bodySHA256` fallback).
/// * `Ok(false)` if it does not match.
/// * `Err(WebhookError::MissingSigningKey)` only when the key is missing.
///
/// All comparisons are constant-time via [`subtle::ConstantTimeEq`].
pub fn validate_webhook_signature(
    signing_key: &str,
    signature: &str,
    url: &str,
    raw_body: &str,
) -> Result<bool, WebhookError> {
    if signing_key.is_empty() {
        return Err(WebhookError::MissingSigningKey);
    }
    if signature.is_empty() {
        return Ok(false);
    }

    // ------------------------------------------------------------------
    // Scheme A — RELAY/SWML/JSON: hex(HMAC-SHA1(key, url + raw_body))
    // ------------------------------------------------------------------
    let mut a_input = String::with_capacity(url.len() + raw_body.len());
    a_input.push_str(url);
    a_input.push_str(raw_body);
    let expected_a = hex_hmac_sha1(signing_key, &a_input);
    if safe_eq(&expected_a, signature) {
        return Ok(true);
    }

    // ------------------------------------------------------------------
    // Scheme B — Compat/cXML form
    // Two param shapes (parsed form params, then empty-for-JSON-on-compat),
    // crossed with the URL port-normalization candidates.
    // ------------------------------------------------------------------
    let parsed_params = parse_form_body(raw_body);
    let empty_params: Vec<(String, Vec<String>)> = Vec::new();
    let param_shapes: [&[(String, Vec<String>)]; 2] = [&parsed_params, &empty_params];

    for candidate_url in candidate_urls(url) {
        for shape in param_shapes.iter() {
            let concat = sorted_concat_params(shape);
            let mut b_input = String::with_capacity(candidate_url.len() + concat.len());
            b_input.push_str(&candidate_url);
            b_input.push_str(&concat);
            let expected_b = b64_hmac_sha1(signing_key, &b_input);
            if safe_eq(&expected_b, signature) {
                // bodySHA256 (if present) must match too.
                if check_body_sha256(&candidate_url, raw_body) {
                    return Ok(true);
                }
                // Otherwise keep trying — caller may have signed a different
                // shape that doesn't carry the bodySHA256 constraint.
            }
        }
    }

    Ok(false)
}

/// Legacy `@signalwire/compatibility-api` drop-in entry point.
///
/// If `params_or_raw_body` is [`ParamsOrBody::Body`], delegates to
/// [`validate_webhook_signature`] (Scheme A then Scheme B with parsed form).
///
/// If it's [`ParamsOrBody::Params`], runs Scheme B directly with those
/// pre-parsed form params (plus URL port normalization).
///
/// `bodySHA256` verification is skipped in the `Params` variant — there
/// is no raw body to hash.
pub fn validate_request(
    signing_key: &str,
    signature: &str,
    url: &str,
    params_or_raw_body: &ParamsOrBody,
) -> Result<bool, WebhookError> {
    if signing_key.is_empty() {
        return Err(WebhookError::MissingSigningKey);
    }
    if signature.is_empty() {
        return Ok(false);
    }

    match params_or_raw_body {
        ParamsOrBody::Body(b) => validate_webhook_signature(signing_key, signature, url, b),
        ParamsOrBody::Params(params) => {
            let concat = sorted_concat_params(params);
            for candidate_url in candidate_urls(url) {
                let mut b_input = String::with_capacity(candidate_url.len() + concat.len());
                b_input.push_str(&candidate_url);
                b_input.push_str(&concat);
                let expected_b = b64_hmac_sha1(signing_key, &b_input);
                if safe_eq(&expected_b, signature) {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}

// ---------------------------------------------------------------------------
//  Tests — canonical vectors A, B, C from porting-sdk/webhooks.md.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- canonical vectors (verbatim from porting-sdk/webhooks.md) ------------

    const VECTOR_A_KEY: &str = "PSKtest1234567890abcdef";
    const VECTOR_A_URL: &str = "https://example.ngrok.io/webhook";
    const VECTOR_A_BODY: &str =
        r#"{"event":"call.state","params":{"call_id":"abc-123","state":"answered"}}"#;
    const VECTOR_A_SIG: &str = "c3c08c1fefaf9ee198a100d5906765a6f394bf0f";

    const VECTOR_B_KEY: &str = "12345";
    const VECTOR_B_URL: &str = "https://mycompany.com/myapp.php?foo=1&bar=2";
    const VECTOR_B_SIG: &str = "RSOYDt4T1cUTdK1PDd93/VVr8B8=";

    const VECTOR_C_KEY: &str = "PSKtest1234567890abcdef";
    const VECTOR_C_BODY: &str = r#"{"event":"call.state"}"#;
    const VECTOR_C_URL: &str = "https://example.ngrok.io/webhook?bodySHA256=69f3cbfc18e386ef8236cb7008cd5a54b7fed637a8cb3373b5a1591d7f0fd5f4";
    const VECTOR_C_SIG: &str = "dfO9ek8mxyFtn2nMz24plPmPfIY=";

    fn vector_b_params() -> Vec<(String, Vec<String>)> {
        // Same order Python's `dict` iterates in for the canonical vector —
        // but Scheme B sorts by key so order here only affects test brevity.
        vec![
            ("CallSid".to_string(), vec!["CA1234567890ABCDE".to_string()]),
            ("Caller".to_string(), vec!["+14158675309".to_string()]),
            ("Digits".to_string(), vec!["1234".to_string()]),
            ("From".to_string(), vec!["+14158675309".to_string()]),
            ("To".to_string(), vec!["+18005551212".to_string()]),
        ]
    }

    fn vector_b_form_body() -> String {
        // Manually URL-encode `+` so the on-the-wire body matches what HTTP
        // middleware actually sees.
        let pairs = [
            ("CallSid", "CA1234567890ABCDE"),
            ("Caller", "+14158675309"),
            ("Digits", "1234"),
            ("From", "+14158675309"),
            ("To", "+18005551212"),
        ];
        pairs
            .iter()
            .map(|(k, v)| format!("{}={}", k, v.replace('+', "%2B")))
            .collect::<Vec<_>>()
            .join("&")
    }

    // -- Scheme A -------------------------------------------------------------

    #[test]
    fn scheme_a_positive_canonical_vector() {
        // Vector A: known JSON body + URL + key produces the known hex digest.
        let r = validate_webhook_signature(VECTOR_A_KEY, VECTOR_A_SIG, VECTOR_A_URL, VECTOR_A_BODY);
        assert_eq!(r, Ok(true), "canonical vector A must validate");
    }

    #[test]
    fn scheme_a_negative_tampered_body() {
        let tampered = VECTOR_A_BODY.replace("answered", "ringing");
        let r = validate_webhook_signature(VECTOR_A_KEY, VECTOR_A_SIG, VECTOR_A_URL, &tampered);
        assert_eq!(r, Ok(false), "tampered body must NOT validate");
    }

    #[test]
    fn scheme_a_negative_wrong_key() {
        let r = validate_webhook_signature("wrong-key", VECTOR_A_SIG, VECTOR_A_URL, VECTOR_A_BODY);
        assert_eq!(r, Ok(false));
    }

    #[test]
    fn scheme_a_negative_wrong_url() {
        let r = validate_webhook_signature(
            VECTOR_A_KEY,
            VECTOR_A_SIG,
            "https://example.ngrok.io/different",
            VECTOR_A_BODY,
        );
        assert_eq!(r, Ok(false));
    }

    // -- Scheme B -------------------------------------------------------------

    #[test]
    fn scheme_b_positive_canonical_form_via_raw_body() {
        // Vector B: form params delivered as the raw form-encoded body.
        let body = vector_b_form_body();
        let r = validate_webhook_signature(VECTOR_B_KEY, VECTOR_B_SIG, VECTOR_B_URL, &body);
        assert_eq!(r, Ok(true), "canonical vector B must validate via raw body");
    }

    #[test]
    fn scheme_b_positive_via_validate_request_params() {
        // validate_request with pre-parsed params goes straight to Scheme B.
        let params = ParamsOrBody::Params(vector_b_params());
        let r = validate_request(VECTOR_B_KEY, VECTOR_B_SIG, VECTOR_B_URL, &params);
        assert_eq!(r, Ok(true));
    }

    #[test]
    fn scheme_b_positive_via_validate_request_body() {
        // validate_request with a string body delegates to the combined
        // validator — and the combined validator finds Scheme B.
        let body = vector_b_form_body();
        let r = validate_request(
            VECTOR_B_KEY,
            VECTOR_B_SIG,
            VECTOR_B_URL,
            &ParamsOrBody::Body(body),
        );
        assert_eq!(r, Ok(true));
    }

    #[test]
    fn body_sha256_canonical_vector_c() {
        // Vector C: JSON body on compat surface, signature over URL with bodySHA256.
        let r = validate_webhook_signature(VECTOR_C_KEY, VECTOR_C_SIG, VECTOR_C_URL, VECTOR_C_BODY);
        assert_eq!(r, Ok(true), "canonical vector C must validate");
    }

    #[test]
    fn body_sha256_mismatch_rejected() {
        // HMAC matches URL+'' for some shape, but bodySHA256 mismatches
        // the actual body — must fail.
        let wrong_body = r#"{"event":"DIFFERENT"}"#;
        let r = validate_webhook_signature(VECTOR_C_KEY, VECTOR_C_SIG, VECTOR_C_URL, wrong_body);
        assert_eq!(r, Ok(false));
    }

    // -- URL port normalization ----------------------------------------------

    fn b64_sig(key: &str, url: &str, params: &[(&str, &str)]) -> String {
        let mut data = url.to_string();
        let mut sorted: Vec<(&str, &str)> = params.to_vec();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        for (k, v) in sorted {
            data.push_str(k);
            data.push_str(v);
        }
        b64_hmac_sha1(key, &data)
    }

    #[test]
    fn signature_with_port_accepted_when_request_has_no_port() {
        // Backend signed with :443 — request URL has no port → accept.
        let key = "test-key";
        let sig = b64_sig(key, "https://example.com:443/webhook", &[]);
        let r = validate_webhook_signature(key, &sig, "https://example.com/webhook", "{}");
        assert_eq!(r, Ok(true));
    }

    #[test]
    fn signature_without_port_accepted_when_request_has_standard_port() {
        let key = "test-key";
        let sig = b64_sig(key, "https://example.com/webhook", &[]);
        let r = validate_webhook_signature(key, &sig, "https://example.com:443/webhook", "{}");
        assert_eq!(r, Ok(true));
    }

    #[test]
    fn http_port_80_normalization() {
        let key = "test-key";
        let sig = b64_sig(key, "http://example.com:80/path", &[]);
        let r = validate_webhook_signature(key, &sig, "http://example.com/path", "");
        assert_eq!(r, Ok(true));
    }

    // -- Repeated form keys --------------------------------------------------

    #[test]
    fn repeated_keys_concat_in_submission_order() {
        // To=a&To=b → signing string `URL + ToaTob`, deterministic.
        let key = "test-key";
        let url = "https://example.com/hook";
        let body = "To=a&To=b";
        let expected_data = format!("{}ToaTob", url);
        let sig = b64_hmac_sha1(key, &expected_data);
        let r = validate_webhook_signature(key, &sig, url, body);
        assert_eq!(r, Ok(true));
    }

    #[test]
    fn repeated_keys_swapped_order_is_a_different_signature() {
        // To=b&To=a is a different submission and yields a different digest.
        let key = "test-key";
        let url = "https://example.com/hook";
        let data_ab = format!("{}ToaTob", url);
        let sig_for_ab = b64_hmac_sha1(key, &data_ab);
        let r_match = validate_webhook_signature(key, &sig_for_ab, url, "To=a&To=b");
        let r_mismatch = validate_webhook_signature(key, &sig_for_ab, url, "To=b&To=a");
        assert_eq!(r_match, Ok(true));
        assert_eq!(r_mismatch, Ok(false));
    }

    // -- Error modes ---------------------------------------------------------

    #[test]
    fn missing_signature_returns_false() {
        let r = validate_webhook_signature(VECTOR_A_KEY, "", VECTOR_A_URL, VECTOR_A_BODY);
        assert_eq!(r, Ok(false));
    }

    #[test]
    fn missing_signing_key_returns_error() {
        let r = validate_webhook_signature("", "sig", VECTOR_A_URL, VECTOR_A_BODY);
        assert_eq!(r, Err(WebhookError::MissingSigningKey));
    }

    #[test]
    fn malformed_signature_returns_false_without_panicking() {
        // Wrong length, weird chars, base64 noise — none should panic.
        for garbage in ["xyz", "!!!!", &"a".repeat(100), "%%notbase64%%"] {
            let r = validate_webhook_signature(VECTOR_A_KEY, garbage, VECTOR_A_URL, VECTOR_A_BODY);
            assert_eq!(r, Ok(false), "garbage {garbage:?} should not panic and should not validate");
        }
    }

    // -- validate_request dispatch -------------------------------------------

    #[test]
    fn validate_request_string_arg_delegates_to_combined_validator() {
        let r = validate_request(
            VECTOR_A_KEY,
            VECTOR_A_SIG,
            VECTOR_A_URL,
            &ParamsOrBody::Body(VECTOR_A_BODY.to_string()),
        );
        assert_eq!(r, Ok(true));
    }

    #[test]
    fn validate_request_params_arg_runs_scheme_b_directly() {
        let params = ParamsOrBody::Params(vector_b_params());
        let r = validate_request(VECTOR_B_KEY, VECTOR_B_SIG, VECTOR_B_URL, &params);
        assert_eq!(r, Ok(true));
    }

    #[test]
    fn validate_request_missing_signing_key_returns_error() {
        let r = validate_request("", "sig", VECTOR_A_URL, &ParamsOrBody::Body("".into()));
        assert_eq!(r, Err(WebhookError::MissingSigningKey));
    }

    // -- Constant-time compare proof ----------------------------------------

    #[test]
    fn safe_eq_uses_subtle_constant_time() {
        // The validator must not branch on signature contents. We assert
        // safe_eq is exposed as a function (not inlined macros) and that
        // a well-known byte-equal pair returns true while length-mismatch
        // returns false. The deeper "is it actually constant-time" is
        // delegated to the `subtle` crate's audited implementation.
        assert!(safe_eq("abc", "abc"));
        assert!(!safe_eq("abc", "abd"));
        assert!(!safe_eq("abc", "abcd"));
        assert!(!safe_eq("", "x"));
    }

    #[test]
    fn validator_source_uses_subtle_constant_time_eq() {
        // Source-level audit: the spec mandates constant-time compare.
        // We assert the module imports `subtle::ConstantTimeEq` and
        // that the production code path (everything before the
        // `#[cfg(test)] mod tests` block) does not compare a digest
        // with plain `==`.
        let full_src = include_str!("webhook.rs");
        assert!(
            full_src.contains("subtle::ConstantTimeEq"),
            "webhook validator must import subtle::ConstantTimeEq"
        );
        assert!(
            full_src.contains(".ct_eq("),
            "safe_eq must call .ct_eq(...) (the constant-time API)"
        );

        // Strip the test module so we audit only production code.
        let prod_src = match full_src.find("#[cfg(test)]") {
            Some(idx) => &full_src[..idx],
            None => full_src,
        };

        // Negative: production code must not do `expected == signature`
        // anywhere — that's the leaky pattern the constant-time compare
        // exists to replace.
        for forbidden in [
            "expected_a == signature",
            "expected_b == signature",
            "expected == signature",
        ] {
            assert!(
                !prod_src.contains(forbidden),
                "production code must not use plain == on digest: found {forbidden:?}"
            );
        }
    }
}
