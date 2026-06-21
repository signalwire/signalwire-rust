// Full success (2xx) + error (4xx/5xx) REST coverage for the `compatibility`
// (Twilio-compatible LAML) spec group.
//
// Drives `client.compat().*` against the live `mock_signalwire` HTTP server and
// asserts on both the SDK return value (success) / error status (error) and the
// recorded journal entry (method, path, matched_route, response_status).
//
// Compat paths embed the account SID, which the mocktest harness mints per-test
// as a random project. Paths are therefore built with
// `common::mocktest::account_path(...)` rather than hard-coded.
//
// Coverage: 78 of the 79 `compatibility.*` routes. The only gap is
// `compatibility.list_available_phone_number_resources_by_country`
// (GET .../AvailablePhoneNumbers/{IsoCountry}) which has no SDK surface — the
// SDK only exposes the per-country `/Local` and `/TollFree` sub-collections.

#[path = "common/mod.rs"]
mod common;

use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Assert the last journal entry is a success on the expected method/path/route.
fn assert_ok(method: &str, path: &str, route: &str) {
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, method, "method for {route}");
    assert_eq!(e.path, path, "path for {route}");
    assert_eq!(
        e.matched_route.as_deref(),
        Some(route),
        "matched_route for {route}"
    );
    let status = e.response_status.expect("response_status");
    assert!(
        (200..400).contains(&status),
        "expected 2xx/3xx for {route}, got {status}"
    );
}

/// Assert the last journal entry recorded the staged error status on the route.
fn assert_err_journal(route: &str, status: i64) {
    let e = common::mocktest::journal_last();
    assert_eq!(e.matched_route.as_deref(), Some(route), "route for {route}");
    assert_eq!(
        e.response_status,
        Some(status),
        "response_status for {route}"
    );
}

// ===========================================================================
// Accounts collection (top-level — no AccountSid prefix)
//   list_accounts / create_subprojects / get_account / update_account
// ===========================================================================

const ACCOUNTS: &str = "/api/laml/2010-04-01/Accounts";

#[test]
fn test_compat_list_accounts_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.compat().accounts().list(&json!({})).expect("list");
    assert!(body.is_object());
    assert_ok("GET", ACCOUNTS, "compatibility.list_accounts");
}

#[test]
fn test_compat_list_accounts_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_accounts", 500, json!({"error": "boom"}));
    let err = c.compat().accounts().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_accounts", 500);
}

#[test]
fn test_compat_create_subprojects_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .compat()
        .accounts()
        .create(&json!({"FriendlyName": "Sub"}))
        .expect("create");
    assert!(body.is_object());
    assert_ok("POST", ACCOUNTS, "compatibility.create_subprojects");
}

#[test]
fn test_compat_create_subprojects_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.create_subprojects", 422, json!({"e": "v"}));
    let err = c.compat().accounts().create(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.create_subprojects", 422);
}

#[test]
fn test_compat_get_account_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.compat().accounts().get("AC123").expect("get");
    assert!(body.is_object());
    assert_ok(
        "GET",
        &format!("{ACCOUNTS}/AC123"),
        "compatibility.get_account",
    );
}

#[test]
fn test_compat_get_account_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.get_account", 404, json!({"error": "nf"}));
    let err = c.compat().accounts().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.get_account", 404);
}

#[test]
fn test_compat_update_account_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .compat()
        .accounts()
        .update("AC123", &json!({"FriendlyName": "New"}))
        .expect("update");
    assert!(body.is_object());
    assert_ok(
        "POST",
        &format!("{ACCOUNTS}/AC123"),
        "compatibility.update_account",
    );
}

#[test]
fn test_compat_update_account_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_account", 422, json!({"e": "v"}));
    let err = c
        .compat()
        .accounts()
        .update("AC123", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.update_account", 422);
}

// ===========================================================================
// Applications
// ===========================================================================

#[test]
fn test_compat_list_applications_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().applications().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Applications"),
        "compatibility.list_applications",
    );
}

#[test]
fn test_compat_list_applications_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_applications", 500, json!({"e": 1}));
    let err = c.compat().applications().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_applications", 500);
}

#[test]
fn test_compat_create_application_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .applications()
        .create(&json!({"FriendlyName": "App"}))
        .expect("create");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Applications"),
        "compatibility.create_application",
    );
}

#[test]
fn test_compat_create_application_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.create_application", 422, json!({"e": 1}));
    let err = c
        .compat()
        .applications()
        .create(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.create_application", 422);
}

#[test]
fn test_compat_get_application_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().applications().get("AP1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Applications/AP1"),
        "compatibility.get_application",
    );
}

#[test]
fn test_compat_get_application_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.get_application", 404, json!({"e": 1}));
    let err = c.compat().applications().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.get_application", 404);
}

#[test]
fn test_compat_update_application_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .applications()
        .update("AP1", &json!({"FriendlyName": "X"}))
        .expect("update");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Applications/AP1"),
        "compatibility.update_application",
    );
}

#[test]
fn test_compat_update_application_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_application", 404, json!({"e": 1}));
    let err = c
        .compat()
        .applications()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_application", 404);
}

#[test]
fn test_compat_delete_application_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().applications().delete("AP1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Applications/AP1"),
        "compatibility.delete_application",
    );
}

#[test]
fn test_compat_delete_application_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_application", 404, json!({"e": 1}));
    let err = c
        .compat()
        .applications()
        .delete("missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_application", 404);
}

// ===========================================================================
// Available phone numbers
//   list_available_phone_number_resources (AvailablePhoneNumbers)
//   search_local_available_phone_numbers (.../{country}/Local)
//   search_toll_free_available_phone_numbers (.../{country}/TollFree)
// (by_country .../{country} is the accepted gap — no SDK surface)
// ===========================================================================

#[test]
fn test_compat_list_available_phone_number_resources_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .list_available_countries(&json!({}))
        .expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("AvailablePhoneNumbers"),
        "compatibility.list_available_phone_number_resources",
    );
}

#[test]
fn test_compat_list_available_phone_number_resources_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.list_available_phone_number_resources",
        500,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .phone_numbers()
        .list_available_countries(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_available_phone_number_resources", 500);
}

#[test]
fn test_compat_search_local_available_phone_numbers_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .search_local("US", &json!({}))
        .expect("search_local");
    assert_ok(
        "GET",
        &common::mocktest::account_path("AvailablePhoneNumbers/US/Local"),
        "compatibility.search_local_available_phone_numbers",
    );
}

#[test]
fn test_compat_search_local_available_phone_numbers_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.search_local_available_phone_numbers",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .phone_numbers()
        .search_local("ZZ", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.search_local_available_phone_numbers", 404);
}

#[test]
fn test_compat_search_toll_free_available_phone_numbers_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .search_toll_free("US", &json!({}))
        .expect("search_toll_free");
    assert_ok(
        "GET",
        &common::mocktest::account_path("AvailablePhoneNumbers/US/TollFree"),
        "compatibility.search_toll_free_available_phone_numbers",
    );
}

#[test]
fn test_compat_search_toll_free_available_phone_numbers_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.search_toll_free_available_phone_numbers",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .phone_numbers()
        .search_toll_free("ZZ", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal(
        "compatibility.search_toll_free_available_phone_numbers",
        404,
    );
}

// ===========================================================================
// Calls (CRUD now covered by the real CompatCalls list/create/get/delete)
//   plus recordings / streams sub-collections
// ===========================================================================

#[test]
fn test_compat_list_all_calls_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().calls().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Calls"),
        "compatibility.list_all_calls",
    );
}

#[test]
fn test_compat_list_all_calls_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_all_calls", 500, json!({"e": 1}));
    let err = c.compat().calls().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_all_calls", 500);
}

#[test]
fn test_compat_create_a_call_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .calls()
        .create(&json!({"To": "+15555550000", "From": "+15555551111"}))
        .expect("create");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Calls"),
        "compatibility.create_a_call",
    );
}

#[test]
fn test_compat_create_a_call_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.create_a_call", 422, json!({"e": 1}));
    let err = c.compat().calls().create(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.create_a_call", 422);
}

#[test]
fn test_compat_retrieve_a_call_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().calls().get("CA1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Calls/CA1"),
        "compatibility.retrieve_a_call",
    );
}

#[test]
fn test_compat_retrieve_a_call_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_a_call", 404, json!({"e": 1}));
    let err = c.compat().calls().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_a_call", 404);
}

#[test]
fn test_compat_update_a_call_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .calls()
        .update("CA1", &json!({"Status": "completed"}))
        .expect("update");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Calls/CA1"),
        "compatibility.update_a_call",
    );
}

#[test]
fn test_compat_update_a_call_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_a_call", 404, json!({"e": 1}));
    let err = c
        .compat()
        .calls()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_a_call", 404);
}

#[test]
fn test_compat_delete_a_call_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().calls().delete("CA1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Calls/CA1"),
        "compatibility.delete_a_call",
    );
}

#[test]
fn test_compat_delete_a_call_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_a_call", 404, json!({"e": 1}));
    let err = c.compat().calls().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_a_call", 404);
}

#[test]
fn test_compat_create_recording_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .calls()
        .start_recording("CA1", &json!({}))
        .expect("start_recording");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Calls/CA1/Recordings"),
        "compatibility.create_recording",
    );
}

#[test]
fn test_compat_create_recording_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.create_recording", 404, json!({"e": 1}));
    let err = c
        .compat()
        .calls()
        .start_recording("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.create_recording", 404);
}

#[test]
fn test_compat_update_recording_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .calls()
        .update_recording("CA1", "RE1", &json!({"Status": "paused"}))
        .expect("update_recording");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Calls/CA1/Recordings/RE1"),
        "compatibility.update_recording",
    );
}

#[test]
fn test_compat_update_recording_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_recording", 404, json!({"e": 1}));
    let err = c
        .compat()
        .calls()
        .update_recording("CA1", "missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_recording", 404);
}

#[test]
fn test_compat_create_stream_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .calls()
        .start_stream("CA1", &json!({"Url": "wss://x"}))
        .expect("start_stream");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Calls/CA1/Streams"),
        "compatibility.create_stream",
    );
}

#[test]
fn test_compat_create_stream_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.create_stream", 404, json!({"e": 1}));
    let err = c
        .compat()
        .calls()
        .start_stream("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.create_stream", 404);
}

#[test]
fn test_compat_update_stream_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .calls()
        .stop_stream("CA1", "MZ1", &json!({"Status": "stopped"}))
        .expect("stop_stream");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Calls/CA1/Streams/MZ1"),
        "compatibility.update_stream",
    );
}

#[test]
fn test_compat_update_stream_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_stream", 404, json!({"e": 1}));
    let err = c
        .compat()
        .calls()
        .stop_stream("CA1", "missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_stream", 404);
}

// ===========================================================================
// Conferences (+ participants, recordings, streams)
// ===========================================================================

#[test]
fn test_compat_list_all_conferences_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().conferences().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Conferences"),
        "compatibility.list_all_conferences",
    );
}

#[test]
fn test_compat_list_all_conferences_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_all_conferences", 500, json!({"e": 1}));
    let err = c.compat().conferences().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_all_conferences", 500);
}

#[test]
fn test_compat_retrieve_conference_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().conferences().get("CF1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Conferences/CF1"),
        "compatibility.retrieve_conference",
    );
}

#[test]
fn test_compat_retrieve_conference_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_conference", 404, json!({"e": 1}));
    let err = c.compat().conferences().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_conference", 404);
}

#[test]
fn test_compat_update_conference_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .update("CF1", &json!({"Status": "completed"}))
        .expect("update");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Conferences/CF1"),
        "compatibility.update_conference",
    );
}

#[test]
fn test_compat_update_conference_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_conference", 404, json!({"e": 1}));
    let err = c
        .compat()
        .conferences()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_conference", 404);
}

#[test]
fn test_compat_list_all_participants_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .list_participants("CF1", &json!({}))
        .expect("list_participants");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Conferences/CF1/Participants"),
        "compatibility.list_all_participants",
    );
}

#[test]
fn test_compat_list_all_participants_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_all_participants", 404, json!({"e": 1}));
    let err = c
        .compat()
        .conferences()
        .list_participants("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.list_all_participants", 404);
}

#[test]
fn test_compat_retrieve_participant_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .get_participant("CF1", "CA1")
        .expect("get_participant");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Conferences/CF1/Participants/CA1"),
        "compatibility.retrieve_participant",
    );
}

#[test]
fn test_compat_retrieve_participant_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_participant", 404, json!({"e": 1}));
    let err = c
        .compat()
        .conferences()
        .get_participant("CF1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_participant", 404);
}

#[test]
fn test_compat_update_participant_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .update_participant("CF1", "CA1", &json!({"Muted": "true"}))
        .expect("update_participant");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Conferences/CF1/Participants/CA1"),
        "compatibility.update_participant",
    );
}

#[test]
fn test_compat_update_participant_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_participant", 404, json!({"e": 1}));
    let err = c
        .compat()
        .conferences()
        .update_participant("CF1", "missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_participant", 404);
}

#[test]
fn test_compat_delete_participant_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .remove_participant("CF1", "CA1")
        .expect("remove_participant");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Conferences/CF1/Participants/CA1"),
        "compatibility.delete_participant",
    );
}

#[test]
fn test_compat_delete_participant_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_participant", 404, json!({"e": 1}));
    let err = c
        .compat()
        .conferences()
        .remove_participant("CF1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_participant", 404);
}

#[test]
fn test_compat_list_conference_recordings_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .list_recordings("CF1", &json!({}))
        .expect("list_recordings");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Conferences/CF1/Recordings"),
        "compatibility.list_conference_recordings",
    );
}

#[test]
fn test_compat_list_conference_recordings_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.list_conference_recordings",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .conferences()
        .list_recordings("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.list_conference_recordings", 404);
}

#[test]
fn test_compat_get_conference_recording_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .get_recording("CF1", "RE1")
        .expect("get_recording");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Conferences/CF1/Recordings/RE1"),
        "compatibility.get_conference_recording",
    );
}

#[test]
fn test_compat_get_conference_recording_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.get_conference_recording",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .conferences()
        .get_recording("CF1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.get_conference_recording", 404);
}

#[test]
fn test_compat_update_conference_recording_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .update_recording("CF1", "RE1", &json!({"Status": "paused"}))
        .expect("update_recording");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Conferences/CF1/Recordings/RE1"),
        "compatibility.update_conference_recording",
    );
}

#[test]
fn test_compat_update_conference_recording_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.update_conference_recording",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .conferences()
        .update_recording("CF1", "missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_conference_recording", 404);
}

#[test]
fn test_compat_delete_conference_recording_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .delete_recording("CF1", "RE1")
        .expect("delete_recording");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Conferences/CF1/Recordings/RE1"),
        "compatibility.delete_conference_recording",
    );
}

#[test]
fn test_compat_delete_conference_recording_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.delete_conference_recording",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .conferences()
        .delete_recording("CF1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_conference_recording", 404);
}

#[test]
fn test_compat_create_conference_stream_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .start_stream("CF1", &json!({"Url": "wss://x"}))
        .expect("start_stream");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Conferences/CF1/Streams"),
        "compatibility.create_conference_stream",
    );
}

#[test]
fn test_compat_create_conference_stream_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.create_conference_stream",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .conferences()
        .start_stream("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.create_conference_stream", 404);
}

#[test]
fn test_compat_update_conference_stream_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .conferences()
        .stop_stream("CF1", "MZ1", &json!({"Status": "stopped"}))
        .expect("stop_stream");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Conferences/CF1/Streams/MZ1"),
        "compatibility.update_conference_stream",
    );
}

#[test]
fn test_compat_update_conference_stream_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.update_conference_stream",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .conferences()
        .stop_stream("CF1", "missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_conference_stream", 404);
}

// ===========================================================================
// Faxes (+ media)
// ===========================================================================

#[test]
fn test_compat_list_all_faxes_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().faxes().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Faxes"),
        "compatibility.list_all_faxes",
    );
}

#[test]
fn test_compat_list_all_faxes_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_all_faxes", 500, json!({"e": 1}));
    let err = c.compat().faxes().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_all_faxes", 500);
}

#[test]
fn test_compat_send_fax_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .faxes()
        .create(&json!({"To": "+15555550000", "From": "+15555551111"}))
        .expect("send_fax");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Faxes"),
        "compatibility.send_fax",
    );
}

#[test]
fn test_compat_send_fax_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.send_fax", 422, json!({"e": 1}));
    let err = c.compat().faxes().create(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.send_fax", 422);
}

#[test]
fn test_compat_retrieve_fax_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().faxes().get("FX1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Faxes/FX1"),
        "compatibility.retrieve_fax",
    );
}

#[test]
fn test_compat_retrieve_fax_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_fax", 404, json!({"e": 1}));
    let err = c.compat().faxes().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_fax", 404);
}

#[test]
fn test_compat_update_fax_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .faxes()
        .update("FX1", &json!({"Status": "canceled"}))
        .expect("update");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Faxes/FX1"),
        "compatibility.update_fax",
    );
}

#[test]
fn test_compat_update_fax_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_fax", 404, json!({"e": 1}));
    let err = c
        .compat()
        .faxes()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_fax", 404);
}

#[test]
fn test_compat_delete_fax_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().faxes().delete("FX1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Faxes/FX1"),
        "compatibility.delete_fax",
    );
}

#[test]
fn test_compat_delete_fax_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_fax", 404, json!({"e": 1}));
    let err = c.compat().faxes().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_fax", 404);
}

#[test]
fn test_compat_list_all_fax_media_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .faxes()
        .list_media("FX1", &json!({}))
        .expect("list_media");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Faxes/FX1/Media"),
        "compatibility.list_all_fax_media",
    );
}

#[test]
fn test_compat_list_all_fax_media_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_all_fax_media", 404, json!({"e": 1}));
    let err = c
        .compat()
        .faxes()
        .list_media("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.list_all_fax_media", 404);
}

#[test]
fn test_compat_retrieve_medias_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .faxes()
        .get_media("FX1", "ME1")
        .expect("get_media");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Faxes/FX1/Media/ME1"),
        "compatibility.retrieve_medias",
    );
}

#[test]
fn test_compat_retrieve_medias_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_medias", 404, json!({"e": 1}));
    let err = c
        .compat()
        .faxes()
        .get_media("FX1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_medias", 404);
}

#[test]
fn test_compat_delete_fax_media_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .faxes()
        .delete_media("FX1", "ME1")
        .expect("delete_media");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Faxes/FX1/Media/ME1"),
        "compatibility.delete_fax_media",
    );
}

#[test]
fn test_compat_delete_fax_media_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_fax_media", 404, json!({"e": 1}));
    let err = c
        .compat()
        .faxes()
        .delete_media("FX1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_fax_media", 404);
}

// ===========================================================================
// Phone numbers (Incoming / Imported)
// ===========================================================================

#[test]
fn test_compat_create_imported_phone_number_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .import_number(&json!({"PhoneNumber": "+15555550000"}))
        .expect("import_number");
    assert_ok(
        "POST",
        &common::mocktest::account_path("ImportedPhoneNumbers"),
        "compatibility.create_imported_phone_number",
    );
}

#[test]
fn test_compat_create_imported_phone_number_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.create_imported_phone_number",
        422,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .phone_numbers()
        .import_number(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.create_imported_phone_number", 422);
}

#[test]
fn test_compat_list_incoming_phone_numbers_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().phone_numbers().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("IncomingPhoneNumbers"),
        "compatibility.list_incoming_phone_numbers",
    );
}

#[test]
fn test_compat_list_incoming_phone_numbers_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.list_incoming_phone_numbers",
        500,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .phone_numbers()
        .list(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_incoming_phone_numbers", 500);
}

#[test]
fn test_compat_create_incoming_phone_number_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .purchase(&json!({"PhoneNumber": "+15555550000"}))
        .expect("purchase");
    assert_ok(
        "POST",
        &common::mocktest::account_path("IncomingPhoneNumbers"),
        "compatibility.create_incoming_phone_number",
    );
}

#[test]
fn test_compat_create_incoming_phone_number_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.create_incoming_phone_number",
        422,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .phone_numbers()
        .purchase(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.create_incoming_phone_number", 422);
}

#[test]
fn test_compat_retrieve_incoming_phone_number_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().phone_numbers().get("PN1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("IncomingPhoneNumbers/PN1"),
        "compatibility.retrieve_incoming_phone_number",
    );
}

#[test]
fn test_compat_retrieve_incoming_phone_number_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.retrieve_incoming_phone_number",
        404,
        json!({"e": 1}),
    );
    let err = c.compat().phone_numbers().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_incoming_phone_number", 404);
}

#[test]
fn test_compat_update_incoming_phone_number_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .phone_numbers()
        .update("PN1", &json!({"FriendlyName": "X"}))
        .expect("update");
    assert_ok(
        "POST",
        &common::mocktest::account_path("IncomingPhoneNumbers/PN1"),
        "compatibility.update_incoming_phone_number",
    );
}

#[test]
fn test_compat_update_incoming_phone_number_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.update_incoming_phone_number",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .phone_numbers()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_incoming_phone_number", 404);
}

#[test]
fn test_compat_delete_incoming_phone_number_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().phone_numbers().delete("PN1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("IncomingPhoneNumbers/PN1"),
        "compatibility.delete_incoming_phone_number",
    );
}

#[test]
fn test_compat_delete_incoming_phone_number_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "compatibility.delete_incoming_phone_number",
        404,
        json!({"e": 1}),
    );
    let err = c
        .compat()
        .phone_numbers()
        .delete("missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_incoming_phone_number", 404);
}

// ===========================================================================
// LamlBins (cXML scripts)
// ===========================================================================

#[test]
fn test_compat_list_cxml_scripts_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().laml_bins().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("LamlBins"),
        "compatibility.list_cxml_scripts",
    );
}

#[test]
fn test_compat_list_cxml_scripts_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_cxml_scripts", 500, json!({"e": 1}));
    let err = c.compat().laml_bins().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_cxml_scripts", 500);
}

#[test]
fn test_compat_create_cxml_script_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .laml_bins()
        .create(&json!({"Name": "bin", "Contents": "<Response/>"}))
        .expect("create");
    assert_ok(
        "POST",
        &common::mocktest::account_path("LamlBins"),
        "compatibility.create_cxml_script",
    );
}

#[test]
fn test_compat_create_cxml_script_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.create_cxml_script", 422, json!({"e": 1}));
    let err = c.compat().laml_bins().create(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.create_cxml_script", 422);
}

#[test]
fn test_compat_retrieve_cxml_script_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().laml_bins().get("LB1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("LamlBins/LB1"),
        "compatibility.retrieve_cxml_script",
    );
}

#[test]
fn test_compat_retrieve_cxml_script_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_cxml_script", 404, json!({"e": 1}));
    let err = c.compat().laml_bins().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_cxml_script", 404);
}

#[test]
fn test_compat_update_cxml_script_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .laml_bins()
        .update("LB1", &json!({"Name": "renamed"}))
        .expect("update");
    assert_ok(
        "POST",
        &common::mocktest::account_path("LamlBins/LB1"),
        "compatibility.update_cxml_script",
    );
}

#[test]
fn test_compat_update_cxml_script_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_cxml_script", 404, json!({"e": 1}));
    let err = c
        .compat()
        .laml_bins()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_cxml_script", 404);
}

#[test]
fn test_compat_delete_cxml_script_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().laml_bins().delete("LB1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("LamlBins/LB1"),
        "compatibility.delete_cxml_script",
    );
}

#[test]
fn test_compat_delete_cxml_script_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_cxml_script", 404, json!({"e": 1}));
    let err = c.compat().laml_bins().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_cxml_script", 404);
}

// ===========================================================================
// Messages (+ media)
// ===========================================================================

#[test]
fn test_compat_list_messages_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().messages().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Messages"),
        "compatibility.list_messages",
    );
}

#[test]
fn test_compat_list_messages_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_messages", 500, json!({"e": 1}));
    let err = c.compat().messages().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_messages", 500);
}

#[test]
fn test_compat_create_message_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .messages()
        .create(&json!({"To": "+15555550000", "From": "+15555551111", "Body": "hi"}))
        .expect("create");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Messages"),
        "compatibility.create_message",
    );
}

#[test]
fn test_compat_create_message_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.create_message", 422, json!({"e": 1}));
    let err = c.compat().messages().create(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.create_message", 422);
}

#[test]
fn test_compat_retrieve_message_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().messages().get("MM1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Messages/MM1"),
        "compatibility.retrieve_message",
    );
}

#[test]
fn test_compat_retrieve_message_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_message", 404, json!({"e": 1}));
    let err = c.compat().messages().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_message", 404);
}

#[test]
fn test_compat_update_message_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .messages()
        .update("MM1", &json!({"Body": ""}))
        .expect("update");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Messages/MM1"),
        "compatibility.update_message",
    );
}

#[test]
fn test_compat_update_message_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_message", 404, json!({"e": 1}));
    let err = c
        .compat()
        .messages()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_message", 404);
}

#[test]
fn test_compat_delete_message_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().messages().delete("MM1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Messages/MM1"),
        "compatibility.delete_message",
    );
}

#[test]
fn test_compat_delete_message_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_message", 404, json!({"e": 1}));
    let err = c.compat().messages().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_message", 404);
}

#[test]
fn test_compat_list_media_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .messages()
        .list_media("MM1", &json!({}))
        .expect("list_media");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Messages/MM1/Media"),
        "compatibility.list_media",
    );
}

#[test]
fn test_compat_list_media_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_media", 404, json!({"e": 1}));
    let err = c
        .compat()
        .messages()
        .list_media("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.list_media", 404);
}

#[test]
fn test_compat_retrieve_media_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .messages()
        .get_media("MM1", "ME1")
        .expect("get_media");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Messages/MM1/Media/ME1"),
        "compatibility.retrieve_media",
    );
}

#[test]
fn test_compat_retrieve_media_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_media", 404, json!({"e": 1}));
    let err = c
        .compat()
        .messages()
        .get_media("MM1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_media", 404);
}

#[test]
fn test_compat_delete_message_media_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .messages()
        .delete_media("MM1", "ME1")
        .expect("delete_media");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Messages/MM1/Media/ME1"),
        "compatibility.delete_message_media",
    );
}

#[test]
fn test_compat_delete_message_media_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_message_media", 404, json!({"e": 1}));
    let err = c
        .compat()
        .messages()
        .delete_media("MM1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_message_media", 404);
}

// ===========================================================================
// Queues (+ members)
// ===========================================================================

#[test]
fn test_compat_list_queues_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().queues().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Queues"),
        "compatibility.list_queues",
    );
}

#[test]
fn test_compat_list_queues_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_queues", 500, json!({"e": 1}));
    let err = c.compat().queues().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_queues", 500);
}

#[test]
fn test_compat_create_queue_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .queues()
        .create(&json!({"FriendlyName": "Q"}))
        .expect("create");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Queues"),
        "compatibility.create_queue",
    );
}

#[test]
fn test_compat_create_queue_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.create_queue", 422, json!({"e": 1}));
    let err = c.compat().queues().create(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.create_queue", 422);
}

#[test]
fn test_compat_retrieve_queue_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().queues().get("QU1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Queues/QU1"),
        "compatibility.retrieve_queue",
    );
}

#[test]
fn test_compat_retrieve_queue_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_queue", 404, json!({"e": 1}));
    let err = c.compat().queues().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_queue", 404);
}

#[test]
fn test_compat_update_queue_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .queues()
        .update("QU1", &json!({"FriendlyName": "Q2"}))
        .expect("update");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Queues/QU1"),
        "compatibility.update_queue",
    );
}

#[test]
fn test_compat_update_queue_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_queue", 404, json!({"e": 1}));
    let err = c
        .compat()
        .queues()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_queue", 404);
}

#[test]
fn test_compat_delete_queue_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().queues().delete("QU1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Queues/QU1"),
        "compatibility.delete_queue",
    );
}

#[test]
fn test_compat_delete_queue_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_queue", 404, json!({"e": 1}));
    let err = c.compat().queues().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_queue", 404);
}

#[test]
fn test_compat_list_all_queue_members_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .queues()
        .list_members("QU1", &json!({}))
        .expect("list_members");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Queues/QU1/Members"),
        "compatibility.list_all_queue_members",
    );
}

#[test]
fn test_compat_list_all_queue_members_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_all_queue_members", 404, json!({"e": 1}));
    let err = c
        .compat()
        .queues()
        .list_members("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.list_all_queue_members", 404);
}

#[test]
fn test_compat_retrieve_queue_member_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .queues()
        .get_member("QU1", "CA1")
        .expect("get_member");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Queues/QU1/Members/CA1"),
        "compatibility.retrieve_queue_member",
    );
}

#[test]
fn test_compat_retrieve_queue_member_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_queue_member", 404, json!({"e": 1}));
    let err = c
        .compat()
        .queues()
        .get_member("QU1", "missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_queue_member", 404);
}

#[test]
fn test_compat_update_queue_member_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .queues()
        .dequeue_member("QU1", "CA1", &json!({"Url": "https://x"}))
        .expect("dequeue_member");
    assert_ok(
        "POST",
        &common::mocktest::account_path("Queues/QU1/Members/CA1"),
        "compatibility.update_queue_member",
    );
}

#[test]
fn test_compat_update_queue_member_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_queue_member", 404, json!({"e": 1}));
    let err = c
        .compat()
        .queues()
        .dequeue_member("QU1", "missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_queue_member", 404);
}

// ===========================================================================
// Recordings
// ===========================================================================

#[test]
fn test_compat_list_recordings_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().recordings().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Recordings"),
        "compatibility.list_recordings",
    );
}

#[test]
fn test_compat_list_recordings_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_recordings", 500, json!({"e": 1}));
    let err = c.compat().recordings().list(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_recordings", 500);
}

#[test]
fn test_compat_retrieve_recording_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().recordings().get("RE1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Recordings/RE1"),
        "compatibility.retrieve_recording",
    );
}

#[test]
fn test_compat_retrieve_recording_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_recording", 404, json!({"e": 1}));
    let err = c.compat().recordings().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_recording", 404);
}

#[test]
fn test_compat_delete_recording_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().recordings().delete("RE1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Recordings/RE1"),
        "compatibility.delete_recording",
    );
}

#[test]
fn test_compat_delete_recording_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_recording", 404, json!({"e": 1}));
    let err = c.compat().recordings().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_recording", 404);
}

// ===========================================================================
// Transcriptions
// ===========================================================================

#[test]
fn test_compat_list_transcriptions_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().transcriptions().list(&json!({})).expect("list");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Transcriptions"),
        "compatibility.list_transcriptions",
    );
}

#[test]
fn test_compat_list_transcriptions_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.list_transcriptions", 500, json!({"e": 1}));
    let err = c
        .compat()
        .transcriptions()
        .list(&json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 500);
    assert_err_journal("compatibility.list_transcriptions", 500);
}

#[test]
fn test_compat_retrieve_transcription_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().transcriptions().get("TR1").expect("get");
    assert_ok(
        "GET",
        &common::mocktest::account_path("Transcriptions/TR1"),
        "compatibility.retrieve_transcription",
    );
}

#[test]
fn test_compat_retrieve_transcription_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.retrieve_transcription", 404, json!({"e": 1}));
    let err = c.compat().transcriptions().get("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.retrieve_transcription", 404);
}

#[test]
fn test_compat_delete_transcription_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().transcriptions().delete("TR1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("Transcriptions/TR1"),
        "compatibility.delete_transcription",
    );
}

#[test]
fn test_compat_delete_transcription_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_transcription", 404, json!({"e": 1}));
    let err = c
        .compat()
        .transcriptions()
        .delete("missing")
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_transcription", 404);
}

// ===========================================================================
// Tokens
// ===========================================================================

#[test]
fn test_compat_create_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .tokens()
        .create(&json!({"Ttl": 3600}))
        .expect("create");
    assert_ok(
        "POST",
        &common::mocktest::account_path("tokens"),
        "compatibility.create_token",
    );
}

#[test]
fn test_compat_create_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.create_token", 422, json!({"e": 1}));
    let err = c.compat().tokens().create(&json!({})).expect_err("err");
    assert_eq!(err.status_code(), 422);
    assert_err_journal("compatibility.create_token", 422);
}

#[test]
fn test_compat_update_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat()
        .tokens()
        .update("TK1", &json!({"Ttl": 7200}))
        .expect("update");
    assert_ok(
        "PATCH",
        &common::mocktest::account_path("tokens/TK1"),
        "compatibility.update_token",
    );
}

#[test]
fn test_compat_update_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.update_token", 404, json!({"e": 1}));
    let err = c
        .compat()
        .tokens()
        .update("missing", &json!({}))
        .expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.update_token", 404);
}

#[test]
fn test_compat_delete_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    c.compat().tokens().delete("TK1").expect("delete");
    assert_ok(
        "DELETE",
        &common::mocktest::account_path("tokens/TK1"),
        "compatibility.delete_token",
    );
}

#[test]
fn test_compat_delete_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("compatibility.delete_token", 404, json!({"e": 1}));
    let err = c.compat().tokens().delete("missing").expect_err("err");
    assert_eq!(err.status_code(), 404);
    assert_err_journal("compatibility.delete_token", 404);
}
