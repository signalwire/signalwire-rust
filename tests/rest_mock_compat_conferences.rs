// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_compat_conferences.py.
//
// Covers the full Conferences surface: list/get/update on the conference
// itself plus participant, recording, and stream sub-resources.

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};

const BASE: &str = "/api/laml/2010-04-01/Accounts/test_proj/Conferences";

// ---- Conference itself ---------------------------------------------------

#[test]
fn test_compat_conferences_list_returns_paginated_list() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .list(&json!({}))
        .expect("conferences.list");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("conferences"),
        "expected 'conferences' key, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(
        obj.get("conferences").unwrap().is_array(),
        "expected 'conferences' array"
    );
    assert!(
        obj.get("page").map(|v| v.is_number()).unwrap_or(false),
        "expected numeric 'page'"
    );
}

#[test]
fn test_compat_conferences_list_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .list(&json!({}))
        .expect("conferences.list");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, BASE);
    assert!(
        entry.matched_route.is_some(),
        "spec gap: conferences.list"
    );
}

#[test]
fn test_compat_conferences_get_returns_conference_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .get("CF_TEST")
        .expect("conferences.get");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("friendly_name") || obj.contains_key("status"),
        "expected friendly_name or status, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_conferences_get_journal_records_get_with_sid() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .get("CF_GETSID")
        .expect("conferences.get");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{BASE}/CF_GETSID"));
}

#[test]
fn test_compat_conferences_update_returns_updated_conference() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .update("CF_X", &json!({"Status": "completed"}))
        .expect("conferences.update");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("friendly_name") || obj.contains_key("status"),
        "expected friendly_name or status"
    );
}

#[test]
fn test_compat_conferences_update_journal_records_post_with_status() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .update(
            "CF_UPD",
            &json!({"Status": "completed", "AnnounceUrl": "https://a.b"}),
        )
        .expect("conferences.update");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, format!("{BASE}/CF_UPD"));
    let body = entry.body_object().expect("body object");
    assert_eq!(
        body.get("Status").and_then(Value::as_str),
        Some("completed")
    );
    assert_eq!(
        body.get("AnnounceUrl").and_then(Value::as_str),
        Some("https://a.b")
    );
}

// ---- Participants --------------------------------------------------------

#[test]
fn test_compat_conferences_get_participant_returns_participant() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .get_participant("CF_P", "CA_P")
        .expect("get_participant");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("call_sid") || obj.contains_key("conference_sid"),
        "expected call_sid or conference_sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_conferences_get_participant_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .get_participant("CF_GP", "CA_GP")
        .expect("get_participant");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        format!("{BASE}/CF_GP/Participants/CA_GP")
    );
}

#[test]
fn test_compat_conferences_update_participant_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .update_participant("CF_UP", "CA_UP", &json!({"Muted": true}))
        .expect("update_participant");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("call_sid") || obj.contains_key("conference_sid"),
        "expected call_sid or conference_sid"
    );
}

#[test]
fn test_compat_conferences_update_participant_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .update_participant(
            "CF_M",
            "CA_M",
            &json!({"Muted": true, "Hold": false}),
        )
        .expect("update_participant");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        format!("{BASE}/CF_M/Participants/CA_M")
    );
    let body = entry.body_object().expect("body");
    assert_eq!(body.get("Muted").and_then(Value::as_bool), Some(true));
    assert_eq!(body.get("Hold").and_then(Value::as_bool), Some(false));
}

#[test]
fn test_compat_conferences_remove_participant_returns_dict() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .remove_participant("CF_R", "CA_R")
        .expect("remove_participant");
    assert!(result.is_object());
}

#[test]
fn test_compat_conferences_remove_participant_journal_records_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .remove_participant("CF_RM", "CA_RM")
        .expect("remove_participant");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(
        entry.path,
        format!("{BASE}/CF_RM/Participants/CA_RM")
    );
}

// ---- Recordings ----------------------------------------------------------

#[test]
fn test_compat_conferences_list_recordings_returns_paginated() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .list_recordings("CF_LR", &json!({}))
        .expect("list_recordings");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("recordings"),
        "expected 'recordings' key, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(obj.get("recordings").unwrap().is_array());
}

#[test]
fn test_compat_conferences_list_recordings_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .list_recordings("CF_LRX", &json!({}))
        .expect("list_recordings");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{BASE}/CF_LRX/Recordings"));
}

#[test]
fn test_compat_conferences_get_recording_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .get_recording("CF_GR", "RE_GR")
        .expect("get_recording");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("sid") || obj.contains_key("call_sid"),
        "expected sid or call_sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_conferences_get_recording_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .get_recording("CF_GRX", "RE_GRX")
        .expect("get_recording");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        format!("{BASE}/CF_GRX/Recordings/RE_GRX")
    );
}

#[test]
fn test_compat_conferences_update_recording_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .update_recording(
            "CF_URC",
            "RE_URC",
            &json!({"Status": "paused"}),
        )
        .expect("update_recording");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("sid") || obj.contains_key("status"),
        "expected sid or status, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_conferences_update_recording_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .update_recording("CF_UR", "RE_UR", &json!({"Status": "paused"}))
        .expect("update_recording");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        format!("{BASE}/CF_UR/Recordings/RE_UR")
    );
    let body = entry.body_object().expect("body");
    assert_eq!(
        body.get("Status").and_then(Value::as_str),
        Some("paused")
    );
}

#[test]
fn test_compat_conferences_delete_recording_returns_dict() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .delete_recording("CF_DR", "RE_DR")
        .expect("delete_recording");
    assert!(result.is_object());
}

#[test]
fn test_compat_conferences_delete_recording_journal_records_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .delete_recording("CF_DRX", "RE_DRX")
        .expect("delete_recording");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(
        entry.path,
        format!("{BASE}/CF_DRX/Recordings/RE_DRX")
    );
}

// ---- Streams -------------------------------------------------------------

#[test]
fn test_compat_conferences_start_stream_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .start_stream("CF_SS", &json!({"Url": "wss://a.b/s"}))
        .expect("start_stream");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("sid") || obj.contains_key("name"),
        "expected sid or name, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_conferences_start_stream_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .start_stream(
            "CF_SSX",
            &json!({"Url": "wss://a.b/s", "Name": "strm"}),
        )
        .expect("start_stream");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, format!("{BASE}/CF_SSX/Streams"));
    let body = entry.body_object().expect("body");
    assert_eq!(
        body.get("Url").and_then(Value::as_str),
        Some("wss://a.b/s")
    );
}

#[test]
fn test_compat_conferences_stop_stream_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .conferences()
        .stop_stream("CF_TS", "ST_TS", &json!({"Status": "stopped"}))
        .expect("stop_stream");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("sid") || obj.contains_key("status"),
        "expected sid or status, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_conferences_stop_stream_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .stop_stream("CF_TSX", "ST_TSX", &json!({"Status": "stopped"}))
        .expect("stop_stream");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        format!("{BASE}/CF_TSX/Streams/ST_TSX")
    );
    let body = entry.body_object().expect("body");
    assert_eq!(
        body.get("Status").and_then(Value::as_str),
        Some("stopped")
    );
}
