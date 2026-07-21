// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_fabric_mock.py.
//
// Closes the audit gaps for Fabric: addresses, generic resources,
// SIP-endpoint sub-resources on subscribers, call-flows / conference-rooms
// addresses sub-paths, FabricTokens, and CxmlApplicationsResource.create.

#[path = "common/mod.rs"]
mod common;

use serde_json::Value;
use signalwire::rest::namespaces::generated::fabric_resources_generated as fabric_gen;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Fabric Addresses (read-only, /api/fabric/addresses)
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_addresses_list_returns_data_collection() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .addresses()
        .list(&HashMap::new(), None)
        .expect("addresses.list");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(
        obj.contains_key("data"),
        "missing 'data' in {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/fabric/addresses");
    assert_eq!(
        entry.matched_route.as_deref(),
        Some("fabric.list_fabric_addresses"),
        "expected fabric.list_fabric_addresses, got {:?}",
        entry.matched_route
    );
}

#[test]
fn test_fabric_addresses_get_uses_address_id() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .addresses()
        .get("addr-9001", None)
        .expect("addresses.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/fabric/addresses/addr-9001");
    assert!(entry.matched_route.is_some(), "spec gap: address get");
}

// ---------------------------------------------------------------------------
// CxmlApplicationsResource.create — removed
//
// The regenerated CxmlApplicationsResource exposes no `create` method (only
// delete/get/list/list_addresses/update), so the former
// `test_fabric_cxml_applications_create_raises_not_implemented` test — which
// asserted that calling `create` returned an error without hitting the wire —
// has been removed: there is no longer any `create` symbol to invoke.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// CallFlowsResource.list_addresses — singular 'call_flow' subpath
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_call_flows_list_addresses_uses_singular_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .call_flows()
        .list_addresses("cf-1", &HashMap::new(), None)
        .expect("list_addresses");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"));
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    // singular 'call_flow' (NOT 'call_flows') in the addresses sub-path.
    assert_eq!(entry.path, "/api/fabric/resources/call_flow/cf-1/addresses");
    assert!(
        entry.matched_route.is_some(),
        "spec gap: call-flow addresses sub-path"
    );
}

// ---------------------------------------------------------------------------
// ConferenceRoomsResource.list_addresses — singular 'conference_room'
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_conference_rooms_list_addresses_uses_singular_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .conference_rooms()
        .list_addresses("cr-1", &HashMap::new(), None)
        .expect("list_addresses");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("data"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    // singular 'conference_room'.
    assert_eq!(
        entry.path,
        "/api/fabric/resources/conference_room/cr-1/addresses"
    );
    assert!(entry.matched_route.is_some());
}

// ---------------------------------------------------------------------------
// Subscribers — SIP endpoint per-id ops
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_subscribers_get_sip_endpoint() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .get_sip_endpoint("sub-1", "ep-1", &HashMap::new(), None)
        .expect("get_sip_endpoint");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        "/api/fabric/resources/subscribers/sub-1/sip_endpoints/ep-1"
    );
    assert!(entry.matched_route.is_some());
}

#[test]
fn test_fabric_subscribers_update_sip_endpoint_uses_patch() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .update_sip_endpoint(
            "sub-1",
            "ep-1",
            fabric_gen::SubscribersUpdateSipEndpointRequest::new().username("renamed"), None
        )
        .expect("update_sip_endpoint");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "PATCH");
    assert_eq!(
        entry.path,
        "/api/fabric/resources/subscribers/sub-1/sip_endpoints/ep-1"
    );
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("username").and_then(Value::as_str),
        Some("renamed")
    );
}

#[test]
fn test_fabric_subscribers_delete_sip_endpoint() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .delete_sip_endpoint("sub-1", "ep-1", None)
        .expect("delete_sip_endpoint");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(
        entry.path,
        "/api/fabric/resources/subscribers/sub-1/sip_endpoints/ep-1"
    );
    assert!(entry.matched_route.is_some());
}

// ---------------------------------------------------------------------------
// FabricTokens — every token-creation endpoint
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_tokens_create_invite_token() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .tokens()
        .create_invite_token(
            fabric_gen::FabricTokensCreateInviteTokenRequest::new("addr-1").expires_at(3600), None
        )
        .expect("create_invite_token");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    // singular 'subscriber' segment.
    assert_eq!(entry.path, "/api/fabric/subscriber/invites");
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("address_id").and_then(Value::as_str),
        Some("addr-1")
    );
    assert_eq!(sent.get("expires_at").and_then(Value::as_i64), Some(3600));
}

#[test]
fn test_fabric_tokens_create_embed_token() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .tokens()
        .create_embed_token(fabric_gen::FabricTokensCreateEmbedTokenRequest::new(
            "tok-1",
        ), None)
        .expect("create_embed_token");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, "/api/fabric/embeds/tokens");
    let sent = entry.body_object().expect("body");
    assert_eq!(sent.get("token").and_then(Value::as_str), Some("tok-1"));
}

#[test]
fn test_fabric_tokens_refresh_subscriber_token() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .tokens()
        .refresh_subscriber_token(fabric_gen::FabricTokensRefreshSubscriberTokenRequest::new(
            "abc-123",
        ), None)
        .expect("refresh_subscriber_token");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, "/api/fabric/subscribers/tokens/refresh");
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("refresh_token").and_then(Value::as_str),
        Some("abc-123")
    );
}

// ---------------------------------------------------------------------------
// GenericResources — operations across every resource type
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_resources_list_returns_data_collection() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .resources()
        .list(&HashMap::new(), None)
        .expect("resources.list");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"));
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/fabric/resources");
    assert!(entry.matched_route.is_some());
}

#[test]
fn test_fabric_resources_get_returns_single() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .resources()
        .get("res-1", &std::collections::HashMap::new(), None)
        .expect("resources.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/fabric/resources/res-1");
}

#[test]
fn test_fabric_resources_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .resources()
        .delete("res-2", None)
        .expect("resources.delete");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, "/api/fabric/resources/res-2");
    assert!(entry.matched_route.is_some());
}

#[test]
fn test_fabric_resources_list_addresses() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .resources()
        .list_addresses("res-3", &HashMap::new(), None)
        .expect("list_addresses");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"));
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/fabric/resources/res-3/addresses");
}

#[test]
fn test_fabric_resources_assign_domain_application() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .resources()
        .assign_domain_application(
            "res-4",
            fabric_gen::GenericResourcesAssignDomainApplicationRequest::new("da-7"), None
        )
        .expect("assign_domain_application");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/fabric/resources/res-4/domain_applications"
    );
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("domain_application_id").and_then(Value::as_str),
        Some("da-7")
    );
}
