// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.

//! Phone Numbers namespace — CRUD plus available-number search.
//!
//! Mirrors `signalwire.rest.namespaces.phone_numbers.PhoneNumbersResource`
//! from the Python SDK. Update uses `PUT` (`_update_method = "PUT"`).

use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;
use crate::rest::util::params_to_string_map;

/// Phone number management — standard CRUD (PUT update) plus `search` for
/// available numbers (`GET /api/relay/rest/phone_numbers/search`).
pub struct PhoneNumbersResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> PhoneNumbersResource<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        PhoneNumbersResource {
            client,
            base_path: "/api/relay/rest/phone_numbers".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn client(&self) -> &HttpClient {
        self.client
    }

    fn path(&self, parts: &[&str]) -> String {
        if parts.is_empty() {
            return self.base_path.clone();
        }
        format!("{}/{}", self.base_path, parts.join("/"))
    }

    /// List owned phone numbers (GET basePath).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.base_path, params)
    }

    /// Purchase / create a phone number (POST basePath).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 when `data` fails server-side validation), or the response body is
    /// not valid JSON.
    pub fn create(&self, data: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, data)
    }

    /// Retrieve a single phone number by ID (GET basePath/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 when no number has the given `id`), or the response body is not
    /// valid JSON.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.path(&[id]), &HashMap::new())
    }

    /// Update a phone number by ID (PUT basePath/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 for a missing `id` or 422 when `data` fails validation), or the
    /// response body is not valid JSON.
    pub fn update(&self, id: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        self.client.put(&self.path(&[id]), data)
    }

    /// Release a phone number by ID (DELETE basePath/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 when no number has the given `id`), or the response body is not
    /// valid JSON.
    pub fn delete(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&self.path(&[id]))
    }

    /// Search available phone numbers
    /// (`GET /api/relay/rest/phone_numbers/search`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn search(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.path(&["search"]), &qp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::http_client::StubTransport;
    use serde_json::json;

    fn make() -> (HttpClient, std::sync::Arc<StubTransport>) {
        HttpClient::with_stub("proj", "tok", "https://test.signalwire.com")
    }

    #[test]
    fn test_base_path() {
        let (c, _) = make();
        let pn = PhoneNumbersResource::new(&c);
        assert_eq!(pn.base_path(), "/api/relay/rest/phone_numbers");
    }

    #[test]
    fn test_search_path() {
        let (c, stub) = make();
        stub.set_response(200, r#"{"data":[]}"#);
        let pn = PhoneNumbersResource::new(&c);
        pn.search(&json!({"area_code": "512"})).unwrap();
        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "GET");
        assert!(reqs[0].1.contains("/api/relay/rest/phone_numbers/search"));
        assert!(reqs[0].1.contains("area_code=512"));
    }

    #[test]
    fn test_update_uses_put() {
        let (c, stub) = make();
        stub.set_response(200, "{}");
        let pn = PhoneNumbersResource::new(&c);
        pn.update("PN_X", &json!({"name": "x"})).unwrap();
        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PUT");
        assert!(reqs[0].1.contains("/api/relay/rest/phone_numbers/PN_X"));
    }
}
