use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// Chat API namespace — token generation.
///
/// Mirrors `signalwire.rest.namespaces.chat.ChatResource`. Rooted at
/// `/api/chat/tokens`.
pub struct ChatResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> ChatResource<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        ChatResource {
            client,
            base_path: "/api/chat/tokens".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// POST `/api/chat/tokens` — create a chat token.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 422
    /// when `params` fails validation), or the response body is not valid JSON.
    pub fn create_token(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }
}
