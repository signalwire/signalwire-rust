use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::Value;

use super::error::SignalWireRestError;
use super::request_options::{RequestOptions, resolve, status_is_retryable};

/// Wraps an [`HttpTransport::execute`] failure message so it can be preserved as
/// a [`SignalWireRestError`]'s `source()` (`std::error::Error`) — the underlying
/// transport failure (connection refused, DNS, reset, TLS) reduces to a `String`
/// at the `HttpTransport` boundary; this newtype gives that message a real
/// `std::error::Error` identity so the cause chain survives instead of being
/// flattened into the error's display message alone.
#[derive(Debug)]
struct TransportFailure(String);

impl fmt::Display for TransportFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TransportFailure {}

/// Trait for the HTTP transport layer.
///
/// Production code uses a real implementation (e.g. ureq), while
/// tests inject a mock.
pub trait HttpTransport: Send + Sync {
    /// Execute a single HTTP request and return `(status_code, body)`.
    ///
    /// # Errors
    /// Returns `Err(String)` describing the failure when the request cannot
    /// be performed — the method is unsupported, the connection to the Space
    /// cannot be established (transport/network failure), or the response body
    /// cannot be read. A non-2xx HTTP status is *not* an error here; it is
    /// returned as the status code for the caller to interpret.
    ///
    /// `timeout` is the per-attempt wall-clock deadline (from the resolved
    /// [`RequestOptions`]); a real transport applies it to this single call and
    /// surfaces an exceed as an `Err` (which the client wraps into its typed
    /// transport error). Stub transports ignore it.
    fn execute(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
        timeout: Duration,
    ) -> Result<(u16, String), String>;

    /// Like [`execute`](Self::execute) but also returns the RESPONSE headers
    /// (§6.6 error-observability). The default delegates to `execute` and
    /// reports no headers, so existing (stub/mock) transports need no change;
    /// the real [`UreqTransport`] overrides this to capture the response headers
    /// so the client can surface the platform `request-id` on an error. Returns
    /// `(status_code, response_headers, body)`.
    ///
    /// # Errors
    /// Same as [`execute`](Self::execute): `Err(String)` when the request cannot
    /// be performed (unsupported method, transport/network failure, or the body
    /// cannot be read). A non-2xx status is returned as the status, not an error.
    fn execute_with_headers(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
        timeout: Duration,
    ) -> Result<(u16, HashMap<String, String>, String), String> {
        let (status, body) = self.execute(method, url, headers, body, timeout)?;
        Ok((status, HashMap::new(), body))
    }
}

/// Real HTTP transport backed by ureq.
///
/// This is the production transport — every REST namespace operation
/// goes through `ureq::Agent::request()` to a real HTTP endpoint. The
/// REST audit fixture (`audit_rest_transport.py`) drives the wire
/// shape (method, path, headers, body) end-to-end against this code,
/// so any regression in serialization is caught.
pub struct UreqTransport {
    agent: ureq::Agent,
    /// Whether `SIGNALWIRE_REST_CA_FILE` supplied a custom trust anchor at
    /// construction. Recorded because the resulting `TlsConfig` is silently
    /// unused on a plaintext URL — see the downgrade refusal in `execute_raw`.
    custom_ca: bool,
}

impl Default for UreqTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl UreqTransport {
    /// Build the real network transport.
    ///
    /// The agent carries **no** fixed global timeout — the per-attempt
    /// deadline comes from the resolved `RequestOptions.timeout` and is
    /// applied at call time. `http_status_as_error(false)` keeps a non-2xx
    /// response an ordinary response for the client to interpret, rather
    /// than a transport error.
    ///
    /// TLS verifies against the bundled webpki (Mozilla) roots and, like all
    /// rustls users, ignores `SSL_CERT_FILE` and the OS trust store. To
    /// trust a private or self-signed CA, point `SIGNALWIRE_REST_CA_FILE` at
    /// a PEM bundle; it is then loaded as the **only** trust anchor.
    /// Verification is always performed — there is no accept-invalid escape
    /// hatch.
    pub fn new() -> Self {
        // No fixed global timeout on the agent: the per-attempt deadline is
        // supplied per request (from the resolved RequestOptions.timeout) and
        // applied at call time in `execute`. `http_status_as_error(false)` keeps
        // a non-2xx a normal response (the client interprets the status), never
        // a transport error.
        let mut builder = ureq::Agent::config_builder().http_status_as_error(false);

        // ureq verifies against the bundled webpki (Mozilla) roots by default,
        // and — like all rustls users — ignores SSL_CERT_FILE / the OS store.
        // To trust a private / self-signed CA over HTTPS (e.g. the porting-sdk
        // test CA, or a corporate proxy CA), set SIGNALWIRE_REST_CA_FILE to a
        // PEM bundle; we load it as the *only* trust anchor. Real verification
        // against a caller-chosen CA — never disabled / accept-invalid.
        let custom_ca = match custom_ca_tls_config() {
            Some(tls_config) => {
                builder = builder.tls_config(tls_config);
                true
            }
            None => false,
        };

        let agent: ureq::Agent = builder.build().into();
        UreqTransport { agent, custom_ca }
    }
}

/// Build a ureq `TlsConfig` trusting *only* the PEM CA bundle named by
/// `SIGNALWIRE_REST_CA_FILE`, or `None` when the env var is unset/empty (the
/// default webpki-roots path). Panics with a clear message if the file is set
/// but unreadable / contains no certificate — a misconfigured CA should fail
/// loudly, not silently fall back to the wrong trust store.
fn custom_ca_tls_config() -> Option<ureq::tls::TlsConfig> {
    let ca_path = std::env::var("SIGNALWIRE_REST_CA_FILE")
        .ok()
        .filter(|s| !s.is_empty())?;
    let pem = std::fs::read(&ca_path)
        .unwrap_or_else(|e| panic!("read SIGNALWIRE_REST_CA_FILE {ca_path}: {e}"));
    let cert = ureq::tls::Certificate::from_pem(&pem)
        .unwrap_or_else(|e| panic!("parse SIGNALWIRE_REST_CA_FILE {ca_path}: {e}"));
    let root_certs = ureq::tls::RootCerts::new_with_certs(&[cert]);
    Some(
        ureq::tls::TlsConfig::builder()
            .root_certs(root_certs)
            .build(),
    )
}

impl UreqTransport {
    /// Perform the request and return `(status, response_headers, body)`. Shared
    /// by [`execute`](HttpTransport::execute) (which drops the headers) and
    /// [`execute_with_headers`](HttpTransport::execute_with_headers) (which keeps
    /// them for §6.6 request-id observability).
    fn execute_raw(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
        timeout: Duration,
    ) -> Result<(u16, HashMap<String, String>, String), String> {
        // NO SILENT DOWNGRADE. Setting SIGNALWIRE_REST_CA_FILE is a request to
        // verify the peer against that CA — meaningless over plaintext. The
        // agent's TlsConfig is chosen at construction and simply goes unused on
        // an `http://` URL, so a caller who set the CA and a plain base URL (or
        // whose paginated `next` link came back as `http://`) would have the
        // request sent in the clear with no diagnostic. Refuse instead, naming
        // the setting that would otherwise have been ignored.
        if self.custom_ca && !url.starts_with("https://") {
            return Err(format!(
                "SIGNALWIRE_REST_CA_FILE is set (TLS verification requested) but the request \
                 URL is not https: {url} — refusing to downgrade to plaintext. Use an https:// \
                 base URL, or unset SIGNALWIRE_REST_CA_FILE to send in the clear deliberately."
            ));
        }
        // Apply the resolved per-attempt timeout to THIS request (request-level
        // config overrides the agent default). A timeout surfaces as
        // `ureq::Error::Timeout`, mapped below into the typed transport error.
        let response_result = match method.to_ascii_uppercase().as_str() {
            "GET" => {
                let mut req = self
                    .agent
                    .get(url)
                    .config()
                    .timeout_global(Some(timeout))
                    .build();
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                req.call()
            }
            "POST" => {
                let mut req = self
                    .agent
                    .post(url)
                    .config()
                    .timeout_global(Some(timeout))
                    .build();
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                match body {
                    Some(b) => req.send(b),
                    None => req.send_empty(),
                }
            }
            "PUT" => {
                let mut req = self
                    .agent
                    .put(url)
                    .config()
                    .timeout_global(Some(timeout))
                    .build();
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                match body {
                    Some(b) => req.send(b),
                    None => req.send_empty(),
                }
            }
            "PATCH" => {
                let mut req = self
                    .agent
                    .patch(url)
                    .config()
                    .timeout_global(Some(timeout))
                    .build();
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                match body {
                    Some(b) => req.send(b),
                    None => req.send_empty(),
                }
            }
            "DELETE" => {
                let mut req = self
                    .agent
                    .delete(url)
                    .config()
                    .timeout_global(Some(timeout))
                    .build();
                for (k, v) in headers {
                    req = req.header(k, v);
                }
                req.call()
            }
            other => {
                return Err(format!("Unsupported HTTP method: {other}"));
            }
        };

        let mut response =
            response_result.map_err(|e| format!("HTTP {method} {url} failed: {e}"))?;
        let status = response.status().as_u16();
        // Capture response headers (lowercased names) before consuming the body,
        // so the client can surface the platform request-id on an error (§6.6).
        let mut resp_headers = HashMap::new();
        for (name, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                resp_headers.insert(name.as_str().to_ascii_lowercase(), v.to_string());
            }
        }
        let body_str = response
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("HTTP {method} {url} body read failed: {e}"))?;
        Ok((status, resp_headers, body_str))
    }
}

impl HttpTransport for UreqTransport {
    fn execute(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
        timeout: Duration,
    ) -> Result<(u16, String), String> {
        let (status, _headers, body) = self.execute_raw(method, url, headers, body, timeout)?;
        Ok((status, body))
    }

    fn execute_with_headers(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
        timeout: Duration,
    ) -> Result<(u16, HashMap<String, String>, String), String> {
        self.execute_raw(method, url, headers, body, timeout)
    }
}

/// A stub transport that records requests and returns canned responses.
/// Useful for unit testing without network access.
pub struct StubTransport {
    /// Canned response: (`status_code`, body).
    pub response: std::sync::Mutex<(u16, String)>,
    /// Recorded requests: (method, url, body).
    pub requests: std::sync::Mutex<Vec<(String, String, Option<String>)>>,
}

impl StubTransport {
    /// Create a stub that answers every request with `status` and `body`
    /// until [`set_response`](StubTransport::set_response) changes it.
    ///
    /// Requests are recorded in `requests` as `(method, url, body)` for
    /// assertions.
    pub fn new(status: u16, body: &str) -> Self {
        StubTransport {
            response: std::sync::Mutex::new((status, body.to_string())),
            requests: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// # Panics
    ///
    /// Panics if the internal response lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
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
        _timeout: Duration,
    ) -> Result<(u16, String), String> {
        self.requests.lock().unwrap().push((
            method.to_string(),
            url.to_string(),
            body.map(std::string::ToString::to_string),
        ));
        let resp = self.response.lock().unwrap().clone();
        Ok(resp)
    }
}

/// Low-level HTTP client for SignalWire REST APIs.
///
/// Uses Basic Auth with `project_id:token` and returns parsed JSON
/// responses as `serde_json::Value`.
pub struct HttpClient {
    project_id: String,
    token: String,
    base_url: String,
    auth_header: String,
    user_agent: String,
    transport: Box<dyn HttpTransport>,
    /// The client-default request options (timeout / retries / backoff / abort
    /// signal), applied to every request and shallow-overridden by a per-request
    /// [`RequestOptions`]. `None` => the built-in defaults (no retry, 30s
    /// timeout).
    request_options: Option<RequestOptions>,
}

impl HttpClient {
    /// Build a client for `project_id` / `token` against `base_url`, using
    /// `transport` to issue requests.
    ///
    /// The credentials are pre-encoded into an HTTP Basic `Authorization`
    /// header once at construction. Trailing slashes on `base_url` are
    /// trimmed.
    ///
    /// Request behaviour uses the built-in defaults — **no retry**, 30s
    /// timeout. Use [`with_options`](HttpClient::with_options) to change
    /// them.
    pub fn new(
        project_id: &str,
        token: &str,
        base_url: &str,
        transport: Box<dyn HttpTransport>,
    ) -> Self {
        Self::with_options(project_id, token, base_url, transport, None)
    }

    /// Construct with an explicit client-default [`RequestOptions`] (plan 4.2).
    /// `request_options` is the default applied to every request through this
    /// client; a per-request override shallow-merges over it. `None` selects the
    /// built-in defaults (no retry, 30s timeout).
    pub fn with_options(
        project_id: &str,
        token: &str,
        base_url: &str,
        transport: Box<dyn HttpTransport>,
        request_options: Option<RequestOptions>,
    ) -> Self {
        let auth_header = format!("Basic {}", BASE64.encode(format!("{project_id}:{token}")));
        HttpClient {
            project_id: project_id.to_string(),
            token: token.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header,
            user_agent: concat!("signalwire-agents-rust-rest/", env!("CARGO_PKG_VERSION"))
                .to_string(),
            transport,
            request_options,
        }
    }

    /// Create with a stub transport for testing.
    pub fn with_stub(
        project_id: &str,
        token: &str,
        base_url: &str,
    ) -> (Self, std::sync::Arc<StubTransport>) {
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

    /// The project ID used as the HTTP Basic username.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// The API token used as the HTTP Basic password.
    ///
    /// This is a **secret** granting full project API access — never log it
    /// or surface it in an error.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// The base URL every request path is appended to, with any trailing
    /// slash trimmed.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The pre-computed `Authorization` header value —
    /// `Basic <base64(project_id:token)>`.
    ///
    /// This **contains the API token in recoverable form**: base64 is an
    /// encoding, not encryption. Treat it exactly as you would the token.
    pub fn auth_header(&self) -> &str {
        &self.auth_header
    }

    // -- HTTP methods --

    /// Issue a `GET` request to `path` with the given query `params`.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure, reported with status code 0), the API responds with
    /// a non-2xx status (the error carries the status and response body — e.g.
    /// 404 when the addressed resource does not exist), or a 2xx response body
    /// is present but not valid JSON. This is the authoritative description of
    /// the three failure modes shared by every HTTP method on this client.
    /// `params` is optional — `None` sends no query string, matching the
    /// reference's `params=None` default.
    pub fn get(
        &self,
        path: &str,
        params: Option<&HashMap<String, String>>,
    ) -> Result<Value, SignalWireRestError> {
        self.request("GET", path, params, None, None)
    }

    /// The client-default [`RequestOptions`], if any.
    #[must_use]
    pub fn request_options(&self) -> Option<&RequestOptions> {
        self.request_options.as_ref()
    }

    /// `GET` with a per-request [`RequestOptions`] override (shallow-merged over
    /// the client default). See [`get`](Self::get) for the error contract.
    ///
    /// # Errors
    /// Same as [`get`](Self::get).
    pub fn get_with_options(
        &self,
        path: &str,
        params: Option<&HashMap<String, String>>,
        options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.request("GET", path, params, None, options)
    }

    /// Issue a `POST` request to `path` with `data` serialized as the JSON body
    /// and `params` sent as the QUERY STRING.
    ///
    /// The two carriers are independent, exactly as in the reference
    /// (`_base.py` `_request` passes `json=body, params=params`): `data`
    /// becomes the JSON request body and `params` becomes the URL query — a
    /// `POST` may carry both at once. `params` is optional; `None` sends no
    /// query string, matching the wire contract's `params=None` default.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 422
    /// when the payload fails server-side validation), or a 2xx response body
    /// is not valid JSON. See [`get`](Self::get) for the canonical description.
    pub fn post(
        &self,
        path: &str,
        data: Option<&Value>,
        params: Option<&HashMap<String, String>>,
    ) -> Result<Value, SignalWireRestError> {
        self.post_with_options(path, data, params, None)
    }

    /// `POST` with a per-request [`RequestOptions`] override.
    ///
    /// `data` is optional — `None` sends no request body, matching the
    /// reference's `body=None` default.
    ///
    /// # Errors
    /// Same as [`post`](Self::post).
    pub fn post_with_options(
        &self,
        path: &str,
        data: Option<&Value>,
        params: Option<&HashMap<String, String>>,
        options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        let body = data.map(|d| serde_json::to_string(d).unwrap_or_else(|_| "{}".to_string()));
        self.request("POST", path, params, body.as_deref(), options)
    }

    /// Issue a `PUT` request to `path` with `data` serialized as the JSON body.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for a missing resource or 422 when the payload fails validation), or a
    /// 2xx response body is not valid JSON. See [`get`](Self::get).
    pub fn put(&self, path: &str, data: Option<&Value>) -> Result<Value, SignalWireRestError> {
        self.put_with_options(path, data, None)
    }

    /// `PUT` with a per-request [`RequestOptions`] override.
    ///
    /// `data` is optional — `None` sends no request body, matching the
    /// reference's `body=None` default.
    ///
    /// # Errors
    /// Same as [`put`](Self::put).
    pub fn put_with_options(
        &self,
        path: &str,
        data: Option<&Value>,
        options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        let body = data.map(|d| serde_json::to_string(d).unwrap_or_else(|_| "{}".to_string()));
        self.request("PUT", path, None, body.as_deref(), options)
    }

    /// Issue a `PATCH` request to `path` with `data` serialized as the JSON body.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for a missing resource or 422 when the payload fails validation), or a
    /// 2xx response body is not valid JSON. See [`get`](Self::get).
    pub fn patch(&self, path: &str, data: Option<&Value>) -> Result<Value, SignalWireRestError> {
        self.patch_with_options(path, data, None)
    }

    /// `PATCH` with a per-request [`RequestOptions`] override.
    ///
    /// `data` is optional — `None` sends no request body, matching the
    /// reference's `body=None` default.
    ///
    /// # Errors
    /// Same as [`patch`](Self::patch).
    pub fn patch_with_options(
        &self,
        path: &str,
        data: Option<&Value>,
        options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        let body = data.map(|d| serde_json::to_string(d).unwrap_or_else(|_| "{}".to_string()));
        self.request("PATCH", path, None, body.as_deref(), options)
    }

    /// Issue a `DELETE` request to `path`.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// when the addressed resource does not exist), or a 2xx response body is
    /// present but not valid JSON. See [`get`](Self::get).
    pub fn delete(&self, path: &str) -> Result<Value, SignalWireRestError> {
        self.delete_with_options(path, None)
    }

    /// `DELETE` with a per-request [`RequestOptions`] override.
    ///
    /// # Errors
    /// Same as [`delete`](Self::delete).
    pub fn delete_with_options(
        &self,
        path: &str,
        options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.request("DELETE", path, None, None, options)
    }

    // -- Paginated list support --

    /// Return all pages of results, following `links.next`.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if any page request fails: it cannot
    /// reach the Space (transport failure), the API responds with a non-2xx
    /// status, or a 2xx response body is not valid JSON. Pagination follows the
    /// `links.next` cursor returned by each page; a malformed or unreachable
    /// next-page URL surfaces as the underlying request error for that page.
    pub fn list_all(
        &self,
        path: &str,
        params: &HashMap<String, String>,
    ) -> Result<Vec<Value>, SignalWireRestError> {
        let mut all_pages = Vec::new();
        let mut current_path = path.to_string();
        let mut current_params = params.clone();
        // Cycle guard: a broken cursor that keeps handing back the SAME
        // `links.next` would loop forever. Track the next-URLs we have already
        // followed and stop the first time one repeats.
        let mut seen_next: std::collections::HashSet<String> = std::collections::HashSet::new();

        loop {
            let response = self.get(&current_path, Some(&current_params))?;

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
                    if !seen_next.insert(url.to_string()) {
                        // Already followed this exact cursor — the server is
                        // looping; stop rather than spin forever.
                        break;
                    }
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

    #[allow(clippy::too_many_lines)]
    fn request(
        &self,
        method: &str,
        path: &str,
        params: Option<&HashMap<String, String>>,
        body: Option<&str>,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        let mut url = format!("{}{}", self.base_url, path);

        // `params` go on the QUERY STRING (never merged into the JSON body) —
        // the reference routes them the same way for every verb.
        if let Some(params) = params.filter(|p| !p.is_empty()) {
            // Percent-encode keys AND values as application/x-www-form-urlencoded
            // so reserved characters (space, &, =, +, /, unicode) can't corrupt
            // the query or inject extra parameters. Sort by key for a stable,
            // reproducible query string (HashMap iteration order is arbitrary).
            let mut pairs: Vec<(&String, &String)> = params.iter().collect();
            pairs.sort_by(|a, b| a.0.cmp(b.0));
            let mut ser = url::form_urlencoded::Serializer::new(String::new());
            for (k, v) in pairs {
                ser.append_pair(k, v);
            }
            url = format!("{url}?{}", ser.finish());
        }

        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Accept".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), self.auth_header.clone());
        headers.insert("User-Agent".to_string(), self.user_agent.clone());

        // Resolve the effective options: per-request over client-default over
        // built-in. total attempts = retries + 1; retry on a retryable status
        // (idempotency-aware) or a transport error, honoring Retry-After then
        // exponential backoff. abort_signal is checked cooperatively BEFORE
        // every attempt (the honest blocking-client minimum).
        let opts = resolve(self.request_options.as_ref(), request_options);

        let mut attempt: u32 = 0;
        loop {
            attempt += 1;

            if opts.is_aborted() {
                // Cancelled before this attempt — surface as the transport-error
                // family (no response was produced), not a bare error.
                return Err(SignalWireRestError::transport(
                    &format!("{method} {path} cancelled by abort_signal"),
                    &url,
                    method,
                    TransportFailure("request cancelled by abort_signal".to_string()),
                ));
            }

            let outcome =
                self.transport
                    .execute_with_headers(method, &url, &headers, body, opts.timeout);

            match outcome {
                Err(e) => {
                    // Transport failure (connection refused / DNS / reset / TLS /
                    // timeout): the request never produced a response. Retry if
                    // attempts remain, else wrap in the typed error family.
                    if attempt <= opts.retries {
                        Self::sleep(opts.backoff_delay(attempt));
                        continue;
                    }
                    return Err(SignalWireRestError::transport(
                        &format!("{method} {path} failed: {e}"),
                        &url,
                        method,
                        TransportFailure(e),
                    ));
                }
                Ok((status, resp_headers, response_body)) => {
                    if !(200..300).contains(&status) {
                        if attempt <= opts.retries && status_is_retryable(method, status, &opts) {
                            // §6.6: the real RESPONSE headers are now available, so
                            // Retry-After can honor the server's exact delta.
                            let delay = Self::retry_after_seconds(&resp_headers, &response_body)
                                .unwrap_or_else(|| opts.backoff_delay(attempt));
                            Self::sleep(delay);
                            continue;
                        }
                        return Err(SignalWireRestError::new(
                            &format!("{method} {path} returned {status}"),
                            status,
                            &response_body,
                            &url,
                            method,
                        )
                        .with_headers(resp_headers));
                    }

                    // 204 or empty body
                    if status == 204 || response_body.is_empty() {
                        return Ok(serde_json::json!({}));
                    }

                    return serde_json::from_str(&response_body).map_err(|_| {
                        SignalWireRestError::new(
                            &format!("{method} {path} returned non-JSON"),
                            status,
                            &response_body,
                            &url,
                            method,
                        )
                        .with_headers(resp_headers)
                    });
                }
            }
        }
    }

    /// Backoff sleep between retries. A seam so tests can drive the retry loop;
    /// only sleeps for a positive duration (`retry_backoff=0` in the corpus keeps
    /// the differ off the wall clock).
    fn sleep(seconds: f64) {
        if seconds > 0.0 {
            std::thread::sleep(Duration::from_secs_f64(seconds));
        }
    }

    /// Parse a `Retry-After` header (delta-seconds form) from the response, if
    /// the transport surfaced it. The current [`HttpTransport`] boundary returns
    /// only `(status, body)` — it does not carry response headers back — so this
    /// returns `None` and the caller falls back to computed exponential backoff.
    /// (The Retry-After delta and the computed backoff coincide for the pinned
    /// corpus, whose retry cases set `retry_backoff = 0`; wiring the header
    /// through the transport is a follow-up if a future case needs the exact
    /// delta.)
    /// Parse a `Retry-After` response header in delta-seconds form, if present.
    /// Response header names are lowercased by the transport, so look up
    /// `retry-after`. Returns `None` when absent or non-numeric (the caller then
    /// falls back to computed exponential backoff). The HTTP-date form is not
    /// honored (the platform uses delta-seconds).
    fn retry_after_seconds(headers: &HashMap<String, String>, _body: &str) -> Option<f64> {
        headers
            .get("retry-after")
            .and_then(|v| v.trim().parse::<f64>().ok())
            .filter(|s| *s >= 0.0)
    }
}

/// Wrapper so `Arc<StubTransport>` implements `HttpTransport`.
struct StubTransportWrapper(std::sync::Arc<StubTransport>);

impl HttpTransport for StubTransportWrapper {
    fn execute(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
        timeout: Duration,
    ) -> Result<(u16, String), String> {
        self.0.execute(method, url, headers, body, timeout)
    }
}

/// A stub transport that returns a queued sequence of canned responses — one per
/// request — so multi-page flows (each page a distinct body) can be exercised.
/// Records every request like [`StubTransport`]. When the queue is exhausted it
/// keeps returning the last response.
///
/// Test-only (`#[cfg(test)]`): used by the pagination cursor-follow tests; it is
/// not part of the public REST surface.
#[cfg(test)]
pub struct SequencedTransport {
    responses: std::sync::Mutex<std::collections::VecDeque<(u16, String)>>,
    last: std::sync::Mutex<(u16, String)>,
    /// Recorded requests: (method, url, body).
    pub requests: std::sync::Mutex<Vec<(String, String, Option<String>)>>,
}

#[cfg(test)]
impl SequencedTransport {
    /// Create a transport that answers successive requests with each entry
    /// of `responses` in turn.
    ///
    /// Once the queue is exhausted the **last** response repeats for every
    /// further request, so a test can drive N distinct answers and then
    /// settle. An empty `responses` makes every request return `200 {}`.
    ///
    /// Test-only (`#[cfg(test)]`): not part of the public REST surface.
    #[must_use]
    pub fn new(responses: Vec<(u16, String)>) -> Self {
        let last = responses.last().cloned().unwrap_or((200, "{}".to_string()));
        SequencedTransport {
            responses: std::sync::Mutex::new(responses.into_iter().collect()),
            last: std::sync::Mutex::new(last),
            requests: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Build an `HttpTransport` box that shares this `Arc`'s recorded requests.
    #[must_use]
    pub fn wrapper(inner: std::sync::Arc<SequencedTransport>) -> impl HttpTransport {
        SequencedTransportWrapper(inner)
    }
}

#[cfg(test)]
impl HttpTransport for SequencedTransport {
    fn execute(
        &self,
        method: &str,
        url: &str,
        _headers: &HashMap<String, String>,
        body: Option<&str>,
        _timeout: Duration,
    ) -> Result<(u16, String), String> {
        self.requests.lock().unwrap().push((
            method.to_string(),
            url.to_string(),
            body.map(std::string::ToString::to_string),
        ));
        let mut q = self.responses.lock().unwrap();
        match q.pop_front() {
            Some(resp) => {
                *self.last.lock().unwrap() = resp.clone();
                Ok(resp)
            }
            None => Ok(self.last.lock().unwrap().clone()),
        }
    }
}

/// Wrapper so `Arc<SequencedTransport>` implements `HttpTransport`.
#[cfg(test)]
struct SequencedTransportWrapper(std::sync::Arc<SequencedTransport>);

#[cfg(test)]
impl HttpTransport for SequencedTransportWrapper {
    fn execute(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&str>,
        timeout: Duration,
    ) -> Result<(u16, String), String> {
        self.0.execute(method, url, headers, body, timeout)
    }
}

/// Parse a query string into a `HashMap`, percent-DECODING keys and values
/// exactly ONCE.
///
/// This mirrors python's `urllib.parse.parse_qs` in `_pagination.py`: a
/// `links.next` cursor arrives percent-encoded on the wire, is decoded once
/// here into its raw value, and is re-encoded exactly once by [`request`] when
/// the next page is fetched — a net-identity round trip. Storing the raw
/// (still-encoded) value instead would double-encode it (`%2F` → `%252F`) and
/// corrupt the cursor, dropping every subsequent page.
fn parse_query_string(qs: &str) -> HashMap<String, String> {
    url::form_urlencoded::parse(qs.as_bytes())
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect()
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

    /// A transport that returns a fixed status + response HEADERS + body, to
    /// exercise the §6.6 request-id/headers threading (`execute_with_headers`).
    struct HeaderTransport {
        status: u16,
        headers: HashMap<String, String>,
        body: String,
    }

    impl HttpTransport for HeaderTransport {
        fn execute(
            &self,
            _m: &str,
            _u: &str,
            _h: &HashMap<String, String>,
            _b: Option<&str>,
            _t: Duration,
        ) -> Result<(u16, String), String> {
            Ok((self.status, self.body.clone()))
        }

        fn execute_with_headers(
            &self,
            _m: &str,
            _u: &str,
            _h: &HashMap<String, String>,
            _b: Option<&str>,
            _t: Duration,
        ) -> Result<(u16, HashMap<String, String>, String), String> {
            Ok((self.status, self.headers.clone(), self.body.clone()))
        }
    }

    /// §6.6: an HTTP error carries the response headers, and `request_id()`
    /// extracts the platform id from `x-request-id`; Display appends it.
    #[test]
    fn test_error_captures_response_headers_and_request_id() {
        let mut headers = HashMap::new();
        headers.insert("x-request-id".to_string(), "req-abc-123".to_string());
        headers.insert("content-type".to_string(), "application/json".to_string());
        let client = HttpClient::new(
            "proj",
            "tok",
            "https://test.signalwire.com",
            Box::new(HeaderTransport {
                status: 404,
                headers,
                body: r#"{"error":"not found"}"#.to_string(),
            }),
        );
        let err = client.get("/api/missing", None).unwrap_err();
        assert_eq!(err.request_id(), Some("req-abc-123"));
        assert_eq!(
            err.headers()
                .and_then(|h| h.get("content-type"))
                .map(String::as_str),
            Some("application/json")
        );
        assert!(
            err.to_string().contains("(request-id: req-abc-123)"),
            "Display must surface the request id: {err}"
        );
    }

    /// RUST-4 (D1 ripple): a non-2xx error must carry the FULL request URL
    /// (`base_url + path + ?encoded-query`), not the bare path. A caller
    /// correlating the failure needs the real endpoint it hit.
    #[test]
    fn test_status_error_url_is_full_with_query() {
        let (client, stub) = make_client();
        stub.set_response(404, r#"{"error":"missing"}"#);
        let mut params = HashMap::new();
        params.insert("page".to_string(), "2".to_string());
        let err = client.get("/api/missing", Some(&params)).unwrap_err();
        // Full origin, path, AND the encoded query — never just "/api/missing".
        assert_eq!(
            err.url(),
            "https://test.signalwire.com/api/missing?page=2",
            "error.url must be the full request URL incl. query, got {:?}",
            err.url()
        );
    }

    /// RUST-4: a transport failure error must likewise carry the full URL
    /// (base + path + query), not the bare path.
    #[test]
    fn test_transport_error_url_is_full_with_query() {
        struct FailTransport;
        impl HttpTransport for FailTransport {
            fn execute(
                &self,
                _m: &str,
                _u: &str,
                _h: &HashMap<String, String>,
                _b: Option<&str>,
                _t: Duration,
            ) -> Result<(u16, String), String> {
                Err("connection refused".to_string())
            }
        }
        let client = HttpClient::new(
            "proj",
            "tok",
            "https://test.signalwire.com",
            Box::new(FailTransport),
        );
        let mut params = HashMap::new();
        params.insert("q".to_string(), "x y".to_string());
        let err = client.get("/api/x", Some(&params)).unwrap_err();
        assert!(err.is_transport());
        assert_eq!(
            err.url(),
            "https://test.signalwire.com/api/x?q=x+y",
            "transport error.url must be the full request URL incl. query, got {:?}",
            err.url()
        );
    }

    /// RUST-4: a non-JSON 2xx body error must carry the full URL too.
    #[test]
    fn test_non_json_error_url_is_full() {
        let (client, stub) = make_client();
        stub.set_response(200, "not json at all");
        let err = client.get("/api/weird", None).unwrap_err();
        assert_eq!(err.url(), "https://test.signalwire.com/api/weird");
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

        let result = client.get("/api/test", None).unwrap();
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
        client.get("/api/test", Some(&params)).unwrap();

        let reqs = stub.requests.lock().unwrap();
        assert!(reqs[0].1.contains("page=2"));
    }

    #[test]
    fn test_post() {
        let (client, stub) = make_client();
        stub.set_response(201, r#"{"id":"new-1"}"#);

        let data = json!({"name": "test"});
        let result = client.post("/api/test", Some(&data), None).unwrap();
        assert_eq!(result["id"], "new-1");

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "POST");
        assert!(reqs[0].2.as_ref().unwrap().contains("test"));
    }

    /// WIRE CONTRACT: `params` go on the QUERY STRING and `body` goes in the
    /// JSON body — the two carriers are INDEPENDENT, for `post` exactly as for
    /// `get`. The reference (`rest/_base.py` `_request`) calls
    /// `session.request(method, url, json=body, params=params)`, so a POST may
    /// carry both at once and a query param must NEVER be merged into the body
    /// dict (nor a body field leak into the query).
    ///
    /// This is the assertion that fails if `post` is ever wired to route
    /// `params` into the body instead of the query.
    #[test]
    fn test_post_routes_params_to_query_and_body_to_json() {
        let (client, stub) = make_client();
        stub.set_response(201, r#"{"id":"new-1"}"#);

        let mut params = HashMap::new();
        params.insert("expand".to_string(), "detail".to_string());
        params.insert("page".to_string(), "2".to_string());
        let data = json!({"name": "widget", "count": 3});
        client
            .post("/api/test", Some(&data), Some(&params))
            .unwrap();

        let reqs = stub.requests.lock().unwrap();
        let (method, url, body) = reqs.last().expect("a request was recorded");
        assert_eq!(method, "POST");

        // -- query carries the params, and ONLY the params --
        let query = url
            .split_once('?')
            .unwrap_or_else(|| panic!("POST params must be on the query string, got url {url}"))
            .1;
        assert!(
            query.contains("expand=detail") && query.contains("page=2"),
            "both params must be on the query, got {query}"
        );
        assert!(
            !query.contains("name") && !query.contains("widget") && !query.contains("count"),
            "body fields must NOT leak into the query, got {query}"
        );

        // -- body carries the JSON, and ONLY the JSON --
        let sent = body.as_ref().expect("POST must send a JSON body");
        let parsed: Value = serde_json::from_str(sent).expect("body must be JSON");
        assert_eq!(parsed["name"], "widget");
        assert_eq!(parsed["count"], 3);
        assert!(
            parsed.get("expand").is_none() && parsed.get("page").is_none(),
            "query params must NOT be merged into the JSON body, got {sent}"
        );
    }

    /// `params: None` is the reference's `params=None` default — no query
    /// string is emitted at all (not an empty `?`).
    #[test]
    fn test_none_params_emits_no_query_string() {
        let (client, stub) = make_client();
        stub.set_response(200, "{}");

        client.get("/api/test", None).unwrap();
        client
            .post("/api/test", Some(&json!({"a": 1})), None)
            .unwrap();

        let reqs = stub.requests.lock().unwrap();
        for (method, url, _) in reqs.iter() {
            assert!(
                !url.contains('?'),
                "{method} with params=None must emit no query string, got {url}"
            );
        }
    }

    #[test]
    fn test_put() {
        let (client, stub) = make_client();
        stub.set_response(200, r#"{"updated":true}"#);

        let result = client
            .put("/api/test/1", Some(&json!({"name": "updated"})))
            .unwrap();
        assert_eq!(result["updated"], true);

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PUT");
    }

    #[test]
    fn test_patch() {
        let (client, stub) = make_client();
        stub.set_response(200, r#"{"patched":true}"#);

        let result = client
            .patch("/api/test/1", Some(&json!({"field": "val"})))
            .unwrap();
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

        let err = client.get("/api/missing", None).unwrap_err();
        assert_eq!(err.status_code(), 404);
        assert!(err.message().contains("404"));
    }

    #[test]
    fn test_error_on_500() {
        let (client, stub) = make_client();
        stub.set_response(500, "server error");

        let err = client.get("/api/fail", None).unwrap_err();
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

    /// Build an `HttpClient` backed by a `SequencedTransport` returning the
    /// given `(status, body)` pages in order. Returns the client and the shared
    /// transport handle so tests can inspect the recorded request URLs.
    fn make_sequenced_client(
        pages: Vec<(u16, String)>,
    ) -> (HttpClient, std::sync::Arc<SequencedTransport>) {
        let seq = std::sync::Arc::new(SequencedTransport::new(pages));
        let client = HttpClient::new(
            "proj-1",
            "tok-1",
            "https://test.signalwire.com",
            Box::new(SequencedTransport::wrapper(seq.clone())),
        );
        (client, seq)
    }

    /// A `links.next` cursor whose value is already percent-encoded on the wire
    /// (e.g. an opaque cursor containing `/`, `+`, `=`) must be forwarded on the
    /// next page request encoded EXACTLY ONCE — the value the server issued. The
    /// bug: `parse_query_string` stored the raw (still-encoded) value and
    /// `request()` percent-encoded it AGAIN, double-encoding `%2F` into `%252F`.
    /// Python decodes once (`parse_qs`) then encodes once (`urlencode`) — net
    /// identity. This pins that identity for rust's eager `list_all`.
    #[test]
    fn test_list_all_cursor_encoded_value_round_trips_once() {
        // Page 1 hands back an opaque cursor `a/b+c=d` percent-encoded on the wire.
        let page1 = (
            200,
            r#"{"data":[{"id":1}],"links":{"next":"/api/items?cursor=a%2Fb%2Bc%3Dd"}}"#.to_string(),
        );
        let page2 = (
            200,
            r#"{"data":[{"id":2}],"links":{"next":""}}"#.to_string(),
        );
        let (client, seq) = make_sequenced_client(vec![page1, page2]);

        let items = client.list_all("/api/items", &HashMap::new()).unwrap();
        assert_eq!(items.len(), 2);

        let reqs = seq.requests.lock().unwrap();
        assert_eq!(reqs.len(), 2, "exactly two page requests");
        let (_m, url2, _b) = &reqs[1];
        let query2 = url2.split_once('?').expect("page-2 query present").1;
        // The server issued `cursor=a%2Fb%2Bc%3Dd` (value `a/b+c=d`). We must
        // reproduce that exact encoding, NOT double-encode it to `a%252Fb...`.
        assert!(
            !query2.contains("%252F") && !query2.contains("%253D") && !query2.contains("%252B"),
            "cursor value was double-encoded on the next request: {url2}"
        );
        // Decoding the round-tripped cursor must yield the original `a/b+c=d`.
        let params = parse_query_string(query2);
        assert_eq!(
            params.get("cursor").map(String::as_str),
            Some("a/b+c=d"),
            "round-tripped cursor value must decode back to the server's value: {url2}"
        );
    }

    /// `list_all` must keep paging while a `links.next` exists even when a page
    /// returns zero items — the empty-page-with-next ripple (mirror of python's
    /// `_pagination.py` fix: termination is driven ONLY by absence of `next`).
    #[test]
    fn test_list_all_empty_page_with_next_continues() {
        let page1 = (
            200,
            r#"{"data":[],"links":{"next":"/api/items?page=2"}}"#.to_string(),
        );
        let page2 = (
            200,
            r#"{"data":[{"id":7}],"links":{"next":""}}"#.to_string(),
        );
        let (client, seq) = make_sequenced_client(vec![page1, page2]);

        let items = client.list_all("/api/items", &HashMap::new()).unwrap();
        assert_eq!(items.len(), 1, "must fetch page 2 despite empty page 1");
        assert_eq!(items[0]["id"], 7);
        assert_eq!(seq.requests.lock().unwrap().len(), 2);
    }

    /// A server that keeps returning the SAME `links.next` (a broken/looping
    /// cursor) must terminate, not spin forever. `list_all` carries a cycle
    /// guard keyed on the (path, params) it is about to fetch.
    #[test]
    fn test_list_all_repeating_next_terminates() {
        // Every page hands back the identical next cursor — an infinite loop
        // without a guard. The SequencedTransport repeats its last response
        // once the queue drains, so this would never stop.
        let looping = (
            200,
            r#"{"data":[{"id":1}],"links":{"next":"/api/items?cursor=STUCK"}}"#.to_string(),
        );
        let (client, seq) = make_sequenced_client(vec![looping]);

        let items = client.list_all("/api/items", &HashMap::new()).unwrap();
        // The guard stops after re-seeing the same cursor; we must not hang and
        // must not accumulate unboundedly.
        assert!(
            items.len() <= 2,
            "cycle guard must stop a repeating cursor, got {} items",
            items.len()
        );
        assert!(
            seq.requests.lock().unwrap().len() <= 2,
            "cycle guard must bound the number of page requests"
        );
    }

    #[test]
    fn test_parse_query_string() {
        let qs = "page=2&limit=10";
        let parsed = parse_query_string(qs);
        assert_eq!(parsed["page"], "2");
        assert_eq!(parsed["limit"], "10");
    }

    /// `parse_query_string` percent-DECODES values once (mirroring python's
    /// `parse_qs`), so a cursor stored from a `links.next` URL is the decoded
    /// value; the single re-encode in `request()` reproduces the wire form.
    #[test]
    fn test_parse_query_string_decodes_once() {
        let parsed = parse_query_string("cursor=a%2Fb%2Bc%3Dd&x=%20");
        assert_eq!(parsed["cursor"], "a/b+c=d");
        assert_eq!(parsed["x"], " ");
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
        let result = client.get("/api/test", None).unwrap();
        assert!(result.is_object());
    }

    /// Reserved characters in query values must be percent-encoded so they
    /// cannot break the query or inject extra parameters. Regression guard for
    /// the raw-`format!("{k}={v}")` bug (the percent-encoding fix in `request`).
    #[test]
    fn test_query_params_reserved_chars_are_encoded() {
        let (client, stub) = make_client();
        stub.set_response(200, "{}");
        let mut params = HashMap::new();
        // Values carrying reserved chars: `&`, `=`, `+`, space, `/`, unicode.
        params.insert("q".to_string(), "a b&c=d+e/f".to_string());
        params.insert("name".to_string(), "café ☕".to_string());
        client.get("/api/test", Some(&params)).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let (_method, url, _body) = reqs.last().expect("a request was recorded");
        // Raw reserved characters must NOT appear unescaped in the query.
        let query = url.split_once('?').expect("query present").1;
        assert!(
            !query.contains("a b&c=d"),
            "reserved chars leaked unencoded into the query: {url}"
        );
        // The `&`/`=`/space/unicode must be percent- or plus-encoded.
        assert!(query.contains("a+b%26c%3Dd") || query.contains("a%20b%26c%3Dd"));
        assert!(query.contains("caf%C3%A9"));
        // Keys are sorted for a stable query string (name before q).
        assert!(
            query.find("name=").unwrap() < query.find("q=").unwrap(),
            "query params should be sorted by key: {query}"
        );
    }

    /// A real connection-refused failure (dead port, no mock) through the
    /// production `UreqTransport` must surface as the typed `SignalWireRestError`
    /// family — `is_transport() == true`, `status_code() == 0` — NOT a bare
    /// `ureq`/IO error leaking out. Plan 1.3b regression guard.
    #[test]
    fn test_conn_refused_yields_typed_transport_error() {
        // Bind a loopback port then immediately release it so nothing listens —
        // a connection attempt there is refused deterministically.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let dead_port = listener.local_addr().unwrap().port();
        drop(listener);

        let client = HttpClient::new(
            "proj-1",
            "tok-1",
            &format!("http://127.0.0.1:{dead_port}"),
            Box::new(UreqTransport::new()),
        );

        let err = client.get("/api/test", None).unwrap_err();
        assert!(
            err.is_transport(),
            "conn-refused must be a transport error, got: {err}"
        );
        assert_eq!(err.status_code(), 0);
        // The cause chain survives — this is not just a formatted message.
        assert!(std::error::Error::source(&err).is_some());
    }
}
