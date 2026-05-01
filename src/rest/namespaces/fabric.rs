// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.

//! Fabric API namespace — resource composition, addresses, and tokens.
//!
//! Mirrors `signalwire.rest.namespaces.fabric.FabricNamespace` from the
//! Python SDK.

use std::collections::HashMap;

use serde_json::Value;

use crate::rest::crud_resource::CrudResource;
use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// Base path for all Fabric resources.
const BASE: &str = "/api/fabric/resources";

fn join(parts: &[&str]) -> String {
    parts.join("/")
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

// ---------------------------------------------------------------------------
// Top-level namespace
// ---------------------------------------------------------------------------

pub struct Fabric<'a> {
    client: &'a HttpClient,
}

impl<'a> Fabric<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Fabric { client }
    }

    pub fn client(&self) -> &HttpClient {
        self.client
    }

    // -- Sub-resource accessors --

    /// Subscribers resource — full CRUD plus SIP-endpoint sub-resources.
    pub fn subscribers(&self) -> SubscribersResource<'a> {
        SubscribersResource::new(self.client, &format!("{BASE}/subscribers"))
    }

    pub fn sip_endpoints(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{BASE}/sip_endpoints"))
    }

    /// Read-only top-level fabric addresses (NOT under `/resources`).
    pub fn addresses(&self) -> FabricAddresses<'a> {
        FabricAddresses::new(self.client, "/api/fabric/addresses")
    }

    /// Call flows — exposes a singular `call_flow` sub-path for addresses
    /// / versions per the API spec.
    pub fn call_flows(&self) -> CallFlowsResource<'a> {
        CallFlowsResource::new(self.client, &format!("{BASE}/call_flows"))
    }

    pub fn swml_scripts(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{BASE}/swml_scripts"))
    }

    pub fn conversations(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{BASE}/conversations"))
    }

    /// Conference rooms — singular `conference_room` for sub-paths.
    pub fn conference_rooms(&self) -> ConferenceRoomsResource<'a> {
        ConferenceRoomsResource::new(self.client, &format!("{BASE}/conference_rooms"))
    }

    pub fn dial_plans(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{BASE}/dial_plans"))
    }

    pub fn freeclimb_apps(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{BASE}/freeclimb_apps"))
    }

    pub fn call_queues(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{BASE}/call_queues"))
    }

    pub fn ai_agents(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{BASE}/ai_agents"))
    }

    pub fn sip_profiles(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{BASE}/sip_profiles"))
    }

    pub fn phone_numbers(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{BASE}/phone_numbers"))
    }

    /// cXML applications — read/update/delete only (no create).
    pub fn cxml_applications(&self) -> CxmlApplicationsResource<'a> {
        CxmlApplicationsResource::new(
            self.client,
            &format!("{BASE}/cxml_applications"),
        )
    }

    /// Generic resource operations across every fabric resource type.
    pub fn resources(&self) -> GenericResources<'a> {
        GenericResources::new(self.client, BASE)
    }

    /// Fabric token factories (subscriber / guest / invite / embed).
    pub fn tokens(&self) -> FabricTokens<'a> {
        FabricTokens::new(self.client)
    }
}

// ---------------------------------------------------------------------------
// FabricAddresses — read-only top-level resource
// ---------------------------------------------------------------------------

pub struct FabricAddresses<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> FabricAddresses<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        FabricAddresses {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn get(&self, address_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, address_id]);
        self.client.get(&p, &HashMap::new())
    }
}

// ---------------------------------------------------------------------------
// SubscribersResource — CRUD (PUT update) + SIP endpoint sub-resources
// ---------------------------------------------------------------------------

pub struct SubscribersResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> SubscribersResource<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        SubscribersResource {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, subscriber_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id]);
        self.client.get(&p, &HashMap::new())
    }

    pub fn update(
        &self,
        subscriber_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id]);
        self.client.put(&p, params)
    }

    pub fn delete(&self, subscriber_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id]);
        self.client.delete(&p)
    }

    pub fn list_addresses(
        &self,
        subscriber_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, subscriber_id, "addresses"]);
        self.client.get(&p, &qp)
    }

    pub fn list_sip_endpoints(
        &self,
        subscriber_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, subscriber_id, "sip_endpoints"]);
        self.client.get(&p, &qp)
    }

    pub fn create_sip_endpoint(
        &self,
        subscriber_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id, "sip_endpoints"]);
        self.client.post(&p, params)
    }

    pub fn get_sip_endpoint(
        &self,
        subscriber_id: &str,
        endpoint_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id, "sip_endpoints", endpoint_id]);
        self.client.get(&p, &HashMap::new())
    }

    pub fn update_sip_endpoint(
        &self,
        subscriber_id: &str,
        endpoint_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id, "sip_endpoints", endpoint_id]);
        self.client.patch(&p, params)
    }

    pub fn delete_sip_endpoint(
        &self,
        subscriber_id: &str,
        endpoint_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id, "sip_endpoints", endpoint_id]);
        self.client.delete(&p)
    }
}

// ---------------------------------------------------------------------------
// CallFlowsResource — CRUD (PUT) + singular `call_flow` sub-paths
// ---------------------------------------------------------------------------

pub struct CallFlowsResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CallFlowsResource<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CallFlowsResource {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    fn singular(&self) -> String {
        self.base_path.replace("/call_flows", "/call_flow")
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    pub fn update(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.put(&p, params)
    }

    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    /// Sub-resource list — uses singular `call_flow` per the API spec.
    pub fn list_addresses(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let base = self.singular();
        let p = join(&[&base, resource_id, "addresses"]);
        self.client.get(&p, &qp)
    }

    pub fn list_versions(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let base = self.singular();
        let p = join(&[&base, resource_id, "versions"]);
        self.client.get(&p, &qp)
    }

    pub fn deploy_version(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let base = self.singular();
        let p = join(&[&base, resource_id, "versions"]);
        self.client.post(&p, params)
    }
}

// ---------------------------------------------------------------------------
// ConferenceRoomsResource — singular `conference_room` for sub-paths
// ---------------------------------------------------------------------------

pub struct ConferenceRoomsResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> ConferenceRoomsResource<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        ConferenceRoomsResource {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    fn singular(&self) -> String {
        self.base_path.replace("/conference_rooms", "/conference_room")
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    pub fn update(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.put(&p, params)
    }

    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    pub fn list_addresses(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let base = self.singular();
        let p = join(&[&base, resource_id, "addresses"]);
        self.client.get(&p, &qp)
    }
}

// ---------------------------------------------------------------------------
// CxmlApplicationsResource — read/update/delete; create raises
// ---------------------------------------------------------------------------

pub struct CxmlApplicationsResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CxmlApplicationsResource<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CxmlApplicationsResource {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    pub fn update(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.put(&p, params)
    }

    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    /// cXML applications cannot be created via this API.
    ///
    /// Returns an `Err` with a clear "not implemented" message that
    /// mirrors the Python SDK's `NotImplementedError`. No HTTP request
    /// is sent to the server.
    pub fn create(&self, _params: &Value) -> Result<Value, SignalWireRestError> {
        Err(SignalWireRestError::new(
            "cXML applications cannot be created via this API",
            0,
            "",
        ))
    }
}

// ---------------------------------------------------------------------------
// GenericResources — operations across every fabric resource type
// ---------------------------------------------------------------------------

pub struct GenericResources<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> GenericResources<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        GenericResources {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    pub fn list_addresses(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, resource_id, "addresses"]);
        self.client.get(&p, &qp)
    }

    pub fn assign_domain_application(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id, "domain_applications"]);
        self.client.post(&p, params)
    }
}

// ---------------------------------------------------------------------------
// FabricTokens — subscriber / guest / invite / embed token factories
// ---------------------------------------------------------------------------

pub struct FabricTokens<'a> {
    client: &'a HttpClient,
}

impl<'a> FabricTokens<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        FabricTokens { client }
    }

    pub fn create_subscriber_token(
        &self,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .post("/api/fabric/subscribers/tokens", params)
    }

    pub fn refresh_subscriber_token(
        &self,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .post("/api/fabric/subscribers/tokens/refresh", params)
    }

    /// Note the singular `subscriber` segment per the spec.
    pub fn create_invite_token(
        &self,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client.post("/api/fabric/subscriber/invites", params)
    }

    pub fn create_guest_token(
        &self,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client.post("/api/fabric/guests/tokens", params)
    }

    pub fn create_embed_token(
        &self,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client.post("/api/fabric/embeds/tokens", params)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::http_client::StubTransport;

    fn make_fabric() -> (HttpClient, std::sync::Arc<StubTransport>) {
        HttpClient::with_stub("proj", "tok", "https://test.signalwire.com")
    }

    #[test]
    fn test_subscribers_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.subscribers().base_path(),
            "/api/fabric/resources/subscribers"
        );
    }

    #[test]
    fn test_addresses_path() {
        // Top-level addresses live at /api/fabric/addresses (NOT under /resources).
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(f.addresses().base_path(), "/api/fabric/addresses");
    }

    #[test]
    fn test_call_flows_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.call_flows().base_path(),
            "/api/fabric/resources/call_flows"
        );
    }

    #[test]
    fn test_conference_rooms_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.conference_rooms().base_path(),
            "/api/fabric/resources/conference_rooms"
        );
    }

    #[test]
    fn test_resources_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(f.resources().base_path(), "/api/fabric/resources");
    }

    #[test]
    fn test_cxml_applications_create_returns_error() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        let err = f
            .cxml_applications()
            .create(&serde_json::json!({}))
            .unwrap_err();
        assert!(format!("{:?}", err).contains("cXML applications"));
    }
}
