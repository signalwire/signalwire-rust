//! Unified security configuration for SignalWire services.
//!
//! Rust port of Python's `signalwire.core.security_config.SecurityConfig`.
//! Centralizes SSL, CORS, allowed-host, HSTS, and basic-auth settings, loaded
//! from `SWML_*` environment variables. Configures: `__init__`,
//! `get_basic_auth`, `get_cors_config`, `get_security_headers`,
//! `get_ssl_context_kwargs`, `get_url_scheme`, `load_from_env`, `log_config`,
//! `should_allow_host`, `validate_ssl_config`.

use std::collections::HashMap;
use std::env;
use std::path::Path;

use serde_json::{Value, json};

const SSL_ENABLED: &str = "SWML_SSL_ENABLED";
const SSL_CERT_PATH: &str = "SWML_SSL_CERT_PATH";
const SSL_KEY_PATH: &str = "SWML_SSL_KEY_PATH";
const SSL_DOMAIN: &str = "SWML_DOMAIN";
const SSL_VERIFY_MODE: &str = "SWML_SSL_VERIFY_MODE";
const ALLOWED_HOSTS: &str = "SWML_ALLOWED_HOSTS";
const CORS_ORIGINS: &str = "SWML_CORS_ORIGINS";
const MAX_REQUEST_SIZE: &str = "SWML_MAX_REQUEST_SIZE";
const RATE_LIMIT: &str = "SWML_RATE_LIMIT";
const REQUEST_TIMEOUT: &str = "SWML_REQUEST_TIMEOUT";
const USE_HSTS: &str = "SWML_USE_HSTS";
const HSTS_MAX_AGE: &str = "SWML_HSTS_MAX_AGE";
const BASIC_AUTH_USER: &str = "SWML_BASIC_AUTH_USER";
const BASIC_AUTH_PASSWORD: &str = "SWML_BASIC_AUTH_PASSWORD";

const DEFAULT_SSL_VERIFY_MODE: &str = "CERT_REQUIRED";
const DEFAULT_MAX_REQUEST_SIZE: i64 = 10 * 1024 * 1024;
const DEFAULT_RATE_LIMIT: i64 = 60;
const DEFAULT_REQUEST_TIMEOUT: i64 = 30;
const DEFAULT_HSTS_MAX_AGE: i64 = 31_536_000;

/// Centralized, secure-by-default security configuration.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    pub ssl_enabled: bool,
    pub ssl_cert_path: Option<String>,
    pub ssl_key_path: Option<String>,
    pub domain: Option<String>,
    pub ssl_verify_mode: String,
    pub allowed_hosts: Vec<String>,
    pub cors_origins: Vec<String>,
    pub max_request_size: i64,
    pub rate_limit: i64,
    pub request_timeout: i64,
    pub use_hsts: bool,
    pub hsts_max_age: i64,
    pub basic_auth_user: Option<String>,
    pub basic_auth_password: Option<String>,
    basic_auth_autogen_warned: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        SecurityConfig {
            ssl_enabled: false,
            ssl_cert_path: None,
            ssl_key_path: None,
            domain: None,
            ssl_verify_mode: DEFAULT_SSL_VERIFY_MODE.to_string(),
            allowed_hosts: vec!["*".to_string()],
            cors_origins: vec!["*".to_string()],
            max_request_size: DEFAULT_MAX_REQUEST_SIZE,
            rate_limit: DEFAULT_RATE_LIMIT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            use_hsts: true,
            hsts_max_age: DEFAULT_HSTS_MAX_AGE,
            basic_auth_user: None,
            basic_auth_password: None,
            basic_auth_autogen_warned: false,
        }
    }
}

impl SecurityConfig {
    /// Build a config (secure-by-default) and immediately load `SWML_*`
    /// environment overrides — mirrors Python's constructor which loads env
    /// after applying defaults.
    #[must_use]
    pub fn new() -> Self {
        let mut cfg = SecurityConfig::default();
        cfg.load_from_env();
        cfg
    }

    /// Load configuration from `SWML_*` environment variables (overwriting the
    /// current values). Missing vars fall back to the secure defaults.
    pub fn load_from_env(&mut self) {
        let truthy = |v: &str| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes");

        self.ssl_enabled = env::var(SSL_ENABLED).is_ok_and(|v| truthy(&v));
        self.ssl_cert_path = env::var(SSL_CERT_PATH).ok();
        self.ssl_key_path = env::var(SSL_KEY_PATH).ok();
        self.domain = env::var(SSL_DOMAIN).ok();
        self.ssl_verify_mode =
            env::var(SSL_VERIFY_MODE).unwrap_or_else(|_| DEFAULT_SSL_VERIFY_MODE.to_string());

        self.allowed_hosts = parse_list(&env::var(ALLOWED_HOSTS).unwrap_or_else(|_| "*".into()));
        self.cors_origins = parse_list(&env::var(CORS_ORIGINS).unwrap_or_else(|_| "*".into()));
        self.max_request_size = env_int(MAX_REQUEST_SIZE, DEFAULT_MAX_REQUEST_SIZE);
        self.rate_limit = env_int(RATE_LIMIT, DEFAULT_RATE_LIMIT);
        self.request_timeout = env_int(REQUEST_TIMEOUT, DEFAULT_REQUEST_TIMEOUT);

        self.use_hsts = match env::var(USE_HSTS) {
            Ok(v) if !v.is_empty() => !v.eq_ignore_ascii_case("false"),
            _ => true,
        };
        self.hsts_max_age = env_int(HSTS_MAX_AGE, DEFAULT_HSTS_MAX_AGE);

        self.basic_auth_user = env::var(BASIC_AUTH_USER).ok();
        self.basic_auth_password = env::var(BASIC_AUTH_PASSWORD).ok();
    }

    /// Validate SSL configuration. Returns `(is_valid, error_message)`.
    #[must_use]
    pub fn validate_ssl_config(&self) -> (bool, Option<String>) {
        if !self.ssl_enabled {
            return (true, None);
        }
        let Some(cert) = &self.ssl_cert_path else {
            return (
                false,
                Some("SSL enabled but SWML_SSL_CERT_PATH not set".into()),
            );
        };
        let Some(key) = &self.ssl_key_path else {
            return (
                false,
                Some("SSL enabled but SWML_SSL_KEY_PATH not set".into()),
            );
        };
        if !Path::new(cert).exists() {
            return (
                false,
                Some(format!("SSL certificate file not found: {cert}")),
            );
        }
        if !Path::new(key).exists() {
            return (false, Some(format!("SSL key file not found: {key}")));
        }
        (true, None)
    }

    /// SSL context kwargs for the HTTP server (cert/key file paths). Empty when
    /// SSL is disabled or the config is invalid.
    #[must_use]
    pub fn get_ssl_context_kwargs(&self) -> Value {
        if !self.ssl_enabled {
            return json!({});
        }
        let (valid, _err) = self.validate_ssl_config();
        if !valid {
            return json!({});
        }
        json!({
            "ssl_certfile": self.ssl_cert_path,
            "ssl_keyfile": self.ssl_key_path,
        })
    }

    /// Get basic-auth credentials, auto-generating a random password when none
    /// is configured (mirrors Python's `secrets.token_urlsafe` fallback). The
    /// generated password lives only in this process.
    pub fn get_basic_auth(&mut self) -> (String, String) {
        let username = self
            .basic_auth_user
            .clone()
            .unwrap_or_else(|| "signalwire".to_string());
        let password = if let Some(p) = &self.basic_auth_password {
            p.clone()
        } else {
            let p = generate_url_safe_token(32);
            self.basic_auth_password = Some(p.clone());
            if !self.basic_auth_autogen_warned {
                self.basic_auth_autogen_warned = true;
                // Parity with Python's warning that the auto-generated
                // password will 401 external callers — AND print the generated
                // credentials once so a developer can actually authenticate on
                // the first run (r5 deep_dogfood: rust never surfaced the
                // generated password, so authed endpoints were unreachable).
                log::warn!(
                    "basic_auth_password_autogenerated. Generated basic-auth \
                     credentials for this run: username={username} password={p}. \
                     Use these to authenticate, or set SWML_BASIC_AUTH_USER / \
                     SWML_BASIC_AUTH_PASSWORD to pin stable credentials and \
                     suppress this message."
                );
            }
            p
        };
        (username, password)
    }

    /// Security headers to add to responses (HSTS added when HTTPS + enabled).
    #[must_use]
    pub fn get_security_headers(&self, is_https: bool) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("X-Content-Type-Options".into(), "nosniff".into());
        headers.insert("X-Frame-Options".into(), "DENY".into());
        headers.insert("X-XSS-Protection".into(), "1; mode=block".into());
        headers.insert(
            "Referrer-Policy".into(),
            "strict-origin-when-cross-origin".into(),
        );
        if is_https && self.use_hsts {
            headers.insert(
                "Strict-Transport-Security".into(),
                format!("max-age={}; includeSubDomains", self.hsts_max_age),
            );
        }
        headers
    }

    /// Whether a host is allowed (a `*` entry allows all).
    #[must_use]
    pub fn should_allow_host(&self, host: &str) -> bool {
        self.allowed_hosts.iter().any(|h| h == "*") || self.allowed_hosts.iter().any(|h| h == host)
    }

    /// CORS configuration object for the HTTP server.
    #[must_use]
    pub fn get_cors_config(&self) -> Value {
        json!({
            "allow_origins": self.cors_origins,
            "allow_credentials": true,
            "allow_methods": ["*"],
            "allow_headers": ["*"],
        })
    }

    /// The URL scheme implied by the SSL configuration (`https`/`http`).
    #[must_use]
    pub fn get_url_scheme(&self) -> &'static str {
        if self.ssl_enabled { "https" } else { "http" }
    }

    /// Log the current security configuration for a service.
    pub fn log_config(&self, service_name: &str) {
        log::info!(
            "security_config_loaded service={service_name} ssl_enabled={} allowed_hosts={:?} \
             cors_origins={:?} rate_limit={} use_hsts={} has_basic_auth={}",
            self.ssl_enabled,
            self.allowed_hosts,
            self.cors_origins,
            self.rate_limit,
            self.use_hsts,
            self.basic_auth_user.is_some() && self.basic_auth_password.is_some(),
        );
    }
}

fn parse_list(value: &str) -> Vec<String> {
    if value == "*" {
        return vec!["*".to_string()];
    }
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn env_int(var: &str, default: i64) -> i64 {
    env::var(var)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(default)
}

/// Generate a URL-safe random token of `n_bytes` entropy, base64url-encoded.
fn generate_url_safe_token(n_bytes: usize) -> String {
    use base64::Engine as _;
    use rand::RngExt;
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..n_bytes).map(|_| rng.random::<u8>()).collect();
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_secure() {
        let cfg = SecurityConfig::default();
        assert!(!cfg.ssl_enabled);
        assert_eq!(cfg.allowed_hosts, vec!["*"]);
        assert_eq!(cfg.get_url_scheme(), "http");
        assert!(cfg.use_hsts);
    }

    #[test]
    fn test_validate_ssl_disabled_ok() {
        let cfg = SecurityConfig::default();
        assert_eq!(cfg.validate_ssl_config(), (true, None));
        assert_eq!(cfg.get_ssl_context_kwargs(), json!({}));
    }

    #[test]
    fn test_validate_ssl_missing_cert() {
        let cfg = SecurityConfig {
            ssl_enabled: true,
            ..SecurityConfig::default()
        };
        let (valid, err) = cfg.validate_ssl_config();
        assert!(!valid);
        assert!(err.unwrap().contains("SWML_SSL_CERT_PATH"));
    }

    #[test]
    fn test_should_allow_host() {
        let mut cfg = SecurityConfig::default();
        assert!(cfg.should_allow_host("anything.com"));
        cfg.allowed_hosts = vec!["a.com".into(), "b.com".into()];
        assert!(cfg.should_allow_host("a.com"));
        assert!(!cfg.should_allow_host("c.com"));
    }

    #[test]
    fn test_security_headers_hsts() {
        let cfg = SecurityConfig::default();
        let plain = cfg.get_security_headers(false);
        assert_eq!(
            plain.get("X-Frame-Options").map(String::as_str),
            Some("DENY")
        );
        assert!(!plain.contains_key("Strict-Transport-Security"));
        let https = cfg.get_security_headers(true);
        assert!(https.contains_key("Strict-Transport-Security"));
    }

    #[test]
    fn test_get_basic_auth_autogen() {
        let mut cfg = SecurityConfig::default();
        let (user, pass) = cfg.get_basic_auth();
        assert_eq!(user, "signalwire");
        assert!(!pass.is_empty());
        // Second call returns the same (now-cached) password.
        let (_u2, pass2) = cfg.get_basic_auth();
        assert_eq!(pass, pass2);
    }

    #[test]
    fn test_cors_config() {
        let cfg = SecurityConfig::default();
        let cors = cfg.get_cors_config();
        assert_eq!(cors["allow_credentials"], json!(true));
        assert_eq!(cors["allow_origins"], json!(["*"]));
    }

    #[test]
    fn test_parse_list() {
        assert_eq!(parse_list("*"), vec!["*"]);
        assert_eq!(parse_list("a.com, b.com"), vec!["a.com", "b.com"]);
        assert_eq!(parse_list(""), Vec::<String>::new());
    }
}
