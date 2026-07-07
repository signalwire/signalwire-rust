// Cross-language SDK contract for serverless / deployment-mode detection.
//
// Mirrors `signalwire.core.logging_config.get_execution_mode` in the
// Python reference. Order of precedence (FIRST match wins):
//
//   1. GATEWAY_INTERFACE                                     -> "cgi"
//   2. AWS_LAMBDA_FUNCTION_NAME or LAMBDA_TASK_ROOT          -> "lambda"
//   3. FUNCTION_TARGET, K_SERVICE, or GOOGLE_CLOUD_PROJECT   -> "google_cloud_function"
//   4. AZURE_FUNCTIONS_ENVIRONMENT, FUNCTIONS_WORKER_RUNTIME,
//      or AzureWebJobsStorage                                -> "azure_function"
//   5. otherwise                                             -> "server"

use std::env;

/// Detect the SDK's deployment environment based on well-known
/// environment variables.
///
/// Returns one of `"cgi"`, `"lambda"`, `"google_cloud_function"`,
/// `"azure_function"`, or `"server"`.
pub fn get_execution_mode() -> String {
    if is_set("GATEWAY_INTERFACE") {
        return String::from("cgi");
    }
    if is_set("AWS_LAMBDA_FUNCTION_NAME") || is_set("LAMBDA_TASK_ROOT") {
        return String::from("lambda");
    }
    if is_set("FUNCTION_TARGET") || is_set("K_SERVICE") || is_set("GOOGLE_CLOUD_PROJECT") {
        return String::from("google_cloud_function");
    }
    if is_set("AZURE_FUNCTIONS_ENVIRONMENT")
        || is_set("FUNCTIONS_WORKER_RUNTIME")
        || is_set("AzureWebJobsStorage")
    {
        return String::from("azure_function");
    }
    String::from("server")
}

fn is_set(name: &str) -> bool {
    match env::var(name) {
        Ok(v) => !v.is_empty(),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Global logging configuration entry points (Python signalwire.core.logging_config).
// Python builds on `structlog`; the Rust port drives the `log`/`env_logger`
// backend via `crate::logging`. Configuration is guarded by a global
// "configured once" flag exactly as Python's `_logging_configured`.
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicBool, Ordering};

/// Global "configured once" flag (Python's `_logging_configured`).
static LOGGING_CONFIGURED: AtomicBool = AtomicBool::new(false);

/// Strip control characters from a string to prevent log injection.
///
/// Removes the C0/C1 control ranges (`\x00-\x08`, `\x0b`, `\x0c`,
/// `\x0e-\x1f`, `\x7f-\x9f`) — the same set as Python's `_CONTROL_CHAR_RE`.
/// Python operates on a structlog `event_dict`; the Rust surface analog
/// sanitizes a single log value (the unit callers actually have).
#[must_use]
pub fn strip_control_chars(value: &str) -> String {
    value
        .chars()
        .filter(|c| {
            let n = *c as u32;
            !((0x00..=0x08).contains(&n)
                || n == 0x0b
                || n == 0x0c
                || (0x0e..=0x1f).contains(&n)
                || (0x7f..=0x9f).contains(&n))
        })
        .collect()
}

/// Configure the logging system once, globally, from the environment.
///
/// Honors `SIGNALWIRE_LOG_MODE` / `SIGNALWIRE_LOG_LEVEL` via the underlying
/// backend. Idempotent — subsequent calls are no-ops until
/// [`reset_logging_configuration`].
pub fn configure_logging() {
    if LOGGING_CONFIGURED.swap(true, Ordering::SeqCst) {
        return;
    }
    crate::logging::init();
}

/// Reset the configuration flag so a later [`configure_logging`] reconfigures.
pub fn reset_logging_configuration() {
    LOGGING_CONFIGURED.store(false, Ordering::SeqCst);
}

/// Get a logger for `name`, ensuring logging is configured first. The single
/// entry point for SDK logging.
#[must_use]
pub fn get_logger(name: &str) -> crate::logging::Logger {
    configure_logging();
    crate::logging::Logger::new(name)
}

#[cfg(test)]
mod logging_setup_tests {
    use super::*;

    #[test]
    fn test_strip_control_chars_removes_control_bytes() {
        assert_eq!(
            strip_control_chars("hello\x00wor\x1bld\x07!"),
            "helloworld!"
        );
    }

    #[test]
    fn test_strip_control_chars_keeps_printable_and_whitespace() {
        let s = "line1\tcol\nline2\r end";
        assert_eq!(strip_control_chars(s), s);
    }

    #[test]
    fn test_strip_control_chars_removes_c1_range() {
        assert_eq!(strip_control_chars("a\u{0085}b\u{009f}c"), "abc");
    }

    #[test]
    fn test_get_logger_returns_named_logger() {
        get_logger("test.module").debug("configured");
    }

    #[test]
    fn test_configure_and_reset_are_idempotent() {
        reset_logging_configuration();
        configure_logging();
        configure_logging();
        reset_logging_configuration();
        configure_logging();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: clear every env var the function inspects so a leaked
    /// var from another test (or the developer's shell) doesn't poison
    /// the assertion.
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

    /// One big test exercising every branch and the precedence ladder.
    /// `cargo test` runs unit tests in parallel by default; sharing
    /// process env vars means we MUST keep this all in a single test
    /// or risk inter-test races.
    #[test]
    fn execution_mode_parity_branches_and_precedence() {
        // -- server (default) --
        clear_env();
        assert_eq!(get_execution_mode(), "server");

        // -- cgi via GATEWAY_INTERFACE --
        clear_env();
        unsafe {
            env::set_var("GATEWAY_INTERFACE", "CGI/1.1");
        }
        assert_eq!(get_execution_mode(), "cgi");

        // -- lambda via AWS_LAMBDA_FUNCTION_NAME --
        clear_env();
        unsafe {
            env::set_var("AWS_LAMBDA_FUNCTION_NAME", "my-fn");
        }
        assert_eq!(get_execution_mode(), "lambda");

        // -- lambda via LAMBDA_TASK_ROOT --
        clear_env();
        unsafe {
            env::set_var("LAMBDA_TASK_ROOT", "/var/task");
        }
        assert_eq!(get_execution_mode(), "lambda");

        // -- google_cloud_function via FUNCTION_TARGET --
        clear_env();
        unsafe {
            env::set_var("FUNCTION_TARGET", "my_handler");
        }
        assert_eq!(get_execution_mode(), "google_cloud_function");

        // -- google_cloud_function via K_SERVICE --
        clear_env();
        unsafe {
            env::set_var("K_SERVICE", "svc");
        }
        assert_eq!(get_execution_mode(), "google_cloud_function");

        // -- google_cloud_function via GOOGLE_CLOUD_PROJECT --
        clear_env();
        unsafe {
            env::set_var("GOOGLE_CLOUD_PROJECT", "proj");
        }
        assert_eq!(get_execution_mode(), "google_cloud_function");

        // -- azure_function via AZURE_FUNCTIONS_ENVIRONMENT --
        clear_env();
        unsafe {
            env::set_var("AZURE_FUNCTIONS_ENVIRONMENT", "Production");
        }
        assert_eq!(get_execution_mode(), "azure_function");

        // -- azure_function via FUNCTIONS_WORKER_RUNTIME --
        clear_env();
        unsafe {
            env::set_var("FUNCTIONS_WORKER_RUNTIME", "rust");
        }
        assert_eq!(get_execution_mode(), "azure_function");

        // -- azure_function via AzureWebJobsStorage --
        clear_env();
        unsafe {
            env::set_var("AzureWebJobsStorage", "DefaultEndpointsProtocol=https");
        }
        assert_eq!(get_execution_mode(), "azure_function");

        // -- precedence: CGI > Lambda --
        clear_env();
        unsafe {
            env::set_var("GATEWAY_INTERFACE", "CGI/1.1");
            env::set_var("AWS_LAMBDA_FUNCTION_NAME", "my-fn");
        }
        assert_eq!(get_execution_mode(), "cgi");

        // -- precedence: Lambda > GCF --
        clear_env();
        unsafe {
            env::set_var("AWS_LAMBDA_FUNCTION_NAME", "my-fn");
            env::set_var("FUNCTION_TARGET", "h");
        }
        assert_eq!(get_execution_mode(), "lambda");

        // -- precedence: GCF > Azure --
        clear_env();
        unsafe {
            env::set_var("FUNCTION_TARGET", "h");
            env::set_var("AZURE_FUNCTIONS_ENVIRONMENT", "Production");
        }
        assert_eq!(get_execution_mode(), "google_cloud_function");

        // -- cleanup so subsequent tests aren't polluted --
        clear_env();
    }
}
