//! Standalone security hygiene utilities.
//!
//! Mirrors the Python reference's `signalwire.core.security.security_utils`
//! (which itself mirrors the TypeScript SDK's `SecurityUtils`:
//! `filterSensitiveHeaders`, `redactUrl`, `isValidHostname`). The same
//! protections — keeping credentials out of user callbacks and logs, and a
//! reusable hostname sanity check — are available here as idiomatic Rust free
//! functions (`snake_case`, matching the Python free-function shape).

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

/// Header names whose values are credentials/secrets and must never be handed
/// to user callbacks or written to logs. Compared case-insensitively (entries
/// are stored lower-case; the lookup lower-cases the candidate key).
///
/// Internal — the Python reference does not expose `SENSITIVE_HEADERS` on its
/// public surface, so neither do we.
const SENSITIVE_HEADERS: [&str; 5] = [
    "authorization",
    "cookie",
    "x-api-key",
    "proxy-authorization",
    "set-cookie",
];

/// URL credentials: `://user:secret@host` -> `://user:****@host`.
/// Mirrors the Python regex `://([^:@/]+):([^@/]+)@`.
static URL_CREDENTIALS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"://([^:@/]+):([^@/]+)@").expect("static url-credentials regex"));

/// Hostnames must not contain whitespace, slashes, backslashes, or control
/// characters. Mirrors the Python char class `[\s/\\\x00-\x1f\x7f]`.
static HOSTNAME_REJECT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\s/\\\x00-\x1f\x7f]").expect("static hostname-reject regex"));

/// Return a copy of `headers` with sensitive (credential-bearing) headers
/// removed, so request headers can be safely passed to user callbacks or
/// written to logs.
///
/// The sensitivity check is case-insensitive; non-sensitive keys are preserved
/// with their original casing. An empty input yields an empty map.
///
/// `clippy::implicit_hasher`: the concrete `HashMap<String, String>` mirrors
/// Python's `dict[str, str]` header map for parity; generalizing over
/// `BuildHasher` would distort the parity signature the audit maps to Python's
/// plain map (same rationale as `RestClient`'s `**kwargs` map — see
/// `PORT_PHILOSOPHY_RUST.md`, the `implicit_hasher` row).
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn filter_sensitive_headers(headers: &HashMap<String, String>) -> HashMap<String, String> {
    headers
        .iter()
        .filter(|(k, _)| !SENSITIVE_HEADERS.contains(&k.to_ascii_lowercase().as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// Mask the password in a URL's userinfo before logging.
///
/// `https://user:secret@host/path` -> `https://user:****@host/path`. A URL with
/// no embedded credentials is returned unchanged.
#[must_use]
pub fn redact_url(url: &str) -> String {
    URL_CREDENTIALS_RE
        .replace_all(url, "://$1:****@")
        .into_owned()
}

/// Standalone hostname sanity check: reject an empty host and any host
/// containing whitespace, slashes, backslashes, or control characters.
///
/// This is the reusable character-level check, independent of the fuller
/// [`crate::utils::validate_url`] (which also does scheme checks, DNS
/// resolution, and private-IP blocking). Callers that only need to validate a
/// hostname string use this.
#[must_use]
pub fn is_valid_hostname(host: &str) -> bool {
    if host.is_empty() {
        return false;
    }
    !HOSTNAME_REJECT_RE.is_match(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn filter_sensitive_headers_removes_credential_headers() {
        let headers = map(&[
            ("Authorization", "Bearer secret"),
            ("Cookie", "session=abc"),
            ("X-API-Key", "key123"),
            ("Proxy-Authorization", "Basic xxx"),
            ("Set-Cookie", "a=b"),
            ("Content-Type", "application/json"),
            ("X-Request-Id", "req-1"),
        ]);
        let filtered = filter_sensitive_headers(&headers);

        // All five sensitive headers removed (case-insensitive match).
        assert_eq!(filtered.len(), 2);
        assert!(!filtered.contains_key("Authorization"));
        assert!(!filtered.contains_key("Cookie"));
        assert!(!filtered.contains_key("X-API-Key"));
        assert!(!filtered.contains_key("Proxy-Authorization"));
        assert!(!filtered.contains_key("Set-Cookie"));

        // Non-sensitive keys preserved with original casing + value.
        assert_eq!(
            filtered.get("Content-Type").map(String::as_str),
            Some("application/json")
        );
        assert_eq!(
            filtered.get("X-Request-Id").map(String::as_str),
            Some("req-1")
        );
    }

    #[test]
    fn filter_sensitive_headers_is_case_insensitive_on_keys() {
        // Lower- and mixed-case variants of sensitive keys are all stripped.
        let headers = map(&[
            ("authorization", "x"),
            ("AUTHORIZATION", "y"),
            ("CooKie", "z"),
            ("keep-me", "ok"),
        ]);
        let filtered = filter_sensitive_headers(&headers);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered.get("keep-me").map(String::as_str), Some("ok"));
    }

    #[test]
    fn filter_sensitive_headers_empty_input_yields_empty_map() {
        let empty: HashMap<String, String> = HashMap::new();
        assert!(filter_sensitive_headers(&empty).is_empty());
    }

    #[test]
    fn redact_url_masks_password() {
        assert_eq!(
            redact_url("https://user:secret@host/path"),
            "https://user:****@host/path"
        );
    }

    #[test]
    fn redact_url_masks_password_with_special_chars() {
        // The password may contain anything except '@' and '/'.
        assert_eq!(
            redact_url("postgres://admin:p4ss-w0rd.!@db.example.com:5432/app"),
            "postgres://admin:****@db.example.com:5432/app"
        );
    }

    #[test]
    fn redact_url_without_credentials_is_unchanged() {
        let plain = "https://host/path?query=1";
        assert_eq!(redact_url(plain), plain);
        // userinfo with a username but no password is also left alone.
        assert_eq!(
            redact_url("https://user@host/path"),
            "https://user@host/path"
        );
    }

    #[test]
    fn is_valid_hostname_accepts_plain_hosts() {
        assert!(is_valid_hostname("example.com"));
        assert!(is_valid_hostname("sub.domain.example.com"));
        assert!(is_valid_hostname("localhost"));
        assert!(is_valid_hostname("192.168.1.1"));
    }

    #[test]
    fn is_valid_hostname_rejects_empty() {
        assert!(!is_valid_hostname(""));
    }

    #[test]
    fn is_valid_hostname_rejects_whitespace_slashes_and_control_chars() {
        assert!(!is_valid_hostname("bad host"));
        assert!(!is_valid_hostname("host/path"));
        assert!(!is_valid_hostname("host\\path"));
        assert!(!is_valid_hostname("host\tname"));
        assert!(!is_valid_hostname("host\nname"));
        assert!(!is_valid_hostname("host\u{0000}name"));
        assert!(!is_valid_hostname("host\u{007f}name"));
    }
}
