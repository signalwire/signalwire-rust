// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_compat_accounts.py.
//
// Drives `client.compat().accounts().*` against the live mock_signalwire
// HTTP server and asserts on both the SDK return value and the journal.

#[path = "common/mod.rs"]
mod common;

use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// CompatAccounts::create — POST /api/laml/2010-04-01/Accounts
// ---------------------------------------------------------------------------

#[test]
fn test_compat_accounts_create_returns_account_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .accounts()
        .create(&json!({"FriendlyName": "Sub-A"}))
        .expect("accounts.create");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("friendly_name"),
        "expected 'friendly_name' in {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_accounts_create_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .accounts()
        .create(&json!({"FriendlyName": "Sub-B"}))
        .expect("accounts.create");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    // Accounts.create lives at the top-level Accounts collection — no
    // AccountSid prefix.
    assert_eq!(entry.path, "/api/laml/2010-04-01/Accounts");
    let body = entry.body_object().expect("body object");
    assert_eq!(
        body.get("FriendlyName").and_then(Value::as_str),
        Some("Sub-B")
    );
    let status = entry.response_status.expect("response_status");
    assert!((200..400).contains(&status), "response_status = {status}");
}

// ---------------------------------------------------------------------------
// CompatAccounts::get — GET /api/laml/2010-04-01/Accounts/{sid}
// ---------------------------------------------------------------------------

#[test]
fn test_compat_accounts_get_returns_account_for_sid() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c.compat().accounts().get("AC123").expect("accounts.get");
    assert!(result.is_object());
    assert!(
        result.as_object().unwrap().contains_key("friendly_name"),
        "expected 'friendly_name'"
    );
}

#[test]
fn test_compat_accounts_get_journal_records_get_with_sid() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .accounts()
        .get("AC_SAMPLE_SID")
        .expect("accounts.get");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/laml/2010-04-01/Accounts/AC_SAMPLE_SID");
    // GET should not carry a body.
    assert!(
        entry.body.is_null() || matches!(&entry.body, Value::Object(o) if o.is_empty()),
        "GET should have no body, got {:?}",
        entry.body
    );
    assert!(entry.matched_route.is_some(), "spec gap: account-get");
}

// ---------------------------------------------------------------------------
// CompatAccounts::update — POST /api/laml/2010-04-01/Accounts/{sid}
// ---------------------------------------------------------------------------

#[test]
fn test_compat_accounts_update_returns_updated_account() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .accounts()
        .update("AC123", &json!({"FriendlyName": "Renamed"}))
        .expect("accounts.update");
    assert!(result.is_object());
    assert!(
        result.as_object().unwrap().contains_key("friendly_name"),
        "expected 'friendly_name'"
    );
}

#[test]
fn test_compat_accounts_update_journal_records_post_with_sid() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .accounts()
        .update("AC_X", &json!({"FriendlyName": "NewName"}))
        .expect("accounts.update");

    let entry = common::mocktest::journal_last();
    // Twilio-compat update is POST (not PATCH/PUT).
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, "/api/laml/2010-04-01/Accounts/AC_X");
    let body = entry.body_object().expect("body object");
    assert_eq!(
        body.get("FriendlyName").and_then(Value::as_str),
        Some("NewName")
    );
}
