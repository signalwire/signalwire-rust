// Smoke test for the relay_mocktest harness.
//
// Verifies the probe-or-spawn lifecycle works against a running mock_relay
// and that journal_recv / connect handshake produce the expected entries.

#[path = "common/mod.rs"]
mod common;

use common::relay_mocktest;

#[test]
fn test_harness_health_probe_and_connect() {
    let _g = relay_mocktest::begin();
    // The connect handshake should produce a journaled signalwire.connect.
    let client = relay_mocktest::connected_client(&["default"]);
    let connects = relay_mocktest::journal_recv(Some("signalwire.connect"));
    assert!(
        !connects.is_empty(),
        "expected at least one signalwire.connect frame in the journal"
    );
    // The protocol string should have been captured from the response.
    assert!(
        client.protocol.lock().unwrap().is_some(),
        "client.protocol should be set after connect handshake"
    );
    client.disconnect();
}
