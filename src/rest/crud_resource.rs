use std::collections::HashMap;

use serde_json::Value;

use super::error::SignalWireRestError;
use super::http_client::HttpClient;

/// Generic CRUD wrapper around an `HttpClient` and a base API path.
///
/// Provides list / create / get / update / delete for any REST resource
/// that follows the standard SignalWire collection+item URL pattern. This is
/// the single CRUD base for the whole REST layer: the generated per-resource
/// structs compose it (via the `generated_bases` re-export), and it is the
/// public `rest::CrudResource`. The `update` HTTP verb is `PUT` or `PATCH`
/// depending on the resource; the constructor bakes it in alongside the base
/// path.
pub struct CrudResource<'a> {
    client: &'a HttpClient,
    base_path: String,
    update_method: String,
}

impl<'a> CrudResource<'a> {
    /// Construct the CRUD resource; base path (§4) + update verb (§9) baked in.
    #[must_use]
    pub fn new(client: &'a HttpClient, base_path: &str, update_method: &str) -> Self {
        CrudResource {
            client,
            base_path: base_path.to_string(),
            update_method: update_method.to_string(),
        }
    }

    /// The resource's collection base path.
    #[must_use]
    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// The underlying HTTP client.
    #[must_use]
    pub fn client(&self) -> &HttpClient {
        self.client
    }

    /// Build a full path by appending segments to the base path.
    #[must_use]
    pub fn path(&self, parts: &[&str]) -> String {
        if parts.is_empty() {
            return self.base_path.clone();
        }
        format!("{}/{}", self.base_path, parts.join("/"))
    }

    /// List resources (GET basePath).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the underlying GET request cannot
    /// reach the Space (transport failure), the API responds with a non-2xx
    /// status, or the response body is not valid JSON.
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.base_path, params)
    }

    /// Create a new resource (POST basePath).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the underlying POST request cannot
    /// reach the Space (transport failure), the API responds with a non-2xx
    /// status (e.g. 422 when `data` fails server-side validation), or the
    /// response body is not valid JSON.
    pub fn create(&self, data: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, data)
    }

    /// Retrieve a single resource by ID (GET basePath/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the underlying GET request cannot
    /// reach the Space (transport failure), the API responds with a non-2xx
    /// status (e.g. 404 when no resource has the given `id`), or the response
    /// body is not valid JSON.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.path(&[id]), &HashMap::new())
    }

    /// Update a resource by ID (PUT/PATCH basePath/{id}, per `update_method`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the underlying PUT/PATCH request cannot
    /// reach the Space (transport failure), the API responds with a non-2xx
    /// status (e.g. 404 for a missing `id` or 422 when `data` fails
    /// validation), or the response body is not valid JSON.
    pub fn update(&self, id: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        let path = self.path(&[id]);
        if self.update_method.eq_ignore_ascii_case("PUT") {
            self.client.put(&path, data)
        } else {
            self.client.patch(&path, data)
        }
    }

    /// Delete a resource by ID (DELETE basePath/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the underlying DELETE request cannot
    /// reach the Space (transport failure), the API responds with a non-2xx
    /// status (e.g. 404 when no resource has the given `id`), or the response
    /// body is not valid JSON.
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

    fn make_resource() -> (
        crate::rest::http_client::HttpClient,
        std::sync::Arc<StubTransport>,
    ) {
        crate::rest::http_client::HttpClient::with_stub(
            "proj",
            "tok",
            "https://test.signalwire.com",
        )
    }

    #[test]
    fn test_base_path() {
        let (client, _) = make_resource();
        let crud = CrudResource::new(&client, "/api/phone_numbers", "PUT");
        assert_eq!(crud.base_path(), "/api/phone_numbers");
    }

    #[test]
    fn test_path_building() {
        let (client, _) = make_resource();
        let crud = CrudResource::new(&client, "/api/items", "PUT");
        assert_eq!(crud.path(&[]), "/api/items");
        assert_eq!(crud.path(&["123"]), "/api/items/123");
        assert_eq!(crud.path(&["123", "sub"]), "/api/items/123/sub");
    }

    #[test]
    fn test_list() {
        let (client, stub) = make_resource();
        stub.set_response(200, r#"{"data":[{"id":"1"}]}"#);

        let crud = CrudResource::new(&client, "/api/items", "PUT");
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

        let crud = CrudResource::new(&client, "/api/items", "PUT");
        let result = crud.create(&json!({"name": "test"})).unwrap();
        assert_eq!(result["id"], "new-1");

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "POST");
    }

    #[test]
    fn test_get() {
        let (client, stub) = make_resource();
        stub.set_response(200, r#"{"id":"123","name":"item"}"#);

        let crud = CrudResource::new(&client, "/api/items", "PUT");
        let result = crud.get("123").unwrap();
        assert_eq!(result["id"], "123");

        let reqs = stub.requests.lock().unwrap();
        assert!(reqs[0].1.contains("/api/items/123"));
    }

    #[test]
    fn test_update() {
        let (client, stub) = make_resource();
        stub.set_response(200, r#"{"id":"123","name":"updated"}"#);

        let crud = CrudResource::new(&client, "/api/items", "PUT");
        let result = crud.update("123", &json!({"name": "updated"})).unwrap();
        assert_eq!(result["name"], "updated");

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PUT");
    }

    #[test]
    fn test_update_patch() {
        let (client, stub) = make_resource();
        stub.set_response(200, r#"{"id":"123","name":"patched"}"#);

        let crud = CrudResource::new(&client, "/api/items", "PATCH");
        let result = crud.update("123", &json!({"name": "patched"})).unwrap();
        assert_eq!(result["name"], "patched");

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "PATCH");
        assert!(reqs[0].1.contains("/api/items/123"));
    }

    #[test]
    fn test_delete() {
        let (client, stub) = make_resource();
        stub.set_response(204, "");

        let crud = CrudResource::new(&client, "/api/items", "PUT");
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

        let crud = CrudResource::new(&client, "/api/items", "PUT");
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

        let crud = CrudResource::new(&client, "/api/items", "PUT");
        let err = crud.get("missing").unwrap_err();
        assert_eq!(err.status_code(), 404);
    }
}
