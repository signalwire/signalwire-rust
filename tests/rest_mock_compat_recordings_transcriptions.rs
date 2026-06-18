// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_compat_recordings_transcriptions.py.
//
// Covers CompatRecordings and CompatTranscriptions list/get/delete.

#[path = "common/mod.rs"]
mod common;

use serde_json::json;

fn rec_base() -> String {
    common::mocktest::account_path("Recordings")
}

fn tr_base() -> String {
    common::mocktest::account_path("Transcriptions")
}

// ---------------------------------------------------------------------------
// CompatRecordings::list
// ---------------------------------------------------------------------------

#[test]
fn test_compat_recordings_list_returns_paginated() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .recordings()
        .list(&json!({}))
        .expect("recordings.list");
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
fn test_compat_recordings_list_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .recordings()
        .list(&json!({}))
        .expect("recordings.list");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, rec_base());
}

// ---------------------------------------------------------------------------
// CompatRecordings::get
// ---------------------------------------------------------------------------

#[test]
fn test_compat_recordings_get_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .recordings()
        .get("RE_TEST")
        .expect("recordings.get");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("sid") || obj.contains_key("call_sid"),
        "expected sid or call_sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_recordings_get_journal_records_get_with_sid() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .recordings()
        .get("RE_GET")
        .expect("recordings.get");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{}/RE_GET", rec_base()));
}

// ---------------------------------------------------------------------------
// CompatRecordings::delete
// ---------------------------------------------------------------------------

#[test]
fn test_compat_recordings_delete_no_exception() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .recordings()
        .delete("RE_D")
        .expect("recordings.delete");
    assert!(result.is_object());
}

#[test]
fn test_compat_recordings_delete_journal_records_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .recordings()
        .delete("RE_DEL")
        .expect("recordings.delete");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, format!("{}/RE_DEL", rec_base()));
}

// ---------------------------------------------------------------------------
// CompatTranscriptions::list
// ---------------------------------------------------------------------------

#[test]
fn test_compat_transcriptions_list_returns_paginated() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .transcriptions()
        .list(&json!({}))
        .expect("transcriptions.list");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("transcriptions"),
        "expected 'transcriptions' key, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(obj.get("transcriptions").unwrap().is_array());
}

#[test]
fn test_compat_transcriptions_list_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .transcriptions()
        .list(&json!({}))
        .expect("transcriptions.list");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, tr_base());
}

// ---------------------------------------------------------------------------
// CompatTranscriptions::get
// ---------------------------------------------------------------------------

#[test]
fn test_compat_transcriptions_get_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .transcriptions()
        .get("TR_TEST")
        .expect("transcriptions.get");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("sid") || obj.contains_key("duration"),
        "expected sid or duration, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_transcriptions_get_journal_records_get_with_sid() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .transcriptions()
        .get("TR_GET")
        .expect("transcriptions.get");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, format!("{}/TR_GET", tr_base()));
}

// ---------------------------------------------------------------------------
// CompatTranscriptions::delete
// ---------------------------------------------------------------------------

#[test]
fn test_compat_transcriptions_delete_no_exception() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .transcriptions()
        .delete("TR_D")
        .expect("transcriptions.delete");
    assert!(result.is_object());
}

#[test]
fn test_compat_transcriptions_delete_journal_records_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .transcriptions()
        .delete("TR_DEL")
        .expect("transcriptions.delete");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, format!("{}/TR_DEL", tr_base()));
}
