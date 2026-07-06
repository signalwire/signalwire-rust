//! Mountable `axum::Router` bridge for a SWML [`Service`].
//!
//! Python's `WebMixin.as_router` / `SWMLService.as_router` return a FastAPI
//! `APIRouter` — the "embed my routes in a host app" unit a caller mounts into
//! their own application. The Rust port expresses the identical capability with
//! an [`axum::Router`]: [`build_router`] wraps a service's synchronous
//! [`Service::handle_request`] in an async axum fallback handler so the whole
//! service (SWML render, `/swaig`, `/post_prompt`, `/health`, `/ready`) can be
//! mounted with [`axum::Router::nest`] or served directly by any hyper/axum
//! host. This is gated behind the `tower-middleware` Cargo feature (enabled by
//! default), which pulls in `axum`.
//!
//! Copyright (c) 2025 SignalWire. Licensed under the MIT License.

use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderName, HeaderValue, Request, Response, StatusCode};
use http_body_util::BodyExt;

use crate::swml::service::Service;

/// Build a mountable [`axum::Router`] backed by the given shared [`Service`].
///
/// The router routes every request through [`Service::handle_request`], which
/// already implements the service's full HTTP contract (auth, route matching,
/// SWAIG dispatch, health/ready). The `Arc<Service>` snapshot is captured so
/// the returned router owns its state and satisfies axum's `'static` bound.
pub(crate) fn build_router(service: Arc<Service>) -> axum::Router {
    axum::Router::new().fallback(dispatch).with_state(service)
}

/// Async axum handler that adapts a `Request<Body>` to the service's
/// synchronous `handle_request` and back to a `Response<Body>`.
async fn dispatch(State(service): State<Arc<Service>>, request: Request<Body>) -> Response<Body> {
    let method = request.method().as_str().to_string();
    let path = request.uri().path().to_string();

    let mut headers: HashMap<String, String> = HashMap::new();
    for (name, value) in request.headers() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.as_str().to_string(), v.to_string());
        }
    }

    let body_bytes = match request.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "failed to read request body");
        }
    };
    let body = String::from_utf8_lossy(&body_bytes).into_owned();

    let (status, resp_headers, resp_body) = service.handle_request(&method, &path, &headers, &body);

    // `handle_request` returns the framework-free bare-header triple; the HTTP
    // layer re-adds `Content-Type: application/json` for non-empty bodies
    // (mirrors FastAPI's `media_type="application/json"`), unless the core
    // already set a Content-Type.
    let has_content_type = resp_headers
        .keys()
        .any(|k| k.eq_ignore_ascii_case("content-type"));

    let mut builder = Response::builder()
        .status(StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR));
    for (k, v) in resp_headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::try_from(k.as_str()),
            HeaderValue::try_from(v.as_str()),
        ) {
            builder = builder.header(name, value);
        }
    }
    if !has_content_type && !resp_body.is_empty() {
        builder = builder.header(
            HeaderName::from_static("content-type"),
            HeaderValue::from_static("application/json"),
        );
    }
    builder.body(Body::from(resp_body)).unwrap_or_else(|_| {
        error_response(StatusCode::INTERNAL_SERVER_ERROR, "response build failed")
    })
}

fn error_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(Body::from(message.to_string()))
        .expect("static error response is always valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::swml::service::{Service, ServiceOptions};
    use tower::ServiceExt; // for `oneshot`

    fn test_service() -> Service {
        Service::new(ServiceOptions::new("test").route("/svc"))
    }

    #[tokio::test]
    async fn as_router_serves_health_unauthenticated() {
        let router = test_service().as_router();
        let request = Request::builder()
            .method("GET")
            .uri("/health")
            .body(Body::empty())
            .unwrap();
        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body = String::from_utf8_lossy(&body);
        assert!(body.contains("healthy"), "body was: {body}");
    }

    #[tokio::test]
    async fn as_router_is_mountable_via_nest() {
        // The router is a real mountable handler: nest it under a host app.
        let host = axum::Router::new().nest("/agent", test_service().as_router());
        let request = Request::builder()
            .method("GET")
            .uri("/agent/health")
            .body(Body::empty())
            .unwrap();
        let response = host.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
