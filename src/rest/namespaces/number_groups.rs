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

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&self.base_path, &Self::params_to_string_map(params))
    }

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, group_id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&format!("{}/{}", self.base_path, group_id), &HashMap::new())
    }

    pub fn update(
        &self,
        group_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .put(&format!("{}/{}", self.base_path, group_id), params)
    }

    pub fn delete(&self, group_id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .delete(&format!("{}/{}", self.base_path, group_id))
    }

    /// GET `/api/relay/rest/number_groups/{id}/number_group_memberships`
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

    pub fn get_membership(
        &self,
        membership_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = format!(
            "/api/relay/rest/number_group_memberships/{membership_id}"
        );
        self.client.get(&path, &HashMap::new())
    }

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
