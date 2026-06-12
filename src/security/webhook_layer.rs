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
            // Pull the signature header (case-insensitive — axum's
            // HeaderMap already normalises to lowercase).
            let sig: Option<String> = SIGNATURE_HEADERS
                .iter()
                .find_map(|name| req.headers().get(*name))
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            // Reconstruct the URL the platform signed.
            let url = match cfg.url_override.as_ref() {
                Some(u) => format!(
                    "{}{}",
                    u.trim_end_matches('/'),
                    req.uri()
                        .path_and_query()
                        .map(|pq| pq.as_str())
                        .unwrap_or("/")
                ),
                None => reconstruct_url_from_request(req.headers(), req.uri()),
            };

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

            let signature = sig.as_deref().unwrap_or("");
            let valid = validate_webhook_signature(
                cfg.signing_key.as_str(),
                signature,
                &url,
                &raw_body_str,
            )
            .unwrap_or(false);

            if !valid {
                return Ok(forbidden_response());
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
    let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
    format!("{}://{}{}", proto, host, path_and_query)
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v: &HeaderValue| v.to_str().ok())
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
}
