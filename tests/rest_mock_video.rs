// Mock-backed integration tests translated from
// signalwire-python/tests/unit/rest/test_video_mock.py.
//
// Covers room sessions, room recordings, conference tokens, conference
// streams, and individual stream lifecycle.

#[path = "common/mod.rs"]
mod common;

use serde_json::Value;
use signalwire::rest::namespaces::generated::video_resources_generated as video_gen;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Rooms — streams sub-resource
// ---------------------------------------------------------------------------

#[test]
fn test_video_rooms_list_streams_returns_data_collection() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .rooms()
        .list_streams("room-1", &HashMap::new())
        .expect("list_streams");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(
        obj.contains_key("data"),
        "missing 'data' in {:?}",
        obj.keys().collect::<Vec<_>>()
    );
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/rooms/room-1/streams");
    assert!(
        entry.matched_route.is_some(),
        "spec gap: rooms streams list"
    );
}

#[test]
fn test_video_rooms_create_stream_posts_kwargs_in_body() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .rooms()
        .create_stream(
            "room-1",
            video_gen::VideoRoomsCreateStreamRequest::new("rtmp://example.com/live"),
        )
        .expect("create_stream");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, "/api/video/rooms/room-1/streams");
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("url").and_then(Value::as_str),
        Some("rtmp://example.com/live")
    );
}

// ---------------------------------------------------------------------------
// Room Sessions
// ---------------------------------------------------------------------------

#[test]
fn test_video_room_sessions_list_returns_data_collection() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_sessions()
        .list(&HashMap::new())
        .expect("room_sessions.list");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"));
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/room_sessions");
}

#[test]
fn test_video_room_sessions_get_returns_session_object() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_sessions()
        .get("sess-abc")
        .expect("room_sessions.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/room_sessions/sess-abc");
    assert!(entry.matched_route.is_some());
}

#[test]
fn test_video_room_sessions_list_events_uses_subpath() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_sessions()
        .list_events("sess-1", &HashMap::new())
        .expect("list_events");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("data"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/room_sessions/sess-1/events");
}

#[test]
fn test_video_room_sessions_list_recordings_uses_subpath() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_sessions()
        .list_recordings("sess-2", &HashMap::new())
        .expect("list_recordings");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("data"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/room_sessions/sess-2/recordings");
}

// ---------------------------------------------------------------------------
// Room Recordings
// ---------------------------------------------------------------------------

#[test]
fn test_video_room_recordings_list_returns_data_collection() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_recordings()
        .list(&HashMap::new())
        .expect("room_recordings.list");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"));
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/room_recordings");
}

#[test]
fn test_video_room_recordings_get_returns_single() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_recordings()
        .get("rec-xyz", &HashMap::new())
        .expect("room_recordings.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/room_recordings/rec-xyz");
}

#[test]
fn test_video_room_recordings_delete_returns_dict_for_204() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_recordings()
        .delete("rec-del")
        .expect("room_recordings.delete");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, "/api/video/room_recordings/rec-del");
    assert!(entry.matched_route.is_some());
}

#[test]
fn test_video_room_recordings_list_events_uses_subpath() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_recordings()
        .list_events("rec-1", &HashMap::new())
        .expect("list_events");
    assert!(body.is_object());
    assert!(body.as_object().unwrap().contains_key("data"));

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/room_recordings/rec-1/events");
}

// ---------------------------------------------------------------------------
// Conferences — sub-collections (tokens, streams)
// ---------------------------------------------------------------------------

#[test]
fn test_video_conferences_list_conference_tokens() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conferences()
        .list_conference_tokens("conf-1", &HashMap::new())
        .expect("list_conference_tokens");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"));
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(
        entry.path,
        "/api/video/conferences/conf-1/conference_tokens"
    );
}

#[test]
fn test_video_conferences_list_streams() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conferences()
        .list_streams("conf-2", &HashMap::new())
        .expect("list_streams");
    assert!(body.is_object());
    let obj = body.as_object().unwrap();
    assert!(obj.contains_key("data"));
    assert!(obj.get("data").unwrap().is_array());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/conferences/conf-2/streams");
}

// ---------------------------------------------------------------------------
// Conference Tokens
// ---------------------------------------------------------------------------

#[test]
fn test_video_conference_tokens_get_returns_single() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conference_tokens()
        .get("tok-1", &HashMap::new())
        .expect("conference_tokens.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/conference_tokens/tok-1");
    assert!(entry.matched_route.is_some());
}

#[test]
fn test_video_conference_tokens_reset_posts_to_subpath() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conference_tokens()
        .reset("tok-2")
        .expect("conference_tokens.reset");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "POST");
    assert_eq!(entry.path, "/api/video/conference_tokens/tok-2/reset");
    // reset is no-body POST — body should be empty/{} on the wire.
    let body_is_empty =
        entry.body.is_null() || matches!(&entry.body, Value::Object(o) if o.is_empty());
    assert!(body_is_empty, "expected empty body, got {:?}", entry.body);
}

// ---------------------------------------------------------------------------
// Streams (top-level)
// ---------------------------------------------------------------------------

#[test]
fn test_video_streams_get_returns_resource() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .streams()
        .get("stream-1", &HashMap::new())
        .expect("streams.get");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "GET");
    assert_eq!(entry.path, "/api/video/streams/stream-1");
}

#[test]
fn test_video_streams_update_uses_put_with_kwargs() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .streams()
        .update(
            "stream-2",
            video_gen::VideoStreamsUpdateRequest::new("rtmp://example.com/new"),
        )
        .expect("streams.update");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "PUT");
    assert_eq!(entry.path, "/api/video/streams/stream-2");
    let sent = entry.body_object().expect("body");
    assert_eq!(
        sent.get("url").and_then(Value::as_str),
        Some("rtmp://example.com/new")
    );
}

#[test]
fn test_video_streams_delete_returns_dict() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .streams()
        .delete("stream-3")
        .expect("streams.delete");
    assert!(body.is_object());

    let entry = common::mocktest::journal_last();
    assert_eq!(entry.method, "DELETE");
    assert_eq!(entry.path, "/api/video/streams/stream-3");
    assert!(entry.matched_route.is_some());
}
