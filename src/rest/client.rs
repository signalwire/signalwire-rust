use std::env;

use super::http_client::{HttpClient, UreqTransport};

/// Top-level SignalWire REST client.
///
/// Provides lazy access to every API namespace (fabric, calling,
/// `phone_numbers`, datasphere, video, compat, etc.). Credentials can
/// be supplied explicitly or pulled from environment variables.
///
/// Production HTTP transport is `ureq` (sync, blocking, real network
/// I/O). Tests can substitute a stub via [`with_http`].
pub struct RestClient {
    project_id: String,
    token: String,
    space: String,
    base_url: String,
    http: HttpClient,
}

impl RestClient {
    /// Create a new REST client with explicit credentials. The base URL
    /// resolves to `https://{space}`. Use [`with_base_url`] to override
    /// (e.g. for fixture-driven tests pointed at `http://127.0.0.1:N`).
    ///
    /// # Errors
    /// Returns `Err(String)` if any required credential is empty: `project_id`,
    /// `token`, or `space`. No network request is made here.
    pub fn new(project_id: &str, token: &str, space: &str) -> Result<Self, String> {
        if project_id.is_empty() {
            return Err(
                "projectId is required (pass explicitly or set SIGNALWIRE_PROJECT_ID)".to_string(),
            );
        }
        if token.is_empty() {
            return Err(
                "token is required (pass explicitly or set SIGNALWIRE_API_TOKEN)".to_string(),
            );
        }
        if space.is_empty() {
            return Err("space is required (pass explicitly or set SIGNALWIRE_SPACE)".to_string());
        }

        let base_url = format!("https://{space}");
        let http = HttpClient::new(project_id, token, &base_url, Box::new(UreqTransport::new()));

        Ok(RestClient {
            project_id: project_id.to_string(),
            token: token.to_string(),
            space: space.to_string(),
            base_url,
            http,
        })
    }

    /// Create a REST client with an explicit base URL. Used by audit
    /// harnesses and integration tests to point at a local fixture
    /// without going through the `https://{space}` resolution. Production
    /// callers should use [`new`] instead.
    ///
    /// # Errors
    /// Returns `Err(String)` if any required argument is empty: `project_id`,
    /// `token`, or `base_url`. No network request is made here.
    pub fn with_base_url(project_id: &str, token: &str, base_url: &str) -> Result<Self, String> {
        if project_id.is_empty() {
            return Err("projectId is required".to_string());
        }
        if token.is_empty() {
            return Err("token is required".to_string());
        }
        if base_url.is_empty() {
            return Err("base_url is required".to_string());
        }
        let http = HttpClient::new(project_id, token, base_url, Box::new(UreqTransport::new()));
        Ok(RestClient {
            project_id: project_id.to_string(),
            token: token.to_string(),
            space: base_url.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        })
    }

    /// Create a REST client with a specific HTTP client (for testing).
    ///
    /// # Errors
    /// Returns `Err(String)` if any of `project_id`, `token`, or `space` is
    /// empty. No network request is made here.
    pub fn with_http(
        project_id: &str,
        token: &str,
        space: &str,
        http: HttpClient,
    ) -> Result<Self, String> {
        if project_id.is_empty() || token.is_empty() || space.is_empty() {
            return Err("project_id, token, and space are all required".to_string());
        }
        Ok(RestClient {
            project_id: project_id.to_string(),
            token: token.to_string(),
            space: space.to_string(),
            base_url: format!("https://{space}"),
            http,
        })
    }

    /// Create from environment variables.
    ///
    /// # Errors
    /// Returns `Err(String)` if any of `SIGNALWIRE_PROJECT_ID`,
    /// `SIGNALWIRE_API_TOKEN`, or `SIGNALWIRE_SPACE` is unset or empty (they
    /// default to the empty string, which fails the same validation as
    /// [`new`](Self::new)). No network request is made here.
    pub fn from_env() -> Result<Self, String> {
        let project_id = env::var("SIGNALWIRE_PROJECT_ID").unwrap_or_default();
        let token = env::var("SIGNALWIRE_API_TOKEN").unwrap_or_default();
        let space = env::var("SIGNALWIRE_SPACE").unwrap_or_default();
        Self::new(&project_id, &token, &space)
    }

    // -- Accessors --

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn space(&self) -> &str {
        &self.space
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn http(&self) -> &HttpClient {
        &self.http
    }

    // -----------------------------------------------------------------
    // Namespace accessors
    //
    // Each returns a CrudResource or namespace struct bound to the
    // correct API path.  Since CrudResource borrows &HttpClient, the
    // returned resources live as long as `&self`.
    // -----------------------------------------------------------------

    /// Fabric API (sub-resources: subscribers, `sip_endpoints`, `call_flows`, ...).
    pub fn fabric(&self) -> super::namespaces::fabric::Fabric<'_> {
        super::namespaces::fabric::Fabric::new(&self.http)
    }

    /// Calling API (37 call-control commands).
    pub fn calling(&self) -> super::namespaces::calling::Calling<'_> {
        super::namespaces::calling::Calling::new(&self.http, &self.project_id)
    }

    /// Phone numbers (CRUD + available-number `search`).
    pub fn phone_numbers(&self) -> super::namespaces::phone_numbers::PhoneNumbersResource<'_> {
        super::namespaces::phone_numbers::PhoneNumbersResource::new(&self.http)
    }

    /// Datasphere namespace (documents + chunks + search).
    pub fn datasphere(&self) -> super::namespaces::datasphere::DatasphereNamespace<'_> {
        super::namespaces::datasphere::DatasphereNamespace::new(&self.http)
    }

    /// Video API namespace (rooms, sessions, recordings, conferences,
    /// tokens, streams).
    pub fn video(&self) -> super::namespaces::video::Video<'_> {
        super::namespaces::video::Video::new(&self.http)
    }

    /// Compatibility (Twilio-compatible LAML) API namespace.
    ///
    /// Returns a [`Compat`](super::namespaces::compat::Compat) handle whose
    /// sub-resources (`calls`, `messages`, `faxes`, `phone_numbers`,
    /// `conferences`, `recordings`, `transcriptions`, `applications`,
    /// `laml_bins`, `queues`, `tokens`, `accounts`) cover the full Python
    /// `client.compat.*` surface.
    pub fn compat(&self) -> super::namespaces::compat::Compat<'_> {
        super::namespaces::compat::Compat::new(&self.http, &self.project_id)
    }

    /// Addresses (list / create / get / delete).
    pub fn addresses(&self) -> super::namespaces::simple_resources::AddressesResource<'_> {
        super::namespaces::simple_resources::AddressesResource::new(&self.http)
    }

    /// Queues namespace (CRUD + member operations).
    pub fn queues(&self) -> super::namespaces::queues::Queues<'_> {
        super::namespaces::queues::Queues::new(&self.http)
    }

    /// Recordings (list / get / delete).
    pub fn recordings(&self) -> super::namespaces::simple_resources::RecordingsResource<'_> {
        super::namespaces::simple_resources::RecordingsResource::new(&self.http)
    }

    /// Number groups (CRUD + membership operations).
    pub fn number_groups(&self) -> super::namespaces::number_groups::NumberGroups<'_> {
        super::namespaces::number_groups::NumberGroups::new(&self.http)
    }

    /// Verified caller IDs (CRUD + verification flow).
    pub fn verified_callers(
        &self,
    ) -> super::namespaces::verified_callers::VerifiedCallersResource<'_> {
        super::namespaces::verified_callers::VerifiedCallersResource::new(&self.http)
    }

    /// Project SIP profile (singular: singleton resource at
    /// `/api/relay/rest/sip_profile`).
    pub fn sip_profile(&self) -> super::namespaces::sip_profile::SipProfile<'_> {
        super::namespaces::sip_profile::SipProfile::new(&self.http)
    }

    /// Phone number lookup.
    pub fn lookup(&self) -> super::namespaces::lookup::LookupResource<'_> {
        super::namespaces::lookup::LookupResource::new(&self.http)
    }

    /// Short codes (list / get / update).
    pub fn short_codes(&self) -> super::namespaces::simple_resources::ShortCodesResource<'_> {
        super::namespaces::simple_resources::ShortCodesResource::new(&self.http)
    }

    /// Imported phone numbers (create only).
    pub fn imported_numbers(
        &self,
    ) -> super::namespaces::simple_resources::ImportedNumbersResource<'_> {
        super::namespaces::simple_resources::ImportedNumbersResource::new(&self.http)
    }

    /// Multi-factor authentication (sms/call/verify).
    pub fn mfa(&self) -> super::namespaces::mfa::Mfa<'_> {
        super::namespaces::mfa::Mfa::new(&self.http)
    }

    /// Registry (10DLC brands, campaigns, orders, numbers).
    pub fn registry(&self) -> super::namespaces::registry::Registry<'_> {
        super::namespaces::registry::Registry::new(&self.http)
    }

    /// Logs (messages, voice, fax, conferences).
    pub fn logs(&self) -> super::namespaces::logs::Logs<'_> {
        super::namespaces::logs::Logs::new(&self.http)
    }

    /// Project namespace (exposes `tokens` sub-resource).
    pub fn project(&self) -> super::namespaces::project::Project<'_> {
        super::namespaces::project::Project::new(&self.http)
    }

    /// `PubSub` tokens (`create_token` → POST `/api/pubsub/tokens`).
    pub fn pubsub(&self) -> super::namespaces::pubsub::PubSubResource<'_> {
        super::namespaces::pubsub::PubSubResource::new(&self.http)
    }

    /// Chat tokens (`create_token` → POST `/api/chat/tokens`).
    pub fn chat(&self) -> super::namespaces::chat::ChatResource<'_> {
        super::namespaces::chat::ChatResource::new(&self.http)
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_valid() {
        let client = RestClient::new("proj", "tok", "test.signalwire.com").unwrap();
        assert_eq!(client.project_id(), "proj");
        assert_eq!(client.token(), "tok");
        assert_eq!(client.space(), "test.signalwire.com");
        assert_eq!(client.base_url(), "https://test.signalwire.com");
    }

    #[test]
    fn test_new_missing_project() {
        match RestClient::new("", "tok", "space") {
            Err(e) => assert!(e.contains("projectId")),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn test_new_missing_token() {
        match RestClient::new("proj", "", "space") {
            Err(e) => assert!(e.contains("token")),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn test_new_missing_space() {
        match RestClient::new("proj", "tok", "") {
            Err(e) => assert!(e.contains("space")),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn test_phone_numbers_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        let pn = client.phone_numbers();
        assert_eq!(pn.base_path(), "/api/relay/rest/phone_numbers");
    }

    #[test]
    fn test_datasphere_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        let ds = client.datasphere();
        assert_eq!(ds.documents().base_path(), "/api/datasphere/documents");
    }

    #[test]
    fn test_video_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        let v = client.video();
        assert_eq!(v.rooms().base_path(), "/api/video/rooms");
        assert_eq!(v.streams().base_path(), "/api/video/streams");
    }

    #[test]
    fn test_compat_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        let c = client.compat();
        // Compat namespace exposes sub-resources; calls() rooted under the
        // account-scoped base path.
        assert_eq!(
            c.calls().base_path(),
            "/api/laml/2010-04-01/Accounts/proj/Calls"
        );
        assert_eq!(c.account_sid(), "proj");
    }

    #[test]
    fn test_addresses_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.addresses().base_path(), "/api/relay/rest/addresses");
    }

    #[test]
    fn test_queues_path() {
        // Python ships queues at `/api/relay/rest/queues`. Rust now matches
        // that path through the dedicated Queues namespace.
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.queues().base_path(), "/api/relay/rest/queues");
    }

    #[test]
    fn test_recordings_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.recordings().base_path(),
            "/api/relay/rest/recordings"
        );
    }

    #[test]
    fn test_number_groups_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.number_groups().base_path(),
            "/api/relay/rest/number_groups"
        );
    }

    #[test]
    fn test_verified_callers_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        // Python canonical path is `verified_caller_ids` (not the old
        // `verified_callers`); the dedicated resource corrects it.
        assert_eq!(
            client.verified_callers().base_path(),
            "/api/relay/rest/verified_caller_ids"
        );
    }

    #[test]
    fn test_sip_profile_path() {
        // SIP profile is a singleton resource per project: singular path.
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.sip_profile().base_path(),
            "/api/relay/rest/sip_profile"
        );
    }

    #[test]
    fn test_lookup_path() {
        // Lookup is a single GET operation, not a CRUD resource: the namespace
        // base is `/api/relay/rest/lookup` and `phone_number(e164)` appends
        // `/phone_number/{e164}` (covered end-to-end in rest_relay_coverage).
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.lookup().base_path(), "/api/relay/rest/lookup");
    }

    #[test]
    fn test_short_codes_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.short_codes().base_path(),
            "/api/relay/rest/short_codes"
        );
    }

    #[test]
    fn test_imported_numbers_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.imported_numbers().base_path(),
            "/api/relay/rest/imported_phone_numbers"
        );
    }

    #[test]
    fn test_mfa_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.mfa().base_path(), "/api/relay/rest/mfa");
    }

    #[test]
    fn test_registry_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.registry().brands().base_path(),
            "/api/relay/rest/registry/beta/brands"
        );
    }

    #[test]
    fn test_logs_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.logs().messages().base_path(), "/api/messaging/logs");
        assert_eq!(client.logs().voice().base_path(), "/api/voice/logs");
    }

    #[test]
    fn test_project_path() {
        // Project namespace exposes a `tokens` sub-resource at
        // `/api/project/tokens`.
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.project().tokens().base_path(), "/api/project/tokens");
    }

    #[test]
    fn test_pubsub_path() {
        // Python's PubSubResource.create_token → POST /api/pubsub/tokens.
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.pubsub().base_path(), "/api/pubsub/tokens");
    }

    #[test]
    fn test_chat_path() {
        // Python's ChatResource.create_token → POST /api/chat/tokens.
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.chat().base_path(), "/api/chat/tokens");
    }
}
