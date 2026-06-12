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

/// Env vars mirroring Python `SecurityConfig` (`core/security_config.py`).
const SSL_ENABLED_ENV: &str = "SWML_SSL_ENABLED";
const SSL_CERT_PATH_ENV: &str = "SWML_SSL_CERT_PATH";
const SSL_KEY_PATH_ENV: &str = "SWML_SSL_KEY_PATH";

/// Resolved TLS material: PEM cert + key file *contents*.
struct TlsMaterial {
    certificate: Vec<u8>,
    private_key: Vec<u8>,
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
