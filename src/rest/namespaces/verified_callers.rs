// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.

//! Verified Caller IDs namespace — CRUD plus the verification flow.
//!
//! Mirrors `signalwire.rest.namespaces.verified_callers.VerifiedCallersResource`
//! from the Python SDK. The base path is
//! `/api/relay/rest/verified_caller_ids`, update uses `PUT`
//! (`_update_method = "PUT"`).

use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// Verified caller ID management — standard CRUD (PUT update) plus the
/// two-step phone verification flow (`redial_verification` /
/// `submit_verification`).
pub struct VerifiedCallersResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> VerifiedCallersResource<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        VerifiedCallersResource {
            client,
            base_path: "/api/relay/rest/verified_caller_ids".to_string(),
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

    /// List verified caller IDs (GET basePath).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.base_path, params)
    }

    /// Submit a new caller ID for verification (POST basePath).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 422 when `data` fails server-side validation), or the response body is
    /// not valid JSON.
    pub fn create(&self, data: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, data)
    }

    /// Retrieve a single verified caller ID (GET basePath/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 when no record has the given `id`), or the response body is not
    /// valid JSON.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.path(&[id]), &HashMap::new())
    }

    /// Update a verified caller ID (PUT basePath/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 for a missing `id` or 422 when `data` fails validation), or the
    /// response body is not valid JSON.
    pub fn update(&self, id: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        self.client.put(&self.path(&[id]), data)
    }

    /// Delete a verified caller ID (DELETE basePath/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 when no record has the given `id`), or the response body is not
    /// valid JSON.
    pub fn delete(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&self.path(&[id]))
    }

    /// Redial the verification call (`POST basePath/{id}/verification`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 when no record has the given `caller_id`), or the response body is
    /// not valid JSON.
    pub fn redial_verification(&self, caller_id: &str) -> Result<Value, SignalWireRestError> {
        self.client.post(
            &self.path(&[caller_id, "verification"]),
            &serde_json::json!({}),
        )
    }

    /// Submit a verification code (`PUT basePath/{id}/verification`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 when no record has the given `caller_id`, or 422 when the code is
    /// rejected), or the response body is not valid JSON.
    pub fn submit_verification(
        &self,
        caller_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .put(&self.path(&[caller_id, "verification"]), params)
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
        let r = VerifiedCallersResource::new(&c);
        assert_eq!(r.base_path(), "/api/relay/rest/verified_caller_ids");
    }

    #[test]
    fn test_update_uses_put() {
        let (c, stub) = make();
        stub.set_response(200, "{}");
        let r = VerifiedCallersResource::new(&c);
        r.update("VC_X", &json!({"name": "x"})).unwrap();
        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PUT");
        assert!(
            reqs[0]
                .1
                .contains("/api/relay/rest/verified_caller_ids/VC_X")
        );
    }

    #[test]
    fn test_verification_flow_paths() {
        let (c, stub) = make();
        stub.set_response(200, "{}");
        let r = VerifiedCallersResource::new(&c);
        r.redial_verification("VC_X").unwrap();
        r.submit_verification("VC_X", &json!({"code": "1234"}))
            .unwrap();
        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "POST");
        assert!(
            reqs[0]
                .1
                .contains("/api/relay/rest/verified_caller_ids/VC_X/verification")
        );
        assert_eq!(reqs[1].0, "PUT");
        assert!(
            reqs[1]
                .1
                .contains("/api/relay/rest/verified_caller_ids/VC_X/verification")
        );
    }
}
