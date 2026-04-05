use std::env;

use super::crud_resource::CrudResource;
use super::http_client::{HttpClient, HttpTransport, StubTransport};

/// Top-level SignalWire REST client.
///
/// Provides lazy access to every API namespace (fabric, calling,
/// phone_numbers, datasphere, video, compat, etc.).  Credentials can
/// be supplied explicitly or pulled from environment variables.
pub struct RestClient {
    project_id: String,
    token: String,
    space: String,
    base_url: String,
    http: HttpClient,
}

/// Wrapper so Arc<StubTransport> implements HttpTransport (used internally).
struct StubTransportWrapper2(std::sync::Arc<StubTransport>);

impl HttpTransport for StubTransportWrapper2 {
    fn execute(
        &self,
        method: &str,
        url: &str,
        headers: &std::collections::HashMap<String, String>,
        body: Option<&str>,
    ) -> Result<(u16, String), String> {
        self.0.execute(method, url, headers, body)
    }
}

impl RestClient {
    /// Create a new REST client with explicit credentials.
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
            return Err(
                "space is required (pass explicitly or set SIGNALWIRE_SPACE)".to_string(),
            );
        }

        let base_url = format!("https://{}", space);

        // Use stub transport for now; production would inject a real HTTP transport.
        let stub = std::sync::Arc::new(StubTransport::new(200, "{}"));
        let http = HttpClient::new(
            project_id,
            token,
            &base_url,
            Box::new(StubTransportWrapper2(stub)),
        );

        Ok(RestClient {
            project_id: project_id.to_string(),
            token: token.to_string(),
            space: space.to_string(),
            base_url,
            http,
        })
    }

    /// Create a REST client with a specific HTTP client (for testing).
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
            base_url: format!("https://{}", space),
            http,
        })
    }

    /// Create from environment variables.
    pub fn from_env() -> Result<Self, String> {
        let project_id =
            env::var("SIGNALWIRE_PROJECT_ID").unwrap_or_default();
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

    /// Fabric API (sub-resources: subscribers, sip_endpoints, call_flows, ...).
    pub fn fabric(&self) -> super::namespaces::fabric::Fabric<'_> {
        super::namespaces::fabric::Fabric::new(&self.http)
    }

    /// Calling API (37 call-control commands).
    pub fn calling(&self) -> super::namespaces::calling::Calling<'_> {
        super::namespaces::calling::Calling::new(&self.http, &self.project_id)
    }

    /// Phone numbers.
    pub fn phone_numbers(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/phone_numbers")
    }

    /// Datasphere documents.
    pub fn datasphere(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/datasphere/documents")
    }

    /// Video rooms.
    pub fn video(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/video/rooms")
    }

    /// Compatibility (Twilio-compatible LaML) API.
    pub fn compat(&self) -> CrudResource<'_> {
        CrudResource::new(
            &self.http,
            &format!("/api/laml/2010-04-01/Accounts/{}", self.project_id),
        )
    }

    /// Addresses.
    pub fn addresses(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/addresses")
    }

    /// Queues.
    pub fn queues(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/fabric/resources/queues")
    }

    /// Recordings.
    pub fn recordings(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/recordings")
    }

    /// Number groups.
    pub fn number_groups(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/number_groups")
    }

    /// Verified callers.
    pub fn verified_callers(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/verified_callers")
    }

    /// SIP profiles.
    pub fn sip_profile(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/sip_profiles")
    }

    /// Phone number lookup.
    pub fn lookup(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/lookup/phone_number")
    }

    /// Short codes.
    pub fn short_codes(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/short_codes")
    }

    /// Imported phone numbers.
    pub fn imported_numbers(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/imported_phone_numbers")
    }

    /// Multi-factor authentication.
    pub fn mfa(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/mfa")
    }

    /// Registry (10DLC brands, campaigns, orders).
    pub fn registry(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/registry")
    }

    /// Logs (messages, voice, fax, conferences).
    pub fn logs(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/logs")
    }

    /// Project management.
    pub fn project(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/project")
    }

    /// PubSub tokens.
    pub fn pubsub(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/pubsub")
    }

    /// Chat tokens.
    pub fn chat(&self) -> CrudResource<'_> {
        CrudResource::new(&self.http, "/api/relay/rest/chat")
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
        assert_eq!(ds.base_path(), "/api/datasphere/documents");
    }

    #[test]
    fn test_video_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        let v = client.video();
        assert_eq!(v.base_path(), "/api/video/rooms");
    }

    #[test]
    fn test_compat_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        let c = client.compat();
        assert_eq!(c.base_path(), "/api/laml/2010-04-01/Accounts/proj");
    }

    #[test]
    fn test_addresses_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.addresses().base_path(),
            "/api/relay/rest/addresses"
        );
    }

    #[test]
    fn test_queues_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.queues().base_path(),
            "/api/fabric/resources/queues"
        );
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
        assert_eq!(
            client.verified_callers().base_path(),
            "/api/relay/rest/verified_callers"
        );
    }

    #[test]
    fn test_sip_profile_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.sip_profile().base_path(),
            "/api/relay/rest/sip_profiles"
        );
    }

    #[test]
    fn test_lookup_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.lookup().base_path(),
            "/api/relay/rest/lookup/phone_number"
        );
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
            client.registry().base_path(),
            "/api/relay/rest/registry"
        );
    }

    #[test]
    fn test_logs_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.logs().base_path(), "/api/relay/rest/logs");
    }

    #[test]
    fn test_project_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(
            client.project().base_path(),
            "/api/relay/rest/project"
        );
    }

    #[test]
    fn test_pubsub_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.pubsub().base_path(), "/api/relay/rest/pubsub");
    }

    #[test]
    fn test_chat_path() {
        let client = RestClient::new("proj", "tok", "test.sw.com").unwrap();
        assert_eq!(client.chat().base_path(), "/api/relay/rest/chat");
    }
}
