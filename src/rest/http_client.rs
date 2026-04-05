use std::collections::HashMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;

use super::error::SignalWireRestError;

/// Trait for the HTTP transport layer.
///
/// Production code uses a real implementation (e.g. ureq), while
/// tests inject a mock.
pub trait HttpTransport: Send + Sync {
    fn execute(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<(u16, String), String>;
}

/// A stub transport that records requests and returns canned responses.
/// Useful for unit testing without network access.
pub struct StubTransport {
    /// Canned response: (status_code, body).
    pub response: std::sync::Mutex<(u16, String)>,
    /// Recorded requests: (method, url, body).
    pub requests: std::sync::Mutex<Vec<(String, String, Option<String>)>>,
}

impl StubTransport {
    pub fn new(status: u16, body: &str) -> Self {
        StubTransport {
            response: std::sync::Mutex::new((status, body.to_string())),
            requests: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn set_response(&self, status: u16, body: &str) {
        *self.response.lock().unwrap() = (status, body.to_string());
    }
}

impl HttpTransport for StubTransport {
    fn execute(
        &self,
        method: &str,
        url: &str,
        _headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<(u16, String), String> {
        self.requests.lock().unwrap().push((
            method.to_string(),
            url.to_string(),
            body.map(|s| s.to_string()),
        ));
        let resp = self.response.lock().unwrap().clone();
        Ok(resp)
    }
}

/// Low-level HTTP client for SignalWire REST APIs.
///
/// Uses Basic Auth with project_id:token and returns parsed JSON
/// responses as `serde_json::Value`.
pub struct HttpClient {
    project_id: String,
    token: String,
    base_url: String,
    auth_header: String,
    user_agent: String,
    transport: Box<dyn HttpTransport>,
}

impl HttpClient {
    pub fn new(
        project_id: &str,
        token: &str,
        base_url: &str,
        transport: Box<dyn HttpTransport>,
    ) -> Self {
        let auth_header = format!(
            "Basic {}",
            BASE64.encode(format!("{}:{}", project_id, token))
        );
        HttpClient {
            project_id: project_id.to_string(),
            token: token.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header,
            user_agent: "signalwire-agents-rust-rest/1.0".to_string(),
            transport,
        }
    }

    /// Create with a stub transport for testing.
    pub fn with_stub(project_id: &str, token: &str, base_url: &str) -> (Self, std::sync::Arc<StubTransport>) {
        let stub = std::sync::Arc::new(StubTransport::new(200, "{}"));
        let client = HttpClient::new(
            project_id,
            token,
            base_url,
            Box::new(StubTransportWrapper(stub.clone())),
        );
        (client, stub)
    }

    // -- Accessors --

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn auth_header(&self) -> &str {
        &self.auth_header
    }

    // -- HTTP methods --

    pub fn get(&self, path: &str, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.request("GET", path, params, None)
    }

    pub fn post(&self, path: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        let body = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
        self.request("POST", path, &HashMap::new(), Some(&body))
    }

    pub fn put(&self, path: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        let body = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
        self.request("PUT", path, &HashMap::new(), Some(&body))
    }

    pub fn patch(&self, path: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        let body = serde_json::to_string(data).unwrap_or_else(|_| "{}".to_string());
        self.request("PATCH", path, &HashMap::new(), Some(&body))
    }

    pub fn delete(&self, path: &str) -> Result<Value, SignalWireRestError> {
        self.request("DELETE", path, &HashMap::new(), None)
    }

    // -- Paginated list support --

    /// Return all pages of results, following `links.next`.
    pub fn list_all(&self, path: &str, params: &HashMap<String, String>) -> Result<Vec<Value>, SignalWireRestError> {
        let mut all_pages = Vec::new();
        let mut current_path = path.to_string();
        let mut current_params = params.clone();

        loop {
            let response = self.get(&current_path, &current_params)?;

            // Extract data items
            let data = response
                .get("data")
                .cloned()
                .unwrap_or_else(|| response.clone());
            if let Some(arr) = data.as_array() {
                all_pages.extend(arr.iter().cloned());
            }

            // Check for next page
            let next_url = response
                .get("links")
                .and_then(|l| l.get("next"))
                .and_then(|n| n.as_str());

            match next_url {
                Some(url) if !url.is_empty() => {
                    // Parse next URL
                    if url.starts_with("http") {
                        // Absolute URL -- extract path + query
                        if let Some(q_pos) = url.find('?') {
                            current_path = url[..q_pos].to_string();
                            // Strip base URL from path
                            if current_path.starts_with(&self.base_url) {
                                current_path = current_path[self.base_url.len()..].to_string();
                            }
                            current_params = parse_query_string(&url[q_pos + 1..]);
                        } else {
                            current_path = url.to_string();
                            if current_path.starts_with(&self.base_url) {
                                current_path = current_path[self.base_url.len()..].to_string();
                            }
                            current_params = HashMap::new();
                        }
                    } else {
                        let parts: Vec<&str> = url.splitn(2, '?').collect();
                        current_path = parts[0].to_string();
                        current_params = if parts.len() > 1 {
                            parse_query_string(parts[1])
                        } else {
                            HashMap::new()
                        };
                    }
                }
                _ => break,
            }
        }

        Ok(all_pages)
    }

    // -- Internal request engine --

    fn request(
        &self,
        method: &str,
        path: &str,
        params: &HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<Value, SignalWireRestError> {
        let mut url = format!("{}{}", self.base_url, path);

        if !params.is_empty() {
            let qs: String = params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url = format!("{}?{}", url, qs);
        }

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), self.auth_header.clone());
        headers.insert("User-Agent".to_string(), self.user_agent.clone());

        let (status, response_body) = self
            .transport
            .execute(method, &url, &headers, body)
            .map_err(|e| {
                SignalWireRestError::new(
                    &format!("{} {} failed: {}", method, path, e),
                    0,
                    "",
                )
            })?;

        // Non-2xx
        if status < 200 || status >= 300 {
            return Err(SignalWireRestError::new(
                &format!("{} {} returned {}", method, path, status),
                status,
                &response_body,
            ));
        }

        // 204 or empty body
        if status == 204 || response_body.is_empty() {
            return Ok(serde_json::json!({}));
        }

        serde_json::from_str(&response_body).map_err(|_| {
            SignalWireRestError::new(
                &format!("{} {} returned non-JSON", method, path),
                status,
                &response_body,
            )
        })
    }
}

/// Wrapper so Arc<StubTransport> implements HttpTransport.
struct StubTransportWrapper(std::sync::Arc<StubTransport>);

impl HttpTransport for StubTransportWrapper {
    fn execute(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<(u16, String), String> {
        self.0.execute(method, url, headers, body)
    }
}

/// Parse a query string into a HashMap.
fn parse_query_string(qs: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for pair in qs.split('&') {
        let mut kv = pair.splitn(2, '=');
        if let (Some(k), Some(v)) = (kv.next(), kv.next()) {
            map.insert(k.to_string(), v.to_string());
        }
    }
    map
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_client() -> (HttpClient, std::sync::Arc<StubTransport>) {
        HttpClient::with_stub("proj-1", "tok-1", "https://test.signalwire.com")
    }

    #[test]
    fn test_new() {
        let (client, _stub) = make_client();
        assert_eq!(client.project_id(), "proj-1");
        assert_eq!(client.token(), "tok-1");
        assert_eq!(client.base_url(), "https://test.signalwire.com");
        assert!(client.auth_header().starts_with("Basic "));
    }

    #[test]
    fn test_auth_header_encoding() {
        let (client, _) = make_client();
        let expected = format!("Basic {}", BASE64.encode("proj-1:tok-1"));
        assert_eq!(client.auth_header(), expected);
    }

    #[test]
    fn test_get() {
        let (client, stub) = make_client();
        stub.set_response(200, r#"{"data": [1,2,3]}"#);

        let result = client.get("/api/test", &HashMap::new()).unwrap();
        assert_eq!(result["data"], json!([1, 2, 3]));

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "GET");
        assert!(reqs[0].1.contains("/api/test"));
    }

    #[test]
    fn test_get_with_params() {
        let (client, stub) = make_client();
        stub.set_response(200, "{}");

        let mut params = HashMap::new();
        params.insert("page".to_string(), "2".to_string());
        client.get("/api/test", &params).unwrap();

        let reqs = stub.requests.lock().unwrap();
        assert!(reqs[0].1.contains("page=2"));
    }

    #[test]
    fn test_post() {
        let (client, stub) = make_client();
        stub.set_response(201, r#"{"id":"new-1"}"#);

        let data = json!({"name": "test"});
        let result = client.post("/api/test", &data).unwrap();
        assert_eq!(result["id"], "new-1");

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "POST");
        assert!(reqs[0].2.as_ref().unwrap().contains("test"));
    }

    #[test]
    fn test_put() {
        let (client, stub) = make_client();
        stub.set_response(200, r#"{"updated":true}"#);

        let result = client.put("/api/test/1", &json!({"name": "updated"})).unwrap();
        assert_eq!(result["updated"], true);

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PUT");
    }

    #[test]
    fn test_patch() {
        let (client, stub) = make_client();
        stub.set_response(200, r#"{"patched":true}"#);

        let result = client.patch("/api/test/1", &json!({"field": "val"})).unwrap();
        assert_eq!(result["patched"], true);

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PATCH");
    }

    #[test]
    fn test_delete() {
        let (client, stub) = make_client();
        stub.set_response(204, "");

        let result = client.delete("/api/test/1").unwrap();
        assert!(result.is_object());

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "DELETE");
    }

    #[test]
    fn test_error_on_non_2xx() {
        let (client, stub) = make_client();
        stub.set_response(404, r#"{"error":"not found"}"#);

        let err = client.get("/api/missing", &HashMap::new()).unwrap_err();
        assert_eq!(err.status_code(), 404);
        assert!(err.message().contains("404"));
    }

    #[test]
    fn test_error_on_500() {
        let (client, stub) = make_client();
        stub.set_response(500, "server error");

        let err = client.get("/api/fail", &HashMap::new()).unwrap_err();
        assert_eq!(err.status_code(), 500);
    }

    #[test]
    fn test_list_all_single_page() {
        let (client, stub) = make_client();
        stub.set_response(200, r#"{"data": [{"id":1},{"id":2}]}"#);

        let items = client.list_all("/api/items", &HashMap::new()).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn test_list_all_no_data_key() {
        let (client, stub) = make_client();
        stub.set_response(200, r#"[{"id":1}]"#);

        let items = client.list_all("/api/items", &HashMap::new()).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn test_parse_query_string() {
        let qs = "page=2&limit=10";
        let parsed = parse_query_string(qs);
        assert_eq!(parsed["page"], "2");
        assert_eq!(parsed["limit"], "10");
    }

    #[test]
    fn test_parse_query_string_empty() {
        let parsed = parse_query_string("");
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_204_returns_empty_object() {
        let (client, stub) = make_client();
        stub.set_response(204, "");
        let result = client.delete("/api/test/1").unwrap();
        assert!(result.is_object());
    }

    #[test]
    fn test_empty_body_200() {
        let (client, stub) = make_client();
        stub.set_response(200, "");
        let result = client.get("/api/test", &HashMap::new()).unwrap();
        assert!(result.is_object());
    }
}
