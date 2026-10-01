//! Cross-port `core` namespace, mirroring `signalwire.core.*` in the reference.
//! Houses shared helpers such as the security/logging configuration types.

// Cross-port "core" namespace for helpers that Python keeps under
// `signalwire.core.*`. Currently houses the logging-config module which
// owns `get_execution_mode`.

pub mod auth_handler;
pub mod capabilities;
pub mod config_loader;
pub mod logging_config;
pub mod post_prompt;
pub mod security_config;
pub mod sync_handlers;
