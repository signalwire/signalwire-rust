// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_compat_messages_faxes.py.

#[path = "common/mod.rs"]
mod common;

use serde_json::json;

// ---------------------------------------------------------------------------
// CompatMessages::update
// ---------------------------------------------------------------------------

#[test]
fn test_compat_messages_update_returns_message_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .messages()
        .update("MM_TEST", &json!({"Body": "updated body"}))
        .expect("messages.update");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("body") || obj.contains_key("sid"),
        "expected body or sid in response, got keys {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Messages/MM_TEST"
    );
}

#[test]
fn test_compat_messages_update_journal_records_post_to_message() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .messages()
        .update("MM_U1", &json!({"Body": "x", "Status": "canceled"}))
        .expect("messages.update");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Messages/MM_U1"
    );
    let body = entry.body_object().expect("body object");
    assert_eq!(body.get("Body").and_then(|v| v.as_str()), Some("x"));
    assert_eq!(
        body.get("Status").and_then(|v| v.as_str()),
        Some("canceled")
    );
}

// ---------------------------------------------------------------------------
// CompatMessages::get_media
// ---------------------------------------------------------------------------

#[test]
fn test_compat_messages_get_media_returns_media_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .messages()
        .get_media("MM_GM", "ME_GM")
        .expect("get_media");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("content_type") || obj.contains_key("sid"),
        "expected content_type or sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Messages/MM_GM/Media/ME_GM"
    );
}

#[test]
fn test_compat_messages_get_media_journal_records_get_to_media_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .messages()
        .get_media("MM_X", "ME_X")
        .expect("get_media");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Messages/MM_X/Media/ME_X"
    );
}

// ---------------------------------------------------------------------------
// CompatMessages::delete_media
// ---------------------------------------------------------------------------

#[test]
fn test_compat_messages_delete_media_no_exception_on_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .messages()
        .delete_media("MM_DM", "ME_DM")
        .expect("delete_media");
    assert!(result.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Messages/MM_DM/Media/ME_DM"
    );
}

#[test]
fn test_compat_messages_delete_media_journal_records_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .messages()
        .delete_media("MM_D", "ME_D")
        .expect("delete_media");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Messages/MM_D/Media/ME_D"
    );
}

// ---------------------------------------------------------------------------
// CompatFaxes::update
// ---------------------------------------------------------------------------

#[test]
fn test_compat_faxes_update_returns_fax_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .faxes()
        .update("FX_U", &json!({"Status": "canceled"}))
        .expect("faxes.update");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("status") || obj.contains_key("direction"),
        "expected status or direction, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Faxes/FX_U"
    );
}

#[test]
fn test_compat_faxes_update_journal_records_post_with_status() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .faxes()
        .update("FX_U2", &json!({"Status": "canceled"}))
        .expect("faxes.update");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Faxes/FX_U2"
    );
    let body = entry.body_object().expect("body object");
    assert_eq!(
        body.get("Status").and_then(|v| v.as_str()),
        Some("canceled")
    );
}

// ---------------------------------------------------------------------------
// CompatFaxes::list_media
// ---------------------------------------------------------------------------

#[test]
fn test_compat_faxes_list_media_returns_paginated_list() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .faxes()
        .list_media("FX_LM", &json!({}))
        .expect("list_media");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("media") || obj.contains_key("fax_media"),
        "expected 'media' or 'fax_media' key, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Faxes/FX_LM/Media"
    );
}

#[test]
fn test_compat_faxes_list_media_journal_records_get_to_fax_media() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .faxes()
        .list_media("FX_LM_X", &json!({}))
        .expect("list_media");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Faxes/FX_LM_X/Media"
    );
}

// ---------------------------------------------------------------------------
// CompatFaxes::get_media
// ---------------------------------------------------------------------------

#[test]
fn test_compat_faxes_get_media_returns_fax_media_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .faxes()
        .get_media("FX_GM", "ME_GM")
        .expect("get_media");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("content_type") || obj.contains_key("sid"),
        "expected content_type or sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Faxes/FX_GM/Media/ME_GM"
    );
}

#[test]
fn test_compat_faxes_get_media_journal_records_get_to_specific_media() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .faxes()
        .get_media("FX_G", "ME_G")
        .expect("get_media");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Faxes/FX_G/Media/ME_G"
    );
}

// ---------------------------------------------------------------------------
// CompatFaxes::delete_media
// ---------------------------------------------------------------------------

#[test]
fn test_compat_faxes_delete_media_no_exception_on_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .faxes()
        .delete_media("FX_DM", "ME_DM")
        .expect("delete_media");
    assert!(result.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Faxes/FX_DM/Media/ME_DM"
    );
}

#[test]
fn test_compat_faxes_delete_media_journal_records_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .faxes()
        .delete_media("FX_D", "ME_D")
        .expect("delete_media");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(
        entry.path,
        "/api/laml/2010-04-01/Accounts/test_proj/Faxes/FX_D/Media/ME_D"
    );
}
