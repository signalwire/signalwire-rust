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

/// The username/password pair carried by an HTTP `Authorization: Basic` header,
/// as handed to [`AuthHandler::verify_basic_auth`].
///
/// The reference declares this parameter as FastAPI's `HTTPBasicCredentials`
/// (`auth_handler.py:98`) and reads `credentials.username` / `credentials.password`
/// off it (`:105`, `:108`). The FIELDS are the contract; which web framework's
/// class carries them is idiom. Rust has no baked-in web framework, so the port
/// declares the carrier itself — a plain struct with the same two fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicCredentials {
    username: String,
    password: String,
}

impl BasicCredentials {
    /// Build a credential pair from a decoded username and password.
    #[must_use]
    pub fn new(username: &str, password: &str) -> Self {
        BasicCredentials {
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    /// The username presented by the client.
    #[must_use]
    pub fn username(&self) -> &str {
        &self.username
    }

    /// The password presented by the client.
    #[must_use]
    pub fn password(&self) -> &str {
        &self.password
    }
}

/// The scheme/credentials pair carried by an HTTP `Authorization` header, as
/// handed to [`AuthHandler::verify_bearer_token`].
///
/// The reference declares this parameter as FastAPI's
/// `HTTPAuthorizationCredentials` (`auth_handler.py:113`) and reads
/// `credentials.credentials` off it (`:119`). FastAPI's `HTTPBearer` splits the
/// raw header on its FIRST space: everything before it is `scheme` (`"Bearer"`),
/// everything after is `credentials` (the token). [`parse_header`] reproduces
/// that split, so `scheme` is genuinely populated rather than discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BearerCredentials {
    scheme: String,
    credentials: String,
}

impl BearerCredentials {
    /// Build an authorization pair from an already-split scheme and token.
    #[must_use]
    pub fn new(scheme: &str, credentials: &str) -> Self {
        BearerCredentials {
            scheme: scheme.to_string(),
            credentials: credentials.to_string(),
        }
    }

    /// Split a raw `Authorization` header value into its scheme and credentials
    /// on the FIRST space, matching FastAPI's `HTTPBearer` (`scheme, _, param =
    /// authorization.partition(" ")`). Returns `None` when the header carries no
    /// space, i.e. no scheme — which FastAPI also rejects.
    ///
    /// Splitting on the first space (rather than stripping a fixed `"Bearer "`
    /// prefix) is what keeps `scheme` populated: a fixed-offset strip discards
    /// the scheme, leaving the field permanently empty.
    #[must_use]
    pub fn parse_header(authorization: &str) -> Option<Self> {
        let (scheme, credentials) = authorization.split_once(' ')?;
        Some(BearerCredentials::new(scheme, credentials))
    }

    /// The authorization scheme presented by the client (e.g. `"Bearer"`).
    #[must_use]
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// The credentials presented by the client (the token itself).
    #[must_use]
    pub fn credentials(&self) -> &str {
        &self.credentials
    }
}

/// Unified authentication handler supporting Basic / Bearer / API-key auth.
pub struct AuthHandler {
    basic_user: String,
    basic_password: String,
    bearer_token: Option<String>,
    api_key: Option<String>,
    api_key_header: String,
    /// The config this handler was built from. The reference keeps its ctor arg
    /// as the public attribute `AuthHandler.security_config`
    /// (`auth_handler.py:63`) and re-reads it on every auth decision
    /// (`:77`, `:85`, `:90`, `:95`), so a caller can inspect the config the
    /// handler is actually enforcing.
    security_config: SecurityConfig,
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
            security_config: security_config.clone(),
        }
    }

    /// The security config this handler was built from
    /// (reference attribute `AuthHandler.security_config`).
    #[must_use]
    pub fn security_config(&self) -> &SecurityConfig {
        &self.security_config
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

    /// Verify Basic-auth credentials (timing-safe). Mirrors the reference's
    /// `verify_basic_auth(credentials)`, which reads `credentials.username` and
    /// `credentials.password` and compares each with `secrets.compare_digest`.
    #[must_use]
    pub fn verify_basic_auth(&self, credentials: &BasicCredentials) -> bool {
        compare_digest(credentials.username(), &self.basic_user)
            && compare_digest(credentials.password(), &self.basic_password)
    }

    /// Verify a Bearer token (timing-safe). Returns false if Bearer auth is not
    /// configured. Mirrors the reference's `verify_bearer_token(credentials)`,
    /// which compares `credentials.credentials` against the configured token.
    #[must_use]
    pub fn verify_bearer_token(&self, credentials: &BearerCredentials) -> bool {
        match &self.bearer_token {
            Some(expected) => compare_digest(credentials.credentials(), expected),
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
    /// `optional` is `Option<bool>` because the reference declares it optional
    /// (`optional: bool = False`); `None` takes `false`.
    pub fn get_fastapi_dependency(
        &self,
        optional: Option<bool>,
    ) -> impl Fn(&HashMap<String, String>) -> bool + '_ {
        let optional = optional.unwrap_or(false);
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

        // Split the header on its FIRST space so the scheme is CARRIED rather
        // than discarded, then match the scheme case-insensitively — exactly
        // what FastAPI's HTTPBearer / HTTPBasic do
        // (`get_authorization_scheme_param` partitions on " ", then each
        // compares `scheme.lower()`). A fixed-prefix strip is wrong twice over:
        // it leaves the scheme field permanently empty, and it rejects the
        // lowercase spelling that RFC 7235 allows.
        if let Some(auth) = get("Authorization")
            && let Some(creds) = BearerCredentials::parse_header(auth)
        {
            // Bearer.
            if creds.scheme().eq_ignore_ascii_case("Bearer") && self.verify_bearer_token(&creds) {
                return true;
            }
            // Basic.
            if creds.scheme().eq_ignore_ascii_case("Basic")
                && let Some((u, p)) = decode_basic(creds.credentials())
                && self.verify_basic_auth(&BasicCredentials::new(&u, &p))
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
        assert!(h.verify_basic_auth(&BasicCredentials::new("admin", "secret")));
        assert!(!h.verify_basic_auth(&BasicCredentials::new("admin", "wrong")));
        assert!(!h.verify_basic_auth(&BasicCredentials::new("bad", "secret")));
    }

    #[test]
    fn test_basic_credentials_fields() {
        let c = BasicCredentials::new("admin", "secret");
        assert_eq!(c.username(), "admin");
        assert_eq!(c.password(), "secret");
    }

    #[test]
    fn test_bearer_disabled_by_default() {
        let h = handler();
        assert!(!h.verify_bearer_token(&BearerCredentials::new("Bearer", "anything")));
    }

    #[test]
    fn test_verify_bearer_token() {
        let h = handler().with_bearer_token("tok-123");
        assert!(h.verify_bearer_token(&BearerCredentials::new("Bearer", "tok-123")));
        assert!(!h.verify_bearer_token(&BearerCredentials::new("Bearer", "tok-999")));
    }

    /// The scheme must be POPULATED from the header, not discarded. A
    /// fixed-offset `strip_prefix("Bearer ")` leaves it permanently empty; the
    /// first-space split (FastAPI `HTTPBearer` semantics) carries it.
    #[test]
    fn test_bearer_credentials_parse_header_populates_scheme() {
        let c = BearerCredentials::parse_header("Bearer tok-123").unwrap();
        assert_eq!(c.scheme(), "Bearer");
        assert_eq!(c.credentials(), "tok-123");

        // A non-Bearer scheme is carried verbatim rather than swallowed.
        let d = BearerCredentials::parse_header("Digest abc").unwrap();
        assert_eq!(d.scheme(), "Digest");
        assert_eq!(d.credentials(), "abc");

        // Only the FIRST space splits — a token containing spaces is preserved.
        let e = BearerCredentials::parse_header("Bearer a b c").unwrap();
        assert_eq!(e.scheme(), "Bearer");
        assert_eq!(e.credentials(), "a b c");

        // No space at all → no scheme → rejected, as FastAPI also rejects it.
        assert!(BearerCredentials::parse_header("Bearertok").is_none());
    }

    /// The auth scheme is case-insensitive per RFC 7235, and FastAPI's
    /// `HTTPBearer` compares `scheme.lower() != "bearer"`. The previous
    /// fixed-prefix `strip_prefix("Bearer ")` was case-SENSITIVE and silently
    /// rejected a valid lowercase `authorization: bearer <tok>`.
    #[test]
    fn test_bearer_scheme_is_case_insensitive() {
        let h = handler().with_bearer_token("tok");
        let guard = h.flask_decorator();
        for raw in ["Bearer tok", "bearer tok", "BEARER tok", "BeArEr tok"] {
            let mut headers = HashMap::new();
            headers.insert("Authorization".to_string(), raw.to_string());
            assert!(guard(&headers), "scheme spelling {raw:?} must authenticate");
        }
    }

    /// The Basic scheme is case-insensitive for the same reason
    /// (FastAPI's `HTTPBasic` compares `scheme.lower() != "basic"`).
    #[test]
    fn test_basic_scheme_is_case_insensitive() {
        let h = handler();
        let guard = h.flask_decorator();
        let creds = STANDARD.encode(b"admin:secret");
        for scheme in ["Basic", "basic", "BASIC"] {
            let mut headers = HashMap::new();
            headers.insert("Authorization".to_string(), format!("{scheme} {creds}"));
            assert!(
                guard(&headers),
                "scheme spelling {scheme:?} must authenticate"
            );
        }
    }

    /// A correct token under the WRONG scheme must not authenticate.
    #[test]
    fn test_wrong_scheme_rejected() {
        let h = handler().with_bearer_token("tok");
        let guard = h.flask_decorator();
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Digest tok".to_string());
        assert!(!guard(&headers));
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
        let dep = h.get_fastapi_dependency(Some(true));
        let empty = HashMap::new();
        assert!(dep(&empty)); // optional + no credential → allow
        let required = h.get_fastapi_dependency(None);
        assert!(!required(&empty)); // required + no credential → deny
    }
}
