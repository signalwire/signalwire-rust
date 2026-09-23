// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.
// See LICENSE file in the project root for full license information.

//! Shared HTTPS binding for the SDK's webhook / SWML / agent HTTP servers.
//!
//! Python's `SecurityConfig` reads `SWML_SSL_ENABLED` / `SWML_SSL_CERT_PATH` /
//! `SWML_SSL_KEY_PATH` (see `core/security_config.py`) and, when enabled, hands
//! `ssl_certfile` / `ssl_keyfile` to uvicorn so the server speaks real HTTPS.
//! The Rust port serves with `tiny_http`; this helper mirrors the *same env
//! contract* and binds either a plain (`Server::http`) or a TLS
//! (`Server::https`, via `tiny_http`'s `ssl-rustls` feature) listener.
//!
//! Kept as a `pub(crate)` helper so the three server entry points
//! (`AgentServer::run`, `Service::run`, `AgentBase::run`) share one code path
//! and the public surface is unchanged — TLS is configured entirely through
//! the documented `SWML_SSL_*` environment variables, no new public method.

use std::env;

use super::error::ServerError;

/// Env vars matching `SecurityConfig` (`core/security_config.py`).
const SSL_ENABLED_ENV: &str = "SWML_SSL_ENABLED";
const SSL_CERT_PATH_ENV: &str = "SWML_SSL_CERT_PATH";
const SSL_KEY_PATH_ENV: &str = "SWML_SSL_KEY_PATH";

/// Resolved TLS material: PEM cert + key file *contents*.
struct TlsMaterial {
    certificate: Vec<u8>,
    private_key: Vec<u8>,
}

// Hand-written so a `{:?}` (a test assertion message, a log line, a panic
// payload) can never spill the PRIVATE KEY bytes. A `#[derive(Debug)]` here
// would print the whole PEM.
impl std::fmt::Debug for TlsMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TlsMaterial")
            .field(
                "certificate",
                &format_args!("<{} bytes>", self.certificate.len()),
            )
            .field(
                "private_key",
                &format_args!("<redacted, {} bytes>", self.private_key.len()),
            )
            .finish()
    }
}

/// Read `SWML_SSL_*`. Returns `Ok(Some(..))` only when SSL is enabled *and*
/// both cert and key paths are set and readable. Returns `Ok(None)` when SSL
/// is off (the normal HTTP path). Returns `Err` when SSL is requested but the
/// configuration is incomplete / unreadable — surfaced to the caller so a
/// misconfigured deployment fails loudly instead of silently serving HTTP.
fn resolve_tls_material() -> Result<Option<TlsMaterial>, ServerError> {
    let enabled = env::var(SSL_ENABLED_ENV)
        .is_ok_and(|v| matches!(v.to_ascii_lowercase().as_str(), "true" | "1" | "yes"));
    if !enabled {
        return Ok(None);
    }

    let cert_path = env::var(SSL_CERT_PATH_ENV).ok().filter(|s| !s.is_empty());
    let key_path = env::var(SSL_KEY_PATH_ENV).ok().filter(|s| !s.is_empty());
    let (Some(cert_path), Some(key_path)) = (cert_path, key_path) else {
        return Err(ServerError::TlsConfig {
            message: format!(
                "{SSL_ENABLED_ENV} is set but {SSL_CERT_PATH_ENV} / {SSL_KEY_PATH_ENV} are not both provided"
            ),
        });
    };

    let certificate = std::fs::read(&cert_path).map_err(|e| ServerError::TlsConfig {
        message: format!("read {SSL_CERT_PATH_ENV} {cert_path}: {e}"),
    })?;
    let private_key = std::fs::read(&key_path).map_err(|e| ServerError::TlsConfig {
        message: format!("read {SSL_KEY_PATH_ENV} {key_path}: {e}"),
    })?;
    Ok(Some(TlsMaterial {
        certificate,
        private_key,
    }))
}

/// Bind the server at `addr`, choosing HTTPS when `SWML_SSL_*` requests it and
/// plain HTTP otherwise. Returns `(server, is_https)` so the caller can log the
/// correct scheme.
///
/// Panics (consistent with the existing `run()` bind-failure behavior) on a
/// genuine bind error; SSL-misconfiguration is reported through the returned
/// `Result` so callers can choose how to fail.
pub(crate) fn bind_server(addr: &str) -> Result<(tiny_http::Server, bool), ServerError> {
    if let Some(material) = resolve_tls_material()? {
        let config = tiny_http::SslConfig {
            certificate: material.certificate,
            private_key: material.private_key,
        };
        let server = tiny_http::Server::https(addr, config).map_err(|e| ServerError::Bind {
            addr: addr.to_string(),
            source: e.to_string(),
        })?;
        Ok((server, true))
    } else {
        let server = tiny_http::Server::http(addr).map_err(|e| ServerError::Bind {
            addr: addr.to_string(),
            source: e.to_string(),
        })?;
        Ok((server, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serialize the env mutations below — these tests set process-global
    /// `SWML_SSL_*` vars and would otherwise race each other.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn clear_ssl_env() {
        // SAFETY: the ENV_LOCK guard is held by every caller.
        unsafe {
            env::remove_var(SSL_ENABLED_ENV);
            env::remove_var(SSL_CERT_PATH_ENV);
            env::remove_var(SSL_KEY_PATH_ENV);
        }
    }

    /// THE SILENT-DOWNGRADE CASE. `SWML_SSL_ENABLED=true` with no cert/key must
    /// be an ERROR, never `Ok(None)` — an `Ok(None)` here is what makes a port
    /// bind a PLAIN listener for an operator who asked for HTTPS, with no error
    /// and no warning. (This exact fold — "not configured" collapsing into "TLS
    /// off" — is a live defect in sibling ports and in the Python reference's
    /// `agent_server.py`; rust refuses instead, and this test is what keeps it
    /// refusing.)
    #[test]
    fn ssl_enabled_without_cert_or_key_is_an_error_not_plain_http() {
        let _g = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear_ssl_env();
        // SAFETY: ENV_LOCK held.
        unsafe {
            env::set_var(SSL_ENABLED_ENV, "true");
        }
        let got = resolve_tls_material();
        clear_ssl_env();

        match got {
            Ok(None) => panic!(
                "SWML_SSL_ENABLED=true with no cert/key resolved to the PLAIN-HTTP path — \
                 a silent TLS downgrade: the operator asked for HTTPS and would get \
                 unencrypted HTTP with no diagnostic"
            ),
            Ok(Some(_)) => panic!("resolved TLS material out of thin air (no cert/key were set)"),
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains(SSL_CERT_PATH_ENV) && msg.contains(SSL_KEY_PATH_ENV),
                    "the refusal must name the missing settings; got: {msg}"
                );
            }
        }
    }

    /// Half-configured is the same failure: a cert with no key (and vice versa)
    /// must not fall through to plain HTTP either.
    #[test]
    fn ssl_enabled_with_only_one_of_cert_key_is_an_error() {
        for (set_var, _other) in [
            (SSL_CERT_PATH_ENV, SSL_KEY_PATH_ENV),
            (SSL_KEY_PATH_ENV, SSL_CERT_PATH_ENV),
        ] {
            let _g = ENV_LOCK
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            clear_ssl_env();
            // SAFETY: ENV_LOCK held.
            unsafe {
                env::set_var(SSL_ENABLED_ENV, "true");
                env::set_var(set_var, "/nonexistent/path.pem");
            }
            let got = resolve_tls_material();
            clear_ssl_env();
            assert!(
                matches!(got, Err(ServerError::TlsConfig { .. })),
                "SWML_SSL_ENABLED=true with only {set_var} set must be a TlsConfig error, \
                 not a plain-HTTP fallback; got {got:?}"
            );
        }
    }

    /// Configured but UNREADABLE cert/key is also a refusal, not a downgrade —
    /// a typo'd path or a permissions problem must not silently serve plaintext.
    #[test]
    fn ssl_enabled_with_unreadable_files_is_an_error() {
        let _g = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear_ssl_env();
        // SAFETY: ENV_LOCK held.
        unsafe {
            env::set_var(SSL_ENABLED_ENV, "true");
            env::set_var(SSL_CERT_PATH_ENV, "/nonexistent/cert.pem");
            env::set_var(SSL_KEY_PATH_ENV, "/nonexistent/key.pem");
        }
        let got = resolve_tls_material();
        clear_ssl_env();
        assert!(
            matches!(got, Err(ServerError::TlsConfig { .. })),
            "unreadable cert/key must be a TlsConfig error, not a plain-HTTP fallback; got {got:?}"
        );
    }

    /// CONTROL: SSL genuinely off resolves to the plain path. The refusals above
    /// must be scoped to "HTTPS was requested", not a blanket ban on HTTP.
    #[test]
    fn ssl_disabled_resolves_to_plain_http() {
        let _g = ENV_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        clear_ssl_env();
        let got = resolve_tls_material();
        assert!(
            matches!(got, Ok(None)),
            "with SWML_SSL_ENABLED unset the plain-HTTP path must resolve cleanly; got {got:?}"
        );
    }
}
