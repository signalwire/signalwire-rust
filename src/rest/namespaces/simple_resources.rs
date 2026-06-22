// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.

//! Top-level relay-rest resources whose verb set is NARROWER than full CRUD.
//!
//! Python models each of these as its own `BaseResource` subclass exposing
//! only the verbs the platform actually supports — NOT the generic
//! `CrudResource`. Exposing the missing verbs (e.g. `update` on addresses,
//! `create`/`update`/`delete`/`get`/`list` on imported numbers) would be
//! invented surface absent from both python and the canonical spec, so each
//! type here mirrors python's verb set exactly:
//!
//! | resource          | verbs                  | python source                    |
//! |-------------------|------------------------|----------------------------------|
//! | addresses         | list, create, get, delete | `namespaces/addresses.py`     |
//! | recordings        | list, get, delete      | `namespaces/recordings.py`       |
//! | short_codes       | list, get, update      | `namespaces/short_codes.py`      |
//! | imported_numbers  | create                 | `namespaces/imported_numbers.py` |

use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;
use crate::rest::util::join;

/// Addresses (`/api/relay/rest/addresses`) — list / create / get / delete.
/// Mirrors python `AddressesResource` (no `update`).
pub struct AddressesResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> AddressesResource<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        AddressesResource {
            client,
            base_path: "/api/relay/rest/addresses".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// GET `/api/relay/rest/addresses` — list addresses.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an invalid JSON body.
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.base_path, params)
    }

    /// POST `/api/relay/rest/addresses` — create an address.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status
    /// (e.g. 422), or an invalid JSON body.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// GET `/api/relay/rest/addresses/{id}` — fetch one address.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status
    /// (e.g. 404), or an invalid JSON body.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&join(&[&self.base_path, id]), &HashMap::new())
    }

    /// DELETE `/api/relay/rest/addresses/{id}` — delete an address.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status
    /// (e.g. 404), or an invalid JSON body.
    pub fn delete(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join(&[&self.base_path, id]))
    }
}

/// Recordings (`/api/relay/rest/recordings`) — list / get / delete.
/// Mirrors python `RecordingsResource` (no `create`/`update`).
pub struct RecordingsResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> RecordingsResource<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        RecordingsResource {
            client,
            base_path: "/api/relay/rest/recordings".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// GET `/api/relay/rest/recordings` — list recordings.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an invalid JSON body.
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.base_path, params)
    }

    /// GET `/api/relay/rest/recordings/{id}` — fetch one recording.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status
    /// (e.g. 404), or an invalid JSON body.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&join(&[&self.base_path, id]), &HashMap::new())
    }

    /// DELETE `/api/relay/rest/recordings/{id}` — delete a recording.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status
    /// (e.g. 404), or an invalid JSON body.
    pub fn delete(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join(&[&self.base_path, id]))
    }
}

/// Short codes (`/api/relay/rest/short_codes`) — list / get / update.
/// Mirrors python `ShortCodesResource` (no `create`/`delete`).
pub struct ShortCodesResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> ShortCodesResource<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        ShortCodesResource {
            client,
            base_path: "/api/relay/rest/short_codes".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// GET `/api/relay/rest/short_codes` — list short codes.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status,
    /// or an invalid JSON body.
    pub fn list(&self, params: &HashMap<String, String>) -> Result<Value, SignalWireRestError> {
        self.client.get(&self.base_path, params)
    }

    /// GET `/api/relay/rest/short_codes/{id}` — fetch one short code.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status
    /// (e.g. 404), or an invalid JSON body.
    pub fn get(&self, id: &str) -> Result<Value, SignalWireRestError> {
        self.client
            .get(&join(&[&self.base_path, id]), &HashMap::new())
    }

    /// PUT `/api/relay/rest/short_codes/{id}` — update a short code.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status
    /// (e.g. 404 or 422), or an invalid JSON body.
    pub fn update(&self, id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.put(&join(&[&self.base_path, id]), params)
    }
}

/// Imported phone numbers (`/api/relay/rest/imported_phone_numbers`) — create
/// only. Mirrors python `ImportedNumbersResource` (no list/get/update/delete).
pub struct ImportedNumbersResource<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> ImportedNumbersResource<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        ImportedNumbersResource {
            client,
            base_path: "/api/relay/rest/imported_phone_numbers".to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// POST `/api/relay/rest/imported_phone_numbers` — import a number.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] on transport failure, a non-2xx status
    /// (e.g. 422), or an invalid JSON body.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }
}
