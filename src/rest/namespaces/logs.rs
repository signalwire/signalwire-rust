// Copyright (c) 2025 SignalWire
//
// This file is part of the SignalWire AI Agents SDK.
//
// Licensed under the MIT License.

//! Logs namespace — message, voice, fax, conference logs (read-only).
//!
//! Mirrors `signalwire.rest.namespaces.logs.LogsNamespace`. Each
//! sub-resource binds to a different sub-API path because the upstream
//! specs live in different documents.

use std::collections::HashMap;

use serde_json::Value;

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

fn join(parts: &[&str]) -> String {
    parts.join("/")
}

fn params_to_string_map(params: &Value) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if let Some(obj) = params.as_object() {
        for (k, v) in obj {
            let s = match v {
                Value::String(s) => s.clone(),
                Value::Null => continue,
                other => other.to_string(),
            };
            out.insert(k.clone(), s);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Top-level namespace
// ---------------------------------------------------------------------------

pub struct Logs<'a> {
    client: &'a HttpClient,
}

impl<'a> Logs<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Logs { client }
    }

    pub fn client(&self) -> &HttpClient {
        self.client
    }

    pub fn messages(&self) -> MessageLogs<'a> {
        MessageLogs::new(self.client, "/api/messaging/logs")
    }

    pub fn voice(&self) -> VoiceLogs<'a> {
        VoiceLogs::new(self.client, "/api/voice/logs")
    }

    pub fn fax(&self) -> FaxLogs<'a> {
        FaxLogs::new(self.client, "/api/fax/logs")
    }

    pub fn conferences(&self) -> ConferenceLogs<'a> {
        ConferenceLogs::new(self.client, "/api/logs/conferences")
    }
}

// ---------------------------------------------------------------------------
// MessageLogs
// ---------------------------------------------------------------------------

pub struct MessageLogs<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> MessageLogs<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        MessageLogs {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn get(&self, log_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, log_id]);
        self.client.get(&p, &HashMap::new())
    }
}

// ---------------------------------------------------------------------------
// VoiceLogs
// ---------------------------------------------------------------------------

pub struct VoiceLogs<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> VoiceLogs<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        VoiceLogs {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn get(&self, log_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, log_id]);
        self.client.get(&p, &HashMap::new())
    }

    pub fn list_events(
        &self,
        log_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let p = join(&[&self.base_path, log_id, "events"]);
        self.client.get(&p, &qp)
    }
}

// ---------------------------------------------------------------------------
// FaxLogs
// ---------------------------------------------------------------------------

pub struct FaxLogs<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> FaxLogs<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        FaxLogs {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn get(&self, log_id: &str) -> Result<Value, SignalWireRestError> {
        let p = join(&[&self.base_path, log_id]);
        self.client.get(&p, &HashMap::new())
    }
}

// ---------------------------------------------------------------------------
// ConferenceLogs
// ---------------------------------------------------------------------------

pub struct ConferenceLogs<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> ConferenceLogs<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        ConferenceLogs {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
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
    fn test_paths() {
        let (c, _) = make();
        let l = Logs::new(&c);
        assert_eq!(l.messages().base_path(), "/api/messaging/logs");
        assert_eq!(l.voice().base_path(), "/api/voice/logs");
        assert_eq!(l.fax().base_path(), "/api/fax/logs");
        assert_eq!(l.conferences().base_path(), "/api/logs/conferences");
    }
}
