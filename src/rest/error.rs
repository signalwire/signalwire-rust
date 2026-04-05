use std::fmt;

/// Exception thrown when a SignalWire REST API request fails with a non-2xx status.
#[derive(Debug, Clone)]
pub struct SignalWireRestError {
    message: String,
    status_code: u16,
    response_body: String,
}

impl SignalWireRestError {
    pub fn new(message: &str, status_code: u16, response_body: &str) -> Self {
        SignalWireRestError {
            message: message.to_string(),
            status_code,
            response_body: response_body.to_string(),
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
}

impl fmt::Display for SignalWireRestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SignalWireRestError: {} (HTTP {}): {}",
            self.message, self.status_code, self.response_body
        )
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
        let e = SignalWireRestError::new("not found", 404, r#"{"error":"not found"}"#);
        assert_eq!(e.message(), "not found");
        assert_eq!(e.status_code(), 404);
        assert_eq!(e.response_body(), r#"{"error":"not found"}"#);
    }

    #[test]
    fn test_display() {
        let e = SignalWireRestError::new("fail", 500, "body");
        let s = format!("{}", e);
        assert!(s.contains("SignalWireRestError"));
        assert!(s.contains("500"));
        assert!(s.contains("fail"));
        assert!(s.contains("body"));
    }

    #[test]
    fn test_debug() {
        let e = SignalWireRestError::new("err", 400, "");
        let dbg = format!("{:?}", e);
        assert!(dbg.contains("SignalWireRestError"));
    }

    #[test]
    fn test_clone() {
        let e = SignalWireRestError::new("err", 503, "retry");
        let e2 = e.clone();
        assert_eq!(e.status_code(), e2.status_code());
        assert_eq!(e.message(), e2.message());
    }

    #[test]
    fn test_error_trait() {
        let e = SignalWireRestError::new("err", 500, "");
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn test_zero_status() {
        let e = SignalWireRestError::new("network error", 0, "");
        assert_eq!(e.status_code(), 0);
    }
}
