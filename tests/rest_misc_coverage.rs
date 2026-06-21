// Full REST success + error coverage for the small spec groups:
// datasphere, project, voice/fax/message/conference logs, calling dial,
// chat tokens, and pubsub tokens.
//
// Mirrors the idiom of tests/rest_mock_fabric.rs: sync #[test], the
// common::mocktest harness, a success (2xx) test asserting the journaled
// method/path/matched_route, and an error test that stages a scenario and
// asserts SignalWireRestError::status_code() + the journal's response_status.

#[path = "common/mod.rs"]
mod common;

use serde_json::{Value, json};

// ===========================================================================
// datasphere — c.datasphere().documents()
// ===========================================================================

#[test]
fn test_datasphere_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .datasphere()
        .documents()
        .list(&json!({}))
        .expect("datasphere.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/datasphere/documents");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.list_documents")
    );
}

#[test]
fn test_datasphere_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("datasphere.list_documents", 500, json!({"error": "boom"}));
    let err = c
        .datasphere()
        .documents()
        .list(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.list_documents")
    );
}

#[test]
fn test_datasphere_create_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .datasphere()
        .documents()
        .create(&json!({"url": "https://example.com/doc"}))
        .expect("datasphere.create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/datasphere/documents");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.create_document")
    );
}

#[test]
fn test_datasphere_create_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "datasphere.create_document",
        422,
        json!({"error": "invalid"}),
    );
    let err = c
        .datasphere()
        .documents()
        .create(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.create_document")
    );
}

#[test]
fn test_datasphere_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .datasphere()
        .documents()
        .get("doc-1")
        .expect("datasphere.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/datasphere/documents/doc-1");
    assert_eq!(e.matched_route.as_deref(), Some("datasphere.get_document"));
}

#[test]
fn test_datasphere_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("datasphere.get_document", 404, json!({"error": "nf"}));
    let err = c
        .datasphere()
        .documents()
        .get("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("datasphere.get_document"));
}

#[test]
fn test_datasphere_update_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .datasphere()
        .documents()
        .update("doc-1", &json!({"name": "renamed"}))
        .expect("datasphere.update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PATCH");
    assert_eq!(e.path, "/api/datasphere/documents/doc-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.update_document")
    );
}

#[test]
fn test_datasphere_update_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("datasphere.update_document", 404, json!({"error": "nf"}));
    let err = c
        .datasphere()
        .documents()
        .update("missing", &json!({"name": "x"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.update_document")
    );
}

#[test]
fn test_datasphere_delete_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let _ = c
        .datasphere()
        .documents()
        .delete("doc-1")
        .expect("datasphere.delete");
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/datasphere/documents/doc-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.delete_document")
    );
}

#[test]
fn test_datasphere_delete_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("datasphere.delete_document", 404, json!({"error": "nf"}));
    let err = c
        .datasphere()
        .documents()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.delete_document")
    );
}

#[test]
fn test_datasphere_search_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .datasphere()
        .documents()
        .search(&json!({"query": "hello"}))
        .expect("datasphere.search");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/datasphere/documents/search");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.search_documents")
    );
}

#[test]
fn test_datasphere_search_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("datasphere.search_documents", 422, json!({"error": "bad"}));
    let err = c
        .datasphere()
        .documents()
        .search(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.search_documents")
    );
}

#[test]
fn test_datasphere_list_chunks_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .datasphere()
        .documents()
        .list_chunks("doc-1", &json!({}))
        .expect("datasphere.list_chunks");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/datasphere/documents/doc-1/chunks");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.list_document_chunks")
    );
}

#[test]
fn test_datasphere_list_chunks_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "datasphere.list_document_chunks",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .datasphere()
        .documents()
        .list_chunks("missing", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.list_document_chunks")
    );
}

#[test]
fn test_datasphere_get_chunk_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .datasphere()
        .documents()
        .get_chunk("doc-1", "chunk-1")
        .expect("datasphere.get_chunk");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/datasphere/documents/doc-1/chunks/chunk-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.get_document_chunk")
    );
}

#[test]
fn test_datasphere_get_chunk_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("datasphere.get_document_chunk", 404, json!({"error": "nf"}));
    let err = c
        .datasphere()
        .documents()
        .get_chunk("doc-1", "missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.get_document_chunk")
    );
}

#[test]
fn test_datasphere_delete_chunk_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let _ = c
        .datasphere()
        .documents()
        .delete_chunk("doc-1", "chunk-1")
        .expect("datasphere.delete_chunk");
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/datasphere/documents/doc-1/chunks/chunk-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.delete_document_chunk")
    );
}

#[test]
fn test_datasphere_delete_chunk_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "datasphere.delete_document_chunk",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .datasphere()
        .documents()
        .delete_chunk("doc-1", "missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("datasphere.delete_document_chunk")
    );
}

// ===========================================================================
// project — c.project().tokens()
// ===========================================================================

#[test]
fn test_project_create_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .project()
        .tokens()
        .create(&json!({"name": "tok"}))
        .expect("project.create_token");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/project/tokens");
    assert_eq!(e.matched_route.as_deref(), Some("project.create_token"));
}

#[test]
fn test_project_create_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("project.create_token", 422, json!({"error": "bad"}));
    let err = c
        .project()
        .tokens()
        .create(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("project.create_token"));
}

#[test]
fn test_project_update_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .project()
        .tokens()
        .update("tok-1", &json!({"name": "renamed"}))
        .expect("project.update_token");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PATCH");
    assert_eq!(e.path, "/api/project/tokens/tok-1");
    assert_eq!(e.matched_route.as_deref(), Some("project.update_token"));
}

#[test]
fn test_project_update_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("project.update_token", 404, json!({"error": "nf"}));
    let err = c
        .project()
        .tokens()
        .update("missing", &json!({"name": "x"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("project.update_token"));
}

#[test]
fn test_project_delete_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let _ = c
        .project()
        .tokens()
        .delete("tok-1")
        .expect("project.delete_token");
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/project/tokens/tok-1");
    assert_eq!(e.matched_route.as_deref(), Some("project.delete_token"));
}

#[test]
fn test_project_delete_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("project.delete_token", 404, json!({"error": "nf"}));
    let err = c
        .project()
        .tokens()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("project.delete_token"));
}

// ===========================================================================
// calling dial — c.calling().dial() -> POST /api/calling/calls (call-commands)
// ===========================================================================

#[test]
fn test_calling_dial_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .calling()
        .dial(json!({"url": "https://example.com/swml", "to": "+15551234567"}))
        .expect("calling.dial");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/calling/calls");
    assert_eq!(e.matched_route.as_deref(), Some("calling.call-commands"));
    let body_obj = e.body_object().expect("body object");
    assert_eq!(
        body_obj.get("command").and_then(Value::as_str),
        Some("dial")
    );
}

#[test]
fn test_calling_dial_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("calling.call-commands", 422, json!({"error": "bad"}));
    let err = c
        .calling()
        .dial(json!({"url": "https://example.com/swml"}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("calling.call-commands"));
}

// ===========================================================================
// chat — c.chat().create_token() -> POST /api/chat/tokens
// ===========================================================================

#[test]
fn test_chat_create_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .chat()
        .create_token(&json!({"channels": {"room": {"read": true}}}))
        .expect("chat.create_token");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/chat/tokens");
    assert_eq!(e.matched_route.as_deref(), Some("chat.create_chat_token"));
}

#[test]
fn test_chat_create_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("chat.create_chat_token", 422, json!({"error": "bad"}));
    let err = c.chat().create_token(&json!({})).expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("chat.create_chat_token"));
}

// ===========================================================================
// pubsub — c.pubsub().create_token() -> POST /api/pubsub/tokens
// ===========================================================================

#[test]
fn test_pubsub_create_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .pubsub()
        .create_token(&json!({"channels": {"topic": {"read": true}}}))
        .expect("pubsub.create_token");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/pubsub/tokens");
    assert_eq!(e.matched_route.as_deref(), Some("pubsub.create_token"));
}

#[test]
fn test_pubsub_create_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("pubsub.create_token", 422, json!({"error": "bad"}));
    let err = c
        .pubsub()
        .create_token(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("pubsub.create_token"));
}

// ===========================================================================
// logs.messages — c.logs().messages()  (message.* spec group)
// ===========================================================================

#[test]
fn test_logs_messages_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.logs().messages().list(&json!({})).expect("messages.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/messaging/logs");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("message.list_message_logs")
    );
}

#[test]
fn test_logs_messages_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("message.list_message_logs", 500, json!({"error": "boom"}));
    let err = c
        .logs()
        .messages()
        .list(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("message.list_message_logs")
    );
}

#[test]
fn test_logs_messages_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.logs().messages().get("ml-1").expect("messages.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/messaging/logs/ml-1");
    assert_eq!(e.matched_route.as_deref(), Some("message.get_message_log"));
}

#[test]
fn test_logs_messages_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("message.get_message_log", 404, json!({"error": "nf"}));
    let err = c.logs().messages().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("message.get_message_log"));
}

// ===========================================================================
// logs.voice — c.logs().voice()  (voice.* spec group)
// ===========================================================================

#[test]
fn test_logs_voice_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.logs().voice().list(&json!({})).expect("voice.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/voice/logs");
    assert_eq!(e.matched_route.as_deref(), Some("voice.list_voice_logs"));
}

#[test]
fn test_logs_voice_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("voice.list_voice_logs", 500, json!({"error": "boom"}));
    let err = c.logs().voice().list(&json!({})).expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("voice.list_voice_logs"));
}

#[test]
fn test_logs_voice_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.logs().voice().get("vl-1").expect("voice.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/voice/logs/vl-1");
    assert_eq!(e.matched_route.as_deref(), Some("voice.get_voice_log"));
}

#[test]
fn test_logs_voice_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("voice.get_voice_log", 404, json!({"error": "nf"}));
    let err = c.logs().voice().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("voice.get_voice_log"));
}

#[test]
fn test_logs_voice_list_events_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .logs()
        .voice()
        .list_events("vl-1", &json!({}))
        .expect("voice.list_events");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/voice/logs/vl-1/events");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("voice.list_voice_log_events")
    );
}

#[test]
fn test_logs_voice_list_events_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("voice.list_voice_log_events", 404, json!({"error": "nf"}));
    let err = c
        .logs()
        .voice()
        .list_events("missing", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("voice.list_voice_log_events")
    );
}

// ===========================================================================
// logs.fax — c.logs().fax()  (fax.* spec group)
// ===========================================================================

#[test]
fn test_logs_fax_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.logs().fax().list(&json!({})).expect("fax.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/fax/logs");
    assert_eq!(e.matched_route.as_deref(), Some("fax.list_fax_logs"));
}

#[test]
fn test_logs_fax_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fax.list_fax_logs", 500, json!({"error": "boom"}));
    let err = c.logs().fax().list(&json!({})).expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("fax.list_fax_logs"));
}

#[test]
fn test_logs_fax_get_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.logs().fax().get("fl-1").expect("fax.get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/fax/logs/fl-1");
    assert_eq!(e.matched_route.as_deref(), Some("fax.get_fax_log"));
}

#[test]
fn test_logs_fax_get_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("fax.get_fax_log", 404, json!({"error": "nf"}));
    let err = c.logs().fax().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("fax.get_fax_log"));
}

// ===========================================================================
// logs.conferences — c.logs().conferences()  (logs.* spec group)
// ===========================================================================

#[test]
fn test_logs_conferences_list_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .logs()
        .conferences()
        .list(&json!({}))
        .expect("conferences.list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/logs/conferences");
    assert_eq!(e.matched_route.as_deref(), Some("logs.list_conferences"));
}

#[test]
fn test_logs_conferences_list_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("logs.list_conferences", 500, json!({"error": "boom"}));
    let err = c
        .logs()
        .conferences()
        .list(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("logs.list_conferences"));
}
