// Full success + error REST coverage for the `fabric` spec group.
//
// Every canonical route reachable through a real SDK accessor gets a 2xx
// success test (asserting method, path, and matched_route on the journal) and
// a 4xx/5xx error test (staging a scenario, asserting `status_code()` and the
// journaled `response_status`). Uses the typed accessors on
// `client.fabric()` directly — never `c.http().post(raw_path)`.
//
// Confirmed gaps (no canonical accessor / route, flagged not faked):
//   * dialogflow_agents (5): list/get/update/delete/list_addresses — no accessor.
//   * fabric.list_sip_gateway_addresses (1): doubled canonical path
//     `/sip_gateways/resources/sip_gateways/{id}/addresses`; sip_gateways
//     .list_addresses hits the plain `/sip_gateways/{id}/addresses` route, so
//     the doubled-path route stays uncovered.
//   * fabric.assign_resource_sip_endpoint (1): doubled canonical path
//     `/sip_endpoints/resources/{id}/sip_endpoints` — no accessor.
//
// fabric.assign_resource_phone_route and fabric.list_cxml_application_addresses
// are now COVERED: GenericResources.assign_phone_route and
// CxmlApplicationsResource.list_addresses were added in the fabric parity pass,
// and the tests at the end of this file exercise both (success + error).

#[path = "common/mod.rs"]
mod common;

use serde_json::{Value, json};

const BASE: &str = "/api/fabric/resources";

// ---------------------------------------------------------------------------
// Fabric addresses (read-only, /api/fabric/addresses)
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_addresses_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().addresses().list(&json!({})).expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/fabric/addresses");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_fabric_addresses")
    );
}

#[test]
fn test_fabric_addresses_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.list_fabric_addresses", 500, json!({"error":"boom"}));
    let err = c.fabric().addresses().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_fabric_addresses")
    );
}

#[test]
fn test_fabric_addresses_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().addresses().get("addr-1").expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/fabric/addresses/addr-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.get_fabric_address")
    );
}

#[test]
fn test_fabric_addresses_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.get_fabric_address", 404, json!({"error":"nf"}));
    let err = c.fabric().addresses().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.get_fabric_address")
    );
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_create_embed_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .tokens()
        .create_embed_token(&json!({"allowed_addresses": ["a"]}))
        .expect("embed");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/fabric/embeds/tokens");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_embeds_token")
    );
}

#[test]
fn test_fabric_create_embed_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.create_embeds_token", 422, json!({"error":"bad"}));
    let err = c
        .fabric()
        .tokens()
        .create_embed_token(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_embeds_token")
    );
}

#[test]
fn test_fabric_create_guest_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .tokens()
        .create_guest_token(&json!({"allowed_addresses": ["a"]}))
        .expect("guest");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/fabric/guests/tokens");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_subscriber_guest_token")
    );
}

#[test]
fn test_fabric_create_guest_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.create_subscriber_guest_token",
        422,
        json!({"error":"bad"}),
    );
    let err = c
        .fabric()
        .tokens()
        .create_guest_token(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_subscriber_guest_token")
    );
}

#[test]
fn test_fabric_create_subscriber_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .tokens()
        .create_subscriber_token(&json!({"reference": "ref"}))
        .expect("sub token");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/fabric/subscribers/tokens");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_subscriber_token")
    );
}

#[test]
fn test_fabric_create_subscriber_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.create_subscriber_token",
        422,
        json!({"error":"bad"}),
    );
    let err = c
        .fabric()
        .tokens()
        .create_subscriber_token(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_subscriber_token")
    );
}

#[test]
fn test_fabric_refresh_subscriber_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .tokens()
        .refresh_subscriber_token(&json!({"refresh_token": "rt"}))
        .expect("refresh");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/fabric/subscribers/tokens/refresh");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.refresh_subscriber_token")
    );
}

#[test]
fn test_fabric_refresh_subscriber_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.refresh_subscriber_token",
        422,
        json!({"error":"bad"}),
    );
    let err = c
        .fabric()
        .tokens()
        .refresh_subscriber_token(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.refresh_subscriber_token")
    );
}

#[test]
fn test_fabric_create_invite_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .tokens()
        .create_invite_token(&json!({"email": "x@example.com"}))
        .expect("invite");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    // singular `subscriber` segment.
    assert_eq!(e.path, "/api/fabric/subscriber/invites");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_subscriber_invite_token")
    );
}

#[test]
fn test_fabric_create_invite_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.create_subscriber_invite_token",
        422,
        json!({"error":"bad"}),
    );
    let err = c
        .fabric()
        .tokens()
        .create_invite_token(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_subscriber_invite_token")
    );
}

// ---------------------------------------------------------------------------
// Generic resources
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_resources_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().resources().list(&json!({})).expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, BASE);
    assert_eq!(e.matched_route.as_deref(), Some("fabric.list_resources"));
}

#[test]
fn test_fabric_resources_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.list_resources", 500, json!({"error":"boom"}));
    let err = c.fabric().resources().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.list_resources"));
}

#[test]
fn test_fabric_resources_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().resources().get("res-1").expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/res-1"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.get_resource"));
}

#[test]
fn test_fabric_resources_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.get_resource", 404, json!({"error":"nf"}));
    let err = c.fabric().resources().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.get_resource"));
}

#[test]
fn test_fabric_resources_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().resources().delete("res-2").expect("delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, format!("{BASE}/res-2"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.delete_resource"));
}

#[test]
fn test_fabric_resources_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.delete_resource", 404, json!({"error":"nf"}));
    let err = c.fabric().resources().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.delete_resource"));
}

#[test]
fn test_fabric_resources_list_addresses_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .resources()
        .list_addresses("res-3", &json!({}))
        .expect("list_addresses");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/res-3/addresses"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_resource_addresses")
    );
}

#[test]
fn test_fabric_resources_list_addresses_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.list_resource_addresses", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .resources()
        .list_addresses("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_resource_addresses")
    );
}

#[test]
fn test_fabric_resources_assign_domain_application_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .resources()
        .assign_domain_application("res-4", &json!({"domain_application_id": "da-1"}))
        .expect("assign");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, format!("{BASE}/res-4/domain_applications"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.assign_resource_domain_application")
    );
}

#[test]
fn test_fabric_resources_assign_domain_application_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.assign_resource_domain_application",
        422,
        json!({"error":"bad"}),
    );
    let err = c
        .fabric()
        .resources()
        .assign_domain_application("res-4", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.assign_resource_domain_application")
    );
}

// ---------------------------------------------------------------------------
// cXML applications — read/update(PUT)/delete/list_addresses; create raises.
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_cxml_applications_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .cxml_applications()
        .list(&json!({}))
        .expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/cxml_applications"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_cxml_applications")
    );
}

#[test]
fn test_fabric_cxml_applications_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.list_cxml_applications",
        500,
        json!({"error":"boom"}),
    );
    let err = c
        .fabric()
        .cxml_applications()
        .list(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_cxml_applications")
    );
}

#[test]
fn test_fabric_cxml_applications_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().cxml_applications().get("ca-1").expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/cxml_applications/ca-1"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.get_cxml_application")
    );
}

#[test]
fn test_fabric_cxml_applications_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.get_cxml_application", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .cxml_applications()
        .get("missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.get_cxml_application")
    );
}

#[test]
fn test_fabric_cxml_applications_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .cxml_applications()
        .update("ca-1", &json!({"name": "renamed"}))
        .expect("update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, format!("{BASE}/cxml_applications/ca-1"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.update_cxml_application")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("renamed"));
}

#[test]
fn test_fabric_cxml_applications_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.update_cxml_application", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .cxml_applications()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.update_cxml_application")
    );
}

#[test]
fn test_fabric_cxml_applications_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .cxml_applications()
        .delete("ca-1")
        .expect("delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, format!("{BASE}/cxml_applications/ca-1"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.delete_cxml_application")
    );
}

#[test]
fn test_fabric_cxml_applications_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.delete_cxml_application", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .cxml_applications()
        .delete("missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.delete_cxml_application")
    );
}

// NOTE: fabric.list_cxml_application_addresses is a confirmed gap — the
// CxmlApplicationsResource accessor exposes no `list_addresses` method.

#[test]
fn test_fabric_cxml_applications_create_returns_err_no_request() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let err = c
        .fabric()
        .cxml_applications()
        .create(&json!({"name": "x"}))
        .expect_err("create unsupported");
    assert!(
        err.message().contains("cXML applications"),
        "unexpected message: {}",
        err.message()
    );
    // No HTTP request should have been sent.
    assert!(
        common::mocktest::journal_all().is_empty(),
        "create should not hit the wire"
    );
}

// ---------------------------------------------------------------------------
// Call flows — CRUD (PUT update) + singular `call_flow` sub-paths.
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_call_flows_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().call_flows().list(&json!({})).expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/call_flows"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.list_call_flows"));
}

#[test]
fn test_fabric_call_flows_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.list_call_flows", 500, json!({"error":"boom"}));
    let err = c.fabric().call_flows().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.list_call_flows"));
}

#[test]
fn test_fabric_call_flows_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .call_flows()
        .create(&json!({"name": "cf"}))
        .expect("create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, format!("{BASE}/call_flows"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.create_call_flow"));
}

#[test]
fn test_fabric_call_flows_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.create_call_flow", 422, json!({"error":"bad"}));
    let err = c.fabric().call_flows().create(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.create_call_flow"));
}

#[test]
fn test_fabric_call_flows_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().call_flows().get("cf-1").expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/call_flows/cf-1"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.get_call_flow"));
}

#[test]
fn test_fabric_call_flows_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.get_call_flow", 404, json!({"error":"nf"}));
    let err = c.fabric().call_flows().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.get_call_flow"));
}

#[test]
fn test_fabric_call_flows_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .call_flows()
        .update("cf-1", &json!({"name": "renamed"}))
        .expect("update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, format!("{BASE}/call_flows/cf-1"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.update_call_flow"));
}

#[test]
fn test_fabric_call_flows_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.update_call_flow", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .call_flows()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.update_call_flow"));
}

#[test]
fn test_fabric_call_flows_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().call_flows().delete("cf-1").expect("delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, format!("{BASE}/call_flows/cf-1"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.delete_call_flow"));
}

#[test]
fn test_fabric_call_flows_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.delete_call_flow", 404, json!({"error":"nf"}));
    let err = c.fabric().call_flows().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.delete_call_flow"));
}

#[test]
fn test_fabric_call_flow_list_addresses_singular_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .call_flows()
        .list_addresses("cf-1", &json!({}))
        .expect("list_addresses");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    // singular `call_flow`.
    assert_eq!(e.path, format!("{BASE}/call_flow/cf-1/addresses"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_call_flow_addresses")
    );
}

#[test]
fn test_fabric_call_flow_list_addresses_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.list_call_flow_addresses",
        404,
        json!({"error":"nf"}),
    );
    let err = c
        .fabric()
        .call_flows()
        .list_addresses("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_call_flow_addresses")
    );
}

#[test]
fn test_fabric_call_flow_list_versions_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .call_flows()
        .list_versions("cf-1", &json!({}))
        .expect("list_versions");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/call_flow/cf-1/versions"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_call_flow_versions")
    );
}

#[test]
fn test_fabric_call_flow_list_versions_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.list_call_flow_versions", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .call_flows()
        .list_versions("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_call_flow_versions")
    );
}

#[test]
fn test_fabric_call_flow_deploy_version_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .call_flows()
        .deploy_version("cf-1", &json!({"version": "v2"}))
        .expect("deploy_version");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, format!("{BASE}/call_flow/cf-1/versions"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.deploy_call_flow_version")
    );
}

#[test]
fn test_fabric_call_flow_deploy_version_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.deploy_call_flow_version",
        422,
        json!({"error":"bad"}),
    );
    let err = c
        .fabric()
        .call_flows()
        .deploy_version("cf-1", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.deploy_call_flow_version")
    );
}

// ---------------------------------------------------------------------------
// Conference rooms — CRUD (PUT update) + singular `conference_room` addresses.
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_conference_rooms_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .conference_rooms()
        .list(&json!({}))
        .expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/conference_rooms"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_conference_rooms")
    );
}

#[test]
fn test_fabric_conference_rooms_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.list_conference_rooms", 500, json!({"error":"boom"}));
    let err = c
        .fabric()
        .conference_rooms()
        .list(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_conference_rooms")
    );
}

#[test]
fn test_fabric_conference_rooms_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .conference_rooms()
        .create(&json!({"name": "cr"}))
        .expect("create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, format!("{BASE}/conference_rooms"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_conference_room")
    );
}

#[test]
fn test_fabric_conference_rooms_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.create_conference_room", 422, json!({"error":"bad"}));
    let err = c
        .fabric()
        .conference_rooms()
        .create(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_conference_room")
    );
}

#[test]
fn test_fabric_conference_rooms_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().conference_rooms().get("cr-1").expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/conference_rooms/cr-1"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.get_conference_room")
    );
}

#[test]
fn test_fabric_conference_rooms_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.get_conference_room", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .conference_rooms()
        .get("missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.get_conference_room")
    );
}

#[test]
fn test_fabric_conference_rooms_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .conference_rooms()
        .update("cr-1", &json!({"name": "renamed"}))
        .expect("update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, format!("{BASE}/conference_rooms/cr-1"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.update_conference_room")
    );
}

#[test]
fn test_fabric_conference_rooms_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.update_conference_room", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .conference_rooms()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.update_conference_room")
    );
}

#[test]
fn test_fabric_conference_rooms_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .conference_rooms()
        .delete("cr-1")
        .expect("delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, format!("{BASE}/conference_rooms/cr-1"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.delete_conference_room")
    );
}

#[test]
fn test_fabric_conference_rooms_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.delete_conference_room", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .conference_rooms()
        .delete("missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.delete_conference_room")
    );
}

#[test]
fn test_fabric_conference_room_list_addresses_singular_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .conference_rooms()
        .list_addresses("cr-1", &json!({}))
        .expect("list_addresses");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    // singular `conference_room`.
    assert_eq!(e.path, format!("{BASE}/conference_room/cr-1/addresses"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_conference_room_addresses")
    );
}

#[test]
fn test_fabric_conference_room_list_addresses_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.list_conference_room_addresses",
        404,
        json!({"error":"nf"}),
    );
    let err = c
        .fabric()
        .conference_rooms()
        .list_addresses("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_conference_room_addresses")
    );
}

// ---------------------------------------------------------------------------
// Subscribers — CRUD (PUT update) + addresses + SIP-endpoint sub-resources.
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_subscribers_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().subscribers().list(&json!({})).expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/subscribers"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.list_subscribers"));
}

#[test]
fn test_fabric_subscribers_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.list_subscribers", 500, json!({"error":"boom"}));
    let err = c.fabric().subscribers().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.list_subscribers"));
}

#[test]
fn test_fabric_subscribers_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .create(&json!({"email": "s@example.com"}))
        .expect("create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, format!("{BASE}/subscribers"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.create_subscriber"));
}

#[test]
fn test_fabric_subscribers_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.create_subscriber", 422, json!({"error":"bad"}));
    let err = c
        .fabric()
        .subscribers()
        .create(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.create_subscriber"));
}

#[test]
fn test_fabric_subscribers_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().subscribers().get("sub-1").expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/subscribers/sub-1"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.get_subscriber"));
}

#[test]
fn test_fabric_subscribers_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.get_subscriber", 404, json!({"error":"nf"}));
    let err = c.fabric().subscribers().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.get_subscriber"));
}

#[test]
fn test_fabric_subscribers_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .update("sub-1", &json!({"email": "new@example.com"}))
        .expect("update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, format!("{BASE}/subscribers/sub-1"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.update_subscriber"));
}

#[test]
fn test_fabric_subscribers_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.update_subscriber", 404, json!({"error":"nf"}));
    let err = c
        .fabric()
        .subscribers()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.update_subscriber"));
}

#[test]
fn test_fabric_subscribers_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.fabric().subscribers().delete("sub-1").expect("delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, format!("{BASE}/subscribers/sub-1"));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.delete_subscriber"));
}

#[test]
fn test_fabric_subscribers_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fabric.delete_subscriber", 404, json!({"error":"nf"}));
    let err = c.fabric().subscribers().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("fabric.delete_subscriber"));
}

#[test]
fn test_fabric_subscribers_list_addresses_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .list_addresses("sub-1", &json!({}))
        .expect("list_addresses");
    assert!(body.is_array() || body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/subscribers/sub-1/addresses"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_subscriber_addresses")
    );
}

#[test]
fn test_fabric_subscribers_list_addresses_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.list_subscriber_addresses",
        404,
        json!({"error":"nf"}),
    );
    let err = c
        .fabric()
        .subscribers()
        .list_addresses("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_subscriber_addresses")
    );
}

#[test]
fn test_fabric_subscribers_list_sip_endpoints_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .list_sip_endpoints("sub-1", &json!({}))
        .expect("list_sip_endpoints");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/subscribers/sub-1/sip_endpoints"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_subscriber_sip_endpoints")
    );
}

#[test]
fn test_fabric_subscribers_list_sip_endpoints_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.list_subscriber_sip_endpoints",
        404,
        json!({"error":"nf"}),
    );
    let err = c
        .fabric()
        .subscribers()
        .list_sip_endpoints("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_subscriber_sip_endpoints")
    );
}

#[test]
fn test_fabric_subscribers_create_sip_endpoint_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .create_sip_endpoint("sub-1", &json!({"username": "u"}))
        .expect("create_sip_endpoint");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, format!("{BASE}/subscribers/sub-1/sip_endpoints"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_subscriber_sip_endpoint")
    );
}

#[test]
fn test_fabric_subscribers_create_sip_endpoint_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.create_subscriber_sip_endpoint",
        422,
        json!({"error":"bad"}),
    );
    let err = c
        .fabric()
        .subscribers()
        .create_sip_endpoint("sub-1", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.create_subscriber_sip_endpoint")
    );
}

#[test]
fn test_fabric_subscribers_get_sip_endpoint_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .get_sip_endpoint("sub-1", "ep-1")
        .expect("get_sip_endpoint");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(
        e.path,
        format!("{BASE}/subscribers/sub-1/sip_endpoints/ep-1")
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.get_subscriber_sip_endpoint")
    );
}

#[test]
fn test_fabric_subscribers_get_sip_endpoint_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.get_subscriber_sip_endpoint",
        404,
        json!({"error":"nf"}),
    );
    let err = c
        .fabric()
        .subscribers()
        .get_sip_endpoint("sub-1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.get_subscriber_sip_endpoint")
    );
}

#[test]
fn test_fabric_subscribers_update_sip_endpoint_uses_patch_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .update_sip_endpoint("sub-1", "ep-1", &json!({"username": "renamed"}))
        .expect("update_sip_endpoint");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PATCH");
    assert_eq!(
        e.path,
        format!("{BASE}/subscribers/sub-1/sip_endpoints/ep-1")
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.update_subscriber_sip_endpoint")
    );
}

#[test]
fn test_fabric_subscribers_update_sip_endpoint_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.update_subscriber_sip_endpoint",
        404,
        json!({"error":"nf"}),
    );
    let err = c
        .fabric()
        .subscribers()
        .update_sip_endpoint("sub-1", "missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.update_subscriber_sip_endpoint")
    );
}

#[test]
fn test_fabric_subscribers_delete_sip_endpoint_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .subscribers()
        .delete_sip_endpoint("sub-1", "ep-1")
        .expect("delete_sip_endpoint");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(
        e.path,
        format!("{BASE}/subscribers/sub-1/sip_endpoints/ep-1")
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.delete_subscriber_sip_endpoint")
    );
}

#[test]
fn test_fabric_subscribers_delete_sip_endpoint_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.delete_subscriber_sip_endpoint",
        404,
        json!({"error":"nf"}),
    );
    let err = c
        .fabric()
        .subscribers()
        .delete_sip_endpoint("sub-1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.delete_subscriber_sip_endpoint")
    );
}

// ===========================================================================
// PATCH-update resources (FabricResource): ai_agents, cxml_webhooks,
// swml_webhooks, sip_gateways.
//
// Each generates list/create/get/update(PATCH)/delete (+ list_addresses where
// the canonical route is reachable) via a single macro that emits success +
// error tests with real in-body assertions.
// ===========================================================================

/// Emit success+error tests for a PATCH-update `FabricResource` with full
/// `list`/`create`/`get`/`update`/`delete` + `list_addresses` canonical routes.
macro_rules! fabric_patch_resource_full {
    (
        $mod:ident, $accessor:ident, $seg:literal,
        $rt_list:literal, $rt_create:literal, $rt_get:literal,
        $rt_update:literal, $rt_delete:literal, $rt_addrs:literal
    ) => {
        mod $mod {
            use super::*;

            #[test]
            fn list_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c.fabric().$accessor().list(&json!({})).expect("list");
                assert!(body.is_array() || body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "GET");
                assert_eq!(e.path, format!("{BASE}/{}", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_list));
            }

            #[test]
            fn list_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_list, 500, json!({"error":"boom"}));
                let err = c.fabric().$accessor().list(&json!({})).expect_err("err");
                assert_eq!(err.status_code(), 500);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(500));
                assert_eq!(e.matched_route.as_deref(), Some($rt_list));
            }

            #[test]
            fn create_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c
                    .fabric()
                    .$accessor()
                    .create(&json!({"name": "x"}))
                    .expect("create");
                assert!(body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "POST");
                assert_eq!(e.path, format!("{BASE}/{}", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_create));
            }

            #[test]
            fn create_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_create, 422, json!({"error":"bad"}));
                let err = c
                    .fabric()
                    .$accessor()
                    .create(&json!({}))
                    .expect_err("err");
                assert_eq!(err.status_code(), 422);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(422));
                assert_eq!(e.matched_route.as_deref(), Some($rt_create));
            }

            #[test]
            fn get_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c.fabric().$accessor().get("id-1").expect("get");
                assert!(body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "GET");
                assert_eq!(e.path, format!("{BASE}/{}/id-1", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_get));
            }

            #[test]
            fn get_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_get, 404, json!({"error":"nf"}));
                let err = c.fabric().$accessor().get("missing").expect_err("err");
                assert_eq!(err.status_code(), 404);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(404));
                assert_eq!(e.matched_route.as_deref(), Some($rt_get));
            }

            #[test]
            fn update_uses_patch_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c
                    .fabric()
                    .$accessor()
                    .update("id-1", &json!({"name": "renamed"}))
                    .expect("update");
                assert!(body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "PATCH");
                assert_eq!(e.path, format!("{BASE}/{}/id-1", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_update));
                let sent = e.body_object().expect("body");
                assert_eq!(sent.get("name").and_then(Value::as_str), Some("renamed"));
            }

            #[test]
            fn update_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_update, 404, json!({"error":"nf"}));
                let err = c
                    .fabric()
                    .$accessor()
                    .update("missing", &json!({}))
                    .expect_err("err");
                assert_eq!(err.status_code(), 404);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(404));
                assert_eq!(e.matched_route.as_deref(), Some($rt_update));
            }

            #[test]
            fn delete_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c.fabric().$accessor().delete("id-1").expect("delete");
                assert!(body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "DELETE");
                assert_eq!(e.path, format!("{BASE}/{}/id-1", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_delete));
            }

            #[test]
            fn delete_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_delete, 404, json!({"error":"nf"}));
                let err = c.fabric().$accessor().delete("missing").expect_err("err");
                assert_eq!(err.status_code(), 404);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(404));
                assert_eq!(e.matched_route.as_deref(), Some($rt_delete));
            }

            #[test]
            fn list_addresses_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c
                    .fabric()
                    .$accessor()
                    .list_addresses("id-1", &json!({}))
                    .expect("list_addresses");
                assert!(body.is_array() || body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "GET");
                assert_eq!(e.path, format!("{BASE}/{}/id-1/addresses", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_addrs));
            }

            #[test]
            fn list_addresses_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_addrs, 404, json!({"error":"nf"}));
                let err = c
                    .fabric()
                    .$accessor()
                    .list_addresses("missing", &json!({}))
                    .expect_err("err");
                assert_eq!(err.status_code(), 404);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(404));
                assert_eq!(e.matched_route.as_deref(), Some($rt_addrs));
            }
        }
    };
}

fabric_patch_resource_full!(
    ai_agents,
    ai_agents,
    "ai_agents",
    "fabric.list_ai_agents",
    "fabric.create_ai_agent",
    "fabric.get_ai_agent",
    "fabric.update_ai_agent",
    "fabric.delete_ai_agent",
    "fabric.list_ai_agent_addresses"
);

fabric_patch_resource_full!(
    cxml_webhooks,
    cxml_webhooks,
    "cxml_webhooks",
    "fabric.list_cxml_webhooks",
    "fabric.create_cxml_webhook",
    "fabric.get_cxml_webhook",
    "fabric.update_cxml_webhook",
    "fabric.delete_cxml_webhook",
    "fabric.list_cxml_webhook_addresses"
);

fabric_patch_resource_full!(
    swml_webhooks,
    swml_webhooks,
    "swml_webhooks",
    "fabric.list_swml_webhooks",
    "fabric.create_swml_webhook",
    "fabric.get_swml_webhook",
    "fabric.update_swml_webhook",
    "fabric.delete_swml_webhook",
    "fabric.list_swml_webhook_addresses"
);

// sip_gateways uses PATCH update; its canonical list_addresses route is the
// doubled-path `fabric.list_sip_gateway_addresses`
// (/sip_gateways/resources/sip_gateways/{id}/addresses), which the SDK's
// list_addresses does NOT hit (it targets the plain /sip_gateways/{id}/addresses
// route, which has no canonical operation). So sip_gateways covers only
// list/create/get/update/delete here; list_sip_gateway_addresses stays a gap.
mod sip_gateways {
    use super::*;

    #[test]
    fn list_success() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        let body = c.fabric().sip_gateways().list(&json!({})).expect("list");
        assert!(body.is_object());
        let e = common::mocktest::journal_last();
        assert_eq!(e.method, "GET");
        assert_eq!(e.path, format!("{BASE}/sip_gateways"));
        assert_eq!(e.matched_route.as_deref(), Some("fabric.list_sip_gateways"));
    }

    #[test]
    fn list_error() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        common::mocktest::scenario_set("fabric.list_sip_gateways", 500, json!({"error":"boom"}));
        let err = c.fabric().sip_gateways().list(&json!({})).expect_err("err");
        assert_eq!(err.status_code(), 500);
        let e = common::mocktest::journal_last();
        assert_eq!(e.response_status, Some(500));
        assert_eq!(e.matched_route.as_deref(), Some("fabric.list_sip_gateways"));
    }

    #[test]
    fn create_success() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        let body = c
            .fabric()
            .sip_gateways()
            .create(&json!({"name": "g"}))
            .expect("create");
        assert!(body.is_object());
        let e = common::mocktest::journal_last();
        assert_eq!(e.method, "POST");
        assert_eq!(e.path, format!("{BASE}/sip_gateways"));
        assert_eq!(
            e.matched_route.as_deref(),
            Some("fabric.create_sip_gateway")
        );
    }

    #[test]
    fn create_error() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        common::mocktest::scenario_set("fabric.create_sip_gateway", 422, json!({"error":"bad"}));
        let err = c
            .fabric()
            .sip_gateways()
            .create(&json!({}))
            .expect_err("err");
        assert_eq!(err.status_code(), 422);
        let e = common::mocktest::journal_last();
        assert_eq!(e.response_status, Some(422));
        assert_eq!(
            e.matched_route.as_deref(),
            Some("fabric.create_sip_gateway")
        );
    }

    #[test]
    fn get_success() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        let body = c.fabric().sip_gateways().get("gw-1").expect("get");
        assert!(body.is_object());
        let e = common::mocktest::journal_last();
        assert_eq!(e.method, "GET");
        assert_eq!(e.path, format!("{BASE}/sip_gateways/gw-1"));
        assert_eq!(e.matched_route.as_deref(), Some("fabric.get_sip_gateway"));
    }

    #[test]
    fn get_error() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        common::mocktest::scenario_set("fabric.get_sip_gateway", 404, json!({"error":"nf"}));
        let err = c.fabric().sip_gateways().get("missing").expect_err("err");
        assert_eq!(err.status_code(), 404);
        let e = common::mocktest::journal_last();
        assert_eq!(e.response_status, Some(404));
        assert_eq!(e.matched_route.as_deref(), Some("fabric.get_sip_gateway"));
    }

    #[test]
    fn update_uses_patch_success() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        let body = c
            .fabric()
            .sip_gateways()
            .update("gw-1", &json!({"name": "renamed"}))
            .expect("update");
        assert!(body.is_object());
        let e = common::mocktest::journal_last();
        assert_eq!(e.method, "PATCH");
        assert_eq!(e.path, format!("{BASE}/sip_gateways/gw-1"));
        assert_eq!(
            e.matched_route.as_deref(),
            Some("fabric.update_sip_gateway")
        );
        let sent = e.body_object().expect("body");
        assert_eq!(sent.get("name").and_then(Value::as_str), Some("renamed"));
    }

    #[test]
    fn update_error() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        common::mocktest::scenario_set("fabric.update_sip_gateway", 404, json!({"error":"nf"}));
        let err = c
            .fabric()
            .sip_gateways()
            .update("missing", &json!({}))
            .expect_err("err");
        assert_eq!(err.status_code(), 404);
        let e = common::mocktest::journal_last();
        assert_eq!(e.response_status, Some(404));
        assert_eq!(
            e.matched_route.as_deref(),
            Some("fabric.update_sip_gateway")
        );
    }

    #[test]
    fn delete_success() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        let body = c.fabric().sip_gateways().delete("gw-1").expect("delete");
        assert!(body.is_object());
        let e = common::mocktest::journal_last();
        assert_eq!(e.method, "DELETE");
        assert_eq!(e.path, format!("{BASE}/sip_gateways/gw-1"));
        assert_eq!(
            e.matched_route.as_deref(),
            Some("fabric.delete_sip_gateway")
        );
    }

    #[test]
    fn delete_error() {
        let _g = common::mocktest::begin();
        let c = common::mocktest::client();
        common::mocktest::scenario_set("fabric.delete_sip_gateway", 404, json!({"error":"nf"}));
        let err = c
            .fabric()
            .sip_gateways()
            .delete("missing")
            .expect_err("err");
        assert_eq!(err.status_code(), 404);
        let e = common::mocktest::journal_last();
        assert_eq!(e.response_status, Some(404));
        assert_eq!(
            e.matched_route.as_deref(),
            Some("fabric.delete_sip_gateway")
        );
    }
}

// ===========================================================================
// PUT-update resources (FabricResourcePUT): cxml_scripts, swml_scripts,
// relay_applications, freeswitch_connectors, sip_endpoints.
//
// Each generates list/create/get/update(PUT)/delete + list_addresses.
// ===========================================================================

macro_rules! fabric_put_resource_full {
    (
        $mod:ident, $accessor:ident, $seg:literal,
        $rt_list:literal, $rt_create:literal, $rt_get:literal,
        $rt_update:literal, $rt_delete:literal, $rt_addrs:literal
    ) => {
        mod $mod {
            use super::*;

            #[test]
            fn list_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c.fabric().$accessor().list(&json!({})).expect("list");
                assert!(body.is_array() || body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "GET");
                assert_eq!(e.path, format!("{BASE}/{}", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_list));
            }

            #[test]
            fn list_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_list, 500, json!({"error":"boom"}));
                let err = c.fabric().$accessor().list(&json!({})).expect_err("err");
                assert_eq!(err.status_code(), 500);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(500));
                assert_eq!(e.matched_route.as_deref(), Some($rt_list));
            }

            #[test]
            fn create_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c
                    .fabric()
                    .$accessor()
                    .create(&json!({"name": "x"}))
                    .expect("create");
                assert!(body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "POST");
                assert_eq!(e.path, format!("{BASE}/{}", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_create));
            }

            #[test]
            fn create_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_create, 422, json!({"error":"bad"}));
                let err = c
                    .fabric()
                    .$accessor()
                    .create(&json!({}))
                    .expect_err("err");
                assert_eq!(err.status_code(), 422);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(422));
                assert_eq!(e.matched_route.as_deref(), Some($rt_create));
            }

            #[test]
            fn get_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c.fabric().$accessor().get("id-1").expect("get");
                assert!(body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "GET");
                assert_eq!(e.path, format!("{BASE}/{}/id-1", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_get));
            }

            #[test]
            fn get_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_get, 404, json!({"error":"nf"}));
                let err = c.fabric().$accessor().get("missing").expect_err("err");
                assert_eq!(err.status_code(), 404);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(404));
                assert_eq!(e.matched_route.as_deref(), Some($rt_get));
            }

            #[test]
            fn update_uses_put_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c
                    .fabric()
                    .$accessor()
                    .update("id-1", &json!({"name": "renamed"}))
                    .expect("update");
                assert!(body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "PUT");
                assert_eq!(e.path, format!("{BASE}/{}/id-1", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_update));
                let sent = e.body_object().expect("body");
                assert_eq!(sent.get("name").and_then(Value::as_str), Some("renamed"));
            }

            #[test]
            fn update_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_update, 404, json!({"error":"nf"}));
                let err = c
                    .fabric()
                    .$accessor()
                    .update("missing", &json!({}))
                    .expect_err("err");
                assert_eq!(err.status_code(), 404);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(404));
                assert_eq!(e.matched_route.as_deref(), Some($rt_update));
            }

            #[test]
            fn delete_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c.fabric().$accessor().delete("id-1").expect("delete");
                assert!(body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "DELETE");
                assert_eq!(e.path, format!("{BASE}/{}/id-1", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_delete));
            }

            #[test]
            fn delete_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_delete, 404, json!({"error":"nf"}));
                let err = c.fabric().$accessor().delete("missing").expect_err("err");
                assert_eq!(err.status_code(), 404);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(404));
                assert_eq!(e.matched_route.as_deref(), Some($rt_delete));
            }

            #[test]
            fn list_addresses_success() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                let body = c
                    .fabric()
                    .$accessor()
                    .list_addresses("id-1", &json!({}))
                    .expect("list_addresses");
                assert!(body.is_array() || body.is_object());
                let e = common::mocktest::journal_last();
                assert_eq!(e.method, "GET");
                assert_eq!(e.path, format!("{BASE}/{}/id-1/addresses", $seg));
                assert_eq!(e.matched_route.as_deref(), Some($rt_addrs));
            }

            #[test]
            fn list_addresses_error() {
                let _g = common::mocktest::begin();
                let c = common::mocktest::client();
                common::mocktest::scenario_set($rt_addrs, 404, json!({"error":"nf"}));
                let err = c
                    .fabric()
                    .$accessor()
                    .list_addresses("missing", &json!({}))
                    .expect_err("err");
                assert_eq!(err.status_code(), 404);
                let e = common::mocktest::journal_last();
                assert_eq!(e.response_status, Some(404));
                assert_eq!(e.matched_route.as_deref(), Some($rt_addrs));
            }
        }
    };
}

fabric_put_resource_full!(
    cxml_scripts,
    cxml_scripts,
    "cxml_scripts",
    "fabric.list_cxml_scripts",
    "fabric.create_cxml_script",
    "fabric.get_cxml_script",
    "fabric.update_cxml_script",
    "fabric.delete_cxml_script",
    "fabric.list_cxml_script_addresses"
);

fabric_put_resource_full!(
    swml_scripts,
    swml_scripts,
    "swml_scripts",
    "fabric.list_swml_scripts",
    "fabric.create_swml_script",
    "fabric.get_swml_script",
    "fabric.update_swml_script",
    "fabric.delete_swml_script",
    "fabric.list_swml_script_addresses"
);

fabric_put_resource_full!(
    relay_applications,
    relay_applications,
    "relay_applications",
    "fabric.list_relay_applications",
    "fabric.create_relay_application",
    "fabric.get_relay_application",
    "fabric.update_relay_application",
    "fabric.delete_relay_application",
    "fabric.list_relay_application_addresses"
);

fabric_put_resource_full!(
    freeswitch_connectors,
    freeswitch_connectors,
    "freeswitch_connectors",
    "fabric.list_freeswitch_connectors",
    "fabric.create_freeswitch_connector",
    "fabric.get_freeswitch_connector",
    "fabric.update_freeswitch_connector",
    "fabric.delete_freeswitch_connector",
    "fabric.list_freeswitch_connector_addresses"
);

// sip_endpoints uses PUT update. Its plain list/create/get/update/delete +
// list_addresses are all canonical routes. (The doubled-path
// `fabric.assign_resource_sip_endpoint` has no accessor and stays a gap.)
fabric_put_resource_full!(
    sip_endpoints,
    sip_endpoints,
    "sip_endpoints",
    "fabric.list_sip_endpoints",
    "fabric.create_sip_endpoint",
    "fabric.get_sip_endpoint",
    "fabric.update_sip_endpoint",
    "fabric.delete_sip_endpoint",
    "fabric.list_sip_endpoint_addresses"
);

// ---------------------------------------------------------------------------
// Parity micro-routes added in the fabric parity pass — exercised here so the
// REST-COVERAGE checker sees them hit (success + error).
//   * fabric.list_cxml_application_addresses → GET {BASE}/cxml_applications/{id}/addresses
//   * fabric.assign_resource_phone_route     → POST {BASE}/{id}/phone_routes
// ---------------------------------------------------------------------------

#[test]
fn test_fabric_cxml_application_addresses_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .cxml_applications()
        .list_addresses("ca-1", &json!({}))
        .expect("list_addresses");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, format!("{BASE}/cxml_applications/ca-1/addresses"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_cxml_application_addresses")
    );
}

#[test]
fn test_fabric_cxml_application_addresses_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.list_cxml_application_addresses",
        404,
        json!({"error":"not found"}),
    );
    let err = c
        .fabric()
        .cxml_applications()
        .list_addresses("ca-1", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.list_cxml_application_addresses")
    );
}

#[test]
fn test_fabric_assign_resource_phone_route_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .fabric()
        .resources()
        .assign_phone_route("res-1", &json!({"phone_route_id": "pr-1"}))
        .expect("assign_phone_route");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, format!("{BASE}/res-1/phone_routes"));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.assign_resource_phone_route")
    );
}

#[test]
fn test_fabric_assign_resource_phone_route_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "fabric.assign_resource_phone_route",
        422,
        json!({"error":"invalid"}),
    );
    let err = c
        .fabric()
        .resources()
        .assign_phone_route("res-1", &json!({"phone_route_id": "pr-1"}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("fabric.assign_resource_phone_route")
    );
}
