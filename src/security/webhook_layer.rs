//! Tower / axum middleware for SignalWire webhook signature validation.
//!
//! This module provides a [`tower::Layer`] that buffers the request body,
//! verifies the `X-SignalWire-Signature` (or `X-Twilio-Signature`)
//! header against a configured signing key, and either:
//!
//! - rejects the request with `403 Forbidden` (no body) when the
//!   signature is missing or invalid, or
//! - rebuilds the request with the buffered body so downstream
//!   handlers can re-read it as a normal `axum::body::Body`.
//!
//! The URL passed to the validator honors `X-Forwarded-Proto` /
//! `X-Forwarded-Host` (for reverse-proxy / tunnel deploys) and falls
//! back to scheme-derived-from-`x-forwarded-proto-or-https` plus the
//! `Host` header.
//!
//! The Layer is gated behind the `tower-middleware` Cargo feature
//! (enabled by default). Users who only need the raw validator can
//! depend on `signalwire` with `default-features = false`.
//!
//! Copyright (c) 2025 SignalWire. Licensed under the MIT License.

use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, Request, Response, StatusCode};
use http_body_util::BodyExt;
use tower::{Layer, Service};

use super::webhook::validate_webhook_signature;

/// Header names checked for the signature. Both forms are accepted —
/// the legacy `X-Twilio-Signature` exists for cXML/Compatibility-API
/// integrations migrating from Twilio.
const SIGNATURE_HEADERS: &[&str] = &["x-signalwire-signature", "x-twilio-signature"];

// ---------------------------------------------------------------------------
//  Framework-free decomposed validation core (cross-port contract).
//
//  This is the language-neutral decision core mandated by
//  `porting-sdk/webhooks.md` + the hidden-surface audit's decompose-at-the-
//  boundary ruling: every port ships the SAME shape (dotnet
//  `WebhookValidationMiddleware.Validate`, a Rack/PSGI middleware `.call`,
//  Python `webhook_middleware.validate`, …). The tower [`WebhookLayer`] below
//  is the Rust framework *wrapper* idiom on top of it; this function is the
//  part that is required cross-port.
// ---------------------------------------------------------------------------

/// Framework-free webhook-validation decision.
///
/// Takes a request decomposed into language-neutral primitives and returns
/// either a `403`-shaped response triple to short-circuit with, or `None`
/// to let the downstream handler run.
///
/// The signature is read from the `headers` map (`X-SignalWire-Signature`,
/// falling back to the legacy `X-Twilio-Signature` alias — lookups are
/// case-insensitive). `method` is accepted for a stable signature
/// but is not part of the HMAC. On any failure (missing/bad signature,
/// validator error) the function returns `Some((403, {}, ""))` — no body
/// detail, so which branch tripped is not leaked.
///
/// An empty `signing_key` is a programming error (the key is mandatory
/// configuration). matching `ValueError`, this is caught by a
/// debug assertion; in release builds it degrades to the reject triple
/// rather than authenticating an unsigned request.
///
/// # Returns
/// * `None` if the signature is valid — run the handler.
/// * `Some((403, {}, ""))` otherwise — short-circuit with `403 Forbidden`.
///
/// The concrete `HashMap<String, String>` is the fixed header-map type this
/// validation contract is defined over, so `clippy::implicit_hasher` is
/// suppressed: generalizing over the hasher would change the published
/// signature for no caller benefit.
#[allow(clippy::implicit_hasher)]
#[must_use]
pub fn validate(
    method: &str,
    url: &str,
    headers: &HashMap<String, String>,
    body: &str,
    signing_key: &str,
) -> Option<(u16, HashMap<String, String>, String)> {
    let _ = method; // part of the cross-port contract; not HMAC'd.
    debug_assert!(!signing_key.is_empty(), "signing_key is required");

    let reject = || Some((403u16, HashMap::new(), String::new()));

    if signing_key.is_empty() {
        return reject();
    }

    // Case-insensitive header lookup for the signature (X-SignalWire first,
    // then the legacy X-Twilio alias).
    let signature = SIGNATURE_HEADERS.iter().find_map(|want| {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(want))
            .map(|(_, v)| v.as_str())
    });

    let Some(signature) = signature.filter(|s| !s.is_empty()) else {
        return reject();
    };

    match validate_webhook_signature(signing_key, signature, url, body) {
        Ok(true) => None,
        _ => reject(),
    }
}

/// Tower [`Layer`] that wraps any `Service<Request<Body>>` with
/// SignalWire webhook signature validation.
///
/// Construct with [`WebhookLayer::new`] and pass the customer's
/// Signing Key. Optionally provide an explicit URL via
/// [`WebhookLayer::with_url_override`] when running behind a tunnel
/// or reverse proxy that doesn't set `X-Forwarded-*` headers.
#[derive(Clone)]
pub struct WebhookLayer {
    inner: Arc<WebhookConfig>,
}

struct WebhookConfig {
    signing_key: String,
    url_override: Option<String>,
}

impl WebhookLayer {
    /// Construct a new layer bound to the given Signing Key.
    pub fn new(signing_key: impl Into<String>) -> Self {
        WebhookLayer {
            inner: Arc::new(WebhookConfig {
                signing_key: signing_key.into(),
                url_override: None,
            }),
        }
    }

    /// Override the URL **base** the validator signs against. When set,
    /// reverse-proxy header reconstruction is skipped and the full URL
    /// is built as `<base><request_path_and_query>`.
    ///
    /// `base` is typically `scheme://host[:port]` with no trailing slash
    /// — e.g. `"https://example.ngrok.io"`. Any trailing `/` is
    /// stripped before concatenation.
    #[must_use]
    pub fn with_url_base(mut self, base: impl Into<String>) -> Self {
        let base = base.into();
        let cfg = WebhookConfig {
            signing_key: self.inner.signing_key.clone(),
            url_override: Some(base),
        };
        self.inner = Arc::new(cfg);
        self
    }
}

impl<S> Layer<S> for WebhookLayer {
    type Service = WebhookValidate<S>;

    fn layer(&self, inner: S) -> Self::Service {
        WebhookValidate {
            inner,
            cfg: self.inner.clone(),
        }
    }
}

/// Tower [`Service`] produced by [`WebhookLayer::layer`]. Buffers the
/// request body, validates the signature, and either forwards a
/// reconstructed request to the inner service or short-circuits with
/// a `403 Forbidden`.
#[derive(Clone)]
pub struct WebhookValidate<S> {
    inner: S,
    cfg: Arc<WebhookConfig>,
}

impl<S> Service<Request<Body>> for WebhookValidate<S>
where
    S: Service<Request<Body>, Response = Response<Body>, Error = Infallible>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Response<Body>, Infallible>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        // Clone of inner needed because `call` takes &mut self but we
        // move `inner` into the async block. This is the standard
        // tower pattern.
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);
        let cfg = self.cfg.clone();

        Box::pin(async move {
            // Collect the decomposed primitives, then hand them to the
            // framework-free [`validate`] core so the tower wrapper and the
            // cross-port decision logic never diverge.
            let method = req.method().as_str().to_string();

            // Reconstruct the URL the platform signed.
            let url = match cfg.url_override.as_ref() {
                Some(u) => format!(
                    "{}{}",
                    u.trim_end_matches('/'),
                    req.uri()
                        .path_and_query()
                        .map_or("/", http::uri::PathAndQuery::as_str)
                ),
                None => reconstruct_url_from_request(req.headers(), req.uri()),
            };

            // Snapshot the headers into the language-neutral string map the
            // core consumes (axum's HeaderMap keys are already lowercase).
            let mut header_map: HashMap<String, String> = HashMap::new();
            for (name, value) in req.headers() {
                if let Ok(v) = value.to_str() {
                    header_map.insert(name.as_str().to_string(), v.to_string());
                }
            }

            // Buffer the body fully so we can hand it to the inner
            // service after validation. This is unavoidable: the
            // signature is over the bytes, not the stream.
            let (parts, body) = req.into_parts();
            let collected = match body.collect().await {
                Ok(c) => c.to_bytes(),
                Err(_) => {
                    return Ok(forbidden_response());
                }
            };

            let raw_body_str = match std::str::from_utf8(&collected) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    // Non-UTF-8 body — SignalWire never sends those, but
                    // be defensive: an attacker shouldn't get a different
                    // error path here. Treat as invalid.
                    return Ok(forbidden_response());
                }
            };

            // Framework-free decision core: `None` = pass, `Some(triple)` = reject.
            if let Some((status, _headers, _body)) = validate(
                &method,
                &url,
                &header_map,
                &raw_body_str,
                cfg.signing_key.as_str(),
            ) {
                let mut resp = Response::new(Body::empty());
                *resp.status_mut() = StatusCode::from_u16(status).unwrap_or(StatusCode::FORBIDDEN);
                return Ok(resp);
            }

            // Rebuild the request with the buffered body so downstream
            // handlers can re-read.
            let req = Request::from_parts(parts, Body::from(collected));
            inner.call(req).await
        })
    }
}

fn forbidden_response() -> Response<Body> {
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = StatusCode::FORBIDDEN;
    resp
}

/// Best-effort URL reconstruction that mirrors the spec's
/// "URL reconstruction behind proxies" rules:
///
/// 1. `X-Forwarded-Proto` + `X-Forwarded-Host` (when present together).
/// 2. `Host` + scheme inferred from `X-Forwarded-Proto`, defaulting to
///    `https` (SignalWire only POSTs to https endpoints in production).
/// 3. Fall back to `https://unknown` so the validator at least sees a
///    consistent URL — almost certainly invalid, but won't panic.
fn reconstruct_url_from_request(headers: &HeaderMap, uri: &axum::http::Uri) -> String {
    let proto = header_str(headers, "x-forwarded-proto").unwrap_or("https");
    let host = header_str(headers, "x-forwarded-host")
        .or_else(|| header_str(headers, "host"))
        .unwrap_or("unknown");
    let path_and_query = uri
        .path_and_query()
        .map_or("/", http::uri::PathAndQuery::as_str);
    format!("{proto}://{host}{path_and_query}")
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|v: &HeaderValue| v.to_str().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[test]
    fn reconstruct_url_uses_x_forwarded_pair() {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-proto", "https".parse().unwrap());
        h.insert("x-forwarded-host", "tunnel.example.com".parse().unwrap());
        let req = Request::builder()
            .uri("/swaig?call_id=abc")
            .body(())
            .unwrap();
        let url = reconstruct_url_from_request(&h, req.uri());
        assert_eq!(url, "https://tunnel.example.com/swaig?call_id=abc");
    }

    #[test]
    fn reconstruct_url_falls_back_to_host_header() {
        let mut h = HeaderMap::new();
        h.insert("host", "internal.local:8080".parse().unwrap());
        let req = Request::builder().uri("/").body(()).unwrap();
        let url = reconstruct_url_from_request(&h, req.uri());
        assert_eq!(url, "https://internal.local:8080/");
    }

    #[test]
    fn webhook_layer_with_url_base_stores_value() {
        let layer = WebhookLayer::new("test-key").with_url_base("https://example.com");
        assert_eq!(layer.inner.signing_key, "test-key");
        assert_eq!(
            layer.inner.url_override.as_deref(),
            Some("https://example.com")
        );
    }

    // -- Framework-free decomposed `validate` core --------------------------
    //
    // Canonical Scheme-A vector from porting-sdk/webhooks.md.

    const V_KEY: &str = "PSKtest1234567890abcdef";
    const V_URL: &str = "https://example.ngrok.io/webhook";
    const V_BODY: &str =
        r#"{"event":"call.state","params":{"call_id":"abc-123","state":"answered"}}"#;
    const V_SIG: &str = "c3c08c1fefaf9ee198a100d5906765a6f394bf0f";

    #[test]
    fn validate_core_valid_signature_returns_none() {
        // A valid X-SignalWire-Signature → None (let the handler run).
        let mut headers = HashMap::new();
        headers.insert("x-signalwire-signature".to_string(), V_SIG.to_string());
        let out = validate("POST", V_URL, &headers, V_BODY, V_KEY);
        assert_eq!(out, None, "valid signature must pass (None)");
    }

    #[test]
    fn validate_core_bad_signature_returns_403_triple() {
        let mut headers = HashMap::new();
        headers.insert(
            "x-signalwire-signature".to_string(),
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef".to_string(),
        );
        let out = validate("POST", V_URL, &headers, V_BODY, V_KEY);
        assert_eq!(out, Some((403, HashMap::new(), String::new())));
    }

    #[test]
    fn validate_core_missing_signature_returns_403_triple() {
        // No signature header at all → reject, never panic.
        let headers = HashMap::new();
        let out = validate("POST", V_URL, &headers, V_BODY, V_KEY);
        assert_eq!(out, Some((403, HashMap::new(), String::new())));
    }

    #[test]
    fn validate_core_x_twilio_signature_alias_honored() {
        // The legacy X-Twilio-Signature alias must be accepted for the same
        // signature the X-SignalWire header would carry.
        let mut headers = HashMap::new();
        headers.insert("X-Twilio-Signature".to_string(), V_SIG.to_string());
        let out = validate("POST", V_URL, &headers, V_BODY, V_KEY);
        assert_eq!(out, None, "X-Twilio-Signature alias must be honored");
    }

    #[test]
    fn validate_core_header_lookup_is_case_insensitive() {
        // Header keys may arrive in any casing; the core lowercases-compares.
        let mut headers = HashMap::new();
        headers.insert("X-SignalWire-Signature".to_string(), V_SIG.to_string());
        let out = validate("POST", V_URL, &headers, V_BODY, V_KEY);
        assert_eq!(out, None);
    }
}
