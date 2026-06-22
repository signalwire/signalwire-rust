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

    /// SIP endpoints — full CRUD + `list_addresses`, PUT update
    /// (Python `FabricResourcePUT`).
    pub fn sip_endpoints(&self) -> FabricResourcePUT<'a> {
        FabricResourcePUT::new(self.client, &format!("{BASE}/sip_endpoints"))
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

    /// SWML scripts — full CRUD + `list_addresses`, PUT update
    /// (Python `FabricResourcePUT`).
    pub fn swml_scripts(&self) -> FabricResourcePUT<'a> {
        FabricResourcePUT::new(self.client, &format!("{BASE}/swml_scripts"))
    }

    /// cXML scripts — full CRUD + `list_addresses`, PUT update
    /// (Python `FabricResourcePUT`).
    pub fn cxml_scripts(&self) -> FabricResourcePUT<'a> {
        FabricResourcePUT::new(self.client, &format!("{BASE}/cxml_scripts"))
    }

    /// RELAY applications — full CRUD + `list_addresses`, PUT update
    /// (Python `FabricResourcePUT`).
    pub fn relay_applications(&self) -> FabricResourcePUT<'a> {
        FabricResourcePUT::new(self.client, &format!("{BASE}/relay_applications"))
    }

    /// `FreeSWITCH` connectors — full CRUD + `list_addresses`, PUT update
    /// (Python `FabricResourcePUT`).
    pub fn freeswitch_connectors(&self) -> FabricResourcePUT<'a> {
        FabricResourcePUT::new(self.client, &format!("{BASE}/freeswitch_connectors"))
    }

    /// Conference rooms — singular `conference_room` for sub-paths.
    pub fn conference_rooms(&self) -> ConferenceRoomsResource<'a> {
        ConferenceRoomsResource::new(self.client, &format!("{BASE}/conference_rooms"))
    }

    /// AI agents — full CRUD + `list_addresses`, PATCH update
    /// (Python `FabricResource`).
    pub fn ai_agents(&self) -> FabricResource<'a> {
        FabricResource::new(self.client, &format!("{BASE}/ai_agents"))
    }

    /// SIP gateways — full CRUD + `list_addresses`, PATCH update
    /// (Python `FabricResource`).
    pub fn sip_gateways(&self) -> FabricResource<'a> {
        FabricResource::new(self.client, &format!("{BASE}/sip_gateways"))
    }

    /// cXML webhooks — full CRUD + `list_addresses`, PATCH update
    /// (Python `CxmlWebhooksResource` / `FabricResource`). Normally
    /// auto-materialized via `phone_numbers.set_cxml_webhook`.
    pub fn cxml_webhooks(&self) -> FabricResource<'a> {
        FabricResource::new(self.client, &format!("{BASE}/cxml_webhooks"))
    }

    /// SWML webhooks — full CRUD + `list_addresses`, PATCH update
    /// (Python `SwmlWebhooksResource` / `FabricResource`). Normally
    /// auto-materialized via `phone_numbers.set_swml_webhook`.
    pub fn swml_webhooks(&self) -> FabricResource<'a> {
        FabricResource::new(self.client, &format!("{BASE}/swml_webhooks"))
    }

    /// cXML applications — read/update/delete only (no create).
    pub fn cxml_applications(&self) -> CxmlApplicationsResource<'a> {
        CxmlApplicationsResource::new(self.client, &format!("{BASE}/cxml_applications"))
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

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `address_id` is unknown), or the response body is not valid JSON.
    pub fn get(&self, address_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, address_id]);
        self.client.get(&p, &HashMap::new())
    }
}

// ---------------------------------------------------------------------------
// FabricResource — standard fabric resource: CRUD (PATCH update) + addresses
// ---------------------------------------------------------------------------

/// Standard fabric resource — full CRUD plus `list_addresses`, using
/// `PATCH` for updates. Mirrors Python's `FabricResource`
/// (`CrudWithAddresses` with the default `PATCH` update method).
pub struct FabricResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> FabricResource<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        FabricResource {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the server rejects the supplied fields), or the response body is
    /// not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// PATCH update (Python `FabricResource`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
    pub fn update(&self, resource_id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.patch(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    /// GET `{base}/{id}/addresses` (Python `CrudWithAddresses.list_addresses`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn list_addresses(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, resource_id, "addresses"]);
        self.client.get(&p, &qp)
    }
}

// ---------------------------------------------------------------------------
// FabricResourcePUT — fabric resource that uses PUT for updates
// ---------------------------------------------------------------------------

/// Fabric resource that uses `PUT` for updates (Python `FabricResourcePUT`).
/// Otherwise identical to [`FabricResource`]: full CRUD plus `list_addresses`.
pub struct FabricResourcePUT<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> FabricResourcePUT<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        FabricResourcePUT {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the server rejects the supplied fields), or the response body is
    /// not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// PUT update (Python `FabricResourcePUT`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
    pub fn update(&self, resource_id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.put(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    /// GET `{base}/{id}/addresses` (Python `CrudWithAddresses.list_addresses`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn list_addresses(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, resource_id, "addresses"]);
        self.client.get(&p, &qp)
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

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the server rejects the supplied fields), or the response body is
    /// not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `subscriber_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn get(&self, subscriber_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `subscriber_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
    pub fn update(
        &self,
        subscriber_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id]);
        self.client.put(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `subscriber_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn delete(&self, subscriber_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id]);
        self.client.delete(&p)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `subscriber_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn list_addresses(
        &self,
        subscriber_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, subscriber_id, "addresses"]);
        self.client.get(&p, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `subscriber_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn list_sip_endpoints(
        &self,
        subscriber_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, subscriber_id, "sip_endpoints"]);
        self.client.get(&p, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `subscriber_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
    pub fn create_sip_endpoint(
        &self,
        subscriber_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id, "sip_endpoints"]);
        self.client.post(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `subscriber_id` or `endpoint_id` is unknown), or the response
    /// body is not valid JSON.
    pub fn get_sip_endpoint(
        &self,
        subscriber_id: &str,
        endpoint_id: &str,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id, "sip_endpoints", endpoint_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `subscriber_id` or `endpoint_id` is unknown or 422 if the server
    /// rejects the supplied fields), or the response body is not valid JSON.
    pub fn update_sip_endpoint(
        &self,
        subscriber_id: &str,
        endpoint_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, subscriber_id, "sip_endpoints", endpoint_id]);
        self.client.patch(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `subscriber_id` or `endpoint_id` is unknown), or the response
    /// body is not valid JSON.
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

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the server rejects the supplied fields), or the response body is
    /// not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
    pub fn update(&self, resource_id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.put(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    /// Sub-resource list — uses singular `call_flow` per the API spec.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
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

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
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

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
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
        self.base_path
            .replace("/conference_rooms", "/conference_room")
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the server rejects the supplied fields), or the response body is
    /// not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
    pub fn update(&self, resource_id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.put(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
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

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
    pub fn update(&self, resource_id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.put(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    /// GET `{base}/{id}/addresses` (Python `CrudWithAddresses.list_addresses`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn list_addresses(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, resource_id, "addresses"]);
        self.client.get(&p, &qp)
    }

    /// cXML applications cannot be created via this API.
    ///
    /// Returns an `Err` with a clear "not implemented" message that
    /// mirrors the Python SDK's `NotImplementedError`. No HTTP request
    /// is sent to the server.
    ///
    /// # Errors
    /// Always returns [`SignalWireRestError`]: creation is unsupported, so this
    /// method unconditionally yields a "not implemented" error without
    /// contacting the Space.
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

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn get(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn delete(&self, resource_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id]);
        self.client.delete(&p)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown), or the response body is not valid
    /// JSON.
    pub fn list_addresses(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, resource_id, "addresses"]);
        self.client.get(&p, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
    pub fn assign_domain_application(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id, "domain_applications"]);
        self.client.post(&p, params)
    }

    /// POST `{base}/{id}/phone_routes` (Python `GenericResources.assign_phone_route`).
    ///
    /// Deprecated, mirroring the Python reference.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if `resource_id` is unknown or 422 if the server rejects the
    /// supplied fields), or the response body is not valid JSON.
    pub fn assign_phone_route(
        &self,
        resource_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, resource_id, "phone_routes"]);
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

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the server rejects the supplied fields), or the response body is
    /// not valid JSON.
    pub fn create_subscriber_token(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post("/api/fabric/subscribers/tokens", params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the supplied token cannot be refreshed), or the response body is
    /// not valid JSON.
    pub fn refresh_subscriber_token(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client
            .post("/api/fabric/subscribers/tokens/refresh", params)
    }

    /// Note the singular `subscriber` segment per the spec.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the server rejects the supplied fields), or the response body is
    /// not valid JSON.
    pub fn create_invite_token(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post("/api/fabric/subscriber/invites", params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the server rejects the supplied fields), or the response body is
    /// not valid JSON.
    pub fn create_guest_token(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post("/api/fabric/guests/tokens", params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 if the server rejects the supplied fields), or the response body is
    /// not valid JSON.
    pub fn create_embed_token(&self, params: &Value) -> Result<Value, SignalWireRestError> {
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
    fn test_new_fabric_accessor_paths() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.cxml_scripts().base_path(),
            "/api/fabric/resources/cxml_scripts"
        );
        assert_eq!(
            f.relay_applications().base_path(),
            "/api/fabric/resources/relay_applications"
        );
        assert_eq!(
            f.freeswitch_connectors().base_path(),
            "/api/fabric/resources/freeswitch_connectors"
        );
        assert_eq!(
            f.cxml_webhooks().base_path(),
            "/api/fabric/resources/cxml_webhooks"
        );
        assert_eq!(
            f.swml_webhooks().base_path(),
            "/api/fabric/resources/swml_webhooks"
        );
        assert_eq!(
            f.sip_gateways().base_path(),
            "/api/fabric/resources/sip_gateways"
        );
        assert_eq!(f.ai_agents().base_path(), "/api/fabric/resources/ai_agents");
        assert_eq!(
            f.sip_endpoints().base_path(),
            "/api/fabric/resources/sip_endpoints"
        );
        assert_eq!(
            f.swml_scripts().base_path(),
            "/api/fabric/resources/swml_scripts"
        );
    }

    #[test]
    fn test_fabric_resource_patch_update_and_list_addresses() {
        let (client, stub) = make_fabric();
        let f = Fabric::new(&client);
        stub.set_response(200, r#"{"id":"a"}"#);
        f.ai_agents()
            .update("res-1", &serde_json::json!({"name": "x"}))
            .unwrap();
        f.ai_agents()
            .list_addresses("res-1", &serde_json::json!({}))
            .unwrap();
        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PATCH");
        assert!(reqs[0].1.contains("/ai_agents/res-1"));
        assert_eq!(reqs[1].0, "GET");
        assert!(reqs[1].1.contains("/ai_agents/res-1/addresses"));
    }

    #[test]
    fn test_fabric_resource_put_update() {
        let (client, stub) = make_fabric();
        let f = Fabric::new(&client);
        stub.set_response(200, r#"{"id":"a"}"#);
        f.swml_scripts()
            .update("res-2", &serde_json::json!({"name": "x"}))
            .unwrap();
        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PUT");
        assert!(reqs[0].1.contains("/swml_scripts/res-2"));
    }

    #[test]
    fn test_cxml_applications_create_returns_error() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        let err = f
            .cxml_applications()
            .create(&serde_json::json!({}))
            .unwrap_err();
        assert!(format!("{err:?}").contains("cXML applications"));
    }
}
