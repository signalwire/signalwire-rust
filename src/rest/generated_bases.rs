//! Base resources shared by the REST resource layer.
//!
//! Every REST resource composes one of four base resources, by the method set it
//! needs:
//!
//! | base             | methods                                        |
//! |------------------|------------------------------------------------|
//! | `BaseResource`   | (none — path/client/`base_path` helpers only)  |
//! | `ReadResource`   | `list`, `get`                                  |
//! | `CrudResource`   | `list`, `create`, `get`, `update`, `delete`    |
//! | `FabricResource` | CRUD + `list_addresses`                         |
//!
//! Each base bakes in its resource's base path and, for the write-capable bases,
//! the resource's `update` HTTP verb (`PUT` or `PATCH`). These carry the REST
//! behaviour (path composition, the HTTP verb dispatch, delegation to
//! `HttpClient`); each per-resource struct delegates to its base.

use std::collections::HashMap;

use serde_json::Value;

use super::error::SignalWireRestError;
use super::http_client::HttpClient;

/// Shared path/client helpers for a generated resource bound to a base API path.
///
/// `BaseResource` provides no CRUD verbs of its own; a resource built on it
/// declares every method explicitly. The path helpers here are used by those
/// declared methods.
pub struct BaseResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> BaseResource<'a> {
    /// Construct the base with its resource's collection base path (§4).
    #[must_use]
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        BaseResource {
            client,
            base_path: base_path.to_string(),
        }
    }

    /// The resource's collection base path.
    #[must_use]
    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// The underlying HTTP client (used by generated declared methods).
    #[must_use]
    pub fn client(&self) -> &HttpClient {
        self.client
    }

    /// Build a full path by appending `parts` to the base path.
    #[must_use]
    pub fn path(&self, parts: &[&str]) -> String {
        if parts.is_empty() {
            return self.base_path.clone();
        }
        format!("{}/{}", self.base_path, parts.join("/"))
    }
}

/// A read-only resource: `list` (GET base) + `get` (GET base/{id}).
pub struct ReadResource<'a> {
    base: BaseResource<'a>,
}

impl<'a> ReadResource<'a> {
    /// Construct the read resource; its base path (§4) is baked in.
    #[must_use]
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        ReadResource {
            base: BaseResource::new(client, base_path),
        }
    }

    /// The resource's collection base path.
    #[must_use]
    pub fn base_path(&self) -> &str {
        self.base.base_path()
    }

    /// The underlying HTTP client.
    #[must_use]
    pub fn client(&self) -> &HttpClient {
        self.base.client()
    }

    /// Build a full path by appending `parts` to the base path.
    #[must_use]
    pub fn path(&self, parts: &[&str]) -> String {
        self.base.path(parts)
    }

    /// List resources (GET base path).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.base.client().get(self.base.base_path(), params)
    }

    /// Retrieve a single resource by id (GET base/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.base
            .client()
            .get(&self.base.path(&[id]), &HashMap::new())
    }
}

/// A full CRUD resource: `list`, `create`, `get`, `update`, `delete`.
///
/// The `update` HTTP verb is `PUT` or `PATCH` depending on the resource; the
/// constructor passes it in.
pub struct CrudResource<'a> {
    base: BaseResource<'a>,
    update_method: String,
}

impl<'a> CrudResource<'a> {
    /// Construct the CRUD resource; base path (§4) + update verb (§9) baked in.
    #[must_use]
    pub fn new(client: &'a HttpClient, base_path: &str, update_method: &str) -> Self {
        CrudResource {
            base: BaseResource::new(client, base_path),
            update_method: update_method.to_string(),
        }
    }

    /// The resource's collection base path.
    #[must_use]
    pub fn base_path(&self) -> &str {
        self.base.base_path()
    }

    /// The underlying HTTP client.
    #[must_use]
    pub fn client(&self) -> &HttpClient {
        self.base.client()
    }

    /// Build a full path by appending `parts` to the base path.
    #[must_use]
    pub fn path(&self, parts: &[&str]) -> String {
        self.base.path(parts)
    }

    /// List resources (GET base path).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.base.client().get(self.base.base_path(), params)
    }

    /// Create a new resource (POST base path).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn create(&self, data: &Value) -> Result<Value, SignalWireRestError> {
        self.base.client().post(self.base.base_path(), data)
    }

    /// Retrieve a single resource by id (GET base/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.base
            .client()
            .get(&self.base.path(&[id]), &HashMap::new())
    }

    /// Update a resource by id (PUT/PATCH base/{id}, per `update_method`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn update(&self, id: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        let path = self.base.path(&[id]);
        if self.update_method.eq_ignore_ascii_case("PUT") {
            self.base.client().put(&path, data)
        } else {
            self.base.client().patch(&path, data)
        }
    }

    /// Delete a resource by id (DELETE base/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn delete(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.base.client().delete(&self.base.path(&[id]))
    }
}

/// A Fabric CRUD resource: CRUD + `list_addresses` (GET base/{id}/addresses).
///
/// Fabric resources share the `/api/fabric/resources/*` shape and additionally
/// expose the sub-collection of addresses bound to a resource.
pub struct FabricResource<'a> {
    base: CrudResource<'a>,
}

impl<'a> FabricResource<'a> {
    /// Construct the Fabric resource; base path (§4) + update verb (§9) baked in.
    #[must_use]
    pub fn new(client: &'a HttpClient, base_path: &str, update_method: &str) -> Self {
        FabricResource {
            base: CrudResource::new(client, base_path, update_method),
        }
    }

    /// The resource's collection base path.
    #[must_use]
    pub fn base_path(&self) -> &str {
        self.base.base_path()
    }

    /// The underlying HTTP client.
    #[must_use]
    pub fn client(&self) -> &HttpClient {
        self.base.client()
    }

    /// Build a full path by appending `parts` to the base path.
    #[must_use]
    pub fn path(&self, parts: &[&str]) -> String {
        self.base.path(parts)
    }

    /// List resources (GET base path).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.base.list(params)
    }

    /// Create a new resource (POST base path).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn create(&self, data: &Value) -> Result<Value, SignalWireRestError> {
        self.base.create(data)
    }

    /// Retrieve a single resource by id (GET base/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.base.get(id)
    }

    /// Update a resource by id (PUT/PATCH base/{id}, per `update_method`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn update(&self, id: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        self.base.update(id, data)
    }

    /// Delete a resource by id (DELETE base/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn delete(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.base.delete(id)
    }

    /// List the addresses bound to a resource (GET base/{id}/addresses).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn list_addresses(
        &self,
        id: &str,
        params: &HashMap<String, String>,
    ) -> Result<Value, SignalWireRestError> {
        self.base
            .client()
            .get(&self.base.path(&[id, "addresses"]), params)
    }
}
