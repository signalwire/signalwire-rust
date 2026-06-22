// Mock-backed REST success + error coverage for the `relay-rest` spec group.
//
// Every coverable canonical route in the relay-rest group gets a success
// (2xx) test on the correct path + matched_route, and an error (4xx/5xx)
// test staged via `scenario_set(endpoint_id, status, body)`.
//
// Accepted gaps (not coverable from the Rust SDK surface): the SIP endpoint
// routes (`/api/relay/rest/endpoints/sip*`, 5) and the domain_applications
// routes (`/api/relay/rest/domain_applications*`, 5) have no SDK namespace.

#[path = "common/mod.rs"]
mod common;

use std::collections::HashMap;

use serde_json::{Value, json};

// ===========================================================================
// Phone Numbers — list / search / purchase / get / update (PUT) / release
// ===========================================================================

#[test]
fn test_phone_numbers_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let mut params = HashMap::new();
    params.insert("page_size".to_string(), "10".to_string());
    let body = c.phone_numbers().list(&params).expect("phone_numbers.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/phone_numbers");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_phone_numbers")
    );
}

#[test]
fn test_phone_numbers_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.list_phone_numbers",
        500,
        json!({"error": "boom"}),
    );
    let err = c
        .phone_numbers()
        .list(&HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_phone_numbers")
    );
}

#[test]
fn test_phone_numbers_search_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .phone_numbers()
        .search(&json!({"area_code": "512"}))
        .expect("phone_numbers.search");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/phone_numbers/search");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.search_available_phone_numbers")
    );
    assert_eq!(
        e.query_params.get("area_code").map(Vec::as_slice),
        Some(["512".to_string()].as_slice())
    );
}

#[test]
fn test_phone_numbers_search_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.search_available_phone_numbers",
        400,
        json!({"error": "bad area code"}),
    );
    let err = c
        .phone_numbers()
        .search(&json!({"area_code": "999"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 400);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(400));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.search_available_phone_numbers")
    );
}

#[test]
fn test_phone_numbers_purchase_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .phone_numbers()
        .create(&json!({"number": "+15551230000"}))
        .expect("phone_numbers.create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/phone_numbers");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.purchase_phone_number")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("number").and_then(Value::as_str),
        Some("+15551230000")
    );
}

#[test]
fn test_phone_numbers_purchase_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.purchase_phone_number",
        422,
        json!({"error": "unavailable"}),
    );
    let err = c
        .phone_numbers()
        .create(&json!({"number": "+15550000000"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.purchase_phone_number")
    );
}

#[test]
fn test_phone_numbers_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.phone_numbers().get("pn-1").expect("phone_numbers.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/phone_numbers/pn-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_phone_number")
    );
}

#[test]
fn test_phone_numbers_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.retrieve_phone_number",
        404,
        json!({"error": "nf"}),
    );
    let err = c.phone_numbers().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_phone_number")
    );
}

#[test]
fn test_phone_numbers_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .phone_numbers()
        .update("pn-1", &json!({"name": "Main line"}))
        .expect("phone_numbers.update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/relay/rest/phone_numbers/pn-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_phone_number")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("Main line"));
}

#[test]
fn test_phone_numbers_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.update_phone_number",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .phone_numbers()
        .update("missing", &json!({"name": "x"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_phone_number")
    );
}

#[test]
fn test_phone_numbers_release_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .phone_numbers()
        .delete("pn-1")
        .expect("phone_numbers.delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/relay/rest/phone_numbers/pn-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.release_phone_number")
    );
}

#[test]
fn test_phone_numbers_release_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.release_phone_number",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .phone_numbers()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.release_phone_number")
    );
}

// ===========================================================================
// Addresses — list / create / get / delete (no update route)
// ===========================================================================

#[test]
fn test_addresses_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.addresses().list(&HashMap::new()).expect("addresses.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/addresses");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_addresses")
    );
}

#[test]
fn test_addresses_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.list_addresses", 500, json!({"error": "boom"}));
    let err = c
        .addresses()
        .list(&HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_addresses")
    );
}

#[test]
fn test_addresses_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .addresses()
        .create(&json!({"address_type": "commercial", "first_name": "Ada"}))
        .expect("addresses.create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/addresses");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_address")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("first_name").and_then(Value::as_str), Some("Ada"));
}

#[test]
fn test_addresses_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.create_address",
        422,
        json!({"error": "invalid"}),
    );
    let err = c
        .addresses()
        .create(&json!({"address_type": "x"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_address")
    );
}

#[test]
fn test_addresses_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.addresses().get("addr-1").expect("addresses.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/addresses/addr-1");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.get_address"));
}

#[test]
fn test_addresses_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.get_address", 404, json!({"error": "nf"}));
    let err = c.addresses().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.get_address"));
}

#[test]
fn test_addresses_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.addresses().delete("addr-1").expect("addresses.delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/relay/rest/addresses/addr-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_address")
    );
}

#[test]
fn test_addresses_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.delete_address", 404, json!({"error": "nf"}));
    let err = c.addresses().delete("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_address")
    );
}

// ===========================================================================
// Verified Caller IDs — list / create / get / update (PUT) / delete
//   + redial_verification (POST {id}/verification)
//   + submit_verification (PUT {id}/verification)
// ===========================================================================

#[test]
fn test_verified_callers_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .verified_callers()
        .list(&HashMap::new())
        .expect("verified_callers.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/verified_caller_ids");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_verified_caller_ids")
    );
}

#[test]
fn test_verified_callers_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.list_verified_caller_ids",
        500,
        json!({"error": "boom"}),
    );
    let err = c
        .verified_callers()
        .list(&HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_verified_caller_ids")
    );
}

#[test]
fn test_verified_callers_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .verified_callers()
        .create(&json!({"number": "+15551239999"}))
        .expect("verified_callers.create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/verified_caller_ids");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_verified_caller_id")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("number").and_then(Value::as_str),
        Some("+15551239999")
    );
}

#[test]
fn test_verified_callers_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.create_verified_caller_id",
        422,
        json!({"error": "invalid"}),
    );
    let err = c
        .verified_callers()
        .create(&json!({"number": "bad"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_verified_caller_id")
    );
}

#[test]
fn test_verified_callers_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .verified_callers()
        .get("vc-1")
        .expect("verified_callers.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/verified_caller_ids/vc-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_verified_caller_id")
    );
}

#[test]
fn test_verified_callers_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.retrieve_verified_caller_id",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .verified_callers()
        .get("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_verified_caller_id")
    );
}

#[test]
fn test_verified_callers_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .verified_callers()
        .update("vc-1", &json!({"name": "Office"}))
        .expect("verified_callers.update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/relay/rest/verified_caller_ids/vc-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_verified_caller_id")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("Office"));
}

#[test]
fn test_verified_callers_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.update_verified_caller_id",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .verified_callers()
        .update("missing", &json!({"name": "x"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_verified_caller_id")
    );
}

#[test]
fn test_verified_callers_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .verified_callers()
        .delete("vc-1")
        .expect("verified_callers.delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/relay/rest/verified_caller_ids/vc-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_verified_caller_id")
    );
}

#[test]
fn test_verified_callers_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.delete_verified_caller_id",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .verified_callers()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_verified_caller_id")
    );
}

#[test]
fn test_verified_callers_redial_verification_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .verified_callers()
        .redial_verification("vc-1")
        .expect("redial_verification");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(
        e.path,
        "/api/relay/rest/verified_caller_ids/vc-1/verification"
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.redial_verification_call")
    );
}

#[test]
fn test_verified_callers_redial_verification_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.redial_verification_call",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .verified_callers()
        .redial_verification("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.redial_verification_call")
    );
}

#[test]
fn test_verified_callers_submit_verification_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .verified_callers()
        .submit_verification("vc-1", &json!({"code": "1234"}))
        .expect("submit_verification");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(
        e.path,
        "/api/relay/rest/verified_caller_ids/vc-1/verification"
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.validate_verification_code")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("code").and_then(Value::as_str), Some("1234"));
}

#[test]
fn test_verified_callers_submit_verification_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.validate_verification_code",
        422,
        json!({"error": "bad code"}),
    );
    let err = c
        .verified_callers()
        .submit_verification("vc-1", &json!({"code": "0000"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.validate_verification_code")
    );
}

// ===========================================================================
// Queues — list / create / get / update (PUT) / delete
//   + members: list / next / get
// ===========================================================================

#[test]
fn test_queues_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.queues().list(&json!({})).expect("queues.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/queues");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.list_queues"));
}

#[test]
fn test_queues_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.list_queues", 500, json!({"error": "boom"}));
    let err = c.queues().list(&json!({})).expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.list_queues"));
}

#[test]
fn test_queues_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .queues()
        .create(&json!({"name": "support"}))
        .expect("queues.create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/queues");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.create_queue"));
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("support"));
}

#[test]
fn test_queues_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.create_queue", 422, json!({"error": "invalid"}));
    let err = c.queues().create(&json!({})).expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.create_queue"));
}

#[test]
fn test_queues_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.queues().get("q-1").expect("queues.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/queues/q-1");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.get_queue"));
}

#[test]
fn test_queues_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.get_queue", 404, json!({"error": "nf"}));
    let err = c.queues().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.get_queue"));
}

#[test]
fn test_queues_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .queues()
        .update("q-1", &json!({"name": "renamed"}))
        .expect("queues.update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/relay/rest/queues/q-1");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.update_queue"));
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("renamed"));
}

#[test]
fn test_queues_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.update_queue", 404, json!({"error": "nf"}));
    let err = c
        .queues()
        .update("missing", &json!({"name": "x"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.update_queue"));
}

#[test]
fn test_queues_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.queues().delete("q-1").expect("queues.delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/relay/rest/queues/q-1");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.delete_queue"));
}

#[test]
fn test_queues_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.delete_queue", 404, json!({"error": "nf"}));
    let err = c.queues().delete("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.delete_queue"));
}

#[test]
fn test_queues_list_members_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .queues()
        .list_members("q-1", &json!({}))
        .expect("queues.list_members");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/queues/q-1/members");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_queue_members")
    );
}

#[test]
fn test_queues_list_members_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.list_queue_members", 404, json!({"error": "nf"}));
    let err = c
        .queues()
        .list_members("missing", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_queue_members")
    );
}

#[test]
fn test_queues_next_member_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .queues()
        .get_next_member("q-1")
        .expect("queues.get_next_member");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/queues/q-1/members/next");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_next_queue_member")
    );
}

#[test]
fn test_queues_next_member_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.retrieve_next_queue_member",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .queues()
        .get_next_member("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_next_queue_member")
    );
}

#[test]
fn test_queues_get_member_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .queues()
        .get_member("q-1", "mem-7")
        .expect("queues.get_member");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/queues/q-1/members/mem-7");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_queue_member")
    );
}

#[test]
fn test_queues_get_member_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.retrieve_queue_member",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .queues()
        .get_member("q-1", "missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_queue_member")
    );
}

// ===========================================================================
// Recordings — list / get / delete
// ===========================================================================

#[test]
fn test_recordings_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .recordings()
        .list(&HashMap::new())
        .expect("recordings.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/recordings");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_recordings")
    );
}

#[test]
fn test_recordings_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.list_recordings", 500, json!({"error": "boom"}));
    let err = c
        .recordings()
        .list(&HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_recordings")
    );
}

#[test]
fn test_recordings_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.recordings().get("rec-1").expect("recordings.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/recordings/rec-1");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.get_recording"));
}

#[test]
fn test_recordings_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.get_recording", 404, json!({"error": "nf"}));
    let err = c.recordings().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.get_recording"));
}

#[test]
fn test_recordings_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.recordings().delete("rec-1").expect("recordings.delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/relay/rest/recordings/rec-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_recording")
    );
}

#[test]
fn test_recordings_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.delete_recording", 404, json!({"error": "nf"}));
    let err = c.recordings().delete("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_recording")
    );
}

// ===========================================================================
// Number Groups — list / create / get / update (PUT) / delete
//   + memberships: list / create (group-scoped), get / delete (project-scoped)
// ===========================================================================

#[test]
fn test_number_groups_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .list(&json!({}))
        .expect("number_groups.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/number_groups");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_number_groups")
    );
}

#[test]
fn test_number_groups_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.list_number_groups",
        500,
        json!({"error": "boom"}),
    );
    let err = c.number_groups().list(&json!({})).expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_number_groups")
    );
}

#[test]
fn test_number_groups_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .create(&json!({"name": "group-a"}))
        .expect("number_groups.create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/number_groups");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_number_group")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("group-a"));
}

#[test]
fn test_number_groups_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.create_number_group", 422, json!({"error": "x"}));
    let err = c
        .number_groups()
        .create(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_number_group")
    );
}

#[test]
fn test_number_groups_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.number_groups().get("ng-1").expect("number_groups.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/number_groups/ng-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_number_group")
    );
}

#[test]
fn test_number_groups_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.retrieve_number_group",
        404,
        json!({"error": "nf"}),
    );
    let err = c.number_groups().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_number_group")
    );
}

#[test]
fn test_number_groups_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .update("ng-1", &json!({"name": "renamed"}))
        .expect("number_groups.update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/relay/rest/number_groups/ng-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_number_group")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("renamed"));
}

#[test]
fn test_number_groups_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.update_number_group",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .number_groups()
        .update("missing", &json!({"name": "x"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_number_group")
    );
}

#[test]
fn test_number_groups_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .delete("ng-1")
        .expect("number_groups.delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/relay/rest/number_groups/ng-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_number_group")
    );
}

#[test]
fn test_number_groups_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.delete_number_group",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .number_groups()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_number_group")
    );
}

#[test]
fn test_number_groups_list_memberships_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .list_memberships("ng-1", &json!({}))
        .expect("list_memberships");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(
        e.path,
        "/api/relay/rest/number_groups/ng-1/number_group_memberships"
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_number_group_memberships")
    );
}

#[test]
fn test_number_groups_list_memberships_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.list_number_group_memberships",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .number_groups()
        .list_memberships("missing", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_number_group_memberships")
    );
}

#[test]
fn test_number_groups_add_membership_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .add_membership("ng-1", &json!({"phone_number_id": "pn-1"}))
        .expect("add_membership");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(
        e.path,
        "/api/relay/rest/number_groups/ng-1/number_group_memberships"
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_number_group_membership")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("phone_number_id").and_then(Value::as_str),
        Some("pn-1")
    );
}

#[test]
fn test_number_groups_add_membership_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.create_number_group_membership",
        422,
        json!({"error": "x"}),
    );
    let err = c
        .number_groups()
        .add_membership("ng-1", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_number_group_membership")
    );
}

#[test]
fn test_number_groups_get_membership_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .get_membership("mem-1")
        .expect("get_membership");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/number_group_memberships/mem-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_number_group_membership")
    );
}

#[test]
fn test_number_groups_get_membership_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.retrieve_number_group_membership",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .number_groups()
        .get_membership("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_number_group_membership")
    );
}

#[test]
fn test_number_groups_delete_membership_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .delete_membership("mem-1")
        .expect("delete_membership");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/relay/rest/number_group_memberships/mem-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_number_group_membership")
    );
}

#[test]
fn test_number_groups_delete_membership_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.delete_number_group_membership",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .number_groups()
        .delete_membership("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_number_group_membership")
    );
}

// ===========================================================================
// Short Codes — list / get / update (PUT)
// ===========================================================================

#[test]
fn test_short_codes_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .short_codes()
        .list(&HashMap::new())
        .expect("short_codes.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/short_codes");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_short_codes")
    );
}

#[test]
fn test_short_codes_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.list_short_codes", 500, json!({"error": "boom"}));
    let err = c
        .short_codes()
        .list(&HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_short_codes")
    );
}

#[test]
fn test_short_codes_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.short_codes().get("sc-1").expect("short_codes.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/short_codes/sc-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_short_code")
    );
}

#[test]
fn test_short_codes_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.retrieve_short_code",
        404,
        json!({"error": "nf"}),
    );
    let err = c.short_codes().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_short_code")
    );
}

#[test]
fn test_short_codes_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .short_codes()
        .update("sc-1", &json!({"name": "Marketing"}))
        .expect("short_codes.update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/relay/rest/short_codes/sc-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_short_code")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("Marketing"));
}

#[test]
fn test_short_codes_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.update_short_code", 404, json!({"error": "nf"}));
    let err = c
        .short_codes()
        .update("missing", &json!({"name": "x"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_short_code")
    );
}

// ===========================================================================
// Imported Phone Numbers — create
// ===========================================================================

#[test]
fn test_imported_numbers_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .imported_numbers()
        .create(&json!({"number": "+15551234567", "sip_username": "alice"}))
        .expect("imported_numbers.create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/imported_phone_numbers");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_imported_phone_number")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("number").and_then(Value::as_str),
        Some("+15551234567")
    );
}

#[test]
fn test_imported_numbers_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.create_imported_phone_number",
        422,
        json!({"error": "invalid"}),
    );
    let err = c
        .imported_numbers()
        .create(&json!({"number": "bad"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_imported_phone_number")
    );
}

// ===========================================================================
// MFA — sms / call / verify
// ===========================================================================

#[test]
fn test_mfa_sms_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .mfa()
        .sms(&json!({"to": "+15551234567", "from": "+15559876543"}))
        .expect("mfa.sms");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/mfa/sms");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.request_mfa_sms")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("to").and_then(Value::as_str), Some("+15551234567"));
}

#[test]
fn test_mfa_sms_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.request_mfa_sms",
        422,
        json!({"error": "invalid"}),
    );
    let err = c.mfa().sms(&json!({"to": "bad"})).expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.request_mfa_sms")
    );
}

#[test]
fn test_mfa_call_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .mfa()
        .call(&json!({"to": "+15551234567", "from": "+15559876543"}))
        .expect("mfa.call");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/mfa/call");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.request_mfa_call")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("to").and_then(Value::as_str), Some("+15551234567"));
}

#[test]
fn test_mfa_call_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.request_mfa_call",
        422,
        json!({"error": "invalid"}),
    );
    let err = c
        .mfa()
        .call(&json!({"to": "bad"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.request_mfa_call")
    );
}

#[test]
fn test_mfa_verify_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .mfa()
        .verify("req-1", &json!({"token": "123456"}))
        .expect("mfa.verify");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/mfa/req-1/verify");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.verify_mfa_token")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("token").and_then(Value::as_str), Some("123456"));
}

#[test]
fn test_mfa_verify_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.verify_mfa_token", 404, json!({"error": "nf"}));
    let err = c
        .mfa()
        .verify("missing", &json!({"token": "000000"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.verify_mfa_token")
    );
}

// ===========================================================================
// SIP Profile (singleton) — retrieve (GET) / update (PUT)
// ===========================================================================

#[test]
fn test_sip_profile_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.sip_profile().get().expect("sip_profile.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/sip_profile");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_sip_profile")
    );
}

#[test]
fn test_sip_profile_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.retrieve_sip_profile",
        500,
        json!({"error": "boom"}),
    );
    let err = c.sip_profile().get().expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_sip_profile")
    );
}

#[test]
fn test_sip_profile_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .sip_profile()
        .update(&json!({"domain": "myco.sip.signalwire.com"}))
        .expect("sip_profile.update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/relay/rest/sip_profile");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_sip_profile")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("domain").and_then(Value::as_str),
        Some("myco.sip.signalwire.com")
    );
}

#[test]
fn test_sip_profile_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.update_sip_profile", 422, json!({"error": "x"}));
    let err = c
        .sip_profile()
        .update(&json!({"domain": "bad"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_sip_profile")
    );
}

// ===========================================================================
// Lookup — GET /api/relay/rest/lookup/phone_number/{e164_number}
// ===========================================================================

#[test]
fn test_lookup_phone_number_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .lookup()
        .phone_number("+15551234567")
        .expect("lookup.phone_number");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/lookup/phone_number/+15551234567");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.lookup_phone_number")
    );
}

#[test]
fn test_lookup_phone_number_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.lookup_phone_number",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .lookup()
        .phone_number("+10000000000")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.lookup_phone_number")
    );
}

// ===========================================================================
// 10DLC Registry — brands / campaigns / orders / numbers
// ===========================================================================

#[test]
fn test_registry_brands_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.registry().brands().list(&json!({})).expect("brands.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/registry/beta/brands");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.list_brands"));
}

#[test]
fn test_registry_brands_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.list_brands", 500, json!({"error": "boom"}));
    let err = c
        .registry()
        .brands()
        .list(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.list_brands"));
}

#[test]
fn test_registry_brands_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .brands()
        .create(&json!({"display_name": "Acme"}))
        .expect("brands.create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/relay/rest/registry/beta/brands");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.create_brand"));
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("display_name").and_then(Value::as_str),
        Some("Acme")
    );
}

#[test]
fn test_registry_brands_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.create_brand", 422, json!({"error": "invalid"}));
    let err = c
        .registry()
        .brands()
        .create(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.create_brand"));
}

#[test]
fn test_registry_brands_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.registry().brands().get("brand-1").expect("brands.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/registry/beta/brands/brand-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_brand")
    );
}

#[test]
fn test_registry_brands_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.retrieve_brand", 404, json!({"error": "nf"}));
    let err = c
        .registry()
        .brands()
        .get("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_brand")
    );
}

#[test]
fn test_registry_brands_list_campaigns_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .brands()
        .list_campaigns("brand-1", &json!({}))
        .expect("list_campaigns");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(
        e.path,
        "/api/relay/rest/registry/beta/brands/brand-1/campaigns"
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_campaigns")
    );
}

#[test]
fn test_registry_brands_list_campaigns_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.list_campaigns", 404, json!({"error": "nf"}));
    let err = c
        .registry()
        .brands()
        .list_campaigns("missing", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_campaigns")
    );
}

#[test]
fn test_registry_brands_create_campaign_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .brands()
        .create_campaign("brand-1", &json!({"usecase": "LOW_VOLUME"}))
        .expect("create_campaign");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(
        e.path,
        "/api/relay/rest/registry/beta/brands/brand-1/campaigns"
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_campaign")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("usecase").and_then(Value::as_str),
        Some("LOW_VOLUME")
    );
}

#[test]
fn test_registry_brands_create_campaign_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.create_campaign", 422, json!({"error": "x"}));
    let err = c
        .registry()
        .brands()
        .create_campaign("brand-1", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.create_campaign")
    );
}

#[test]
fn test_registry_campaigns_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .campaigns()
        .get("camp-1")
        .expect("campaigns.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/registry/beta/campaigns/camp-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_campaign")
    );
}

#[test]
fn test_registry_campaigns_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.retrieve_campaign", 404, json!({"error": "nf"}));
    let err = c
        .registry()
        .campaigns()
        .get("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_campaign")
    );
}

#[test]
fn test_registry_campaigns_update_uses_put_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .campaigns()
        .update("camp-1", &json!({"description": "Updated"}))
        .expect("campaigns.update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/relay/rest/registry/beta/campaigns/camp-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_campaign")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("description").and_then(Value::as_str),
        Some("Updated")
    );
}

#[test]
fn test_registry_campaigns_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.update_campaign", 404, json!({"error": "nf"}));
    let err = c
        .registry()
        .campaigns()
        .update("missing", &json!({"description": "x"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.update_campaign")
    );
}

#[test]
fn test_registry_campaigns_list_numbers_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .campaigns()
        .list_numbers("camp-1", &json!({}))
        .expect("list_numbers");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(
        e.path,
        "/api/relay/rest/registry/beta/campaigns/camp-1/numbers"
    );
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_number_assignments")
    );
}

#[test]
fn test_registry_campaigns_list_numbers_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.list_number_assignments",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .registry()
        .campaigns()
        .list_numbers("missing", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.list_number_assignments")
    );
}

#[test]
fn test_registry_campaigns_list_orders_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .campaigns()
        .list_orders("camp-1", &json!({}))
        .expect("list_orders");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(
        e.path,
        "/api/relay/rest/registry/beta/campaigns/camp-1/orders"
    );
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.list_orders"));
}

#[test]
fn test_registry_campaigns_list_orders_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.list_orders", 404, json!({"error": "nf"}));
    let err = c
        .registry()
        .campaigns()
        .list_orders("missing", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.list_orders"));
}

#[test]
fn test_registry_campaigns_create_order_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .campaigns()
        .create_order("camp-1", &json!({"numbers": ["pn-1", "pn-2"]}))
        .expect("create_order");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(
        e.path,
        "/api/relay/rest/registry/beta/campaigns/camp-1/orders"
    );
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.create_order"));
    let sent = e.body_object().expect("body");
    let arr = sent
        .get("numbers")
        .and_then(Value::as_array)
        .expect("numbers array");
    let items: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
    assert_eq!(items, vec!["pn-1", "pn-2"]);
}

#[test]
fn test_registry_campaigns_create_order_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.create_order", 422, json!({"error": "x"}));
    let err = c
        .registry()
        .campaigns()
        .create_order("camp-1", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.create_order"));
}

#[test]
fn test_registry_orders_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.registry().orders().get("order-1").expect("orders.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/relay/rest/registry/beta/orders/order-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_order")
    );
}

#[test]
fn test_registry_orders_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("relay-rest.retrieve_order", 404, json!({"error": "nf"}));
    let err = c
        .registry()
        .orders()
        .get("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_order")
    );
}

#[test]
fn test_registry_numbers_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .registry()
        .numbers()
        .delete("num-1")
        .expect("numbers.delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/relay/rest/registry/beta/numbers/num-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_number_assignment")
    );
}

#[test]
fn test_registry_numbers_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "relay-rest.delete_number_assignment",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .registry()
        .numbers()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.delete_number_assignment")
    );
}
