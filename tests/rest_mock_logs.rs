// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_logs_mock.py.
//
// Covers MessageLogs, VoiceLogs, FaxLogs, ConferenceLogs.

#[path = "common/mod.rs"]
mod common;

// ---------------------------------------------------------------------------
// Message Logs — /api/messaging/logs
// ---------------------------------------------------------------------------

#[test]
fn test_logs_messages_list_returns_dict() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .logs()
        .messages()
        .list(&std::collections::HashMap::<String, String>::new())
        .expect("messages.list");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/messaging/logs");
    assert_eq!(
        entry.matched_route.as_deref(),
        Some("message.list_message_logs"),
        "expected message.list_message_logs, got {:?}",
        entry.matched_route
    );
}

#[test]
fn test_logs_messages_get_uses_id_in_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.logs().messages().get("ml-42").expect("messages.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/messaging/logs/ml-42");
    assert!(
        entry.matched_route.is_some(),
        "spec gap: message log retrieve"
    );
}

// ---------------------------------------------------------------------------
// Voice Logs — /api/voice/logs
// ---------------------------------------------------------------------------

#[test]
fn test_logs_voice_list_returns_dict() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .logs()
        .voice()
        .list(&std::collections::HashMap::<String, String>::new())
        .expect("voice.list");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/voice/logs");
    assert_eq!(
        entry.matched_route.as_deref(),
        Some("voice.list_voice_logs")
    );
}

#[test]
fn test_logs_voice_get_uses_id_in_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.logs().voice().get("vl-99").expect("voice.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/voice/logs/vl-99");
}

// ---------------------------------------------------------------------------
// Fax Logs — /api/fax/logs
// ---------------------------------------------------------------------------

#[test]
fn test_logs_fax_list_returns_dict() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .logs()
        .fax()
        .list(&std::collections::HashMap::<String, String>::new())
        .expect("fax.list");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/fax/logs");
    assert_eq!(entry.matched_route.as_deref(), Some("fax.list_fax_logs"));
}

#[test]
fn test_logs_fax_get_uses_id_in_path() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.logs().fax().get("fl-7").expect("fax.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/fax/logs/fl-7");
}

// ---------------------------------------------------------------------------
// Conference Logs — /api/logs/conferences
// ---------------------------------------------------------------------------

#[test]
fn test_logs_conferences_list_returns_dict() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .logs()
        .conferences()
        .list(&std::collections::HashMap::<String, String>::new())
        .expect("conferences.list");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/logs/conferences");
    assert_eq!(
        entry.matched_route.as_deref(),
        Some("logs.list_conferences")
    );
}
