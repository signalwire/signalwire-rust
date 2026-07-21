use std::collections::HashMap;

use serde_json::Value;

use super::error::SignalWireRestError;
use super::http_client::HttpClient;
use super::pagination::PaginatedIterator;
use super::request_options::RequestOptions;

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
        self.list_with_options(params, None)
    }

    /// `list` with a per-request [`RequestOptions`] override (plan 4.2). The
    /// options control transport behavior (timeout / retry / cancellation) and
    /// are NEVER serialized into the request — they are forwarded to the HTTP
    /// layer only.
    ///
    /// # Errors
    /// As [`list`](Self::list).
    pub fn list_with_options(
        &self,
        params: &HashMap<String, String>,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .get_with_options(&self.base_path, params, request_options)
    }

    /// Iterate every item across all pages of this resource's list endpoint.
    ///
    /// [`list`](Self::list) returns a single raw page (the server's first
    /// response). For endpoints that paginate on the wire (a `links.next`
    /// cursor in the response body), `paginate` returns a lazy
    /// [`PaginatedIterator`] that follows those cursors and yields each item
    /// under the `"data"` key:
    ///
    /// ```no_run
    /// # use std::collections::HashMap;
    /// # use signalwire::rest::CrudResource;
    /// # fn demo(resource: &CrudResource<'_>) {
    /// for item in resource.paginate(&HashMap::new()) {
    ///     let item = item.expect("page fetch failed");
    ///     // ... use item ...
    /// }
    /// # }
    /// ```
    ///
    /// Mirrors the Python reference's `ReadResource.paginate(**params)`, wiring
    /// the resource layer to the tested [`PaginatedIterator`] so callers no
    /// longer hand-build the path + cursor loop. Construction is lazy — no HTTP
    /// is dispatched until the iterator is first stepped.
    #[must_use]
    pub fn paginate(&self, params: &HashMap<String, String>) -> PaginatedIterator<'a> {
        PaginatedIterator::new(self.client, &self.base_path, params.clone(), "data", None)
    }

    /// `paginate` with a per-request [`RequestOptions`] override (plan 4.2)
    /// forwarded to every page GET. Options are never serialized.
    #[must_use]
    pub fn paginate_with_options(
        &self,
        params: &HashMap<String, String>,
        request_options: Option<RequestOptions>,
    ) -> PaginatedIterator<'a> {
        PaginatedIterator::new(
            self.client,
            &self.base_path,
            params.clone(),
            "data",
            request_options,
        )
    }

    /// Create a new resource (POST basePath).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the underlying POST request cannot
    /// reach the Space (transport failure), the API responds with a non-2xx
    /// status (e.g. 422 when `data` fails server-side validation), or the
    /// response body is not valid JSON.
    pub fn create(&self, data: &Value) -> Result<Value, SignalWireRestError> {
        self.create_with_options(data, None)
    }

    /// `create` with a per-request [`RequestOptions`] override (plan 4.2).
    /// Options are forwarded to the HTTP layer only, never serialized into the
    /// body.
    ///
    /// # Errors
    /// As [`create`](Self::create).
    pub fn create_with_options(
        &self,
        data: &Value,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .post_with_options(&self.base_path, data, request_options)
    }

    /// Retrieve a single resource by ID (GET basePath/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the underlying GET request cannot
    /// reach the Space (transport failure), the API responds with a non-2xx
    /// status (e.g. 404 when no resource has the given `id`), or the response
    /// body is not valid JSON.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.get_with_options(id, None)
    }

    /// `get` with a per-request [`RequestOptions`] override (plan 4.2). Options
    /// are forwarded to the HTTP layer only.
    ///
    /// # Errors
    /// As [`get`](Self::get).
    pub fn get_with_options(
        &self,
        id: &str,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .get_with_options(&self.path(&[id]), &HashMap::new(), request_options)
    }

    /// Update a resource by ID (PUT/PATCH basePath/{id}, per `update_method`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the underlying PUT/PATCH request cannot
    /// reach the Space (transport failure), the API responds with a non-2xx
    /// status (e.g. 404 for a missing `id` or 422 when `data` fails
    /// validation), or the response body is not valid JSON.
    pub fn update(&self, id: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        self.update_with_options(id, data, None)
    }

    /// `update` with a per-request [`RequestOptions`] override (plan 4.2).
    /// Options are forwarded to the HTTP layer only, never serialized into the
    /// body.
    ///
    /// # Errors
    /// As [`update`](Self::update).
    pub fn update_with_options(
        &self,
        id: &str,
        data: &Value,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        let path = self.path(&[id]);
        if self.update_method.eq_ignore_ascii_case("PUT") {
            self.client.put_with_options(&path, data, request_options)
        } else {
            self.client.patch_with_options(&path, data, request_options)
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
        self.delete_with_options(id, None)
    }

    /// `delete` with a per-request [`RequestOptions`] override (plan 4.2).
    /// Options are forwarded to the HTTP layer only.
    ///
    /// # Errors
    /// As [`delete`](Self::delete).
    pub fn delete_with_options(
        &self,
        id: &str,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.client
            .delete_with_options(&self.path(&[id]), request_options)
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

    #[test]
    fn test_paginate_is_lazy() {
        // Constructing the iterator dispatches no HTTP until first stepped.
        let (client, stub) = make_resource();
        let crud = CrudResource::new(&client, "/api/items", "PATCH");
        let _it = crud.paginate(&HashMap::new());
        assert!(stub.requests.lock().unwrap().is_empty());
    }

    #[test]
    fn test_paginate_follows_cursor_two_pages() {
        // paginate() must walk every page: page 1 carries links.next, page 2 is
        // terminal. All items across both pages are yielded, in order, and the
        // second GET follows the cursor from links.next.
        use crate::rest::http_client::SequencedTransport;

        let responses = vec![
            (
                200,
                r#"{"data":[{"id":"1"},{"id":"2"}],
                    "links":{"next":"/api/items?page_token=PA_page2"}}"#
                    .to_string(),
            ),
            (
                200,
                r#"{"data":[{"id":"3"}],"links":{"next":null}}"#.to_string(),
            ),
        ];
        let transport = std::sync::Arc::new(SequencedTransport::new(responses));
        let client = crate::rest::http_client::HttpClient::new(
            "proj",
            "tok",
            "https://test.signalwire.com",
            Box::new(SequencedTransport::wrapper(transport.clone())),
        );

        let crud = CrudResource::new(&client, "/api/items", "PATCH");
        let ids: Vec<String> = crud
            .paginate(&HashMap::new())
            .map(|item| item.unwrap()["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids, vec!["1", "2", "3"]);

        // Exactly two page requests; the second followed links.next's cursor.
        let reqs = transport.requests.lock().unwrap();
        assert_eq!(reqs.len(), 2);
        assert!(reqs[0].1.contains("/api/items"));
        assert!(reqs[1].1.contains("page_token=PA_page2"));
    }
}
