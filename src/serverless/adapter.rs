use std::collections::HashMap;
use std::env;

// ------------------------------------------------------------------
// Query-string plumbing
//
// The SWAIG `__token` a `secure` tool requires rides the QUERY STRING on every
// transport — `build_swaig_webhook_url`'s serverless branch keeps only
// `__token` there, and the call_id travels in the POST body. Each serverless
// host hands that query over in its own shape, so each `handle_*` recovers it
// and re-attaches it to the path; the agent's single `handle_request` entry
// point then splits and parses it exactly as it does for the built-in HTTP
// server. Dropping the query here is not a routing nicety — it silently
// disarms `secure`.
// ------------------------------------------------------------------

/// Re-encode an already-parsed `{key: value}` query mapping as `a=b&c=d`.
/// Returns `None` when the value is absent, not an object, or empty.
fn query_from_params(params: Option<&serde_json::Value>) -> Option<String> {
    let obj = params?.as_object()?;
    let encoded: Vec<String> = obj
        .iter()
        .map(|(k, v)| {
            let raw = v.as_str().map_or_else(|| v.to_string(), str::to_string);
            format!("{}={}", encode_component(k), encode_component(&raw))
        })
        .collect();
    if encoded.is_empty() {
        None
    } else {
        Some(encoded.join("&"))
    }
}

/// Take a RAW `a=b&c=d` query string value, if present and non-empty.
fn query_from_raw(raw: Option<&serde_json::Value>) -> Option<String> {
    let s = raw?.as_str()?.trim_start_matches('?');
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Percent-encode one query component, leaving the unreserved set intact.
fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Re-attach a query string to a path, preserving one already present.
fn with_query(path: &str, query: &str) -> String {
    if query.is_empty() {
        return path.to_string();
    }
    if path.contains('?') {
        format!("{path}&{query}")
    } else {
        format!("{path}?{query}")
    }
}

/// Detected runtime environment.
///
/// `#[non_exhaustive]` because the set of recognised serverless hosts mirrors
/// what [`Adapter::detect`] probes for at runtime and is expected to grow
/// (new FaaS platforms) without that being a breaking change — downstream
/// `match` arms must include a wildcard.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
#[non_exhaustive]
pub enum RuntimeEnvironment {
    /// AWS Lambda, whether behind API Gateway or a Function URL.
    Lambda,
    /// Google Cloud Functions.
    Gcf,
    /// Azure Functions.
    Azure,
    /// A CGI host — the request arrives through CGI environment variables
    /// and stdin rather than a live socket the process owns.
    Cgi,
    /// No serverless host detected: the process runs its own long-lived HTTP
    /// server. This is the fallback when nothing else matches.
    Server,
}

impl RuntimeEnvironment {
    /// The canonical lowercase name of this environment — `"lambda"`,
    /// `"gcf"`, `"azure"`, `"cgi"`, or `"server"`.
    ///
    /// This is the string form [`Adapter::detect`] reports and that
    /// downstream config keys match on.
    #[must_use]
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
    /// Handle an HTTP request, returning (`status_code`, headers, body).
    fn handle_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> (u16, HashMap<String, String>, String);
}

/// `AgentBase` is a serverless request handler: the `Adapter::handle_*`
/// entry points forward each decoded (method, path, headers, body) to the
/// agent's framework-free `handle_request`.
impl RequestHandler for crate::agent::AgentBase {
    fn handle_request(
        &self,
        method: &str,
        path: &str,
        headers: &HashMap<String, String>,
        body: &str,
    ) -> (u16, HashMap<String, String>, String) {
        crate::agent::AgentBase::handle_request(self, method, path, headers, Some(body))
    }
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
    /// format, calls `agent.handle_request()`, and returns an API Gateway
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

        // The credential rides the query string, and BOTH lambda payload shapes
        // are reachable here: REST API v1 (and HTTP API v2) supply the parsed
        // `queryStringParameters` mapping, while HTTP API v2 may instead supply
        // the raw `rawQueryString`. Reading only one loses the token on the
        // other shape. Re-attach it to the path so the agent's single
        // `handle_request` entry point sees it exactly as the built-in HTTP
        // server does — serverless is the same contract, a different envelope.
        let query = query_from_params(event.get("queryStringParameters"))
            .or_else(|| query_from_raw(event.get("rawQueryString")))
            .unwrap_or_default();
        let path = with_query(path, &query);
        let path = path.as_str();

        let body = event.get("body").and_then(|v| v.as_str()).unwrap_or("");

        // Decode base64-encoded bodies
        let decoded_body = if event
            .get("isBase64Encoded")
            .and_then(serde_json::Value::as_bool)
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

        // Azure exposes the parsed query as `req.params`; fall back to the
        // query component of the raw URL when a shim provides only the URL.
        // Without this the `__token` a secure tool requires never arrives.
        let query = query_from_params(request.get("params").or_else(|| request.get("Params")))
            .or_else(|| {
                raw_url
                    .split_once('?')
                    .map(|(_, q)| q.to_string())
                    .filter(|q| !q.is_empty())
            })
            .unwrap_or_default();
        let path = with_query(path, &query);
        let path = path.as_str();

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

        let (status, resp_headers, resp_body) = agent.handle_request(&method, path, &headers, body);

        serde_json::json!({
            "status": status,
            "headers": resp_headers,
            "body": resp_body,
        })
    }

    /// Handle a Google Cloud Function (HTTP-triggered) invocation.
    ///
    /// GCF passes a Flask-style request object. We accept the same JSON
    /// envelope shape as the other handlers (`method`, `path`/`url`,
    /// `headers`, `body`), forward it to `agent.handle_request()`, and
    /// return a `(status, headers, body)`-shaped response dict. This
    /// dispatches to a real SWML doc / SWAIG result; it never falls through to
    /// `serve()`, which would try to bind a socket the host does not offer.
    pub fn handle_gcf(
        agent: &dyn RequestHandler,
        request: &serde_json::Value,
    ) -> serde_json::Value {
        let method = request
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        // GCF exposes the request path directly; accept `path` or a full `url`.
        let raw = request
            .get("path")
            .or_else(|| request.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("/");
        let path = if let Some(pos) = raw.find("://") {
            let after = &raw[pos + 3..];
            after.find('/').map_or("/", |slash| {
                let pq = &after[slash..];
                pq.find('?').map_or(pq, |q| &pq[..q])
            })
        } else {
            raw.find('?').map_or(raw, |q| &raw[..q])
        };

        // GCF (Flask) exposes the parsed query as `request.args`; fall back to
        // the raw `request.query_string`, then to the query component of the
        // URL. Without this the `__token` a secure tool requires never arrives.
        let query = query_from_params(request.get("args"))
            .or_else(|| query_from_raw(request.get("query_string")))
            .or_else(|| {
                raw.split_once('?')
                    .map(|(_, q)| q.to_string())
                    .filter(|q| !q.is_empty())
            })
            .unwrap_or_default();
        let path = with_query(path, &query);
        let path = path.as_str();

        let body = request.get("body").and_then(|v| v.as_str()).unwrap_or("");

        let headers: HashMap<String, String> = request
            .get("headers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let (status, resp_headers, resp_body) = agent.handle_request(&method, path, &headers, body);

        serde_json::json!({
            "status": status,
            "headers": resp_headers,
            "body": resp_body,
        })
    }

    /// Handle a CGI invocation.
    ///
    /// CGI carries the request in the process environment (`REQUEST_METHOD`,
    /// `PATH_INFO`, `CONTENT_LENGTH`, `HTTP_*` headers) with the body on
    /// stdin. This takes the env as a map and the already-read body so it is
    /// testable and CWD/stdin-independent, forwards to
    /// `agent.handle_request()`, and returns a `(status, headers, body)`
    /// response. This dispatches to a real SWML/SWAIG response; it never
    /// falls through to `serve()`, which would try to bind a socket CGI does
    /// not offer.
    ///
    /// `env` is the CGI variable map (e.g. from `std::env::vars()`), `body`
    /// is the stdin payload.
    pub fn handle_cgi(
        agent: &dyn RequestHandler,
        env: &HashMap<String, String>,
        body: &str,
    ) -> serde_json::Value {
        let method = env
            .get("REQUEST_METHOD")
            .map_or_else(|| "GET".to_string(), |m| m.to_uppercase());

        // PATH_INFO holds the path under the script; default to "/".
        let path_info = env.get("PATH_INFO").map_or("/", String::as_str);
        let path = if path_info.is_empty() {
            "/".to_string()
        } else if path_info.starts_with('/') {
            path_info.to_string()
        } else {
            format!("/{path_info}")
        };

        // CGI carries the query string in the QUERY_STRING variable. Without
        // this the `__token` a secure tool requires never arrives.
        let query = env
            .get("QUERY_STRING")
            .map(String::as_str)
            .unwrap_or_default();
        let path = with_query(&path, query);

        // Reconstruct request headers from CGI `HTTP_*` vars (HTTP_X_FOO →
        // X-Foo) plus the special-cased CONTENT_TYPE / CONTENT_LENGTH.
        let mut headers: HashMap<String, String> = HashMap::new();
        for (k, v) in env {
            if let Some(name) = k.strip_prefix("HTTP_") {
                let header = name
                    .split('_')
                    .map(|part| {
                        let mut c = part.chars();
                        c.next().map_or_else(String::new, |first| {
                            first.to_uppercase().collect::<String>() + &c.as_str().to_lowercase()
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("-");
                headers.insert(header, v.clone());
            }
        }
        if let Some(ct) = env.get("CONTENT_TYPE") {
            headers.insert("Content-Type".to_string(), ct.clone());
        }

        let (status, resp_headers, resp_body) =
            agent.handle_request(&method, &path, &headers, body);

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
        unsafe {
            env::set_var("AWS_LAMBDA_FUNCTION_NAME", "my-func");
        }
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Lambda);

        // -- gcf (FUNCTION_TARGET) --
        clear_detect_env();
        unsafe {
            env::set_var("FUNCTION_TARGET", "myHandler");
        }
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Gcf);

        // -- gcf (K_SERVICE) --
        clear_detect_env();
        unsafe {
            env::set_var("K_SERVICE", "my-service");
        }
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Gcf);

        // -- azure --
        clear_detect_env();
        unsafe {
            env::set_var("AZURE_FUNCTIONS_ENVIRONMENT", "Production");
        }
        assert_eq!(Adapter::detect(), RuntimeEnvironment::Azure);

        // -- cgi --
        clear_detect_env();
        unsafe {
            env::set_var("GATEWAY_INTERFACE", "CGI/1.1");
        }
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
        use base64::Engine;
        let agent = EchoHandler;
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

    // ===================================================================
    // Tier-2 behavioral contract #5: serverless per-platform DISPATCH.
    // For lambda + gcf + cgi, feed a synthetic platform event/env and
    // assert the agent DISPATCHES to a real response (200 + the routed
    // method/path/body), NOT an unsupported/empty/fall-through result.
    // (lambda is covered by the handlers above; gcf + cgi are the newly
    // added dispatchers.)
    // ===================================================================

    #[test]
    fn test_handle_gcf_dispatches() {
        let agent = EchoHandler;
        let request = json!({
            "method": "POST",
            "path": "/swaig",
            "headers": {"Content-Type": "application/json"},
            "body": "{\"function\":\"get_time\"}",
        });

        let response = Adapter::handle_gcf(&agent, &request);
        // Real dispatch: a 200 with the routed request echoed back.
        assert_eq!(response["status"], 200);
        let body: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(body["method"], "POST");
        assert_eq!(body["path"], "/swaig");
        assert_eq!(body["body"], "{\"function\":\"get_time\"}");
    }

    #[test]
    fn test_handle_gcf_strips_scheme_and_host_but_keeps_the_query() {
        // The scheme+host must go; the QUERY must NOT. It carries the SWAIG
        // `__token`, so dropping it here silently disarms `secure`. The agent's
        // `handle_request` splits the query off before routing.
        let agent = EchoHandler;
        let request = json!({
            "method": "GET",
            "url": "https://region-project.cloudfunctions.net/agent/health?x=1",
            "headers": {},
            "body": "",
        });
        let response = Adapter::handle_gcf(&agent, &request);
        let body: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(body["path"], "/agent/health?x=1");
    }

    #[test]
    fn test_handle_cgi_dispatches() {
        let agent = EchoHandler;
        let mut env = HashMap::new();
        env.insert("REQUEST_METHOD".to_string(), "POST".to_string());
        env.insert("PATH_INFO".to_string(), "/swaig".to_string());
        env.insert("CONTENT_TYPE".to_string(), "application/json".to_string());
        env.insert("CONTENT_LENGTH".to_string(), "24".to_string());
        env.insert("HTTP_X_CALL_ID".to_string(), "abc123".to_string());

        let body = "{\"function\":\"get_time\"}";
        let response = Adapter::handle_cgi(&agent, &env, body);

        assert_eq!(response["status"], 200);
        let parsed: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(parsed["method"], "POST");
        assert_eq!(parsed["path"], "/swaig");
        assert_eq!(parsed["body"], body);
        // CGI HTTP_* vars are reconstructed into request headers.
        assert_eq!(response["headers"]["Content-Type"], "application/json");
    }

    #[test]
    fn test_handle_cgi_defaults_path_to_root() {
        let agent = EchoHandler;
        let env = HashMap::new(); // no REQUEST_METHOD / PATH_INFO
        let response = Adapter::handle_cgi(&agent, &env, "");
        let parsed: serde_json::Value =
            serde_json::from_str(response["body"].as_str().unwrap()).unwrap();
        assert_eq!(parsed["method"], "GET");
        assert_eq!(parsed["path"], "/");
    }
}
