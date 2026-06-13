use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// Project API namespace — exposes the API token sub-resource.
///
/// Mirrors `signalwire.rest.namespaces.project.ProjectNamespace`.
pub struct Project<'a> {
    client: &'a HttpClient,
}

impl<'a> Project<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Project { client }
    }

    pub fn tokens(&self) -> ProjectTokens<'a> {
        ProjectTokens::new(self.client)
    }
}

/// Project API token management.
///
/// Mirrors `ProjectTokens` from the Python SDK. Rooted at
/// `/api/project/tokens`.
pub struct ProjectTokens<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> ProjectTokens<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        ProjectTokens {
            client,
            base_path: "/api/project/tokens".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// POST `/api/project/tokens` — create a new project API token.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 422
    /// when `params` fails validation), or the response body is not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// PATCH `/api/project/tokens/{token_id}` — update a project API token.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `token_id` or 422 when `params` fails validation), or the
    /// response body is not valid JSON.
    pub fn update(&self, token_id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client
            .patch(&format!("{}/{}", self.base_path, token_id), params)
    }

    /// DELETE `/api/project/tokens/{token_id}` — delete a project API token.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `token_id`), or the response body is not valid JSON.
    pub fn delete(&self, token_id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .delete(&format!("{}/{}", self.base_path, token_id))
    }
}
