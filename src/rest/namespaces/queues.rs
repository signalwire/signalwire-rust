use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// Queues namespace.
///
/// Mirrors `signalwire.rest.namespaces.queues.QueuesResource`. CRUD over
/// `/api/relay/rest/queues` plus the queue-member operations.
pub struct Queues<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> Queues<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Queues {
            client,
            base_path: "/api/relay/rest/queues".to_string(),
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

    /// GET `/api/relay/rest/queues` — list queues.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&self.base_path, &Self::params_to_string_map(params))
    }

    /// POST `/api/relay/rest/queues` — create a queue.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 422
    /// when `params` fails validation), or the response body is not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// GET `/api/relay/rest/queues/{queue_id}` — fetch one queue.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `queue_id`), or the response body is not valid JSON.
    pub fn get(&self, queue_id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&format!("{}/{}", self.base_path, queue_id), &HashMap::new())
    }

    /// PUT `/api/relay/rest/queues/{queue_id}` — update a queue.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `queue_id` or 422 when `params` fails validation), or the
    /// response body is not valid JSON.
    pub fn update(
        &self,
        queue_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .put(&format!("{}/{}", self.base_path, queue_id), params)
    }

    /// DELETE `/api/relay/rest/queues/{queue_id}` — delete a queue.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `queue_id`), or the response body is not valid JSON.
    pub fn delete(&self, queue_id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .delete(&format!("{}/{}", self.base_path, queue_id))
    }

    /// GET `/api/relay/rest/queues/{queue_id}/members` — list queue members.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `queue_id`), or the response body is not valid JSON.
    pub fn list_members(
        &self,
        queue_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!("{}/{}/members", self.base_path, queue_id);
        self.client
            .get(&path, &Self::params_to_string_map(params))
    }

    /// GET `/api/relay/rest/queues/{queue_id}/members/next` — fetch the next
    /// member to be served.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `queue_id`), or the response body is not valid JSON.
    pub fn get_next_member(
        &self,
        queue_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!("{}/{}/members/next", self.base_path, queue_id);
        self.client.get(&path, &HashMap::new())
    }

    /// GET `/api/relay/rest/queues/{queue_id}/members/{member_id}` — fetch one
    /// queue member.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `queue_id` or `member_id`), or the response body is not
    /// valid JSON.
    pub fn get_member(
        &self,
        queue_id: &str,
        member_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!("{}/{}/members/{}", self.base_path, queue_id, member_id);
        self.client.get(&path, &HashMap::new())
    }
}
