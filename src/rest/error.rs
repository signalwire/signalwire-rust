use std::fmt;
use std::sync::Arc;

/// Exception thrown when a SignalWire REST API request fails with a non-2xx
/// status, OR when the request never reached a response at all (a transport
/// failure — connection refused, DNS failure, connection reset, TLS error).
///
/// Carries the full failure envelope — HTTP status, response body, request URL,
/// and request method — so a caller can branch on 400-vs-404-vs-422 and inspect
/// the server's error body. Mirrors the Python reference's `SignalWireRestError`
/// `(status_code, body, url, method)` constructor.
///
/// A transport failure is folded into this SAME type (not a parallel subtype)
/// via the `is_transport()` discriminator: `status_code` is `0` (this port's
/// existing "no status" convention) and `response_body` is empty. This is the
/// equivalent of the Python reference's `SignalWireRestTransportError` subclass
/// with `status_code=None` — mirroring the Go port's `Transport bool` field
/// rather than a parallel type. The underlying transport error is preserved via
/// `source()` (Rust's equivalent of Python's `raise ... from exc`), so
/// `std::error::Error::source()` still sees through to the original cause.
#[derive(Debug, Clone)]
pub struct SignalWireRestError {
    message: String,
    status_code: u16,
    /// The server's response body (HTTP-status errors) or empty (transport
    /// failures, which never receive a body).
    response_body: String,
    url: String,
    method: String,
    /// `true` when this error represents a transport-level failure (the request
    /// never reached a response), in which case `status_code` is `0` — the
    /// equivalent of the Python reference's `status_code=None`. `false` for an
    /// HTTP-status error (a real >= 400 response).
    is_transport: bool,
    /// The underlying transport error (connection refused, DNS, reset, TLS),
    /// preserved so `std::error::Error::source()` sees through to it — the Rust
    /// equivalent of Python's `raise SignalWireRestTransportError(...) from exc`.
    /// `None` for an HTTP-status error.
    source: Option<Arc<dyn std::error::Error + Send + Sync + 'static>>,
}

impl SignalWireRestError {
    /// Construct an HTTP-status error with its full envelope. `url` is the
    /// request path/URL and `method` the HTTP verb (`GET`, `POST`, …) of the
    /// failed request.
    pub fn new(
        message: &str,
        status_code: u16,
        response_body: &str,
        url: &str,
        method: &str,
    ) -> Self {
        SignalWireRestError {
            message: message.to_string(),
            status_code,
            response_body: response_body.to_string(),
            url: url.to_string(),
            method: method.to_string(),
            is_transport: false,
            source: None,
        }
    }

    /// Construct a **transport-level** error — the request never reached a
    /// response (connection refused, DNS failure, connection reset, TLS error).
    /// `status_code` is `0` (this port's existing sentinel for "no HTTP status",
    /// the equivalent of the Python reference's
    /// `SignalWireRestTransportError(status_code=None)`) and `is_transport()`
    /// reports `true`. `cause` is the underlying transport error, preserved as
    /// this error's `source()` so the cause chain survives — the Rust
    /// equivalent of Python's `raise SignalWireRestTransportError(...) from exc`.
    pub fn transport<E>(message: &str, url: &str, method: &str, cause: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        SignalWireRestError {
            message: message.to_string(),
            status_code: 0,
            response_body: String::new(),
            url: url.to_string(),
            method: method.to_string(),
            is_transport: true,
            source: Some(Arc::new(cause)),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// The HTTP status code of the failed request. `0` for a transport failure
    /// (the request never reached a response) — see [`is_transport`](Self::is_transport)
    /// to distinguish that from a real HTTP status of 0.
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// `true` when this error represents a transport-level failure (the request
    /// never reached a response, e.g. connection refused / DNS / reset / TLS),
    /// in which case [`status_code`](Self::status_code) is `0` — the equivalent
    /// of the Python reference's `SignalWireRestTransportError` /
    /// `status_code=None`. `false` for an HTTP-status error (a real >= 400
    /// response).
    pub fn is_transport(&self) -> bool {
        self.is_transport
    }

    pub fn response_body(&self) -> &str {
        &self.response_body
    }

    /// The request URL/path of the failed request.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// The HTTP method of the failed request.
    pub fn method(&self) -> &str {
        &self.method
    }
}

impl fmt::Display for SignalWireRestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_transport {
            write!(
                f,
                "SignalWireRestError: {} (transport failure)",
                self.message
            )?;
        } else {
            write!(
                f,
                "SignalWireRestError: {} (HTTP {}): {}",
                self.message, self.status_code, self.response_body
            )?;
        }
        if !self.method.is_empty() || !self.url.is_empty() {
            write!(f, " [{} {}]", self.method, self.url)?;
        }
        Ok(())
    }
}

impl std::error::Error for SignalWireRestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// Error returned by [`RestClient`](super::RestClient) constructors when a
/// required credential/URL is missing or empty.
///
/// D9-rust: the constructors previously returned `Result<Self, String>` — a
/// stringly-typed error a caller can only `.to_string()` and log. This is the
/// typed replacement: a caller can `match` on WHICH field was missing (and, for
/// [`Self::MissingCredential`], read the env var to set) instead of parsing a
/// message. Implements [`std::error::Error`] so it composes with `?` and
/// `Box<dyn Error>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestClientBuilderError {
    /// A required credential was empty. Carries the field name (`"project_id"` /
    /// `"token"` / `"space"`) and the environment variable that can supply it.
    MissingCredential {
        /// The constructor argument that was empty.
        field: &'static str,
        /// The environment variable a caller may set instead.
        env_var: &'static str,
    },
    /// A required non-credential argument (e.g. `base_url`) was empty.
    MissingField {
        /// The constructor argument that was empty.
        field: &'static str,
    },
}

impl fmt::Display for RestClientBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RestClientBuilderError::MissingCredential { field, env_var } => {
                write!(f, "{field} is required (pass explicitly or set {env_var})")
            }
            RestClientBuilderError::MissingField { field } => {
                write!(f, "{field} is required")
            }
        }
    }
}

impl std::error::Error for RestClientBuilderError {}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let e = SignalWireRestError::new(
            "not found",
            404,
            r#"{"error":"not found"}"#,
            "/api/missing",
            "GET",
        );
        assert_eq!(e.message(), "not found");
        assert_eq!(e.status_code(), 404);
        assert_eq!(e.response_body(), r#"{"error":"not found"}"#);
        assert_eq!(e.url(), "/api/missing");
        assert_eq!(e.method(), "GET");
        assert!(!e.is_transport());
    }

    #[test]
    fn test_display() {
        let e = SignalWireRestError::new("fail", 500, "body", "/api/x", "POST");
        let s = format!("{e}");
        assert!(s.contains("SignalWireRestError"));
        assert!(s.contains("500"));
        assert!(s.contains("fail"));
        assert!(s.contains("body"));
        assert!(s.contains("POST"));
        assert!(s.contains("/api/x"));
    }

    #[test]
    fn test_debug() {
        let e = SignalWireRestError::new("err", 400, "", "/api/x", "GET");
        let dbg = format!("{e:?}");
        assert!(dbg.contains("SignalWireRestError"));
    }

    #[test]
    fn test_clone() {
        let e = SignalWireRestError::new("err", 503, "retry", "/api/x", "PUT");
        let e2 = e.clone();
        assert_eq!(e.status_code(), e2.status_code());
        assert_eq!(e.message(), e2.message());
        assert_eq!(e.url(), e2.url());
        assert_eq!(e.method(), e2.method());
    }

    #[test]
    fn test_error_trait() {
        let e = SignalWireRestError::new("err", 500, "", "/api/x", "GET");
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn test_zero_status() {
        // A real HTTP status of 0 is still an HTTP-status error (is_transport
        // == false), distinct from an actual transport failure.
        let e = SignalWireRestError::new("network error", 0, "", "/api/x", "GET");
        assert_eq!(e.status_code(), 0);
        assert!(!e.is_transport());
    }

    /// A transport failure (connection refused / DNS / reset / TLS — the request
    /// never reached a response) must be a member of the `SignalWireRestError`
    /// family with `status_code` 0 / `is_transport()` true, NOT a bare IO/transport
    /// error. Plan 1.3b.
    #[test]
    fn test_transport_constructor() {
        let cause = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "conn refused");
        let e = SignalWireRestError::transport(
            "GET /api/x failed: conn refused",
            "/api/x",
            "GET",
            cause,
        );
        assert_eq!(e.status_code(), 0);
        assert!(e.is_transport());
        assert_eq!(e.url(), "/api/x");
        assert_eq!(e.method(), "GET");
        assert!(e.message().contains("conn refused"));
    }

    /// The underlying transport error is preserved as `source()` — the Rust
    /// equivalent of Python's `raise ... from exc` — so a caller can inspect the
    /// original cause instead of just a message string.
    #[test]
    fn test_transport_source_preserved() {
        let cause = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "conn refused");
        let e = SignalWireRestError::transport("failed", "/api/x", "GET", cause);
        let source = std::error::Error::source(&e).expect("source must be preserved");
        assert!(source.to_string().contains("conn refused"));
    }

    /// An HTTP-status error has no source (nothing to unwrap to).
    #[test]
    fn test_http_error_has_no_source() {
        let e = SignalWireRestError::new("not found", 404, "", "/api/x", "GET");
        assert!(std::error::Error::source(&e).is_none());
    }

    #[test]
    fn test_transport_display() {
        let cause = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "conn refused");
        let e = SignalWireRestError::transport("GET /api/x failed", "/api/x", "GET", cause);
        let s = format!("{e}");
        assert!(s.contains("transport failure"));
        assert!(!s.contains("HTTP"));
    }
}
