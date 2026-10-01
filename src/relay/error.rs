//! Typed error for the RELAY client surface.
//!
//! The REST layer already ships a proper error enum
//! ([`SignalWireRestError`](crate::rest::SignalWireRestError)); this is the
//! RELAY-side analogue. Before this, the relay client returned
//! `Result<_, String>` everywhere — a stringly-typed failure channel that
//! forces callers to pattern-match on message *text* to react differently to
//! (say) an auth rejection vs. a transport drop vs. a timeout. [`RelayError`]
//! makes the failure modes a closed, inspectable set, exactly like the REST
//! exemplar.
//!
//! Design notes (idiomatic Rust, matching the REST error's bar):
//! - The data each failure carries lives **in the variant**, so callers react
//!   by `match`ing rather than calling accessor getters. That keeps the public
//!   surface to the type + its trait impls (`Display` / [`std::error::Error`])
//!   and means the audit sees no new methods.
//! - `#[non_exhaustive]` because the failure set mirrors transport/server
//!   conditions that can grow (new RELAY error classes) without that being a
//!   breaking change — downstream `match` must carry a wildcard.
//! - Every variant's [`Display`] is human-actionable and preserves the same
//!   context the old `format!("…")` strings carried, so log output is no worse.

use std::fmt;

/// Something went wrong talking to the RELAY WebSocket service.
///
/// Returned by the connection-lifecycle and blocking-RPC methods on
/// [`Client`](crate::relay::Client) (e.g. `connect`, `authenticate_blocking`,
/// `execute_blocking`, `dial_blocking`, `send_message_blocking`).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RelayError {
    /// A required environment variable was missing when building the client
    /// from the environment (`from_env`). Carries the variable name.
    MissingEnv {
        /// The environment variable that was not set.
        var: String,
    },

    /// The underlying WebSocket transport failed to connect, upgrade, read, or
    /// write. Carries a human description of the transport-level cause.
    Transport {
        /// What was being attempted (e.g. the target URL or operation).
        context: String,
        /// The transport library's error rendered to text.
        source: String,
    },

    /// The RELAY server rejected the `signalwire.connect` authentication
    /// handshake. Carries the server-reported reason.
    Auth {
        /// The server's error message (or a default if none was supplied).
        message: String,
    },

    /// The server returned a JSON-RPC error for a request (e.g. a failed
    /// `messaging.send` / `calling.dial`). Carries the inner method, the
    /// server's error message, and the server's numeric error code.
    Rpc {
        /// The RELAY method that failed (e.g. `messaging.send`).
        method: String,
        /// The server's error message.
        message: String,
        /// The server's error code, as the reference's `RelayError.code`
        /// (`client.py:1330-1332`). Negative values are the reference's
        /// client-side sentinels (`-1` for a timeout / closed connection); a
        /// JSON-RPC `error.code` or a non-2xx result `code` is carried
        /// verbatim. `None` when the frame carried no parseable code.
        code: Option<i64>,
    },

    /// A blocking call did not receive its response within the deadline.
    /// Carries what was being awaited so the log line is actionable.
    Timeout {
        /// What timed out (e.g. `signalwire.connect` or a dial tag).
        what: String,
    },

    /// A `dial` completed with a `failed` state or never produced an answer
    /// before the dial deadline. Carries the reason / tag.
    DialFailed {
        /// The dial reason or tag context.
        reason: String,
    },

    /// A required argument was invalid (e.g. `send_message` with neither body
    /// nor media). Carries the validation message.
    InvalidArgument {
        /// The validation failure description.
        message: String,
    },
}

impl RelayError {
    /// Build a [`RelayError::Transport`] from a context and any `Display` cause.
    /// Convenience for the `map_err(|e| …)` call sites in the client.
    pub fn transport(context: impl Into<String>, source: impl fmt::Display) -> Self {
        RelayError::Transport {
            context: context.into(),
            source: source.to_string(),
        }
    }

    /// Build a [`RelayError::MissingEnv`] for the named variable.
    pub fn missing_env(var: impl Into<String>) -> Self {
        RelayError::MissingEnv { var: var.into() }
    }

    /// The server-reported error code, mirroring the reference's
    /// `RelayError.code` (`client.py:1330-1332`).
    ///
    /// Only a [`RelayError::Rpc`] carries a code the *server* chose. The
    /// remaining variants are client-side conditions the reference also raises
    /// as `RelayError`, and it uses the sentinel `-1` for each of them
    /// (`client.py:798,829,836,842,1218`), so they report `Some(-1)` here for
    /// the same reason: a caller switching on the code must be able to tell a
    /// transport/timeout failure from a server rejection without parsing the
    /// message text.
    ///
    /// `None` only when an `Rpc` frame carried no parseable `code`.
    pub fn code(&self) -> Option<i64> {
        match self {
            RelayError::Rpc { code, .. } => *code,
            _ => Some(-1),
        }
    }

    /// The server's (or the client-side condition's) error message, undecorated
    /// — matching the `RelayError.message`, which holds the raw
    /// server text and leaves the `"RELAY error {code}: {message}"` decoration
    /// to `Display` (`client.py:1331-1333`).
    ///
    /// This is deliberately NOT `to_string()`: `Display` prefixes `RelayError:`
    /// and per-variant context, and a caller that wants to surface or re-wrap
    /// the server's own wording needs it unwrapped.
    pub fn message(&self) -> &str {
        match self {
            RelayError::MissingEnv { var } => var,
            RelayError::Transport { source, .. } => source,
            RelayError::Auth { message }
            | RelayError::Rpc { message, .. }
            | RelayError::InvalidArgument { message } => message,
            RelayError::Timeout { what } => what,
            RelayError::DialFailed { reason } => reason,
        }
    }
}

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelayError::MissingEnv { var } => {
                write!(f, "RelayError: {var} not set")
            }
            RelayError::Transport { context, source } => {
                write!(f, "RelayError: transport ({context}): {source}")
            }
            RelayError::Auth { message } => {
                write!(f, "RelayError: auth error: {message}")
            }
            // Includes the code when the server supplied one, matching the
            // reference's `f"RELAY error {code}: {message}"` (`client.py:1333`)
            // while keeping the method context this port already carried.
            RelayError::Rpc {
                method,
                message,
                code,
            } => match code {
                Some(c) => write!(f, "RelayError: {method} failed ({c}): {message}"),
                None => write!(f, "RelayError: {method} failed: {message}"),
            },
            RelayError::Timeout { what } => {
                write!(f, "RelayError: timed out waiting for {what}")
            }
            RelayError::DialFailed { reason } => {
                write!(f, "RelayError: dial failed: {reason}")
            }
            RelayError::InvalidArgument { message } => {
                write!(f, "RelayError: invalid argument: {message}")
            }
        }
    }
}

impl std::error::Error for RelayError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variants_carry_their_data() {
        let e = RelayError::missing_env("SIGNALWIRE_PROJECT_ID");
        assert_eq!(
            e,
            RelayError::MissingEnv {
                var: "SIGNALWIRE_PROJECT_ID".into()
            }
        );

        let t = RelayError::transport("WS connect to wss://x", "handshake refused");
        match t {
            RelayError::Transport { context, source } => {
                assert!(context.contains("wss://x"));
                assert_eq!(source, "handshake refused");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_display_is_actionable() {
        assert!(
            RelayError::Auth {
                message: "bad token".into()
            }
            .to_string()
            .contains("bad token")
        );
        assert!(
            RelayError::Timeout {
                what: "signalwire.connect".into()
            }
            .to_string()
            .contains("signalwire.connect")
        );
        assert!(
            RelayError::Rpc {
                method: "messaging.send".into(),
                message: "rejected".into(),
                code: Some(422),
            }
            .to_string()
            .contains("messaging.send")
        );
    }

    #[test]
    fn test_code_and_message_readback() {
        // The reference's `RelayError(code, message)` keeps BOTH readable
        // (`client.py:1330-1332`); a caller dispatching on the server's code must
        // not have to parse `Display` text to get it.
        let rpc = RelayError::Rpc {
            method: "calling.play".into(),
            message: "not authorized".into(),
            code: Some(401),
        };
        assert_eq!(rpc.code(), Some(401));
        assert_eq!(rpc.message(), "not authorized");
        // `message()` is the RAW server text, not the decorated Display form.
        assert!(rpc.to_string().contains("401"));
        assert_ne!(rpc.message(), rpc.to_string());

        // Client-side conditions use the reference's `-1` sentinel
        // (`client.py:798,829,836,842,1218`).
        assert_eq!(
            RelayError::Timeout {
                what: "signalwire.connect".into()
            }
            .code(),
            Some(-1)
        );
    }

    #[test]
    fn test_is_std_error() {
        let e = RelayError::DialFailed {
            reason: "no answer".into(),
        };
        let _: &dyn std::error::Error = &e;
    }
}
