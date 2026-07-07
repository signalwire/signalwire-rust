//! Unified authentication handler.
//!
//! Port of Python `signalwire.core.auth_handler.AuthHandler`. Supports Basic
//! Auth, Bearer tokens, and API keys across SignalWire services. Python wires
//! this into FastAPI (`get_fastapi_dependency`) and Flask (`flask_decorator`);
//! the Rust port has no baked-in web framework, so those two entry points are
//! the Rust-idiom equivalent: a returned guard closure that inspects a header
//! map (`HashMap<String, String>`) and reports whether the request is
//! authenticated. The core verify methods are framework-agnostic and portable
//! 1:1.

use std::collections::HashMap;

use serde_json::{Value, json};
use subtle::ConstantTimeEq;

use crate::core::security_config::SecurityConfig;

/// Constant-time string comparison (timing-safe), matching Python's
/// `secrets.compare_digest`.
fn compare_digest(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}

/// Unified authentication handler supporting Basic / Bearer / API-key auth.
pub struct AuthHandler {
    basic_user: String,
    basic_password: String,
    bearer_token: Option<String>,
    api_key: Option<String>,
    api_key_header: String,
}

impl AuthHandler {
    /// Initialize an auth handler from a [`SecurityConfig`].
    ///
    /// Basic auth is always enabled (backward compatibility); Bearer and API
    /// key are enabled when `bearer_token` / `api_key` are supplied. Python
    #[must_use]
    pub fn new(security_config: &mut SecurityConfig) -> Self {
        let (user, password) = security_config.get_basic_auth();
        AuthHandler {
            basic_user: user,
            basic_password: password,
            bearer_token: None,
            api_key: None,
            api_key_header: "X-API-Key".to_string(),
        }
    }

    /// Enable Bearer-token auth with the given token.
    #[must_use]
    pub fn with_bearer_token(mut self, token: &str) -> Self {
        self.bearer_token = Some(token.to_string());
        self
    }

    /// Enable API-key auth with the given key and header name.
    #[must_use]
    pub fn with_api_key(mut self, key: &str, header: &str) -> Self {
        self.api_key = Some(key.to_string());
        self.api_key_header = header.to_string();
        self
    }

    /// Verify Basic-auth credentials (timing-safe).
    #[must_use]
    pub fn verify_basic_auth(&self, username: &str, password: &str) -> bool {
        compare_digest(username, &self.basic_user) && compare_digest(password, &self.basic_password)
    }

    /// Verify a Bearer token (timing-safe). Returns false if Bearer auth is not
    /// configured.
    #[must_use]
    pub fn verify_bearer_token(&self, token: &str) -> bool {
        match &self.bearer_token {
            Some(expected) => compare_digest(token, expected),
            None => false,
        }
    }

    /// Verify an API key (timing-safe). Returns false if API-key auth is not
    /// configured.
    #[must_use]
    pub fn verify_api_key(&self, api_key: &str) -> bool {
        match &self.api_key {
            Some(expected) => compare_digest(api_key, expected),
            None => false,
        }
    }

    /// Rust-idiom equivalent of Python's `get_fastapi_dependency`: returns a
    /// guard closure that inspects a request's headers and reports whether the
    /// request is authenticated by any enabled method. `optional` makes an
    /// un-credentialed request pass (matching FastAPI's optional dependency).
    pub fn get_fastapi_dependency(
        &self,
        optional: bool,
    ) -> impl Fn(&HashMap<String, String>) -> bool + '_ {
        move |headers| {
            if self.authenticate_headers(headers) {
                return true;
            }
            // No credential present at all + optional → allow.
            optional && !Self::has_any_credential(headers, &self.api_key_header)
        }
    }

    /// Rust-idiom equivalent of Python's `flask_decorator`: returns a guard
    /// closure that authenticates a request's header map. In Python this wraps
    /// a view function; in Rust a caller applies the returned guard in its
    /// handler (no decorator syntax).
    pub fn flask_decorator(&self) -> impl Fn(&HashMap<String, String>) -> bool + '_ {
        move |headers| self.authenticate_headers(headers)
    }

    /// Get information about the configured auth methods.
    #[must_use]
    pub fn get_auth_info(&self) -> Value {
        let mut info = serde_json::Map::new();
        info.insert(
            "basic".to_string(),
            json!({"enabled": true, "username": self.basic_user}),
        );
        if self.bearer_token.is_some() {
            info.insert(
                "bearer".to_string(),
                json!({"enabled": true, "hint": "Use Authorization: Bearer <token>"}),
            );
        }
        if self.api_key.is_some() {
            info.insert(
                "api_key".to_string(),
                json!({
                    "enabled": true,
                    "header": self.api_key_header,
                    "hint": format!("Use {}: <key>", self.api_key_header),
                }),
            );
        }
        Value::Object(info)
    }

    // ── Private helpers ──────────────────────────────────────────────────

    /// Authenticate a request by any enabled method (Bearer → API key →
    /// Basic), mirroring `flask_decorator`'s precedence.
    fn authenticate_headers(&self, headers: &HashMap<String, String>) -> bool {
        let get = |name: &str| -> Option<&String> {
            headers
                .get(name)
                .or_else(|| headers.get(&name.to_lowercase()))
        };

        // Bearer.
        if let Some(auth) = get("Authorization") {
            if let Some(token) = auth.strip_prefix("Bearer ")
                && self.verify_bearer_token(token)
            {
                return true;
            }
            // Basic.
            if let Some(b64) = auth.strip_prefix("Basic ")
                && let Some((u, p)) = decode_basic(b64)
                && self.verify_basic_auth(&u, &p)
            {
                return true;
            }
        }
        // API key.
        if let Some(key) = get(&self.api_key_header)
            && self.verify_api_key(key)
        {
            return true;
        }
        false
    }

    fn has_any_credential(headers: &HashMap<String, String>, api_key_header: &str) -> bool {
        let get = |name: &str| -> bool {
            headers.contains_key(name) || headers.contains_key(&name.to_lowercase())
        };
        get("Authorization") || get(api_key_header)
    }
}

fn decode_basic(b64: &str) -> Option<(String, String)> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    let decoded = STANDARD.decode(b64).ok()?;
    let s = String::from_utf8(decoded).ok()?;
    let (u, p) = s.split_once(':')?;
    Some((u.to_string(), p.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;

    fn handler() -> AuthHandler {
        let mut cfg = SecurityConfig::default();
        cfg.basic_auth_user = Some("admin".to_string());
        cfg.basic_auth_password = Some("secret".to_string());
        AuthHandler::new(&mut cfg)
    }

    #[test]
    fn test_verify_basic_auth() {
        let h = handler();
        assert!(h.verify_basic_auth("admin", "secret"));
        assert!(!h.verify_basic_auth("admin", "wrong"));
        assert!(!h.verify_basic_auth("bad", "secret"));
    }

    #[test]
    fn test_bearer_disabled_by_default() {
        let h = handler();
        assert!(!h.verify_bearer_token("anything"));
    }

    #[test]
    fn test_verify_bearer_token() {
        let h = handler().with_bearer_token("tok-123");
        assert!(h.verify_bearer_token("tok-123"));
        assert!(!h.verify_bearer_token("tok-999"));
    }

    #[test]
    fn test_verify_api_key() {
        let h = handler().with_api_key("key-abc", "X-API-Key");
        assert!(h.verify_api_key("key-abc"));
        assert!(!h.verify_api_key("key-xyz"));
    }

    #[test]
    fn test_api_key_disabled_by_default() {
        let h = handler();
        assert!(!h.verify_api_key("anything"));
    }

    #[test]
    fn test_get_auth_info_basic_only() {
        let h = handler();
        let info = h.get_auth_info();
        assert_eq!(info["basic"]["enabled"], true);
        assert_eq!(info["basic"]["username"], "admin");
        assert!(info.get("bearer").is_none());
        assert!(info.get("api_key").is_none());
    }

    #[test]
    fn test_get_auth_info_all_methods() {
        let h = handler()
            .with_bearer_token("t")
            .with_api_key("k", "X-Custom-Key");
        let info = h.get_auth_info();
        assert_eq!(info["bearer"]["enabled"], true);
        assert_eq!(info["api_key"]["header"], "X-Custom-Key");
        assert!(
            info["api_key"]["hint"]
                .as_str()
                .unwrap()
                .contains("X-Custom-Key")
        );
    }

    #[test]
    fn test_flask_decorator_guard_basic() {
        let h = handler();
        let guard = h.flask_decorator();
        let mut headers = HashMap::new();
        let creds = STANDARD.encode(b"admin:secret");
        headers.insert("Authorization".to_string(), format!("Basic {creds}"));
        assert!(guard(&headers));
        let mut bad = HashMap::new();
        bad.insert(
            "Authorization".to_string(),
            format!("Basic {}", STANDARD.encode(b"admin:wrong")),
        );
        assert!(!guard(&bad));
    }

    #[test]
    fn test_flask_decorator_guard_bearer() {
        let h = handler().with_bearer_token("tok");
        let guard = h.flask_decorator();
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer tok".to_string());
        assert!(guard(&headers));
    }

    #[test]
    fn test_fastapi_dependency_optional_allows_no_credentials() {
        let h = handler();
        let dep = h.get_fastapi_dependency(true);
        let empty = HashMap::new();
        assert!(dep(&empty)); // optional + no credential → allow
        let required = h.get_fastapi_dependency(false);
        assert!(!required(&empty)); // required + no credential → deny
    }
}
