use std::env;

use super::error::RestClientBuilderError;
use super::http_client::{HttpClient, UreqTransport};
use super::namespaces::generated::client_tree_generated as tree;
use super::request_options::RequestOptions;

/// Top-level SignalWire REST client.
///
/// Provides lazy access to every API namespace (fabric, calling,
/// `phone_numbers`, datasphere, video, etc.). Credentials can
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

/// Validate a required credential is non-empty, else the typed missing-credential
/// error naming the field + the env var that supplies it.
fn require_credential(
    field: &'static str,
    env_var: &'static str,
    value: &str,
) -> Result<(), RestClientBuilderError> {
    if value.is_empty() {
        Err(RestClientBuilderError::MissingCredential { field, env_var })
    } else {
        Ok(())
    }
}

impl RestClient {
    /// Create a new REST client with explicit credentials. The base URL
    /// resolves to `https://{space}`. Use [`with_base_url`] to override
    /// (e.g. for fixture-driven tests pointed at `http://127.0.0.1:N`).
    ///
    /// # Errors
    /// Returns [`RestClientBuilderError::MissingCredential`] if any required
    /// credential is empty: `project_id`, `token`, or `space`. No network
    /// request is made here.
    pub fn new(project_id: &str, token: &str, space: &str) -> Result<Self, RestClientBuilderError> {
        require_credential("project_id", "SIGNALWIRE_PROJECT_ID", project_id)?;
        require_credential("token", "SIGNALWIRE_API_TOKEN", token)?;
        require_credential("space", "SIGNALWIRE_SPACE", space)?;

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
    /// Returns [`RestClientBuilderError`] if any required argument is empty:
    /// `project_id`, `token`, or `base_url`. No network request is made here.
    pub fn with_base_url(
        project_id: &str,
        token: &str,
        base_url: &str,
    ) -> Result<Self, RestClientBuilderError> {
        Self::with_base_url_and_options(project_id, token, base_url, None)
    }

    /// Create a REST client with an explicit base URL AND a client-default
    /// [`RequestOptions`] (plan 4.2) — the request-options envelope applied to
    /// every request through this client (timeout / retries / backoff / abort
    /// signal), shallow-overridden by any per-request override. Used by the
    /// envelope-dump harness and by callers who want default retry/timeout
    /// behavior pointed at a fixture.
    ///
    /// # Errors
    /// Returns [`RestClientBuilderError`] if any required argument is empty:
    /// `project_id`, `token`, or `base_url`. No network request is made here.
    pub fn with_base_url_and_options(
        project_id: &str,
        token: &str,
        base_url: &str,
        request_options: Option<RequestOptions>,
    ) -> Result<Self, RestClientBuilderError> {
        require_credential("project_id", "SIGNALWIRE_PROJECT_ID", project_id)?;
        require_credential("token", "SIGNALWIRE_API_TOKEN", token)?;
        if base_url.is_empty() {
            return Err(RestClientBuilderError::MissingField { field: "base_url" });
        }
        let http = HttpClient::with_options(
            project_id,
            token,
            base_url,
            Box::new(UreqTransport::new()),
            request_options,
        );
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
    /// Returns [`RestClientBuilderError::MissingCredential`] if any of
    /// `project_id`, `token`, or `space` is empty. No network request is made
    /// here.
    pub fn with_http(
        project_id: &str,
        token: &str,
        space: &str,
        http: HttpClient,
    ) -> Result<Self, RestClientBuilderError> {
        require_credential("project_id", "SIGNALWIRE_PROJECT_ID", project_id)?;
        require_credential("token", "SIGNALWIRE_API_TOKEN", token)?;
        require_credential("space", "SIGNALWIRE_SPACE", space)?;
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
    /// Returns [`RestClientBuilderError::MissingCredential`] if any of
    /// `SIGNALWIRE_PROJECT_ID`, `SIGNALWIRE_API_TOKEN`, or `SIGNALWIRE_SPACE` is
    /// unset or empty (they default to the empty string, which fails the same
    /// validation as [`new`](Self::new)). No network request is made here.
    pub fn from_env() -> Result<Self, RestClientBuilderError> {
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
    // The REST resource surface is GENERATED (scripts/generate_rest.py) from
    // the canonical specs + x-sdk-* markup into
    // src/rest/namespaces/generated/. The hand client composes the generated
    // resource tree; each accessor constructs the generated resource with the
    // client's `HttpClient` (base paths baked in per §4).
    //
    // Since every generated resource borrows `&HttpClient`, the returned
    // resources live as long as `&self`.
    // -----------------------------------------------------------------

    /// The generated resource tree (flat resources + namespace containers).
    fn tree(&self) -> tree::GeneratedResourceTree<'_> {
        tree::GeneratedResourceTree::new(&self.http)
    }

    /// Fabric API namespace container (subscribers, `sip_endpoints`,
    /// `call_flows`, resources, tokens, addresses, ...).
    pub fn fabric(&self) -> tree::FabricNamespace<'_> {
        self.tree().fabric()
    }

    /// Calling API (call-control command dispatch).
    pub fn calling(
        &self,
    ) -> super::namespaces::generated::calling_resources_generated::Calling<'_> {
        self.tree().calling()
    }

    /// Phone numbers (CRUD + available-number `search` + `set_*` wrappers).
    pub fn phone_numbers(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::PhoneNumbers<'_> {
        self.tree().phone_numbers()
    }

    /// Datasphere namespace (documents + chunks + search).
    pub fn datasphere(&self) -> tree::DatasphereNamespace<'_> {
        self.tree().datasphere()
    }

    /// Video API namespace (rooms, sessions, recordings, conferences,
    /// tokens, streams).
    pub fn video(&self) -> tree::VideoNamespace<'_> {
        self.tree().video()
    }

    /// Addresses (list / create / get / delete).
    pub fn addresses(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::Addresses<'_> {
        self.tree().addresses()
    }

    /// Queues namespace (CRUD + member operations).
    pub fn queues(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::Queues<'_> {
        self.tree().queues()
    }

    /// Recordings (list / get / delete).
    pub fn recordings(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::Recordings<'_> {
        self.tree().recordings()
    }

    /// Number groups (CRUD + membership operations).
    pub fn number_groups(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::NumberGroups<'_> {
        self.tree().number_groups()
    }

    /// Verified caller IDs (CRUD + verification flow).
    pub fn verified_callers(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::VerifiedCallers<'_> {
        self.tree().verified_callers()
    }

    /// Project SIP profile (singleton resource at `/api/relay/rest/sip_profile`).
    pub fn sip_profile(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::SipProfile<'_> {
        self.tree().sip_profile()
    }

    /// Phone number lookup.
    pub fn lookup(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::Lookup<'_> {
        self.tree().lookup()
    }

    /// Short codes (list / get / update).
    pub fn short_codes(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::ShortCodes<'_> {
        self.tree().short_codes()
    }

    /// Imported phone numbers (create only).
    pub fn imported_numbers(
        &self,
    ) -> super::namespaces::generated::relay_rest_resources_generated::ImportedNumbers<'_> {
        self.tree().imported_numbers()
    }

    /// Multi-factor authentication (sms/call/verify).
    pub fn mfa(&self) -> super::namespaces::generated::relay_rest_resources_generated::Mfa<'_> {
        self.tree().mfa()
    }

    /// Registry (10DLC brands, campaigns, orders, numbers).
    pub fn registry(&self) -> tree::RegistryNamespace<'_> {
        self.tree().registry()
    }

    /// Logs (messages, voice, fax, conferences).
    pub fn logs(&self) -> tree::LogsNamespace<'_> {
        self.tree().logs()
    }

    /// Messages (`/api/messaging/messages` send + redact) — `create` sends an
    /// outbound SMS/MMS, `update` redacts a previously sent message. Distinct
    /// from the message *logs* under `logs().messages()`.
    pub fn messages(
        &self,
    ) -> super::namespaces::generated::messages_resources_generated::Messages<'_> {
        self.tree().messages()
    }

    /// Project namespace (exposes `tokens` sub-resource).
    pub fn project(&self) -> tree::ProjectNamespace<'_> {
        self.tree().project()
    }

    /// Projects (`/api/projects` CRUD + `rotate_signing_key`) — manage projects
    /// and subprojects. Distinct from the singular `project` token namespace.
    pub fn projects(
        &self,
    ) -> super::namespaces::generated::projects_resources_generated::Projects<'_> {
        self.tree().projects()
    }

    /// `PubSub` tokens (`create_token` → POST `/api/pubsub/tokens`).
    pub fn pubsub(&self) -> super::namespaces::generated::pubsub_resources_generated::PubSub<'_> {
        self.tree().pubsub()
    }

    /// Chat tokens (`create_token` → POST `/api/chat/tokens`).
    pub fn chat(&self) -> super::namespaces::generated::chat_resources_generated::Chat<'_> {
        self.tree().chat()
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

    /// Assert a constructor Result is the given typed builder error (avoids
    /// requiring `Debug` on the `Ok` `RestClient`, which it doesn't implement).
    fn assert_builder_err(
        r: Result<RestClient, RestClientBuilderError>,
        want: &RestClientBuilderError,
    ) {
        match r {
            Err(e) => assert_eq!(&e, want),
            Ok(_) => panic!("expected {want:?}"),
        }
    }

    #[test]
    fn test_new_missing_project() {
        assert_builder_err(
            RestClient::new("", "tok", "space"),
            &RestClientBuilderError::MissingCredential {
                field: "project_id",
                env_var: "SIGNALWIRE_PROJECT_ID",
            },
        );
    }

    #[test]
    fn test_new_missing_token() {
        assert_builder_err(
            RestClient::new("proj", "", "space"),
            &RestClientBuilderError::MissingCredential {
                field: "token",
                env_var: "SIGNALWIRE_API_TOKEN",
            },
        );
    }

    #[test]
    fn test_new_missing_space() {
        assert_builder_err(
            RestClient::new("proj", "tok", ""),
            &RestClientBuilderError::MissingCredential {
                field: "space",
                env_var: "SIGNALWIRE_SPACE",
            },
        );
    }

    /// The typed error's Display preserves the reference's guidance message.
    #[test]
    fn test_builder_error_display() {
        match RestClient::new("", "tok", "space") {
            Err(e) => assert_eq!(
                e.to_string(),
                "project_id is required (pass explicitly or set SIGNALWIRE_PROJECT_ID)"
            ),
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
