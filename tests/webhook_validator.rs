//! Integration tests for webhook signature validation.
//!
//! Lives in `tests/` (not `src/`) so it links the public API exactly
//! as a downstream consumer would. Mirrors
//! `signalwire-python/tests/unit/security/test_webhook_validator.py`
//! 1-for-1 — every assertion has a Python counterpart in the
//! reference suite.
//!
//! These are TRUE tests: each one drives the validator with the real
//! HMAC machinery and asserts on byte-for-byte digests. No mocks,
//! no patches, no stubbing.

use signalwire::security::webhook::{
    validate_request, validate_webhook_signature, ParamsOrBody, WebhookError,
};

// ---------------------------------------------------------------------------
//  Canonical test vectors (verbatim from porting-sdk/webhooks.md)
// ---------------------------------------------------------------------------

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
    vec![
        ("CallSid".into(), vec!["CA1234567890ABCDE".into()]),
        ("Caller".into(), vec!["+14158675309".into()]),
        ("Digits".into(), vec!["1234".into()]),
        ("From".into(), vec!["+14158675309".into()]),
        ("To".into(), vec!["+18005551212".into()]),
    ]
}

fn vector_b_form_body() -> String {
    // Hand-encode `+` so the body matches what HTTP middleware sees on the wire.
    let pairs: &[(&str, &str)] = &[
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

// ---------------------------------------------------------------------------
//  Scheme A — RELAY/JSON (hex)
// ---------------------------------------------------------------------------

#[test]
fn scheme_a_positive_canonical_vector() {
    // Canonical Vector A: known JSON body + URL + key produces the known hex digest.
    let r = validate_webhook_signature(VECTOR_A_KEY, VECTOR_A_SIG, VECTOR_A_URL, VECTOR_A_BODY);
    assert_eq!(r, Ok(true));
}

#[test]
fn scheme_a_negative_tampered_body() {
    // Same key/url, body changed → False.
    let tampered = VECTOR_A_BODY.replace("answered", "ringing");
    let r = validate_webhook_signature(VECTOR_A_KEY, VECTOR_A_SIG, VECTOR_A_URL, &tampered);
    assert_eq!(r, Ok(false));
}

#[test]
fn scheme_a_negative_wrong_key() {
    let r =
        validate_webhook_signature("not-the-real-key", VECTOR_A_SIG, VECTOR_A_URL, VECTOR_A_BODY);
    assert_eq!(r, Ok(false));
}

#[test]
fn scheme_a_negative_wrong_url() {
    // URL is part of the digest input, so a different path must fail.
    let r = validate_webhook_signature(
        VECTOR_A_KEY,
        VECTOR_A_SIG,
        "https://example.ngrok.io/different",
        VECTOR_A_BODY,
    );
    assert_eq!(r, Ok(false));
}

// ---------------------------------------------------------------------------
//  Scheme B — Compat/cXML (base64 form)
// ---------------------------------------------------------------------------

#[test]
fn scheme_b_canonical_form_vector_via_raw_body() {
    // Canonical Vector B: form params delivered as the form-encoded body.
    let body = vector_b_form_body();
    let r = validate_webhook_signature(VECTOR_B_KEY, VECTOR_B_SIG, VECTOR_B_URL, &body);
    assert_eq!(r, Ok(true));
}

#[test]
fn scheme_b_canonical_form_vector_via_validate_request_params() {
    // validate_request(..., Params) goes straight to Scheme B.
    let r = validate_request(
        VECTOR_B_KEY,
        VECTOR_B_SIG,
        VECTOR_B_URL,
        &ParamsOrBody::Params(vector_b_params()),
    );
    assert_eq!(r, Ok(true));
}

#[test]
fn scheme_b_negative_tampered_param() {
    // Same URL, single param tweaked → digest changes → False.
    let mut params = vector_b_params();
    params[0].1[0] = "CHANGED".to_string();
    let r = validate_request(
        VECTOR_B_KEY,
        VECTOR_B_SIG,
        VECTOR_B_URL,
        &ParamsOrBody::Params(params),
    );
    assert_eq!(r, Ok(false));
}

#[test]
fn body_sha256_canonical_vector_c() {
    // Canonical Vector C: JSON on compat surface, URL carries bodySHA256.
    let r = validate_webhook_signature(VECTOR_C_KEY, VECTOR_C_SIG, VECTOR_C_URL, VECTOR_C_BODY);
    assert_eq!(r, Ok(true));
}

#[test]
fn body_sha256_mismatch_rejected() {
    // HMAC matches URL+'' but bodySHA256 in the URL doesn't match the body.
    let wrong_body = r#"{"event":"DIFFERENT"}"#;
    let r = validate_webhook_signature(VECTOR_C_KEY, VECTOR_C_SIG, VECTOR_C_URL, wrong_body);
    assert_eq!(r, Ok(false));
}

// ---------------------------------------------------------------------------
//  URL port normalization
// ---------------------------------------------------------------------------

fn b64_sig(key: &str, url: &str, params: &[(&str, &str)]) -> String {
    use base64::Engine as _;
    use hmac::{Hmac, Mac};
    use sha1::Sha1;
    type HmacSha1 = Hmac<Sha1>;

    let mut data = url.to_string();
    let mut sorted: Vec<(&str, &str)> = params.to_vec();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    for (k, v) in sorted {
        data.push_str(k);
        data.push_str(v);
    }
    let mut mac = HmacSha1::new_from_slice(key.as_bytes()).unwrap();
    mac.update(data.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

#[test]
fn https_with_port_accepted_when_request_has_no_port() {
    // Backend signed `https://example.com:443/webhook`; we receive
    // `https://example.com/webhook` — must accept.
    let key = "test-key";
    let sig = b64_sig(key, "https://example.com:443/webhook", &[]);
    let r = validate_webhook_signature(key, &sig, "https://example.com/webhook", "{}");
    assert_eq!(r, Ok(true));
}

#[test]
fn https_without_port_accepted_when_request_has_standard_port() {
    // Reverse direction: signed without port, request has :443.
    let key = "test-key";
    let sig = b64_sig(key, "https://example.com/webhook", &[]);
    let r = validate_webhook_signature(key, &sig, "https://example.com:443/webhook", "{}");
    assert_eq!(r, Ok(true));
}

#[test]
fn http_port_80_normalization_both_directions() {
    let key = "test-key";
    // signed-with-:80 → received-without
    let sig = b64_sig(key, "http://example.com:80/path", &[]);
    let r = validate_webhook_signature(key, &sig, "http://example.com/path", "");
    assert_eq!(r, Ok(true));
    // signed-without → received-with-:80
    let sig = b64_sig(key, "http://example.com/path", &[]);
    let r = validate_webhook_signature(key, &sig, "http://example.com:80/path", "");
    assert_eq!(r, Ok(true));
}

// ---------------------------------------------------------------------------
//  Repeated form keys
// ---------------------------------------------------------------------------

#[test]
fn repeated_form_keys_concat_in_submission_order() {
    // Body `To=a&To=b` → signing string `URL + ToaTob`, deterministic.
    use base64::Engine as _;
    use hmac::{Hmac, Mac};
    use sha1::Sha1;
    type HmacSha1 = Hmac<Sha1>;

    let key = "test-key";
    let url = "https://example.com/hook";
    let body = "To=a&To=b";
    let expected_data = format!("{}ToaTob", url);
    let mut mac = HmacSha1::new_from_slice(key.as_bytes()).unwrap();
    mac.update(expected_data.as_bytes());
    let sig = base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());
    let r = validate_webhook_signature(key, &sig, url, body);
    assert_eq!(r, Ok(true));
}

#[test]
fn repeated_form_keys_swapped_order_yields_different_signature() {
    use base64::Engine as _;
    use hmac::{Hmac, Mac};
    use sha1::Sha1;
    type HmacSha1 = Hmac<Sha1>;

    let key = "test-key";
    let url = "https://example.com/hook";
    let data_ab = format!("{}ToaTob", url);
    let mut mac = HmacSha1::new_from_slice(key.as_bytes()).unwrap();
    mac.update(data_ab.as_bytes());
    let sig_for_ab =
        base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

    // body_ab matches the signature; body_ba must NOT.
    assert_eq!(
        validate_webhook_signature(key, &sig_for_ab, url, "To=a&To=b"),
        Ok(true)
    );
    assert_eq!(
        validate_webhook_signature(key, &sig_for_ab, url, "To=b&To=a"),
        Ok(false)
    );
}

// ---------------------------------------------------------------------------
//  Error modes
// ---------------------------------------------------------------------------

#[test]
fn missing_signature_returns_false_not_error() {
    let r = validate_webhook_signature(VECTOR_A_KEY, "", VECTOR_A_URL, VECTOR_A_BODY);
    assert_eq!(r, Ok(false));
}

#[test]
fn missing_signing_key_returns_error() {
    let r = validate_webhook_signature("", "anything", VECTOR_A_URL, VECTOR_A_BODY);
    assert_eq!(r, Err(WebhookError::MissingSigningKey));
}

#[test]
fn malformed_signature_returns_false_without_panicking() {
    for garbage in ["xyz", "!!!!", &"a".repeat(200), "%%notbase64%%"] {
        let r = validate_webhook_signature(VECTOR_A_KEY, garbage, VECTOR_A_URL, VECTOR_A_BODY);
        assert_eq!(r, Ok(false), "garbage signature {garbage:?} must not panic");
    }
}

// ---------------------------------------------------------------------------
//  validate_request dispatch
// ---------------------------------------------------------------------------

#[test]
fn validate_request_with_body_string_delegates_to_combined_validator() {
    // String 4th-arg behaves identically to validate_webhook_signature.
    let r = validate_request(
        VECTOR_A_KEY,
        VECTOR_A_SIG,
        VECTOR_A_URL,
        &ParamsOrBody::Body(VECTOR_A_BODY.into()),
    );
    assert_eq!(r, Ok(true));
}

#[test]
fn validate_request_with_params_runs_scheme_b_directly() {
    let r = validate_request(
        VECTOR_B_KEY,
        VECTOR_B_SIG,
        VECTOR_B_URL,
        &ParamsOrBody::Params(vector_b_params()),
    );
    assert_eq!(r, Ok(true));
}

#[test]
fn validate_request_missing_signing_key_returns_error() {
    let r = validate_request(
        "",
        "sig",
        VECTOR_A_URL,
        &ParamsOrBody::Body(VECTOR_A_BODY.into()),
    );
    assert_eq!(r, Err(WebhookError::MissingSigningKey));
}

// ---------------------------------------------------------------------------
//  Constant-time compare proof — exercises the public API path
// ---------------------------------------------------------------------------

#[test]
fn equal_length_garbage_does_not_short_circuit() {
    // Build a signature of the correct length filled with 'a' chars.
    // The validator must still iterate through the constant-time
    // compare and return False — not throw, not panic.
    let len = VECTOR_A_SIG.len();
    let fake = "a".repeat(len);
    let r = validate_webhook_signature(VECTOR_A_KEY, &fake, VECTOR_A_URL, VECTOR_A_BODY);
    assert_eq!(r, Ok(false));
}
