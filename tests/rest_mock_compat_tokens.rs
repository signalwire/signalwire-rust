// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_compat_tokens.py.
//
// Covers CompatTokens::create / update (PATCH) / delete.

#[path = "common/mod.rs"]
mod common;

use serde_json::{Value, json};

fn base() -> String {
    common::mocktest::account_path("tokens")
}

// ---------------------------------------------------------------------------
// create
// ---------------------------------------------------------------------------

#[test]
fn test_compat_tokens_create_returns_token_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .tokens()
        .create(&json!({"Ttl": 3600}))
        .expect("tokens.create");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("token") || obj.contains_key("id"),
        "expected token or id, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_tokens_create_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .tokens()
        .create(&json!({"Ttl": 3600, "Name": "api-key"}))
        .expect("tokens.create");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, base());
    let body = entry.body_object().expect("body");
    assert_eq!(body.get("Ttl").and_then(Value::as_i64), Some(3600));
    assert_eq!(body.get("Name").and_then(Value::as_str), Some("api-key"));
}

// ---------------------------------------------------------------------------
// update (PATCH)
// ---------------------------------------------------------------------------

#[test]
fn test_compat_tokens_update_returns_token_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .tokens()
        .update("TK_U", &json!({"Ttl": 7200}))
        .expect("tokens.update");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("token") || obj.contains_key("id"),
        "expected token or id, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_tokens_update_journal_records_patch() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .tokens()
        .update("TK_UU", &json!({"Ttl": 7200}))
        .expect("tokens.update");

    let entry = common::mocktest::journal_last();
    // CompatTokens.update uses PATCH (BaseResource semantics).
    assert_eq!(entry.method, "PATCH");
    assert_eq!(entry.path, format!("{}/TK_UU", base()));
    let body = entry.body_object().expect("body");
    assert_eq!(body.get("Ttl").and_then(Value::as_i64), Some(7200));
}

// ---------------------------------------------------------------------------
// delete
// ---------------------------------------------------------------------------

#[test]
fn test_compat_tokens_delete_no_exception() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c.compat().tokens().delete("TK_D").expect("tokens.delete");
    assert!(result.is_object());
}

#[test]
fn test_compat_tokens_delete_journal_records_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().tokens().delete("TK_DEL").expect("tokens.delete");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, format!("{}/TK_DEL", base()));
}
