//! Session security: HMAC-signed tokens and timing-safe auth.
//!
//! [`session_manager::SessionManager`] creates and validates HMAC-SHA256 tokens
//! (encoding `function:call_id:expiry`) with constant-time comparison;
//! [`security_utils`] holds the basic-auth and comparison helpers.

pub mod security_utils;
pub mod session_manager;
pub mod webhook;

#[cfg(feature = "tower-middleware")]
pub mod webhook_layer;

pub use security_utils::{filter_sensitive_headers, is_valid_hostname, redact_url};
pub use session_manager::SessionManager;
pub use webhook::{ParamsOrBody, WebhookError, validate_request, validate_webhook_signature};

#[cfg(feature = "tower-middleware")]
pub use webhook_layer::{WebhookLayer, WebhookValidate, validate};
