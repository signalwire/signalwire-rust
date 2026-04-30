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
    if is_set("FUNCTION_TARGET")
        || is_set("K_SERVICE")
        || is_set("GOOGLE_CLOUD_PROJECT")
    {
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
        unsafe { env::set_var("GATEWAY_INTERFACE", "CGI/1.1"); }
        assert_eq!(get_execution_mode(), "cgi");

        // -- lambda via AWS_LAMBDA_FUNCTION_NAME --
        clear_env();
        unsafe { env::set_var("AWS_LAMBDA_FUNCTION_NAME", "my-fn"); }
        assert_eq!(get_execution_mode(), "lambda");

        // -- lambda via LAMBDA_TASK_ROOT --
        clear_env();
        unsafe { env::set_var("LAMBDA_TASK_ROOT", "/var/task"); }
        assert_eq!(get_execution_mode(), "lambda");

        // -- google_cloud_function via FUNCTION_TARGET --
        clear_env();
        unsafe { env::set_var("FUNCTION_TARGET", "my_handler"); }
        assert_eq!(get_execution_mode(), "google_cloud_function");

        // -- google_cloud_function via K_SERVICE --
        clear_env();
        unsafe { env::set_var("K_SERVICE", "svc"); }
        assert_eq!(get_execution_mode(), "google_cloud_function");

        // -- google_cloud_function via GOOGLE_CLOUD_PROJECT --
        clear_env();
        unsafe { env::set_var("GOOGLE_CLOUD_PROJECT", "proj"); }
        assert_eq!(get_execution_mode(), "google_cloud_function");

        // -- azure_function via AZURE_FUNCTIONS_ENVIRONMENT --
        clear_env();
        unsafe { env::set_var("AZURE_FUNCTIONS_ENVIRONMENT", "Production"); }
        assert_eq!(get_execution_mode(), "azure_function");

        // -- azure_function via FUNCTIONS_WORKER_RUNTIME --
        clear_env();
        unsafe { env::set_var("FUNCTIONS_WORKER_RUNTIME", "rust"); }
        assert_eq!(get_execution_mode(), "azure_function");

        // -- azure_function via AzureWebJobsStorage --
        clear_env();
        unsafe { env::set_var("AzureWebJobsStorage", "DefaultEndpointsProtocol=https"); }
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
