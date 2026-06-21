use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// PubSub API namespace — token generation.
///
/// Mirrors `signalwire.rest.namespaces.pubsub.PubSubResource`. Rooted at
/// `/api/pubsub/tokens`.
pub struct PubSubResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> PubSubResource<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        PubSubResource {
            client,
            base_path: "/api/pubsub/tokens".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// POST `/api/pubsub/tokens` — create a `PubSub` token.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 422
    /// when `params` fails validation), or the response body is not valid JSON.
    pub fn create_token(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }
}
