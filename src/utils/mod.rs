// Cross-port "utils" namespace mirroring `signalwire.utils.*` in the
// Python reference. Houses serverless detection, URL validation, etc.

pub mod schema_utils;
pub mod url_validator;

pub use schema_utils::{SchemaUtils, SchemaValidationError};
pub use url_validator::validate_url;

use crate::core::logging_config::get_execution_mode;

/// Cross-language SDK contract: `signalwire.utils.is_serverless_mode`
/// returns `true` whenever the SDK is running inside any short-lived /
/// event-driven invocation environment (anything other than `"server"`).
///
/// Mirrors `signalwire.utils.is_serverless_mode` in the Python
/// reference. The actual detection ladder lives in
/// `core::logging_config::get_execution_mode`; this helper just maps
/// "anything except 'server'" -> `true`.
pub fn is_serverless_mode() -> bool {
    get_execution_mode() != "server"
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// Helper: clear every env var the function inspects.
    fn clear_env() {
        unsafe {
            env::remove_var("GATEWAY_INTERFACE");
            env::remove_var("AWS_LAMBDA_FUNCTION_NAME");
            env::remove_var("LAMBDA_TASK_ROOT");
            env::remove_var("FUNCTION_TARGET");
            env::remove_var("K_SERVICE");
            env::remove_var("GOOGLE_CLOUD_PROJECT");
            env::remove_var("AZURE_FUNCTIONS_ENVIRONMENT");
            env::remove_var("FUNCTIONS_WORKER_RUNTIME");
            env::remove_var("AzureWebJobsStorage");
        }
    }

    #[test]
    fn is_serverless_mode_parity() {
        // server (default) -> false
        clear_env();
        assert!(!is_serverless_mode());

        // lambda -> true
        clear_env();
        unsafe { env::set_var("AWS_LAMBDA_FUNCTION_NAME", "my-fn"); }
        assert!(is_serverless_mode());

        // CGI -> true (CGI is short-lived, counts as serverless).
        clear_env();
        unsafe { env::set_var("GATEWAY_INTERFACE", "CGI/1.1"); }
        assert!(is_serverless_mode());

        // azure -> true
        clear_env();
        unsafe { env::set_var("AZURE_FUNCTIONS_ENVIRONMENT", "Production"); }
        assert!(is_serverless_mode());

        clear_env();
    }
}
