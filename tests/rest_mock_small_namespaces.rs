// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_small_namespaces_mock.py.
//
// Covers the small-namespace gap closure: addresses, recordings,
// short_codes, imported_numbers, mfa, sip_profile, number_groups,
// project.tokens, datasphere.documents.get_chunk, queues.get_member.

#[path = "common/mod.rs"]
mod common;

use std::collections::HashMap;

use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Addresses
// ---------------------------------------------------------------------------

#[test]
fn test_small_addresses_list() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let mut params = HashMap::new();
    params.insert("page_size".to_string(), "10".to_string());
    let body = c.addresses().list(&params).expect("addresses.list");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"), "missing 'data' key");
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/relay/rest/addresses");
    assert!(entry.matched_route.is_some());
    let page_size = entry
        .query_params
        .get("page_size")
        .expect("page_size missing");
    assert_eq!(page_size.as_slice(), &["10".to_string()]);
}

#[test]
fn test_small_addresses_create() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .addresses()
        .create(&json!({
            "address_type": "commercial",
            "first_name": "Ada",
            "last_name": "Lovelace",
            "country": "US",
        }))
        .expect("addresses.create");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("id"), "expected 'id' in response");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, "/api/relay/rest/addresses");
    let sent = entry.body_object().expect("body object");
    assert_eq!(
        sent.get("address_type").and_then(Value::as_str),
        Some("commercial")
    );
    assert_eq!(sent.get("first_name").and_then(Value::as_str), Some("Ada"));
    assert_eq!(sent.get("country").and_then(Value::as_str), Some("US"));
}

#[test]
fn test_small_addresses_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.addresses().get("addr-123").expect("addresses.get");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/relay/rest/addresses/addr-123");
    assert!(entry.matched_route.is_some());
}

#[test]
fn test_small_addresses_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.addresses().delete("addr-123").expect("addresses.delete");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, "/api/relay/rest/addresses/addr-123");
    let status = entry.response_status.expect("response_status");
    assert!(
        matches!(status, 200 | 202 | 204),
        "response_status = {status}, want 200/202/204"
    );
}

// ---------------------------------------------------------------------------
// Recordings
// ---------------------------------------------------------------------------

#[test]
fn test_small_recordings_list() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let mut params = HashMap::new();
    params.insert("page_size".to_string(), "5".to_string());
    let body = c.recordings().list(&params).expect("recordings.list");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"), "missing 'data'");
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/relay/rest/recordings");
    assert_eq!(
        entry
            .query_params
            .get("page_size")
            .expect("page_size missing"),
        &vec!["5".to_string()]
    );
}

#[test]
fn test_small_recordings_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.recordings().get("rec-123").expect("recordings.get");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/relay/rest/recordings/rec-123");
}

#[test]
fn test_small_recordings_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.recordings().delete("rec-123").expect("recordings.delete");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, "/api/relay/rest/recordings/rec-123");
    let status = entry.response_status.expect("response_status");
    assert!(
        matches!(status, 200 | 202 | 204),
        "response_status = {status}"
    );
}

// ---------------------------------------------------------------------------
// Short Codes
// ---------------------------------------------------------------------------

#[test]
fn test_small_short_codes_list() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let mut params = HashMap::new();
    params.insert("page_size".to_string(), "20".to_string());
    let body = c.short_codes().list(&params).expect("short_codes.list");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"));
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/relay/rest/short_codes");
}

#[test]
fn test_small_short_codes_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.short_codes().get("sc-1").expect("short_codes.get");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/relay/rest/short_codes/sc-1");
}

#[test]
fn test_small_short_codes_update() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .short_codes()
        .update("sc-1", &json!({"name": "Marketing SMS"}))
        .expect("short_codes.update");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "PUT");
    assert_eq!(entry.path, "/api/relay/rest/short_codes/sc-1");
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("name").and_then(Value::as_str),
        Some("Marketing SMS")
    );
}

// ---------------------------------------------------------------------------
// Imported Numbers
// ---------------------------------------------------------------------------

#[test]
fn test_small_imported_numbers_create() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .imported_numbers()
        .create(&json!({
            "number": "+15551234567",
            "sip_username": "alice",
            "sip_password": "secret",
            "sip_proxy": "sip.example.com",
        }))
        .expect("imported_numbers.create");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, "/api/relay/rest/imported_phone_numbers");
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("number").and_then(Value::as_str),
        Some("+15551234567")
    );
    assert_eq!(
        sent.get("sip_username").and_then(Value::as_str),
        Some("alice")
    );
    assert_eq!(
        sent.get("sip_proxy").and_then(Value::as_str),
        Some("sip.example.com")
    );
}

// ---------------------------------------------------------------------------
// MFA
// ---------------------------------------------------------------------------

#[test]
fn test_small_mfa_call() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .mfa()
        .call(&json!({
            "to": "+15551234567",
            "from_": "+15559876543",
            "message": "Your code is {code}",
        }))
        .expect("mfa.call");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, "/api/relay/rest/mfa/call");
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("to").and_then(Value::as_str),
        Some("+15551234567")
    );
    assert_eq!(
        sent.get("from_").and_then(Value::as_str),
        Some("+15559876543")
    );
    assert_eq!(
        sent.get("message").and_then(Value::as_str),
        Some("Your code is {code}")
    );
}

// ---------------------------------------------------------------------------
// SIP Profile
// ---------------------------------------------------------------------------

#[test]
fn test_small_sip_profile_update() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .sip_profile()
        .update(&json!({
            "domain": "myco.sip.signalwire.com",
            "default_codecs": ["PCMU", "PCMA"],
        }))
        .expect("sip_profile.update");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(
        obj.contains_key("domain") || obj.contains_key("default_codecs"),
        "expected domain or default_codecs, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "PUT");
    assert_eq!(entry.path, "/api/relay/rest/sip_profile");
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("domain").and_then(Value::as_str),
        Some("myco.sip.signalwire.com")
    );
    let codecs = sent
        .get("default_codecs")
        .and_then(Value::as_array)
        .expect("default_codecs array");
    let codec_strs: Vec<&str> = codecs.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(codec_strs, vec!["PCMU", "PCMA"]);
}

// ---------------------------------------------------------------------------
// Number Groups
// ---------------------------------------------------------------------------

#[test]
fn test_small_number_groups_list_memberships() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .list_memberships("ng-1", &json!({"page_size": "10"}))
        .expect("list_memberships");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"));
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        "/api/relay/rest/number_groups/ng-1/number_group_memberships"
    );
    assert_eq!(
        entry
            .query_params
            .get("page_size")
            .expect("page_size missing"),
        &vec!["10".to_string()]
    );
}

#[test]
fn test_small_number_groups_delete_membership() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .number_groups()
        .delete_membership("mem-1")
        .expect("delete_membership");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, "/api/relay/rest/number_group_memberships/mem-1");
    let status = entry.response_status.expect("response_status");
    assert!(
        matches!(status, 200 | 202 | 204),
        "response_status = {status}"
    );
}

// ---------------------------------------------------------------------------
// Project tokens
// ---------------------------------------------------------------------------

#[test]
fn test_small_project_tokens_update() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .project()
        .tokens()
        .update("tok-1", &json!({"name": "renamed-token"}))
        .expect("project.tokens.update");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "PATCH");
    assert_eq!(entry.path, "/api/project/tokens/tok-1");
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("name").and_then(Value::as_str),
        Some("renamed-token")
    );
}

#[test]
fn test_small_project_tokens_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .project()
        .tokens()
        .delete("tok-1")
        .expect("project.tokens.delete");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, "/api/project/tokens/tok-1");
    let status = entry.response_status.expect("response_status");
    assert!(
        matches!(status, 200 | 202 | 204),
        "response_status = {status}"
    );
}

// ---------------------------------------------------------------------------
// Datasphere
// ---------------------------------------------------------------------------

#[test]
fn test_small_datasphere_get_chunk() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .datasphere()
        .documents()
        .get_chunk("doc-1", "chunk-99")
        .expect("get_chunk");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("id"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/datasphere/documents/doc-1/chunks/chunk-99");
}

// ---------------------------------------------------------------------------
// Queues
// ---------------------------------------------------------------------------

#[test]
fn test_small_queues_get_member() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .queues()
        .get_member("q-1", "mem-7")
        .expect("queues.get_member");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(
        obj.contains_key("queue_id") || obj.contains_key("call_id"),
        "expected queue_id or call_id, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/relay/rest/queues/q-1/members/mem-7");
}
