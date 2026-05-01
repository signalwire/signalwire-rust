// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_compat_queues.py.
//
// Covers CompatQueues::update, list_members, get_member, dequeue_member.

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};

const BASE: &str = "/api/laml/2010-04-01/Accounts/test_proj/Queues";

// ---------------------------------------------------------------------------
// update
// ---------------------------------------------------------------------------

#[test]
fn test_compat_queues_update_returns_queue_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .queues()
        .update("QU_U", &json!({"FriendlyName": "updated"}))
        .expect("queues.update");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("friendly_name") || obj.contains_key("sid"),
        "expected friendly_name or sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_queues_update_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .queues()
        .update(
            "QU_UU",
            &json!({"FriendlyName": "renamed", "MaxSize": 200}),
        )
        .expect("queues.update");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, format!("{BASE}/QU_UU"));
    let body = entry.body_object().expect("body");
    assert_eq!(
        body.get("FriendlyName").and_then(Value::as_str),
        Some("renamed")
    );
    assert_eq!(body.get("MaxSize").and_then(Value::as_i64), Some(200));
}

// ---------------------------------------------------------------------------
// list_members
// ---------------------------------------------------------------------------

#[test]
fn test_compat_queues_list_members_returns_paginated() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .queues()
        .list_members("QU_LM", &json!({}))
        .expect("list_members");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("queue_members"),
        "expected 'queue_members', got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(obj.get("queue_members").unwrap().is_array());
}

#[test]
fn test_compat_queues_list_members_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .queues()
        .list_members("QU_LMX", &json!({}))
        .expect("list_members");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{BASE}/QU_LMX/Members"));
}

// ---------------------------------------------------------------------------
// get_member
// ---------------------------------------------------------------------------

#[test]
fn test_compat_queues_get_member_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .queues()
        .get_member("QU_GM", "CA_GM")
        .expect("get_member");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("call_sid") || obj.contains_key("queue_sid"),
        "expected call_sid or queue_sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_queues_get_member_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .queues()
        .get_member("QU_GMX", "CA_GMX")
        .expect("get_member");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        format!("{BASE}/QU_GMX/Members/CA_GMX")
    );
}

// ---------------------------------------------------------------------------
// dequeue_member
// ---------------------------------------------------------------------------

#[test]
fn test_compat_queues_dequeue_member_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .queues()
        .dequeue_member("QU_DM", "CA_DM", &json!({"Url": "https://a.b"}))
        .expect("dequeue_member");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("call_sid") || obj.contains_key("queue_sid"),
        "expected call_sid or queue_sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_queues_dequeue_member_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .queues()
        .dequeue_member(
            "QU_DMX",
            "CA_DMX",
            &json!({"Url": "https://a.b/url", "Method": "POST"}),
        )
        .expect("dequeue_member");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        format!("{BASE}/QU_DMX/Members/CA_DMX")
    );
    let body = entry.body_object().expect("body");
    assert_eq!(
        body.get("Url").and_then(Value::as_str),
        Some("https://a.b/url")
    );
    assert_eq!(
        body.get("Method").and_then(Value::as_str),
        Some("POST")
    );
}
