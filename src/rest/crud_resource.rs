use std::collections::HashMap;

use serde_json::Value;

use super::error::SignalWireRestError;
use super::http_client::HttpClient;

/// Generic CRUD wrapper around an HttpClient and a base API path.
///
/// Provides list / create / get / update / delete for any REST resource
/// that follows the standard SignalWire collection+item URL pattern.
pub struct CrudResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CrudResource<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CrudResource {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn client(&self) -> &HttpClient {
        self.client
    }

    /// Build a full path by appending segments to the base path.
    fn path(&self, parts: &[&str]) -> String {
        if parts.is_empty() {
            return self.base_path.clone();
        }
        format!("{}/{}", self.base_path, parts.join("/"))
    }

    /// List resources (GET basePath).
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.base_path, params)
    }

    /// Create a new resource (POST basePath).
    pub fn create(&self, data: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, data)
    }

    /// Retrieve a single resource by ID (GET basePath/{id}).
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.path(&[id]), &HashMap::new())
    }

    /// Update a resource by ID (PUT basePath/{id}).
    pub fn update(&self, id: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        self.client.put(&self.path(&[id]), data)
    }

    /// Delete a resource by ID (DELETE basePath/{id}).
    pub fn delete(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&self.path(&[id]))
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::http_client::StubTransport;
    use serde_json::json;

    fn make_resource() -> (crate::rest::http_client::HttpClient, std::sync::Arc<StubTransport>) {
        crate::rest::http_client::HttpClient::with_stub(
            "proj",
            "tok",
            "https://test.signalwire.com",
        )
    }

    #[test]
    fn test_base_path() {
        let (client, _) = make_resource();
        let crud = CrudResource::new(&client, "/api/phone_numbers");
        assert_eq!(crud.base_path(), "/api/phone_numbers");
    }

    #[test]
    fn test_path_building() {
        let (client, _) = make_resource();
        let crud = CrudResource::new(&client, "/api/items");
        assert_eq!(crud.path(&[]), "/api/items");
        assert_eq!(crud.path(&["123"]), "/api/items/123");
        assert_eq!(crud.path(&["123", "sub"]), "/api/items/123/sub");
    }

    #[test]
    fn test_list() {
        let (client, stub) = make_resource();
        stub.set_response(200, r#"{"data":[{"id":"1"}]}"#);

        let crud = CrudResource::new(&client, "/api/items");
        let result = crud.list(&HashMap::new()).unwrap();
        assert_eq!(result["data"][0]["id"], "1");

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "GET");
        assert!(reqs[0].1.contains("/api/items"));
    }

    #[test]
    fn test_create() {
        let (client, stub) = make_resource();
        stub.set_response(201, r#"{"id":"new-1"}"#);

        let crud = CrudResource::new(&client, "/api/items");
        let result = crud.create(&json!({"name": "test"})).unwrap();
        assert_eq!(result["id"], "new-1");

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "POST");
    }

    #[test]
    fn test_get() {
        let (client, stub) = make_resource();
        stub.set_response(200, r#"{"id":"123","name":"item"}"#);

        let crud = CrudResource::new(&client, "/api/items");
        let result = crud.get("123").unwrap();
        assert_eq!(result["id"], "123");

        let reqs = stub.requests.lock().unwrap();
        assert!(reqs[0].1.contains("/api/items/123"));
    }

    #[test]
    fn test_update() {
        let (client, stub) = make_resource();
        stub.set_response(200, r#"{"id":"123","name":"updated"}"#);

        let crud = CrudResource::new(&client, "/api/items");
        let result = crud.update("123", &json!({"name": "updated"})).unwrap();
        assert_eq!(result["name"], "updated");

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PUT");
    }

    #[test]
    fn test_delete() {
        let (client, stub) = make_resource();
        stub.set_response(204, "");

        let crud = CrudResource::new(&client, "/api/items");
        let result = crud.delete("123").unwrap();
        assert!(result.is_object());

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "DELETE");
        assert!(reqs[0].1.contains("/api/items/123"));
    }

    #[test]
    fn test_list_with_params() {
        let (client, stub) = make_resource();
        stub.set_response(200, r#"{"data":[]}"#);

        let crud = CrudResource::new(&client, "/api/items");
        let mut params = HashMap::new();
        params.insert("page".to_string(), "3".to_string());
        crud.list(&params).unwrap();

        let reqs = stub.requests.lock().unwrap();
        assert!(reqs[0].1.contains("page=3"));
    }

    #[test]
    fn test_error_propagation() {
        let (client, stub) = make_resource();
        stub.set_response(404, r#"{"error":"not found"}"#);

        let crud = CrudResource::new(&client, "/api/items");
        let err = crud.get("missing").unwrap_err();
        assert_eq!(err.status_code(), 404);
    }
}
