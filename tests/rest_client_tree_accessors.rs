// Regression guard: every `RestClient::<resource>()` tree accessor is
// reachable at runtime and drives a real request onto the shared
// `mock_signalwire` server.
//
// Why this file exists: the five accessors below (`calling`, `mfa`,
// `number_groups`, `queues`, `sip_profile`) were reported as signature drift
// against the Python oracle. The drift was an ENUMERATOR defect (a stale
// class-rename table mapping the generated struct names onto namespace class
// names the reference no longer defines), not a missing capability — but the
// only way to know that is to prove the accessors actually work. These tests
// pin that proof so a future "fix" that deletes or rewires an accessor is
// caught by the test suite and not just by the surface gate.
//
// Each test goes through `RestClient::<accessor>()` explicitly (never through
// an intermediate handle) and asserts the request landed on the mock with the
// expected HTTP method and the expected spec `operationId` route.

#[path = "common/mod.rs"]
mod common;

use std::collections::HashMap;

use signalwire::rest::namespaces::generated::calling_resources_generated as calling_gen;
use signalwire::rest::namespaces::generated::relay_rest_resources_generated as relay_gen;

#[test]
fn test_tree_accessor_calling_reaches_the_wire() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    // `RestClient::calling()` -> generated `Calling` resource.
    let _ = c
        .calling()
        .update(calling_gen::CallingUpdateRequest::new("call-id"), None);
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    // `calling` is a command-dispatch namespace: every command POSTs to the
    // single `/calling/calls` route, discriminated by the body's `command`.
    assert_eq!(e.matched_route.as_deref(), Some("calling.call-commands"));
}

#[test]
fn test_tree_accessor_mfa_reaches_the_wire() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    // `RestClient::mfa()` -> generated `Mfa` resource.
    let _ = c
        .mfa()
        .sms(relay_gen::MfaSmsRequest::new("+15551230000"), None);
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.request_mfa_sms")
    );
}

#[test]
fn test_tree_accessor_number_groups_reaches_the_wire() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    // `RestClient::number_groups()` -> generated `NumberGroups` resource.
    let _ = c.number_groups().get("ng-id", None);
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_number_group")
    );
}

#[test]
fn test_tree_accessor_queues_reaches_the_wire() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    // `RestClient::queues()` -> generated `Queues` resource.
    let _ = c.queues().list(&HashMap::new(), None);
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.matched_route.as_deref(), Some("relay-rest.list_queues"));
}

#[test]
fn test_tree_accessor_sip_profile_reaches_the_wire() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    // `RestClient::sip_profile()` -> generated `SipProfile` resource.
    let _ = c.sip_profile().get(&HashMap::new(), None);
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("relay-rest.retrieve_sip_profile")
    );
}
