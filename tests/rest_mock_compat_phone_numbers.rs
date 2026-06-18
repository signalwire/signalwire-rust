// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_compat_phone_numbers.py.

#[path = "common/mod.rs"]
mod common;

use serde_json::json;

// ---------------------------------------------------------------------------
// CompatPhoneNumbers::list
// ---------------------------------------------------------------------------

#[test]
fn test_compat_phone_numbers_list_returns_paginated_list() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .phone_numbers()
        .list(&serde_json::json!({}))
        .expect("phone_numbers.list");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("incoming_phone_numbers"),
        "expected 'incoming_phone_numbers' key, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(
        obj.get("incoming_phone_numbers").unwrap().is_array(),
        "expected list value"
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers")
    );
}

#[test]
fn test_compat_phone_numbers_list_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .list(&serde_json::json!({}))
        .expect("phone_numbers.list");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers")
    );
}

// ---------------------------------------------------------------------------
// CompatPhoneNumbers::get
// ---------------------------------------------------------------------------

#[test]
fn test_compat_phone_numbers_get_returns_phone_number_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .phone_numbers()
        .get("PN_TEST")
        .expect("phone_numbers.get");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("phone_number") || obj.contains_key("sid"),
        "expected phone_number or sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers/PN_TEST")
    );
}

#[test]
fn test_compat_phone_numbers_get_journal_records_get_with_sid() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .get("PN_GET")
        .expect("phone_numbers.get");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers/PN_GET")
    );
}

// ---------------------------------------------------------------------------
// CompatPhoneNumbers::update
// ---------------------------------------------------------------------------

#[test]
fn test_compat_phone_numbers_update_returns_phone_number_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .phone_numbers()
        .update("PN_U", &json!({"FriendlyName": "updated"}))
        .expect("phone_numbers.update");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("phone_number") || obj.contains_key("sid"),
        "expected phone_number or sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers/PN_U")
    );
}

#[test]
fn test_compat_phone_numbers_update_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .update(
            "PN_UU",
            &json!({"FriendlyName": "updated", "VoiceUrl": "https://a.b/v"}),
        )
        .expect("phone_numbers.update");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers/PN_UU")
    );
    let body = entry.body_object().expect("body object");
    assert_eq!(
        body.get("FriendlyName").and_then(|v| v.as_str()),
        Some("updated")
    );
    assert_eq!(
        body.get("VoiceUrl").and_then(|v| v.as_str()),
        Some("https://a.b/v")
    );
}

// ---------------------------------------------------------------------------
// CompatPhoneNumbers::delete
// ---------------------------------------------------------------------------

#[test]
fn test_compat_phone_numbers_delete_no_exception() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .phone_numbers()
        .delete("PN_D")
        .expect("phone_numbers.delete");
    assert!(result.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers/PN_D")
    );
}

#[test]
fn test_compat_phone_numbers_delete_journal_records_delete() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .delete("PN_DEL")
        .expect("phone_numbers.delete");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers/PN_DEL")
    );
}

// ---------------------------------------------------------------------------
// CompatPhoneNumbers::purchase
// ---------------------------------------------------------------------------

#[test]
fn test_compat_phone_numbers_purchase_returns_purchased_number() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .phone_numbers()
        .purchase(&json!({"PhoneNumber": "+15555550100"}))
        .expect("phone_numbers.purchase");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("phone_number") || obj.contains_key("sid"),
        "expected phone_number or sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers")
    );
}

#[test]
fn test_compat_phone_numbers_purchase_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .purchase(&json!({
            "PhoneNumber": "+15555550100",
            "FriendlyName": "Main",
        }))
        .expect("phone_numbers.purchase");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("IncomingPhoneNumbers")
    );
    let body = entry.body_object().expect("body object");
    assert_eq!(
        body.get("PhoneNumber").and_then(|v| v.as_str()),
        Some("+15555550100")
    );
    assert_eq!(
        body.get("FriendlyName").and_then(|v| v.as_str()),
        Some("Main")
    );
}

// ---------------------------------------------------------------------------
// CompatPhoneNumbers::import_number
// ---------------------------------------------------------------------------

#[test]
fn test_compat_phone_numbers_import_number_returns_imported() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .phone_numbers()
        .import_number(&json!({"PhoneNumber": "+15555550111"}))
        .expect("phone_numbers.import_number");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("phone_number") || obj.contains_key("sid"),
        "expected phone_number or sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    // Note: ImportedPhoneNumbers (not Incoming).
    assert_eq!(
        entry.path,
        common::mocktest::account_path("ImportedPhoneNumbers")
    );
}

#[test]
fn test_compat_phone_numbers_import_number_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .import_number(&json!({
            "PhoneNumber": "+15555550111",
            "VoiceUrl": "https://a.b/v",
        }))
        .expect("phone_numbers.import_number");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("ImportedPhoneNumbers")
    );
    let body = entry.body_object().expect("body object");
    assert_eq!(
        body.get("PhoneNumber").and_then(|v| v.as_str()),
        Some("+15555550111")
    );
}

// ---------------------------------------------------------------------------
// CompatPhoneNumbers::list_available_countries
// ---------------------------------------------------------------------------

#[test]
fn test_compat_phone_numbers_list_available_countries_returns_collection() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .phone_numbers()
        .list_available_countries(&json!({}))
        .expect("list_available_countries");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("countries"),
        "expected 'countries' key, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(
        obj.get("countries").unwrap().is_array(),
        "expected list value"
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("AvailablePhoneNumbers")
    );
}

#[test]
fn test_compat_phone_numbers_list_available_countries_journal_records_get() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .list_available_countries(&json!({}))
        .expect("list_available_countries");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("AvailablePhoneNumbers")
    );
}

// ---------------------------------------------------------------------------
// CompatPhoneNumbers::search_toll_free
// ---------------------------------------------------------------------------

#[test]
fn test_compat_phone_numbers_search_toll_free_returns_available_numbers() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .phone_numbers()
        .search_toll_free("US", &json!({"AreaCode": "800"}))
        .expect("search_toll_free");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("available_phone_numbers"),
        "expected 'available_phone_numbers' key, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(
        obj.get("available_phone_numbers").unwrap().is_array(),
        "expected list value"
    );

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("AvailablePhoneNumbers/US/TollFree")
    );
}

#[test]
fn test_compat_phone_numbers_search_toll_free_journal_records_get_with_query() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .search_toll_free("US", &json!({"AreaCode": "888"}))
        .expect("search_toll_free");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        common::mocktest::account_path("AvailablePhoneNumbers/US/TollFree")
    );
    // AreaCode goes on the query string.
    let area_code = entry
        .query_params
        .get("AreaCode")
        .expect("AreaCode query param missing");
    assert_eq!(area_code.as_slice(), &["888".to_string()]);
}
