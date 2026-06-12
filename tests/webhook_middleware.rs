//! Integration tests for the Tower [`WebhookLayer`] middleware.
//!
//! Spins up an axum router with the layer, drives requests through
//! `tower::ServiceExt::oneshot`, and asserts:
//!
//! - Valid signature → handler invoked, 200 returned.
//! - Invalid signature → 403, handler NOT invoked.
//! - Missing header → 403.
//! - Raw body forwarded — handler can re-read the body bytes.
//! - URL reconstruction honors `X-Forwarded-Proto` / `X-Forwarded-Host`.
//!
//! These exercise the real tower::Service glue; nothing is mocked.
//!
//! Gated on the `tower-middleware` feature (default-enabled).

#![cfg(feature = "tower-middleware")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderValue, Request, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use http_body_util::BodyExt;
use tower::ServiceExt;

use signalwire::security::webhook_layer::WebhookLayer;

// ---------------------------------------------------------------------------
//  Canonical signing fixture
//
//  Vector A from porting-sdk/webhooks.md. The signed URL is
//  `https://example.ngrok.io/webhook`, so the layer's `with_url_base`
//  takes the host portion `https://example.ngrok.io` and the request
//  URI `/webhook` makes up the path.
// ---------------------------------------------------------------------------

const KEY: &str = "PSKtest1234567890abcdef";
const BODY: &str = r#"{"event":"call.state","params":{"call_id":"abc-123","state":"answered"}}"#;
const VALID_SIG: &str = "c3c08c1fefaf9ee198a100d5906765a6f394bf0f";

// Hits-counter wrapping the inner handler so we can assert that an
// invalid request is rejected BEFORE the handler runs.
#[derive(Clone, Default)]
struct HitCounter(Arc<AtomicUsize>);

impl HitCounter {
    fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

async fn echo_handler(State(hits): State<HitCounter>, body: axum::body::Bytes) -> impl IntoResponse {
    hits.0.fetch_add(1, Ordering::SeqCst);
    // Return the body verbatim so the test can confirm the bytes survived.
    (StatusCode::OK, body)
}

fn build_router(layer: WebhookLayer, hits: HitCounter) -> Router {
    Router::new()
        .route("/webhook", post(echo_handler))
        .layer(layer)
        .with_state(hits)
}

async fn read_body(resp: axum::response::Response) -> Vec<u8> {
    resp.into_body().collect().await.unwrap().to_bytes().to_vec()
}

// ---------------------------------------------------------------------------
//  Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn valid_signature_invokes_handler_and_returns_200() {
    let hits = HitCounter::default();
    let layer = WebhookLayer::new(KEY).with_url_base("https://example.ngrok.io");
    let app = build_router(layer, hits.clone());

    let req = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header("x-signalwire-signature", VALID_SIG)
        .body(Body::from(BODY.as_bytes().to_vec()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(hits.count(), 1, "handler must run exactly once");

    // Echo handler returned the buffered body — proves the layer
    // forwarded the raw bytes intact.
    let echoed = read_body(resp).await;
    assert_eq!(echoed, BODY.as_bytes());
}

#[tokio::test]
async fn invalid_signature_returns_403_and_does_not_invoke_handler() {
    let hits = HitCounter::default();
    let layer = WebhookLayer::new(KEY).with_url_base("https://example.ngrok.io");
    let app = build_router(layer, hits.clone());

    let req = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header("x-signalwire-signature", "deadbeef0000000000000000000000000000000")
        .body(Body::from(BODY.as_bytes().to_vec()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(hits.count(), 0, "handler must NOT run on invalid signature");
}

#[tokio::test]
async fn missing_signature_header_returns_403() {
    let hits = HitCounter::default();
    let layer = WebhookLayer::new(KEY).with_url_base("https://example.ngrok.io");
    let app = build_router(layer, hits.clone());

    let req = Request::builder()
        .method("POST")
        .uri("/webhook")
        .body(Body::from(BODY.as_bytes().to_vec()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(hits.count(), 0);
}

#[tokio::test]
async fn tampered_body_is_rejected() {
    let hits = HitCounter::default();
    let layer = WebhookLayer::new(KEY).with_url_base("https://example.ngrok.io");
    let app = build_router(layer, hits.clone());

    let tampered = BODY.replace("answered", "ringing");

    let req = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header("x-signalwire-signature", VALID_SIG)
        .body(Body::from(tampered.into_bytes()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(hits.count(), 0);
}

#[tokio::test]
async fn x_twilio_signature_header_is_also_accepted() {
    // Legacy compat — the layer accepts X-Twilio-Signature too.
    let hits = HitCounter::default();
    let layer = WebhookLayer::new(KEY).with_url_base("https://example.ngrok.io");
    let app = build_router(layer, hits.clone());

    let req = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header("x-twilio-signature", VALID_SIG)
        .body(Body::from(BODY.as_bytes().to_vec()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(hits.count(), 1);
}

#[tokio::test]
async fn url_reconstructed_from_x_forwarded_headers_when_no_override() {
    // Without an explicit URL override, the layer reconstructs the
    // URL from X-Forwarded-Proto + X-Forwarded-Host. Sign for the
    // reconstructed URL and assert success.
    use hmac::{Hmac, KeyInit, Mac};
    use sha1::Sha1;
    type HmacSha1 = Hmac<Sha1>;

    let key = "fwd-key";
    let url = "https://tunnel.example.com/webhook";
    let body = r#"{"hello":"world"}"#;
    let mut mac = HmacSha1::new_from_slice(key.as_bytes()).unwrap();
    mac.update(format!("{url}{body}").as_bytes());
    let sig: String = mac
        .finalize()
        .into_bytes()
        .iter()
        .fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        });

    let hits = HitCounter::default();
    let layer = WebhookLayer::new(key);
    let app = build_router(layer, hits.clone());

    let req = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header("x-forwarded-proto", "https")
        .header("x-forwarded-host", "tunnel.example.com")
        .header("x-signalwire-signature", HeaderValue::from_str(&sig).unwrap())
        .body(Body::from(body.as_bytes().to_vec()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(hits.count(), 1);
}

#[tokio::test]
async fn handler_can_re_read_the_buffered_body() {
    // The echo handler reads the body via axum::body::Bytes — proves
    // the layer rebuilt the request with the buffered body before
    // forwarding to the inner service. (If the layer dropped the
    // body, the handler would receive empty bytes and the response
    // would not echo BODY.)
    let hits = HitCounter::default();
    let layer = WebhookLayer::new(KEY).with_url_base("https://example.ngrok.io");
    let app = build_router(layer, hits.clone());

    let req = Request::builder()
        .method("POST")
        .uri("/webhook")
        .header("x-signalwire-signature", VALID_SIG)
        .body(Body::from(BODY.as_bytes().to_vec()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let echoed = read_body(resp).await;
    assert_eq!(
        std::str::from_utf8(&echoed).unwrap(),
        BODY,
        "handler must see the original body bytes"
    );
}
