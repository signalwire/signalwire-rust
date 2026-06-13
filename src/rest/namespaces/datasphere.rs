use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// Datasphere API namespace — exposes documents.
///
/// Mirrors `signalwire.rest.namespaces.datasphere.DatasphereNamespace`.
pub struct DatasphereNamespace<'a> {
    client: &'a HttpClient,
}

impl<'a> DatasphereNamespace<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        DatasphereNamespace { client }
    }

    pub fn documents(&self) -> DatasphereDocuments<'a> {
        DatasphereDocuments::new(self.client)
    }
}

/// Datasphere documents resource.
///
/// Rooted at `/api/datasphere/documents` with CRUD plus search and chunk
/// sub-paths.
pub struct DatasphereDocuments<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> DatasphereDocuments<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        DatasphereDocuments {
            client,
            base_path: "/api/datasphere/documents".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    fn params_to_string_map(params: &Value) -> HashMap<String, String> {
        let mut out = HashMap::new();
        if let Some(obj) = params.as_object() {
            for (k, v) in obj {
                let s = match v {
                    Value::String(s) => s.clone(),
                    Value::Null => continue,
                    other => other.to_string(),
                };
                out.insert(k.clone(), s);
            }
        }
        out
    }

    /// GET `/api/datasphere/documents` — list documents.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&self.base_path, &Self::params_to_string_map(params))
    }

    /// POST `/api/datasphere/documents` — create a document.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 422
    /// when `params` fails validation), or the response body is not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// GET `/api/datasphere/documents/{document_id}` — fetch one document.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `document_id`), or the response body is not valid JSON.
    pub fn get(&self, document_id: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(
            &format!("{}/{}", self.base_path, document_id),
            &HashMap::new(),
        )
    }

    /// PUT `/api/datasphere/documents/{document_id}` — update a document.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `document_id` or 422 when `params` fails validation), or
    /// the response body is not valid JSON.
    pub fn update(&self, document_id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client
            .put(&format!("{}/{}", self.base_path, document_id), params)
    }

    /// DELETE `/api/datasphere/documents/{document_id}` — delete a document.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `document_id`), or the response body is not valid JSON.
    pub fn delete(&self, document_id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .delete(&format!("{}/{}", self.base_path, document_id))
    }

    /// POST `/api/datasphere/documents/search` — semantic search over documents.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 422
    /// when `params` fails validation), or the response body is not valid JSON.
    pub fn search(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client
            .post(&format!("{}/search", self.base_path), params)
    }

    /// GET `/api/datasphere/documents/{document_id}/chunks` — list chunks of a
    /// document.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `document_id`), or the response body is not valid JSON.
    pub fn list_chunks(
        &self,
        document_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!("{}/{}/chunks", self.base_path, document_id);
        self.client.get(&path, &Self::params_to_string_map(params))
    }

    /// GET `/api/datasphere/documents/{document_id}/chunks/{chunk_id}` — fetch
    /// one chunk.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `document_id` or `chunk_id`), or the response body is not
    /// valid JSON.
    pub fn get_chunk(
        &self,
        document_id: &str,
        chunk_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!("{}/{}/chunks/{}", self.base_path, document_id, chunk_id);
        self.client.get(&path, &HashMap::new())
    }

    /// DELETE `/api/datasphere/documents/{document_id}/chunks/{chunk_id}` —
    /// delete one chunk.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `document_id` or `chunk_id`), or the response body is not
    /// valid JSON.
    pub fn delete_chunk(
        &self,
        document_id: &str,
        chunk_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!("{}/{}/chunks/{}", self.base_path, document_id, chunk_id);
        self.client.delete(&path)
    }
}
