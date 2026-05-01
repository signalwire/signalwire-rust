use std::collections::HashMap;

use serde_json::{json, Value};

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// Compat (Twilio-compatible LAML) API namespace.
///
/// Mirrors `signalwire.rest.namespaces.compat.CompatNamespace` from the
/// Python SDK. The base path is
/// `/api/laml/2010-04-01/Accounts/{account_sid}` and every sub-resource
/// is rooted under that.
pub struct Compat<'a> {
    client: &'a HttpClient,
    account_sid: String,
}

impl<'a> Compat<'a> {
    pub fn new(client: &'a HttpClient, account_sid: &str) -> Self {
        Compat {
            client,
            account_sid: account_sid.to_string(),
        }
    }

    pub fn client(&self) -> &HttpClient {
        self.client
    }

    pub fn account_sid(&self) -> &str {
        &self.account_sid
    }

    fn base(&self) -> String {
        format!("/api/laml/2010-04-01/Accounts/{}", self.account_sid)
    }

    // -- Sub-resource accessors --

    pub fn accounts(&self) -> CompatAccounts<'a> {
        CompatAccounts::new(self.client, "/api/laml/2010-04-01/Accounts")
    }

    pub fn calls(&self) -> CompatCalls<'a> {
        CompatCalls::new(self.client, &format!("{}/Calls", self.base()))
    }

    pub fn messages(&self) -> CompatMessages<'a> {
        CompatMessages::new(self.client, &format!("{}/Messages", self.base()))
    }

    pub fn faxes(&self) -> CompatFaxes<'a> {
        CompatFaxes::new(self.client, &format!("{}/Faxes", self.base()))
    }

    pub fn conferences(&self) -> CompatConferences<'a> {
        CompatConferences::new(self.client, &format!("{}/Conferences", self.base()))
    }

    pub fn phone_numbers(&self) -> CompatPhoneNumbers<'a> {
        CompatPhoneNumbers::new(self.client, &format!("{}/IncomingPhoneNumbers", self.base()))
    }

    pub fn applications(&self) -> CompatApplications<'a> {
        CompatApplications::new(self.client, &format!("{}/Applications", self.base()))
    }

    pub fn laml_bins(&self) -> CompatLamlBins<'a> {
        CompatLamlBins::new(self.client, &format!("{}/LamlBins", self.base()))
    }

    pub fn queues(&self) -> CompatQueues<'a> {
        CompatQueues::new(self.client, &format!("{}/Queues", self.base()))
    }

    pub fn recordings(&self) -> CompatRecordings<'a> {
        CompatRecordings::new(self.client, &format!("{}/Recordings", self.base()))
    }

    pub fn transcriptions(&self) -> CompatTranscriptions<'a> {
        CompatTranscriptions::new(self.client, &format!("{}/Transcriptions", self.base()))
    }

    pub fn tokens(&self) -> CompatTokens<'a> {
        CompatTokens::new(self.client, &format!("{}/tokens", self.base()))
    }
}

// -----------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------

fn join_path(base: &str, parts: &[&str]) -> String {
    if parts.is_empty() {
        return base.to_string();
    }
    format!("{}/{}", base, parts.join("/"))
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

// -----------------------------------------------------------------
// CompatAccounts
// -----------------------------------------------------------------

pub struct CompatAccounts<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatAccounts<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatAccounts {
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

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn update(&self, sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&join_path(&self.base_path, &[sid]), params)
    }
}

// -----------------------------------------------------------------
// CompatCalls
// -----------------------------------------------------------------

pub struct CompatCalls<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatCalls<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatCalls {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn update(&self, sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&join_path(&self.base_path, &[sid]), params)
    }

    /// POST /Calls/{sid}/Recordings — start a new recording on the call.
    pub fn start_recording(
        &self,
        call_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[call_sid, "Recordings"]);
        self.client.post(&path, params)
    }

    /// POST /Calls/{sid}/Recordings/{rec_sid} — update a specific recording.
    pub fn update_recording(
        &self,
        call_sid: &str,
        recording_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[call_sid, "Recordings", recording_sid]);
        self.client.post(&path, params)
    }

    /// POST /Calls/{sid}/Streams — start a new stream on the call.
    pub fn start_stream(
        &self,
        call_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[call_sid, "Streams"]);
        self.client.post(&path, params)
    }

    /// POST /Calls/{sid}/Streams/{stream_sid} — stop / update a stream.
    pub fn stop_stream(
        &self,
        call_sid: &str,
        stream_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[call_sid, "Streams", stream_sid]);
        self.client.post(&path, params)
    }
}

// -----------------------------------------------------------------
// CompatMessages
// -----------------------------------------------------------------

pub struct CompatMessages<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatMessages<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatMessages {
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

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn update(&self, sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&join_path(&self.base_path, &[sid]), params)
    }

    pub fn delete(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join_path(&self.base_path, &[sid]))
    }

    pub fn list_media(
        &self,
        message_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let path = join_path(&self.base_path, &[message_sid, "Media"]);
        self.client.get(&path, &qp)
    }

    pub fn get_media(
        &self,
        message_sid: &str,
        media_sid: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[message_sid, "Media", media_sid]);
        self.client.get(&path, &HashMap::new())
    }

    pub fn delete_media(
        &self,
        message_sid: &str,
        media_sid: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[message_sid, "Media", media_sid]);
        self.client.delete(&path)
    }
}

// -----------------------------------------------------------------
// CompatFaxes
// -----------------------------------------------------------------

pub struct CompatFaxes<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatFaxes<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatFaxes {
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

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn update(&self, sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&join_path(&self.base_path, &[sid]), params)
    }

    pub fn delete(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join_path(&self.base_path, &[sid]))
    }

    pub fn list_media(&self, fax_sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let path = join_path(&self.base_path, &[fax_sid, "Media"]);
        self.client.get(&path, &qp)
    }

    pub fn get_media(
        &self,
        fax_sid: &str,
        media_sid: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[fax_sid, "Media", media_sid]);
        self.client.get(&path, &HashMap::new())
    }

    pub fn delete_media(
        &self,
        fax_sid: &str,
        media_sid: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[fax_sid, "Media", media_sid]);
        self.client.delete(&path)
    }
}

// -----------------------------------------------------------------
// CompatConferences
// -----------------------------------------------------------------

pub struct CompatConferences<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatConferences<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatConferences {
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

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn update(&self, sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&join_path(&self.base_path, &[sid]), params)
    }

    pub fn list_participants(
        &self,
        conference_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let path = join_path(&self.base_path, &[conference_sid, "Participants"]);
        self.client.get(&path, &qp)
    }

    pub fn get_participant(
        &self,
        conference_sid: &str,
        call_sid: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[conference_sid, "Participants", call_sid]);
        self.client.get(&path, &HashMap::new())
    }

    pub fn update_participant(
        &self,
        conference_sid: &str,
        call_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[conference_sid, "Participants", call_sid]);
        self.client.post(&path, params)
    }

    pub fn remove_participant(
        &self,
        conference_sid: &str,
        call_sid: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[conference_sid, "Participants", call_sid]);
        self.client.delete(&path)
    }

    pub fn list_recordings(
        &self,
        conference_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let path = join_path(&self.base_path, &[conference_sid, "Recordings"]);
        self.client.get(&path, &qp)
    }

    pub fn get_recording(
        &self,
        conference_sid: &str,
        recording_sid: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[conference_sid, "Recordings", recording_sid]);
        self.client.get(&path, &HashMap::new())
    }

    pub fn update_recording(
        &self,
        conference_sid: &str,
        recording_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[conference_sid, "Recordings", recording_sid]);
        self.client.post(&path, params)
    }

    pub fn delete_recording(
        &self,
        conference_sid: &str,
        recording_sid: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[conference_sid, "Recordings", recording_sid]);
        self.client.delete(&path)
    }

    pub fn start_stream(
        &self,
        conference_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[conference_sid, "Streams"]);
        self.client.post(&path, params)
    }

    pub fn stop_stream(
        &self,
        conference_sid: &str,
        stream_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[conference_sid, "Streams", stream_sid]);
        self.client.post(&path, params)
    }
}

// -----------------------------------------------------------------
// CompatPhoneNumbers
// -----------------------------------------------------------------

pub struct CompatPhoneNumbers<'a> {
    client: &'a HttpClient,
    base_path: String,
    available_base: String,
}

impl<'a> CompatPhoneNumbers<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        let available_base =
            base_path.replace("/IncomingPhoneNumbers", "/AvailablePhoneNumbers");
        CompatPhoneNumbers {
            client,
            base_path: base_path.to_string(),
            available_base,
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn available_base(&self) -> &str {
        &self.available_base
    }

    pub fn list(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.base_path, &qp)
    }

    pub fn purchase(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn update(&self, sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&join_path(&self.base_path, &[sid]), params)
    }

    pub fn delete(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join_path(&self.base_path, &[sid]))
    }

    /// POST /ImportedPhoneNumbers — note the path is *Imported*, not *Incoming*.
    pub fn import_number(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        let path = self
            .base_path
            .replace("/IncomingPhoneNumbers", "/ImportedPhoneNumbers");
        self.client.post(&path, params)
    }

    pub fn list_available_countries(
        &self,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        self.client.get(&self.available_base, &qp)
    }

    pub fn search_local(
        &self,
        country: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let path = format!("{}/{}/Local", self.available_base, country);
        self.client.get(&path, &qp)
    }

    pub fn search_toll_free(
        &self,
        country: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let path = format!("{}/{}/TollFree", self.available_base, country);
        self.client.get(&path, &qp)
    }
}

// -----------------------------------------------------------------
// CompatApplications
// -----------------------------------------------------------------

pub struct CompatApplications<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatApplications<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatApplications {
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

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn update(&self, sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&join_path(&self.base_path, &[sid]), params)
    }

    pub fn delete(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join_path(&self.base_path, &[sid]))
    }
}

// -----------------------------------------------------------------
// CompatLamlBins
// -----------------------------------------------------------------

pub struct CompatLamlBins<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatLamlBins<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatLamlBins {
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

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn update(&self, sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&join_path(&self.base_path, &[sid]), params)
    }

    pub fn delete(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join_path(&self.base_path, &[sid]))
    }
}

// -----------------------------------------------------------------
// CompatQueues
// -----------------------------------------------------------------

pub struct CompatQueues<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatQueues<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatQueues {
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

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn update(&self, sid: &str, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&join_path(&self.base_path, &[sid]), params)
    }

    pub fn delete(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join_path(&self.base_path, &[sid]))
    }

    pub fn list_members(
        &self,
        queue_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let qp = params_to_string_map(params);
        let path = join_path(&self.base_path, &[queue_sid, "Members"]);
        self.client.get(&path, &qp)
    }

    pub fn get_member(
        &self,
        queue_sid: &str,
        call_sid: &str,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[queue_sid, "Members", call_sid]);
        self.client.get(&path, &HashMap::new())
    }

    pub fn dequeue_member(
        &self,
        queue_sid: &str,
        call_sid: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        let path = join_path(&self.base_path, &[queue_sid, "Members", call_sid]);
        self.client.post(&path, params)
    }
}

// -----------------------------------------------------------------
// CompatRecordings
// -----------------------------------------------------------------

pub struct CompatRecordings<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatRecordings<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatRecordings {
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

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn delete(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join_path(&self.base_path, &[sid]))
    }
}

// -----------------------------------------------------------------
// CompatTranscriptions
// -----------------------------------------------------------------

pub struct CompatTranscriptions<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatTranscriptions<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatTranscriptions {
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

    pub fn get(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.get(&join_path(&self.base_path, &[sid]), &HashMap::new())
    }

    pub fn delete(&self, sid: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join_path(&self.base_path, &[sid]))
    }
}

// -----------------------------------------------------------------
// CompatTokens
// -----------------------------------------------------------------

pub struct CompatTokens<'a> {
    client: &'a HttpClient,
    base_path: String,
}

impl<'a> CompatTokens<'a> {
    pub fn new(client: &'a HttpClient, base_path: &str) -> Self {
        CompatTokens {
            client,
            base_path: base_path.to_string(),
        }
    }

    pub fn base_path(&self) -> &str {
        &self.base_path
    }

    pub fn create(&self, params: &Value) -> Result<Value, SignalWireRestError> {
        self.client.post(&self.base_path, params)
    }

    pub fn update(
        &self,
        token_id: &str,
        params: &Value,
    ) -> Result<Value, SignalWireRestError> {
        self.client.patch(&join_path(&self.base_path, &[token_id]), params)
    }

    pub fn delete(&self, token_id: &str) -> Result<Value, SignalWireRestError> {
        self.client.delete(&join_path(&self.base_path, &[token_id]))
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::http_client::StubTransport;

    fn make_compat() -> (HttpClient, std::sync::Arc<StubTransport>) {
        HttpClient::with_stub("proj", "tok", "https://test.signalwire.com")
    }

    #[test]
    fn test_calls_base_path() {
        let (c, _) = make_compat();
        let n = Compat::new(&c, "test_proj");
        assert_eq!(
            n.calls().base_path(),
            "/api/laml/2010-04-01/Accounts/test_proj/Calls"
        );
    }

    #[test]
    fn test_messages_base_path() {
        let (c, _) = make_compat();
        let n = Compat::new(&c, "test_proj");
        assert_eq!(
            n.messages().base_path(),
            "/api/laml/2010-04-01/Accounts/test_proj/Messages"
        );
    }

    #[test]
    fn test_phone_numbers_base_path() {
        let (c, _) = make_compat();
        let n = Compat::new(&c, "test_proj");
        let pn = n.phone_numbers();
        assert_eq!(
            pn.base_path(),
            "/api/laml/2010-04-01/Accounts/test_proj/IncomingPhoneNumbers"
        );
        assert_eq!(
            pn.available_base(),
            "/api/laml/2010-04-01/Accounts/test_proj/AvailablePhoneNumbers"
        );
    }

    #[test]
    fn test_start_stream_path() {
        let (c, stub) = make_compat();
        stub.set_response(200, "{}");
        let n = Compat::new(&c, "test_proj");
        n.calls()
            .start_stream("CA_X", &json!({"Url": "wss://x"}))
            .unwrap();
        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "POST");
        assert!(reqs[0]
            .1
            .contains("/api/laml/2010-04-01/Accounts/test_proj/Calls/CA_X/Streams"));
    }

    #[test]
    fn test_import_number_path() {
        let (c, stub) = make_compat();
        stub.set_response(200, "{}");
        let n = Compat::new(&c, "test_proj");
        n.phone_numbers()
            .import_number(&json!({"PhoneNumber": "+15555550000"}))
            .unwrap();
        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "POST");
        assert!(reqs[0]
            .1
            .contains("/api/laml/2010-04-01/Accounts/test_proj/ImportedPhoneNumbers"));
    }
}
