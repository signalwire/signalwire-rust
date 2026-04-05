use crate::rest::crud_resource::CrudResource;
use crate::rest::http_client::HttpClient;

/// Base path for all Fabric resources.
const BASE: &str = "/api/fabric/resources";

/// Fabric API namespace.
///
/// Groups all Fabric sub-resources (subscribers, SIP endpoints, call flows,
/// SWML scripts, conference rooms, AI agents, etc.) under a single object.
pub struct Fabric<'a> {
    client: &'a HttpClient,
}

impl<'a> Fabric<'a> {
    pub fn new(client: &'a HttpClient) -> Self {
        Fabric { client }
    }

    pub fn client(&self) -> &HttpClient {
        self.client
    }

    // -- Sub-resource accessors --

    pub fn subscribers(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/subscribers", BASE))
    }

    pub fn sip_endpoints(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/sip_endpoints", BASE))
    }

    pub fn addresses(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/addresses", BASE))
    }

    pub fn call_flows(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/call_flows", BASE))
    }

    pub fn swml_scripts(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/swml_scripts", BASE))
    }

    pub fn conversations(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/conversations", BASE))
    }

    pub fn conference_rooms(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/conference_rooms", BASE))
    }

    pub fn dial_plans(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/dial_plans", BASE))
    }

    pub fn freeclimb_apps(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/freeclimb_apps", BASE))
    }

    pub fn call_queues(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/call_queues", BASE))
    }

    pub fn ai_agents(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/ai_agents", BASE))
    }

    pub fn sip_profiles(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/sip_profiles", BASE))
    }

    pub fn phone_numbers(&self) -> CrudResource<'a> {
        CrudResource::new(self.client, &format!("{}/phone_numbers", BASE))
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::http_client::StubTransport;

    fn make_fabric() -> (HttpClient, std::sync::Arc<StubTransport>) {
        HttpClient::with_stub("proj", "tok", "https://test.signalwire.com")
    }

    #[test]
    fn test_subscribers_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.subscribers().base_path(),
            "/api/fabric/resources/subscribers"
        );
    }

    #[test]
    fn test_sip_endpoints_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.sip_endpoints().base_path(),
            "/api/fabric/resources/sip_endpoints"
        );
    }

    #[test]
    fn test_addresses_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.addresses().base_path(),
            "/api/fabric/resources/addresses"
        );
    }

    #[test]
    fn test_call_flows_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.call_flows().base_path(),
            "/api/fabric/resources/call_flows"
        );
    }

    #[test]
    fn test_swml_scripts_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.swml_scripts().base_path(),
            "/api/fabric/resources/swml_scripts"
        );
    }

    #[test]
    fn test_conversations_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.conversations().base_path(),
            "/api/fabric/resources/conversations"
        );
    }

    #[test]
    fn test_conference_rooms_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.conference_rooms().base_path(),
            "/api/fabric/resources/conference_rooms"
        );
    }

    #[test]
    fn test_dial_plans_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.dial_plans().base_path(),
            "/api/fabric/resources/dial_plans"
        );
    }

    #[test]
    fn test_freeclimb_apps_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.freeclimb_apps().base_path(),
            "/api/fabric/resources/freeclimb_apps"
        );
    }

    #[test]
    fn test_call_queues_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.call_queues().base_path(),
            "/api/fabric/resources/call_queues"
        );
    }

    #[test]
    fn test_ai_agents_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.ai_agents().base_path(),
            "/api/fabric/resources/ai_agents"
        );
    }

    #[test]
    fn test_sip_profiles_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.sip_profiles().base_path(),
            "/api/fabric/resources/sip_profiles"
        );
    }

    #[test]
    fn test_phone_numbers_path() {
        let (client, _) = make_fabric();
        let f = Fabric::new(&client);
        assert_eq!(
            f.phone_numbers().base_path(),
            "/api/fabric/resources/phone_numbers"
        );
    }
}
