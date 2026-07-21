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
//!
//! `CrudResource` is defined once in [`crate::rest::crud_resource`] (also the
//! public `rest::CrudResource`) and re-exported here — there is a single CRUD
//! base, not a generated-layer duplicate.

use std::collections::HashMap;

use percent_encoding::{AsciiSet, CONTROLS};
use serde_json::Value;

use super::error::SignalWireRestError;
use super::http_client::HttpClient;
use super::pagination::PaginatedIterator;
use super::request_options::RequestOptions;

/// Characters escaped when percent-encoding a URL path segment. Starts from the
/// full control set and adds every character that is NOT an RFC 3986 unreserved
/// character (`ALPHA / DIGIT / - . _ ~`): the generic/sub delimiters, space, and
/// the query/fragment introducers. This keeps common ids (`ORD-123`, `a_b.c`)
/// byte-identical on the wire while escaping anything that could break out of
/// the segment.
const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'&')
    .add(b'+')
    .add(b',')
    .add(b'@')
    .add(b'[')
    .add(b']')
    .add(b'\\')
    .add(b'^')
    .add(b'|');

/// The single canonical CRUD base. `CrudResource` is defined once in
/// `crud_resource.rs` (also the public `rest::CrudResource`); the generated
/// resource layer composes it through this module, so it is re-exported here to
/// keep the generated `use crate::rest::generated_bases::CrudResource` imports
/// resolving against the one definition.
pub use super::crud_resource::CrudResource;

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
    ///
    /// Returns the client with the base's own `'a` lifetime so callers (e.g.
    /// [`ReadResource::paginate`]) can hand it to a [`PaginatedIterator`] that
    /// outlives the `&self` borrow.
    #[must_use]
    pub fn client(&self) -> &'a HttpClient {
        self.client
    }

    /// Build a full path by appending `parts` to the base path.
    ///
    /// Each part is percent-encoded as a URL path segment: a resource id
    /// containing reserved characters (space, `/`, `?`, `#`, unicode, ...) is
    /// escaped so it can't break out of its segment or corrupt the request line.
    /// The base path itself is trusted (composed from the spec) and passed
    /// through verbatim.
    #[must_use]
    pub fn path(&self, parts: &[&str]) -> String {
        if parts.is_empty() {
            return self.base_path.clone();
        }
        let encoded: Vec<String> = parts
            .iter()
            .map(|p| percent_encoding::utf8_percent_encode(p, PATH_SEGMENT).to_string())
            .collect();
        format!("{}/{}", self.base_path, encoded.join("/"))
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
        self.list_with_options(params, None)
    }

    /// `list` with a per-request [`RequestOptions`] override (plan 4.2). Options
    /// are forwarded to the HTTP layer only, never serialized.
    ///
    /// # Errors
    /// As [`list`](Self::list).
    pub fn list_with_options(
        &self,
        params: &HashMap<String, String>,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.base
            .client()
            .get_with_options(self.base.base_path(), params, request_options)
    }

    /// Iterate every item across all pages of this resource's list endpoint.
    ///
    /// Returns a lazy [`PaginatedIterator`] that follows the response's
    /// `links.next` cursor and yields each item under the `"data"` key. Mirrors
    /// the Python reference's `ReadResource.paginate(**params)`; see
    /// [`CrudResource::paginate`](super::CrudResource::paginate) for the full
    /// contract.
    #[must_use]
    pub fn paginate(&self, params: &HashMap<String, String>) -> PaginatedIterator<'a> {
        PaginatedIterator::new(
            self.base.client(),
            self.base.base_path(),
            params.clone(),
            "data",
            None,
        )
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
            self.base.client(),
            self.base.base_path(),
            params.clone(),
            "data",
            request_options,
        )
    }

    /// Retrieve a single resource by id (GET base/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
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
        self.base.client().get_with_options(
            &self.base.path(&[id]),
            &HashMap::new(),
            request_options,
        )
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

    /// `list` with a per-request [`RequestOptions`] override (plan 4.2).
    ///
    /// # Errors
    /// As [`list`](Self::list).
    pub fn list_with_options(
        &self,
        params: &HashMap<String, String>,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.base.list_with_options(params, request_options)
    }

    /// Create a new resource (POST base path).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn create(&self, data: &Value) -> Result<Value, SignalWireRestError> {
        self.base.create(data)
    }

    /// `create` with a per-request [`RequestOptions`] override (plan 4.2).
    ///
    /// # Errors
    /// As [`create`](Self::create).
    pub fn create_with_options(
        &self,
        data: &Value,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.base.create_with_options(data, request_options)
    }

    /// Retrieve a single resource by id (GET base/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.base.get(id)
    }

    /// `get` with a per-request [`RequestOptions`] override (plan 4.2).
    ///
    /// # Errors
    /// As [`get`](Self::get).
    pub fn get_with_options(
        &self,
        id: &str,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.base.get_with_options(id, request_options)
    }

    /// Update a resource by id (PUT/PATCH base/{id}, per `update_method`).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn update(&self, id: &str, data: &Value) -> Result<Value, SignalWireRestError> {
        self.base.update(id, data)
    }

    /// `update` with a per-request [`RequestOptions`] override (plan 4.2).
    ///
    /// # Errors
    /// As [`update`](Self::update).
    pub fn update_with_options(
        &self,
        id: &str,
        data: &Value,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.base.update_with_options(id, data, request_options)
    }

    /// Delete a resource by id (DELETE base/{id}).
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an unparseable response body.
    pub fn delete(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.base.delete(id)
    }

    /// `delete` with a per-request [`RequestOptions`] override (plan 4.2).
    ///
    /// # Errors
    /// As [`delete`](Self::delete).
    pub fn delete_with_options(
        &self,
        id: &str,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.base.delete_with_options(id, request_options)
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
        self.list_addresses_with_options(id, params, None)
    }

    /// `list_addresses` with a per-request [`RequestOptions`] override (plan 4.2).
    ///
    /// # Errors
    /// As [`list_addresses`](Self::list_addresses).
    pub fn list_addresses_with_options(
        &self,
        id: &str,
        params: &HashMap<String, String>,
        request_options: Option<&RequestOptions>,
    ) -> Result<Value, SignalWireRestError> {
        self.base.client().get_with_options(
            &self.base.path(&[id, "addresses"]),
            params,
            request_options,
        )
    }
}

#[cfg(test)]
mod path_encoding_tests {
    use super::*;

    fn stub_client() -> HttpClient {
        let (client, _stub) = HttpClient::with_stub("p", "t", "https://x.signalwire.com");
        client
    }

    /// Path segments (resource ids) with reserved characters are percent-encoded
    /// so an id cannot break out of its segment or corrupt the request line.
    /// Regression guard for the raw-`parts.join("/")` bug.
    #[test]
    fn path_encodes_reserved_chars_in_segments() {
        let client = stub_client();
        let base = BaseResource::new(&client, "/api/fabric/resources");
        // A slash inside an id must NOT introduce a new path segment; a space,
        // `?`, `#`, and unicode must be escaped.
        let p = base.path(&["a/b c?d#e", "sub"]);
        assert_eq!(p, "/api/fabric/resources/a%2Fb%20c%3Fd%23e/sub");
    }

    /// RFC 3986 unreserved characters common in ids stay byte-identical so the
    /// wire path the server expects is unchanged.
    #[test]
    fn path_preserves_unreserved_chars() {
        let client = stub_client();
        let base = BaseResource::new(&client, "/api/orders");
        assert_eq!(base.path(&["ORD-123_v.2~x"]), "/api/orders/ORD-123_v.2~x");
    }

    /// The empty-parts case returns the base path verbatim.
    #[test]
    fn path_empty_parts_is_base() {
        let client = stub_client();
        let base = BaseResource::new(&client, "/api/orders");
        assert_eq!(base.path(&[]), "/api/orders");
    }
}
