use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// SIP Profile (singleton resource) namespace.
///
/// Mirrors `signalwire.rest.namespaces.sip_profile.SipProfileResource`.
/// Rooted at `/api/relay/rest/sip_profile` (singular) — the project has
/// exactly one SIP profile, so there is no list/create/delete.
pub struct SipProfile<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> SipProfile<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        SipProfile {
            client,
            base_path: "/api/relay/rest/sip_profile".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// GET `/api/relay/rest/sip_profile` — fetch the current profile.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn get(&self) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&self.base_path, &std::collections::HashMap::new())
    }

    /// PUT `/api/relay/rest/sip_profile` — update the SIP profile.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 422
    /// when `params` fails validation), or the response body is not valid JSON.
    pub fn update(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.put(&self.base_path, params)
    }
}
