//! Phone Number Lookup namespace.
//!
//! Mirrors the Python reference (`signalwire.rest.namespaces.lookup`): a single
//! lookup operation, NOT a CRUD resource. The only canonical route is
//! `GET /api/relay/rest/lookup/phone_number/{e164}`
//! (`relay-rest.lookup_phone_number`).

use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

const BASE: &str = "/api/relay/rest/lookup";

/// Phone number lookup (carrier / CNAM).
pub struct LookupResource<'a> {
    client: &'a HttpClient,
}

impl<'a> LookupResource<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        LookupResource { client }
    }

    /// Base path for this namespace (`/api/relay/rest/lookup`).
    pub fn base_path(&self) -> &str {
        BASE
    }

    /// Look up carrier / CNAM data for an E.164 phone number.
    /// `GET /api/relay/rest/lookup/phone_number/{e164}`.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (notably
    /// 404 if the number is unknown), or the response body is not valid JSON.
    pub fn phone_number(&self, e164: &str) -> Result<Value, SignalWireRestError> {
        let p = format!("{BASE}/phone_number/{e164}");
        self.client.get(&p, &HashMap::new())
    }
}
