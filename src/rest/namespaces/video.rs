// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.

//! Video API namespace — rooms, sessions, recordings, conferences, tokens,
//! streams.
//!
//! Mirrors `signalwire.rest.namespaces.video.VideoNamespace` from the Python
//! SDK. Each sub-resource exposes the methods present on the upstream
//! Python class.

use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;
use crate::rest::util::{join, params_to_string_map};

const BASE: &str = "/api/video";

// ---------------------------------------------------------------------------
// Top-level namespace
// ---------------------------------------------------------------------------

pub struct Video<'a> {
    client: &'a HttpClient,
}

impl<'a> Video<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Video { client }
    }

    pub fn client(&self) -> &HttpClient {
        self.client
    }

    pub fn rooms(&self) -> VideoRooms<'a> {
        VideoRooms::new(self.client, &format!("{BASE}/rooms"))
    }

    pub fn room_tokens(&self) -> VideoRoomTokens<'a> {
        VideoRoomTokens::new(self.client, &format!("{BASE}/room_tokens"))
    }

    pub fn room_sessions(&self) -> VideoRoomSessions<'a> {
        VideoRoomSessions::new(self.client, &format!("{BASE}/room_sessions"))
    }

    pub fn room_recordings(&self) -> VideoRoomRecordings<'a> {
        VideoRoomRecordings::new(self.client, &format!("{BASE}/room_recordings"))
    }

    pub fn conferences(&self) -> VideoConferences<'a> {
        VideoConferences::new(self.client, &format!("{BASE}/conferences"))
    }

    pub fn conference_tokens(&self) -> VideoConferenceTokens<'a> {
        VideoConferenceTokens::new(self.client, &format!("{BASE}/conference_tokens"))
    }

    pub fn streams(&self) -> VideoStreams<'a> {
        VideoStreams::new(self.client, &format!("{BASE}/streams"))
    }
}

// ---------------------------------------------------------------------------
// VideoRooms — full CRUD (PUT update) plus streams sub-resource
// ---------------------------------------------------------------------------

pub struct VideoRooms<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> VideoRooms<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        VideoRooms {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `room_id` is unknown), or the response body is not valid JSON.
    pub fn get(&self, room_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, room_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// `VideoRooms` uses PUT for update.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `room_id` is unknown), or the response body is not valid JSON.
    pub fn update(&self, room_id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, room_id]);
        self.client.put(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `room_id` is unknown), or the response body is not valid JSON.
    pub fn delete(&self, room_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, room_id]);
        self.client.delete(&p)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `room_id` is unknown), or the response body is not valid JSON.
    pub fn list_streams(
        &self,
        room_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, room_id, "streams"]);
        self.client.get(&p, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `room_id` is unknown), or the response body is not valid JSON.
    pub fn create_stream(
        &self,
        room_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, room_id, "streams"]);
        self.client.post(&p, params)
    }
}

// ---------------------------------------------------------------------------
// VideoRoomTokens — POST-only token factory
// ---------------------------------------------------------------------------

pub struct VideoRoomTokens<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> VideoRoomTokens<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        VideoRoomTokens {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }
}

// ---------------------------------------------------------------------------
// VideoRoomSessions
// ---------------------------------------------------------------------------

pub struct VideoRoomSessions<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> VideoRoomSessions<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        VideoRoomSessions {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `session_id` is unknown), or the response body is not valid JSON.
    pub fn get(&self, session_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, session_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `session_id` is unknown), or the response body is not valid JSON.
    pub fn list_events(
        &self,
        session_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, session_id, "events"]);
        self.client.get(&p, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `session_id` is unknown), or the response body is not valid JSON.
    pub fn list_members(
        &self,
        session_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, session_id, "members"]);
        self.client.get(&p, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `session_id` is unknown), or the response body is not valid JSON.
    pub fn list_recordings(
        &self,
        session_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, session_id, "recordings"]);
        self.client.get(&p, &qp)
    }
}

// ---------------------------------------------------------------------------
// VideoRoomRecordings
// ---------------------------------------------------------------------------

pub struct VideoRoomRecordings<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> VideoRoomRecordings<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        VideoRoomRecordings {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `recording_id` is unknown), or the response body is not valid JSON.
    pub fn get(&self, recording_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, recording_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `recording_id` is unknown), or the response body is not valid JSON.
    pub fn delete(&self, recording_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, recording_id]);
        self.client.delete(&p)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `recording_id` is unknown), or the response body is not valid JSON.
    pub fn list_events(
        &self,
        recording_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, recording_id, "events"]);
        self.client.get(&p, &qp)
    }
}

// ---------------------------------------------------------------------------
// VideoConferences — CRUD (PUT update) plus token / stream subpaths
// ---------------------------------------------------------------------------

pub struct VideoConferences<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> VideoConferences<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        VideoConferences {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status, or the
    /// response body is not valid JSON.
    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `conference_id` is unknown), or the response body is not valid JSON.
    pub fn get(&self, conference_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, conference_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `conference_id` is unknown), or the response body is not valid JSON.
    pub fn update(
        &self,
        conference_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, conference_id]);
        self.client.put(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `conference_id` is unknown), or the response body is not valid JSON.
    pub fn delete(&self, conference_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, conference_id]);
        self.client.delete(&p)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `conference_id` is unknown), or the response body is not valid JSON.
    pub fn list_conference_tokens(
        &self,
        conference_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, conference_id, "conference_tokens"]);
        self.client.get(&p, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `conference_id` is unknown), or the response body is not valid JSON.
    pub fn list_streams(
        &self,
        conference_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, conference_id, "streams"]);
        self.client.get(&p, &qp)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `conference_id` is unknown), or the response body is not valid JSON.
    pub fn create_stream(
        &self,
        conference_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, conference_id, "streams"]);
        self.client.post(&p, params)
    }
}

// ---------------------------------------------------------------------------
// VideoConferenceTokens — get + reset
// ---------------------------------------------------------------------------

pub struct VideoConferenceTokens<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> VideoConferenceTokens<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        VideoConferenceTokens {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `token_id` is unknown), or the response body is not valid JSON.
    pub fn get(&self, token_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, token_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// POST {base}/{id}/reset — no-body POST per Python.
    ///
    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `token_id` is unknown), or the response body is not valid JSON.
    pub fn reset(&self, token_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, token_id, "reset"]);
        self.client.post(&p, &serde_json::json!({}))
    }
}

// ---------------------------------------------------------------------------
// VideoStreams — get / update (PUT) / delete
// ---------------------------------------------------------------------------

pub struct VideoStreams<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> VideoStreams<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        VideoStreams {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `stream_id` is unknown), or the response body is not valid JSON.
    pub fn get(&self, stream_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, stream_id]);
        self.client.get(&p, &HashMap::new())
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `stream_id` is unknown), or the response body is not valid JSON.
    pub fn update(&self, stream_id: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, stream_id]);
        self.client.put(&p, params)
    }

    /// # Errors
    /// Returns [`SignalWireRestError`] if the request cannot reach the Space
    /// (transport failure), the API responds with a non-2xx status (404 if
    /// `stream_id` is unknown), or the response body is not valid JSON.
    pub fn delete(&self, stream_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, stream_id]);
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
    fn test_rooms_path() {
        let (c, _) = make();
        let v = Video::new(&c);
        assert_eq!(v.rooms().base_path(), "/api/video/rooms");
    }

    #[test]
    fn test_room_sessions_path() {
        let (c, _) = make();
        let v = Video::new(&c);
        assert_eq!(v.room_sessions().base_path(), "/api/video/room_sessions");
    }

    #[test]
    fn test_streams_path() {
        let (c, _) = make();
        let v = Video::new(&c);
        assert_eq!(v.streams().base_path(), "/api/video/streams");
    }
}
