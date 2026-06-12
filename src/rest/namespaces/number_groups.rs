use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// Number Groups namespace.
///
/// Mirrors `signalwire.rest.namespaces.number_groups.NumberGroupsResource`.
/// Provides standard CRUD over `/api/relay/rest/number_groups` plus the
/// membership endpoints rooted at the same collection and at the
/// project-scoped `/api/relay/rest/number_group_memberships` collection.
pub struct NumberGroups<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> NumberGroups<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        NumberGroups {
            client,
            base_path: "/api/relay/rest/number_groups".to_string(),
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

    /// GET `/api/relay/rest/number_groups` — list number groups.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&self.base_path, &Self::params_to_string_map(params))
    }

    /// POST `/api/relay/rest/number_groups` — create a number group.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 422
    /// when `params` fails validation), or the response body is not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// GET `/api/relay/rest/number_groups/{group_id}` — fetch one number group.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `group_id`), or the response body is not valid JSON.
    pub fn get(&self, group_id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&format!("{}/{}", self.base_path, group_id), &HashMap::new())
    }

    /// PUT `/api/relay/rest/number_groups/{group_id}` — update a number group.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `group_id` or 422 when `params` fails validation), or the
    /// response body is not valid JSON.
    pub fn update(
        &self,
        group_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .put(&format!("{}/{}", self.base_path, group_id), params)
    }

    /// DELETE `/api/relay/rest/number_groups/{group_id}` — delete a number group.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `group_id`), or the response body is not valid JSON.
    pub fn delete(&self, group_id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .delete(&format!("{}/{}", self.base_path, group_id))
    }

    /// GET `/api/relay/rest/number_groups/{id}/number_group_memberships`
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `group_id`), or the response body is not valid JSON.
    pub fn list_memberships(
        &self,
        group_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!(
            "{}/{}/number_group_memberships",
            self.base_path, group_id
        );
        self.client
            .get(&path, &Self::params_to_string_map(params))
    }

    /// POST `/api/relay/rest/number_groups/{id}/number_group_memberships` — add
    /// a number to the group.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `group_id` or 422 when `params` fails validation), or the
    /// response body is not valid JSON.
    pub fn add_membership(
        &self,
        group_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!(
            "{}/{}/number_group_memberships",
            self.base_path, group_id
        );
        self.client.post(&path, params)
    }

    /// GET `/api/relay/rest/number_group_memberships/{membership_id}` — fetch
    /// one membership.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `membership_id`), or the response body is not valid JSON.
    pub fn get_membership(
        &self,
        membership_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!(
            "/api/relay/rest/number_group_memberships/{membership_id}"
        );
        self.client.get(&path, &HashMap::new())
    }

    /// DELETE `/api/relay/rest/number_group_memberships/{membership_id}` —
    /// remove a number from a group.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (e.g. 404
    /// for an unknown `membership_id`), or the response body is not valid JSON.
    pub fn delete_membership(
        &self,
        membership_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!(
            "/api/relay/rest/number_group_memberships/{membership_id}"
        );
        self.client.delete(&path)
    }
}
