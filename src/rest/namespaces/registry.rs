// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.

//! 10DLC Campaign Registry namespace — brands, campaigns, orders, numbers.
//!
//! Mirrors `signalwire.rest.namespaces.registry.RegistryNamespace`. Every
//! sub-resource lives under `/api/relay/rest/registry/beta`.

use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

const BASE: &str = "/api/relay/rest/registry/beta";

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

pub struct Registry<'a> {
    client: &'a HttpClient,
}

impl<'a> Registry<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Registry { client }
    }

    pub fn client(&self) -> &HttpClient {
        self.client
    }

    pub fn brands(&self) -> RegistryBrands<'a> {
        RegistryBrands::new(self.client, &format!("{BASE}/brands"))
    }

    pub fn campaigns(&self) -> RegistryCampaigns<'a> {
        RegistryCampaigns::new(self.client, &format!("{BASE}/campaigns"))
    }

    pub fn orders(&self) -> RegistryOrders<'a> {
        RegistryOrders::new(self.client, &format!("{BASE}/orders"))
    }

    pub fn numbers(&self) -> RegistryNumbers<'a> {
        RegistryNumbers::new(self.client, &format!("{BASE}/numbers"))
    }
}

// ---------------------------------------------------------------------------
// Brands
// ---------------------------------------------------------------------------

pub struct RegistryBrands<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> RegistryBrands<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        RegistryBrands {
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

    pub fn get(&self, brand_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, brand_id]);
        self.client.get(&p, &HashMap::new())
    }

    pub fn list_campaigns(
        &self,
        brand_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, brand_id, "campaigns"]);
        self.client.get(&p, &qp)
    }

    pub fn create_campaign(
        &self,
        brand_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, brand_id, "campaigns"]);
        self.client.post(&p, params)
    }
}

// ---------------------------------------------------------------------------
// Campaigns — note update uses PUT
// ---------------------------------------------------------------------------

pub struct RegistryCampaigns<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> RegistryCampaigns<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        RegistryCampaigns {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn get(&self, campaign_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, campaign_id]);
        self.client.get(&p, &HashMap::new())
    }

    pub fn update(
        &self,
        campaign_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, campaign_id]);
        self.client.put(&p, params)
    }

    pub fn list_numbers(
        &self,
        campaign_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, campaign_id, "numbers"]);
        self.client.get(&p, &qp)
    }

    pub fn list_orders(
        &self,
        campaign_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, campaign_id, "orders"]);
        self.client.get(&p, &qp)
    }

    pub fn create_order(
        &self,
        campaign_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, campaign_id, "orders"]);
        self.client.post(&p, params)
    }
}

// ---------------------------------------------------------------------------
// Orders — read-only, retrieve by id
// ---------------------------------------------------------------------------

pub struct RegistryOrders<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> RegistryOrders<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        RegistryOrders {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn get(&self, order_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, order_id]);
        self.client.get(&p, &HashMap::new())
    }
}

// ---------------------------------------------------------------------------
// Numbers — delete only (release)
// ---------------------------------------------------------------------------

pub struct RegistryNumbers<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> RegistryNumbers<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        RegistryNumbers {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn delete(&self, number_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, number_id]);
        self.client.delete(&p)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::http_client::StubTransport;

    fn make() -> (HttpClient, std::sync::Arc<StubTransport>) {
        HttpClient::with_stub("proj", "tok", "https://test.signalwire.com")
    }

    #[test]
    fn test_paths() {
        let (c, _) = make();
        let r = Registry::new(&c);
        assert_eq!(
            r.brands().base_path(),
            "/api/relay/rest/registry/beta/brands"
        );
        assert_eq!(
            r.campaigns().base_path(),
            "/api/relay/rest/registry/beta/campaigns"
        );
        assert_eq!(
            r.orders().base_path(),
            "/api/relay/rest/registry/beta/orders"
        );
        assert_eq!(
            r.numbers().base_path(),
            "/api/relay/rest/registry/beta/numbers"
        );
    }
}
