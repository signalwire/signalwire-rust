// Copyright (c) 2025 SignalWire
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! SSRF-prevention guard for user-supplied URLs.
//!
//! Mirrors Python's `signalwire.utils.url_validator.validate_url`:
//! rejects non-http(s) schemes, missing hostnames, and any URL whose
//! hostname resolves to a private / loopback / link-local / cloud-
//! metadata IP. When `allow_private` is true, OR the
//! `SWML_ALLOW_PRIVATE_URLS` env var is set to `"1"`, `"true"`, or
//! `"yes"` (case-insensitive), the IP-blocklist check is skipped.

use std::env;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::sync::Mutex;

use url::Url;

use crate::logging::Logger;

/// Cross-port SSRF block list. Order matches the Python reference for
/// ease of cross-language review.
pub const BLOCKED_NETWORKS: [&str; 9] = [
    "10.0.0.0/8",
    "172.16.0.0/12",
    "192.168.0.0/16",
    "127.0.0.0/8",
    "169.254.0.0/16", // link-local / cloud metadata
    "0.0.0.0/8",
    "::1/128",
    "fc00::/7",  // IPv6 private (ULA)
    "fe80::/10", // IPv6 link-local
];

/// Pluggable resolver hook. Tests inject a closure to keep the suite
/// hermetic; production resolves via [`std::net::ToSocketAddrs`].
type ResolverFn = Box<dyn Fn(&str) -> Option<Vec<IpAddr>> + Send + Sync>;

static RESOLVER: Mutex<Option<ResolverFn>> = Mutex::new(None);

/// Install a custom resolver (for tests). Pass `None` to clear. `_`-prefixed
/// deliberately — a test-only seam, not part of the public API surface.
///
/// # Panics
///
/// Panics if the internal `RESOLVER` lock is poisoned (another thread
/// panicked while holding it). This does not occur under normal operation.
pub fn _set_resolver(resolver: Option<ResolverFn>) {
    *RESOLVER.lock().unwrap() = resolver;
}

fn resolve(hostname: &str) -> Option<Vec<IpAddr>> {
    if let Some(ref r) = *RESOLVER.lock().unwrap() {
        return r(hostname);
    }
    // Literal IP shortcut
    if let Ok(ip) = hostname.parse::<IpAddr>() {
        return Some(vec![ip]);
    }
    let with_port = format!("{hostname}:0");
    match with_port.to_socket_addrs() {
        Ok(addrs) => {
            let v: Vec<IpAddr> = addrs.map(|sa| sa.ip()).collect();
            if v.is_empty() { None } else { Some(v) }
        }
        Err(_) => None,
    }
}

fn env_allows_private() -> bool {
    matches!(
        env::var("SWML_ALLOW_PRIVATE_URLS")
            .unwrap_or_default()
            .to_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    )
}

fn cidr_contains(cidr: &str, ip: &IpAddr) -> bool {
    let Some((net_str, prefix_str)) = cidr.split_once('/') else {
        return false;
    };
    let prefix: u32 = match prefix_str.parse() {
        Ok(p) => p,
        Err(_) => return false,
    };
    match ip {
        IpAddr::V4(ip4) => {
            let net4: Ipv4Addr = match net_str.parse() {
                Ok(n) => n,
                Err(_) => return false,
            };
            if prefix > 32 {
                return false;
            }
            let mask: u32 = if prefix == 0 { 0 } else { !0u32 << (32 - prefix) };
            (u32::from(*ip4) & mask) == (u32::from(net4) & mask)
        }
        IpAddr::V6(ip6) => {
            let net6: Ipv6Addr = match net_str.parse() {
                Ok(n) => n,
                Err(_) => return false,
            };
            if prefix > 128 {
                return false;
            }
            let ip_bits = u128::from(*ip6);
            let net_bits = u128::from(net6);
            let mask: u128 = if prefix == 0 { 0 } else { !0u128 << (128 - prefix) };
            (ip_bits & mask) == (net_bits & mask)
        }
    }
}

/// Validate that a URL is safe to fetch.
///
/// Mirrors Python's `validate_url(url, allow_private=False) -> bool`.
pub fn validate_url(url: &str, allow_private: bool) -> bool {
    let log = Logger::new("signalwire.url_validator");

    let parsed = match Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            log.warn(&format!("URL validation error: {e}"));
            return false;
        }
    };

    let scheme = parsed.scheme().to_lowercase();
    if scheme != "http" && scheme != "https" {
        log.warn(&format!("URL rejected: invalid scheme {}", parsed.scheme()));
        return false;
    }

    let hostname = match parsed.host_str() {
        Some(h) if !h.is_empty() => h.to_string(),
        _ => {
            log.warn("URL rejected: no hostname");
            return false;
        }
    };

    if allow_private || env_allows_private() {
        return true;
    }

    let ips = match resolve(&hostname) {
        Some(v) if !v.is_empty() => v,
        _ => {
            log.warn(&format!("URL rejected: could not resolve hostname {hostname}"));
            return false;
        }
    };

    for ip in &ips {
        for cidr in BLOCKED_NETWORKS {
            if cidr_contains(cidr, ip) {
                log.warn(&format!(
                    "URL rejected: {hostname} resolves to blocked IP {ip} (in {cidr})"
                ));
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    // The tests deliberately call the `_`-prefixed test-only seam `_set_resolver`.
    #![allow(clippy::used_underscore_items)]
    use super::*;
    use std::sync::Mutex;

    static TEST_GUARD: Mutex<()> = Mutex::new(());

    fn stub_resolver(ip: &str) {
        let ip: IpAddr = ip.parse().unwrap();
        _set_resolver(Some(Box::new(move |_| Some(vec![ip]))));
    }

    fn stub_failed_resolver() {
        _set_resolver(Some(Box::new(|_| None)));
    }

    fn reset_state() {
        _set_resolver(None);
        unsafe { env::remove_var("SWML_ALLOW_PRIVATE_URLS") };
    }

    // --- Scheme ----------------------------------------------------------

    #[test]
    fn http_scheme_allowed() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("1.2.3.4");
        assert!(validate_url("http://example.com", false));
        reset_state();
    }

    #[test]
    fn https_scheme_allowed() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("1.2.3.4");
        assert!(validate_url("https://example.com", false));
        reset_state();
    }

    #[test]
    fn ftp_scheme_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        assert!(!validate_url("ftp://example.com", false));
    }

    #[test]
    fn file_scheme_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        assert!(!validate_url("file:///etc/passwd", false));
    }

    #[test]
    fn javascript_scheme_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        assert!(!validate_url("javascript:alert(1)", false));
    }

    // --- Hostname --------------------------------------------------------

    #[test]
    fn no_hostname_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        assert!(!validate_url("http://", false));
    }

    #[test]
    fn unresolvable_hostname_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_failed_resolver();
        assert!(!validate_url("http://nonexistent.invalid", false));
        reset_state();
    }

    // --- Blocked ranges -------------------------------------------------

    #[test]
    fn loopback_ipv4_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("127.0.0.1");
        assert!(!validate_url("http://localhost", false));
        reset_state();
    }

    #[test]
    fn rfc1918_10_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("10.0.0.5");
        assert!(!validate_url("http://internal", false));
        reset_state();
    }

    #[test]
    fn rfc1918_192_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("192.168.1.1");
        assert!(!validate_url("http://router", false));
        reset_state();
    }

    #[test]
    fn rfc1918_172_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("172.16.0.1");
        assert!(!validate_url("http://corp", false));
        reset_state();
    }

    #[test]
    fn link_local_metadata_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("169.254.169.254");
        assert!(!validate_url("http://metadata", false));
        reset_state();
    }

    #[test]
    fn zero_ip_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("0.0.0.0");
        assert!(!validate_url("http://void", false));
        reset_state();
    }

    #[test]
    fn ipv6_loopback_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("::1");
        assert!(!validate_url("http://[::1]", false));
        reset_state();
    }

    #[test]
    fn ipv6_link_local_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("fe80::1");
        assert!(!validate_url("http://link-local", false));
        reset_state();
    }

    #[test]
    fn ipv6_private_rejected() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("fc00::1");
        assert!(!validate_url("http://ipv6-private", false));
        reset_state();
    }

    #[test]
    fn public_ip_allowed() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        stub_resolver("8.8.8.8");
        assert!(validate_url("http://dns.google", false));
        reset_state();
    }

    // --- allow_private bypass ------------------------------------------

    #[test]
    fn allow_private_param_bypasses_check() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        // No resolver stub: bypass short-circuits before DNS.
        assert!(validate_url("http://10.0.0.5", true));
    }

    #[test]
    fn env_var_bypasses_check() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        unsafe { env::set_var("SWML_ALLOW_PRIVATE_URLS", "true") };
        assert!(validate_url("http://10.0.0.5", false));
        reset_state();
    }

    #[test]
    fn env_var_yes_bypasses_check() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        unsafe { env::set_var("SWML_ALLOW_PRIVATE_URLS", "YES") };
        assert!(validate_url("http://10.0.0.5", false));
        reset_state();
    }

    #[test]
    fn env_var_1_bypasses_check() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        unsafe { env::set_var("SWML_ALLOW_PRIVATE_URLS", "1") };
        assert!(validate_url("http://10.0.0.5", false));
        reset_state();
    }

    #[test]
    fn env_var_false_does_not_bypass() {
        let _g = TEST_GUARD.lock().unwrap();
        reset_state();
        unsafe { env::set_var("SWML_ALLOW_PRIVATE_URLS", "false") };
        stub_resolver("10.0.0.5");
        assert!(!validate_url("http://internal", false));
        reset_state();
    }

    #[test]
    fn blocked_networks_has_all_nine() {
        assert_eq!(BLOCKED_NETWORKS.len(), 9);
    }
}
