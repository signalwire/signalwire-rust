use std::collections::HashMap;
use std::env;

/// Detected runtime environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEnvironment {
    Lambda,
    Gcf,
    Azure,
    Cgi,
    Server,
}

impl RuntimeEnvironment {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeEnvironment::Lambda => "lambda",
            RuntimeEnvironment::Gcf => "gcf",
            RuntimeEnvironment::Azure => "azure",
            RuntimeEnvironment::Cgi => "cgi",
            RuntimeEnvironment::Server => "server",
        }
    }
}

/// Trait that the agent/service must implement so the adapter can
/// forward requests to it.
pub trait RequestHandler {
    /// Handle an HTTP request, returning (status_code, headers, body).
    fn handle_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> (u16, HashMap<String, String>, String);
}

/// Auto-detect and handle serverless environments (Lambda, Azure, GCF, CGI)
/// or fall back to the built-in server.
pub struct Adapter;

impl Adapter {
    /// Detect the current runtime environment.
    pub fn detect() -> RuntimeEnvironment {
        if env::var("AWS_LAMBDA_FUNCTION_NAME").is_ok() {
            return RuntimeEnvironment::Lambda;
        }
        if env::var("FUNCTION_TARGET").is_ok() || env::var("K_SERVICE").is_ok() {
            return RuntimeEnvironment::Gcf;
        }
        if env::var("AZURE_FUNCTIONS_ENVIRONMENT").is_ok() {
            return RuntimeEnvironment::Azure;
        }
        // CGI detection: check for GATEWAY_INTERFACE env var
        if env::var("GATEWAY_INTERFACE").is_ok() {
            return RuntimeEnvironment::Cgi;
        }
        RuntimeEnvironment::Server
    }

    /// Handle an AWS Lambda (API Gateway) invocation.
    ///
    /// Extracts method, path, headers, and body from the API Gateway event
    /// format, calls agent.handle_request(), and returns an API Gateway
    /// compatible response.
    pub fn handle_lambda(
        agent: &dyn RequestHandler,
        event: &serde_json::Value,
    ) -> serde_json::Value {
        let method = event
            .get("httpMethod")
            .and_then(|v| v.as_str())
            .or_else(|| {
                event
                    .get("requestContext")
                    .and_then(|rc| rc.get("http"))
                    .and_then(|h| h.get("method"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("GET")
            .to_uppercase();

        let path = event
            .get("path")
            .or_else(|| event.get("rawPath"))
            .and_then(|v| v.as_str())
            .unwrap_or("/");

        let body = event.get("body").and_then(|v| v.as_str()).unwrap_or("");

        // Decode base64-encoded bodies
        let decoded_body = if event
            .get("isBase64Encoded")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(body)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .unwrap_or_default()
        } else {
            body.to_string()
        };

        // Extract headers
        let headers: HashMap<String, String> = event
            .get("headers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let (status, resp_headers, resp_body) =
            agent.handle_request(&method, path, &headers, &decoded_body);

        serde_json::json!({
            "statusCode": status,
            "headers": resp_headers,
            "body": resp_body,
        })
    }

    /// Handle an Azure Functions invocation.
    pub fn handle_azure(
        agent: &dyn RequestHandler,
        request: &serde_json::Value,
    ) -> serde_json::Value {
        let method = request
            .get("method")
            .or_else(|| request.get("Method"))
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let raw_url = request
            .get("url")
            .or_else(|| request.get("Url"))
            .and_then(|v| v.as_str())
            .unwrap_or("/");

        // Parse the URL to extract just the path
        let path = if let Some(pos) = raw_url.find("://") {
            let after_scheme = &raw_url[pos + 3..];
            if let Some(slash_pos) = after_scheme.find('/') {
                let path_and_query = &after_scheme[slash_pos..];
                if let Some(q) = path_and_query.find('?') {
                    &path_and_query[..q]
                } else {
                    path_and_query
                }
            } else {
                "/"
            }
        } else if let Some(q) = raw_url.find('?') {
            &raw_url[..q]
        } else {
            raw_url
        };

        let body = request
            .get("body")
            .or_else(|| request.get("Body"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let headers: HashMap<String, String> = request
            .get("headers")
            .or_else(|| request.get("Headers"))
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let (status, resp_headers, resp_body) =
            agent.handle_request(&method, path, &headers, body);

        serde_json::json!({
            "status": status,
            "headers": resp_headers,
            "body": resp_body,
        })
    }

    /// Auto-detect the runtime environment and return the environment type.
    /// The caller can then dispatch accordingly.
    pub fn serve_detect() -> RuntimeEnvironment {
        Self::detect()
    }

    /// Return the standard HTTP status text for a given status code.
    pub fn status_text(code: u16) -> &'static str {
        match code {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            413 => "Payload Too Large",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Unknown",
        }
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A simple test handler that echoes method and path.
    struct EchoHandler;

    impl RequestHandler for EchoHandler {
        fn handle_request(
            &self,
            method: &str,
            path: &str,
            _headers: &HashMap<String, String>,
            body: &str,
        ) -> (u16, HashMap<String, String>, String) {
            let mut h = HashMap::new();
            h.insert("Content-Type".to_string(), "application/json".to_string());
            (
                200,
                h,
                serde_json::json!({
                    "method": method,
                    "path": path,
                    "body": body,
                })
                .to_string(),
            )
        }
    }

    /// Helper: clear all serverless-detection env vars.
    fn clear_detect_env() {
        unsafe {
            env::remove_var("AWS_LAMBDA_FUNCTION_NAME");
            env::remove_var("FUNCTION_TARGET");
            env::remove_var("K_SERVICE");
            env::remove_var("AZURE_FUNCTIONS_ENVIRONMENT");
            env::remove_var("GATEWAY_INTERFACE");
        }
    }

    /// Combined test for environment detection to avoid env-var races
    /// between parallel tests.
    #[test]
    fn test_detect_all_environments() {
        // -- server (default) --
        clear_detect_env();
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Server);

        // -- lambda --
        clear_detect_env();
        unsafe { env::set_var("AWS_LAMBDA_FUNCTION_NAME", "my-func"); }
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Lambda);

        // -- gcf (FUNCTION_TARGET) --
        clear_detect_env();
        unsafe { env::set_var("FUNCTION_TARGET", "myHandler"); }
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Gcf);

        // -- gcf (K_SERVICE) --
        clear_detect_env();
        unsafe { env::set_var("K_SERVICE", "my-service"); }
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Gcf);

        // -- azure --
        clear_detect_env();
        unsafe { env::set_var("AZURE_FUNCTIONS_ENVIRONMENT", "Production"); }
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Azure);

        // -- cgi --
        clear_detect_env();
        unsafe { env::set_var("GATEWAY_INTERFACE", "CGI/1.1"); }
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Cgi);

        // cleanup
        clear_detect_env();
    }

    #[test]
    fn test_runtime_environment_as_str() {
        assert_eq!(RuntimeEnvironment::Lambda.as_str(), "lambda");
        assert_eq!(RuntimeEnvironment::Gcf.as_str(), "gcf");
        assert_eq!(RuntimeEnvironment::Azure.as_str(), "azure");
        assert_eq!(RuntimeEnvironment::Cgi.as_str(), "cgi");
        assert_eq!(RuntimeEnvironment::Server.as_str(), "server");
    }

    #[test]
    fn test_handle_lambda_basic() {
        let agent = EchoHandler;
        let event = json!({
            "httpMethod": "POST",
            "path": "/api/test",
            "headers": {"Content-Type": "application/json"},
            "body": "{\"key\":\"value\"}",
        });

        let response = Adapter::handle_lambda(&agent, &event);
        assert_eq!(response["statusCode"], 200);

        let body: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(body["method"], "POST");
        assert_eq!(body["path"], "/api/test");
    }

    #[test]
    fn test_handle_lambda_v2_format() {
        let agent = EchoHandler;
        let event = json!({
            "requestContext": {"http": {"method": "GET"}},
            "rawPath": "/v2/test",
            "headers": {},
        });

        let response = Adapter::handle_lambda(&agent, &event);
        let body: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(body["method"], "GET");
        assert_eq!(body["path"], "/v2/test");
    }

    #[test]
    fn test_handle_lambda_base64_body() {
        let agent = EchoHandler;
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode("decoded body");
        let event = json!({
            "httpMethod": "POST",
            "path": "/",
            "headers": {},
            "body": encoded,
            "isBase64Encoded": true,
        });

        let response = Adapter::handle_lambda(&agent, &event);
        let body: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(body["body"], "decoded body");
    }

    #[test]
    fn test_handle_lambda_defaults() {
        let agent = EchoHandler;
        let event = json!({});

        let response = Adapter::handle_lambda(&agent, &event);
        let body: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(body["method"], "GET");
        assert_eq!(body["path"], "/");
    }

    #[test]
    fn test_handle_azure_basic() {
        let agent = EchoHandler;
        let request = json!({
            "method": "POST",
            "url": "https://app.azurewebsites.net/api/handler",
            "headers": {"Authorization": "Bearer xyz"},
            "body": "hello",
        });

        let response = Adapter::handle_azure(&agent, &request);
        assert_eq!(response["status"], 200);

        let body: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(body["method"], "POST");
        assert_eq!(body["path"], "/api/handler");
        assert_eq!(body["body"], "hello");
    }

    #[test]
    fn test_handle_azure_uppercase_keys() {
        let agent = EchoHandler;
        let request = json!({
            "Method": "PUT",
            "Url": "/api/test",
            "Headers": {},
            "Body": "data",
        });

        let response = Adapter::handle_azure(&agent, &request);
        let body: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(body["method"], "PUT");
    }

    #[test]
    fn test_handle_azure_defaults() {
        let agent = EchoHandler;
        let request = json!({});

        let response = Adapter::handle_azure(&agent, &request);
        let body: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(body["method"], "GET");
        assert_eq!(body["path"], "/");
    }

    #[test]
    fn test_status_text() {
        assert_eq!(Adapter::status_text(200), "OK");
        assert_eq!(Adapter::status_text(201), "Created");
        assert_eq!(Adapter::status_text(204), "No Content");
        assert_eq!(Adapter::status_text(400), "Bad Request");
        assert_eq!(Adapter::status_text(401), "Unauthorized");
        assert_eq!(Adapter::status_text(403), "Forbidden");
        assert_eq!(Adapter::status_text(404), "Not Found");
        assert_eq!(Adapter::status_text(413), "Payload Too Large");
        assert_eq!(Adapter::status_text(500), "Internal Server Error");
        assert_eq!(Adapter::status_text(502), "Bad Gateway");
        assert_eq!(Adapter::status_text(503), "Service Unavailable");
        assert_eq!(Adapter::status_text(999), "Unknown");
    }

    #[test]
    fn test_serve_detect() {
        // Note: serve_detect() just calls detect(), so we only verify
        // it does not panic.  The combined env detection test above
        // covers the full matrix.
        let _env = Adapter::serve_detect();
    }
}
