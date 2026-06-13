//! Typed error for the [`AgentServer`](crate::server::AgentServer) surface and
//! the shared HTTP(S) bind path.
//!
//! Like [`RelayError`](crate::relay::RelayError), this replaces the old
//! `Result<_, String>` returns on the server surface (`register`,
//! `serve_static`, `serve_static_files`, and the internal `bind_server`) with a
//! closed, inspectable failure set — matching the REST layer's
//! [`SignalWireRestError`](crate::rest::SignalWireRestError) exemplar.
//!
//! The data each failure carries lives in the variant (callers `match` rather
//! than call getters), so the only public surface added is the type plus its
//! `Display` / [`std::error::Error`] impls. `#[non_exhaustive]` leaves room to
//! add new failure modes without a breaking change.

use std::fmt;

/// A server-configuration or HTTP-bind operation failed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ServerError {
    /// An agent was registered at a route already in use. Carries the route.
    RouteAlreadyRegistered {
        /// The conflicting route.
        route: String,
    },

    /// A static-file directory could not be served because the path does not
    /// exist or is not a directory. Carries the offending path.
    StaticDir {
        /// The directory path that failed.
        path: String,
        /// Why it failed (missing, not-a-directory, …).
        reason: String,
    },

    /// HTTPS was requested via `SWML_SSL_*` but the configuration was
    /// incomplete or the cert/key files were unreadable.
    TlsConfig {
        /// The misconfiguration description.
        message: String,
    },

    /// The HTTP(S) listener failed to bind the address. Carries the address
    /// and the underlying cause.
    Bind {
        /// The `host:port` that failed to bind.
        addr: String,
        /// The underlying bind error rendered to text.
        source: String,
    },
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::RouteAlreadyRegistered { route } => {
                write!(f, "ServerError: route '{route}' is already registered")
            }
            ServerError::StaticDir { path, reason } => {
                write!(f, "ServerError: static directory '{path}': {reason}")
            }
            ServerError::TlsConfig { message } => {
                write!(f, "ServerError: TLS configuration: {message}")
            }
            ServerError::Bind { addr, source } => {
                write!(f, "ServerError: failed to bind {addr}: {source}")
            }
        }
    }
}

impl std::error::Error for ServerError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_conflict_display_and_match() {
        let e = ServerError::RouteAlreadyRegistered {
            route: "/bot".into(),
        };
        assert!(e.to_string().contains("/bot"));
        assert!(e.to_string().contains("already registered"));
        match e {
            ServerError::RouteAlreadyRegistered { route } => assert_eq!(route, "/bot"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_static_dir_carries_path_and_reason() {
        let e = ServerError::StaticDir {
            path: "/nope".into(),
            reason: "does not exist".into(),
        };
        let s = e.to_string();
        assert!(s.contains("/nope"));
        assert!(s.contains("does not exist"));
    }

    #[test]
    fn test_is_std_error() {
        let e = ServerError::Bind {
            addr: "0.0.0.0:3000".into(),
            source: "address in use".into(),
        };
        let _: &dyn std::error::Error = &e;
        assert!(e.to_string().contains("0.0.0.0:3000"));
    }
}
