use std::fmt;

/// Exception thrown when a SignalWire REST API request fails with a non-2xx status.
///
/// Carries the full failure envelope — HTTP status, response body, request URL,
/// and request method — so a caller can branch on 400-vs-404-vs-422 and inspect
/// the server's error body. Mirrors the Python reference's `SignalWireRestError`
/// `(status_code, body, url, method)` constructor.
#[derive(Debug, Clone)]
pub struct SignalWireRestError {
    message: String,
    status_code: u16,
    response_body: String,
    url: String,
    method: String,
}

impl SignalWireRestError {
    /// Construct the error with its full envelope. `url` is the request path/URL
    /// and `method` the HTTP verb (`GET`, `POST`, …) of the failed request.
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
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn status_code(&self) -> u16 {
        self.status_code
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
        write!(
            f,
            "SignalWireRestError: {} (HTTP {}): {}",
            self.message, self.status_code, self.response_body
        )?;
        if !self.method.is_empty() || !self.url.is_empty() {
            write!(f, " [{} {}]", self.method, self.url)?;
        }
        Ok(())
    }
}

impl std::error::Error for SignalWireRestError {}

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
        let e = SignalWireRestError::new("network error", 0, "", "/api/x", "GET");
        assert_eq!(e.status_code(), 0);
    }
}
