// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_compat_misc.py.
//
// Covers single-method gaps:
//   - CompatApplications::update
//   - CompatLamlBins::update

#[path = "common/mod.rs"]
mod common;

use serde_json::{Value, json};

const APP_BASE: &str = "/api/laml/2010-04-01/Accounts/test_proj/Applications";
const BIN_BASE: &str = "/api/laml/2010-04-01/Accounts/test_proj/LamlBins";

// ---------------------------------------------------------------------------
// CompatApplications::update
// ---------------------------------------------------------------------------

#[test]
fn test_compat_applications_update_returns_application_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .applications()
        .update("AP_U", &json!({"FriendlyName": "updated"}))
        .expect("applications.update");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("friendly_name") || obj.contains_key("sid"),
        "expected friendly_name or sid, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_applications_update_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .applications()
        .update(
            "AP_UU",
            &json!({"FriendlyName": "renamed", "VoiceUrl": "https://a.b/v"}),
        )
        .expect("applications.update");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, format!("{APP_BASE}/AP_UU"));
    let body = entry.body_object().expect("body");
    assert_eq!(
        body.get("FriendlyName").and_then(Value::as_str),
        Some("renamed")
    );
    assert_eq!(
        body.get("VoiceUrl").and_then(Value::as_str),
        Some("https://a.b/v")
    );
}

// ---------------------------------------------------------------------------
// CompatLamlBins::update
// ---------------------------------------------------------------------------

#[test]
fn test_compat_laml_bins_update_returns_bin_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let result = c
        .compat()
        .laml_bins()
        .update("LB_U", &json!({"FriendlyName": "updated"}))
        .expect("laml_bins.update");
    assert!(result.is_object());
    let obj = result.as_object().unwrap();
    assert!(
        obj.contains_key("friendly_name")
            || obj.contains_key("sid")
            || obj.contains_key("contents"),
        "expected friendly_name/sid/contents, got {:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_compat_laml_bins_update_journal_records_post() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .laml_bins()
        .update(
            "LB_UU",
            &json!({"FriendlyName": "renamed", "Contents": "<Response/>"}),
        )
        .expect("laml_bins.update");

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, format!("{BIN_BASE}/LB_UU"));
    let body = entry.body_object().expect("body");
    assert_eq!(
        body.get("FriendlyName").and_then(Value::as_str),
        Some("renamed")
    );
    assert_eq!(
        body.get("Contents").and_then(Value::as_str),
        Some("<Response/>")
    );
}
