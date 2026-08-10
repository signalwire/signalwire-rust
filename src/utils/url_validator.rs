// Copyright (c) 2025 SignalWire
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! SSRF-prevention guard for user-supplied URLs.
//!
//! Rejects non-http(s) schemes, missing hostnames, and any URL whose
//! hostname resolves to a private / loopback / link-local / cloud-
//! metadata IP. When `allow_private` is true, OR the
//! `SWML_ALLOW_PRIVATE_URLS` env var is set to `"1"`, `"true"`, or
//! `"yes"` (case-insensitive), the IP-blocklist check is skipped.

use std::env;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::sync::{Arc, Mutex};

use url::Url;

use crate::logging::Logger;

/// Cross-port SSRF block list. Order matches the wire contract for
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

/// Shared, cheaply-clonable form of a [`ResolverFn`] for the slot's internal
/// storage. The public seam takes an owning `Box`, which cannot be cloned out
/// of the mutex; storing an `Arc` lets [`resolve`] clone the hook out and
/// **release the guard before calling it** (see [`resolve`]).
type SharedResolverFn = Arc<dyn Fn(&str) -> Option<Vec<IpAddr>> + Send + Sync>;

static RESOLVER: Mutex<Option<SharedResolverFn>> = Mutex::new(None);

/// Read the resolver slot, tolerating a poisoned lock.
///
/// The slot holds an `Option<Arc<…>>`, which no operation here can leave in a
/// torn or half-written state, so a panic elsewhere carries no reason to make
/// every subsequent URL validation panic too. Recovering the guard keeps one
/// unrelated panic from bricking SSRF validation for the process lifetime.
fn resolver_slot() -> std::sync::MutexGuard<'static, Option<SharedResolverFn>> {
    RESOLVER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Install a custom resolver (for tests). Pass `None` to clear. `_`-prefixed
/// deliberately — a test-only seam, not part of the public API surface.
pub fn _set_resolver(resolver: Option<ResolverFn>) {
    let shared: Option<SharedResolverFn> =
        resolver.map(|r| Arc::new(move |host: &str| r(host)) as SharedResolverFn);
    *resolver_slot() = shared;
}

fn resolve(hostname: &str) -> Option<Vec<IpAddr>> {
    // Clone the hook out and DROP the guard before invoking it: the hook is
    // caller-supplied code, so calling it under the lock would poison
    // `RESOLVER` process-wide if it panicked, and deadlock if it re-entered
    // `validate_url`.
    let hook: Option<SharedResolverFn> = resolver_slot().clone();
    if let Some(r) = hook {
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
            let mask: u32 = if prefix == 0 {
                0
            } else {
                !0u32 << (32 - prefix)
            };
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
            let mask: u128 = if prefix == 0 {
                0
            } else {
                !0u128 << (128 - prefix)
            };
            (ip_bits & mask) == (net_bits & mask)
        }
    }
}

/// Validate that a URL is safe to fetch.
///
/// Matches `validate_url(url, allow_private=False) -> bool`.
///
/// `allow_private` is `Option<bool>` because the argument is optional;
/// `None` is the omit-it call and takes `false`.
pub fn validate_url(url: &str, allow_private: Option<bool>) -> bool {
    let allow_private = allow_private.unwrap_or(false);
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
            log.warn(&format!(
                "URL rejected: could not resolve hostname {hostname}"
            ));
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

    /// Serialises the tests that mutate the PROCESS-GLOBAL
    /// `SWML_ALLOW_PRIVATE_URLS` env var (`env::set_var` is process-wide and
    /// `unsafe` in edition 2024 precisely because concurrent mutation is UB).
    /// The resolver hook no longer needs serialising for correctness — only
    /// that env var does — so this guard stays until the env global gets a
    /// scoped seam of its own.
    static TEST_GUARD: Mutex<()> = Mutex::new(());

    /// Acquire `TEST_GUARD`, tolerating poisoning. The guarded data is `()`, so
    /// a panic in one test leaves nothing inconsistent — without this, ONE
    /// failing test turns every sibling into a derived `PoisonError` failure and
    /// buries the real error (measured: one planted panic took this module from
    /// `23 passed; 0 failed` to `5 passed; 18 failed`, 17 of them `PoisonError`;
    /// with this helper the same plant gives `22 passed; 1 failed`, 0 poison).
    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        TEST_GUARD
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

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
        let _g = test_guard();
        reset_state();
        stub_resolver("1.2.3.4");
        assert!(validate_url("http://example.com", None));
        reset_state();
    }

    #[test]
    fn https_scheme_allowed() {
        let _g = test_guard();
        reset_state();
        stub_resolver("1.2.3.4");
        assert!(validate_url("https://example.com", None));
        reset_state();
    }

    #[test]
    fn ftp_scheme_rejected() {
        let _g = test_guard();
        reset_state();
        assert!(!validate_url("ftp://example.com", None));
    }

    #[test]
    fn file_scheme_rejected() {
        let _g = test_guard();
        reset_state();
        assert!(!validate_url("file:///etc/passwd", None));
    }

    #[test]
    fn javascript_scheme_rejected() {
        let _g = test_guard();
        reset_state();
        assert!(!validate_url("javascript:alert(1)", None));
    }

    // --- Hostname --------------------------------------------------------

    #[test]
    fn no_hostname_rejected() {
        let _g = test_guard();
        reset_state();
        assert!(!validate_url("http://", None));
    }

    #[test]
    fn unresolvable_hostname_rejected() {
        let _g = test_guard();
        reset_state();
        stub_failed_resolver();
        assert!(!validate_url("http://nonexistent.invalid", None));
        reset_state();
    }

    // --- Blocked ranges -------------------------------------------------

    #[test]
    fn loopback_ipv4_rejected() {
        let _g = test_guard();
        reset_state();
        stub_resolver("127.0.0.1");
        assert!(!validate_url("http://localhost", None));
        reset_state();
    }

    #[test]
    fn rfc1918_10_rejected() {
        let _g = test_guard();
        reset_state();
        stub_resolver("10.0.0.5");
        assert!(!validate_url("http://internal", None));
        reset_state();
    }

    #[test]
    fn rfc1918_192_rejected() {
        let _g = test_guard();
        reset_state();
        stub_resolver("192.168.1.1");
        assert!(!validate_url("http://router", None));
        reset_state();
    }

    #[test]
    fn rfc1918_172_rejected() {
        let _g = test_guard();
        reset_state();
        stub_resolver("172.16.0.1");
        assert!(!validate_url("http://corp", None));
        reset_state();
    }

    #[test]
    fn link_local_metadata_rejected() {
        let _g = test_guard();
        reset_state();
        stub_resolver("169.254.169.254");
        assert!(!validate_url("http://metadata", None));
        reset_state();
    }

    #[test]
    fn zero_ip_rejected() {
        let _g = test_guard();
        reset_state();
        stub_resolver("0.0.0.0");
        assert!(!validate_url("http://void", None));
        reset_state();
    }

    #[test]
    fn ipv6_loopback_rejected() {
        let _g = test_guard();
        reset_state();
        stub_resolver("::1");
        assert!(!validate_url("http://[::1]", None));
        reset_state();
    }

    #[test]
    fn ipv6_link_local_rejected() {
        let _g = test_guard();
        reset_state();
        stub_resolver("fe80::1");
        assert!(!validate_url("http://link-local", None));
        reset_state();
    }

    #[test]
    fn ipv6_private_rejected() {
        let _g = test_guard();
        reset_state();
        stub_resolver("fc00::1");
        assert!(!validate_url("http://ipv6-private", None));
        reset_state();
    }

    #[test]
    fn public_ip_allowed() {
        let _g = test_guard();
        reset_state();
        stub_resolver("8.8.8.8");
        assert!(validate_url("http://dns.google", None));
        reset_state();
    }

    // --- allow_private bypass ------------------------------------------

    #[test]
    fn allow_private_param_bypasses_check() {
        let _g = test_guard();
        reset_state();
        // No resolver stub: bypass short-circuits before DNS.
        assert!(validate_url("http://10.0.0.5", Some(true)));
    }

    #[test]
    fn env_var_bypasses_check() {
        let _g = test_guard();
        reset_state();
        unsafe { env::set_var("SWML_ALLOW_PRIVATE_URLS", "true") };
        assert!(validate_url("http://10.0.0.5", None));
        reset_state();
    }

    #[test]
    fn env_var_yes_bypasses_check() {
        let _g = test_guard();
        reset_state();
        unsafe { env::set_var("SWML_ALLOW_PRIVATE_URLS", "YES") };
        assert!(validate_url("http://10.0.0.5", None));
        reset_state();
    }

    #[test]
    fn env_var_1_bypasses_check() {
        let _g = test_guard();
        reset_state();
        unsafe { env::set_var("SWML_ALLOW_PRIVATE_URLS", "1") };
        assert!(validate_url("http://10.0.0.5", None));
        reset_state();
    }

    #[test]
    fn env_var_false_does_not_bypass() {
        let _g = test_guard();
        reset_state();
        unsafe { env::set_var("SWML_ALLOW_PRIVATE_URLS", "false") };
        stub_resolver("10.0.0.5");
        assert!(!validate_url("http://internal", None));
        reset_state();
    }

    #[test]
    fn blocked_networks_has_all_nine() {
        assert_eq!(BLOCKED_NETWORKS.len(), 9);
    }
}
