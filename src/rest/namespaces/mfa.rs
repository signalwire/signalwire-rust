use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// MFA (Multi-Factor Authentication) namespace.
///
/// Mirrors `signalwire.rest.namespaces.mfa.MfaResource` from the Python SDK.
/// Rooted at `/api/relay/rest/mfa` with `sms`, `call`, and `verify`
/// sub-paths.
pub struct Mfa<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> Mfa<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Mfa {
            client,
            base_path: "/api/relay/rest/mfa".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// POST /api/relay/rest/mfa/sms — send a one-time code over SMS.
    pub fn sms(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&format!("{}/sms", self.base_path), params)
    }

    /// POST /api/relay/rest/mfa/call — deliver a one-time code via voice.
    pub fn call(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&format!("{}/call", self.base_path), params)
    }

    /// POST `/api/relay/rest/mfa/{request_id}/verify` — verify a code.
    pub fn verify(
        &self,
        request_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .post(&format!("{}/{}/verify", self.base_path, request_id), params)
    }
}
