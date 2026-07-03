// Full success (2xx) + error (4xx/5xx) REST coverage for the `video` spec
// group. Mirrors the idiom of tests/rest_mock_fabric.rs: sync `#[test]`, the
// `common::mocktest` harness, journal assertions on method/path/matched_route,
// and `scenario_set` for the error path.
//
// Coverage: 30 of 33 video routes. Confirmed gaps (no SDK accessor / routing
// collision, not faked):
//   * video.list_logs  — no logs accessor on the video namespace
//   * video.get_log    — no logs accessor on the video namespace
//   * video.get_room   — wire-identical to get_room_by_name (GET
//     /api/video/rooms/{x}); the mock always resolves the ambiguous path to
//     get_room_by_name, so get_room_by_name is covered and get_room is a gap.

#[path = "common/mod.rs"]
mod common;

use serde_json::{Value, json};
use signalwire::rest::namespaces::generated::video_resources_generated as video_gen;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Rooms
// ---------------------------------------------------------------------------

#[test]
fn test_video_create_room_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .rooms()
        .create(&json!({"name": "my-room"}))
        .expect("create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/video/rooms");
    assert_eq!(e.matched_route.as_deref(), Some("video.create_room"));
}

#[test]
fn test_video_create_room_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.create_room", 422, json!({"error": "bad"}));
    let err = c
        .video()
        .rooms()
        .create(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("video.create_room"));
}

#[test]
fn test_video_list_rooms_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.video().rooms().list(&HashMap::new()).expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/rooms");
    assert_eq!(e.matched_route.as_deref(), Some("video.list_rooms"));
}

#[test]
fn test_video_list_rooms_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.list_rooms", 500, json!({"error": "boom"}));
    let err = c
        .video()
        .rooms()
        .list(&HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("video.list_rooms"));
}

// get_room_by_name — GET /api/video/rooms/{name}. The mock resolves the
// ambiguous GET-by-id path here (get_room is the wire-identical gap).
#[test]
fn test_video_get_room_by_name_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.video().rooms().get("my-room").expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/rooms/my-room");
    assert_eq!(e.matched_route.as_deref(), Some("video.get_room_by_name"));
}

#[test]
fn test_video_get_room_by_name_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.get_room_by_name", 404, json!({"error": "nf"}));
    let err = c.video().rooms().get("missing").expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("video.get_room_by_name"));
}

#[test]
fn test_video_update_room_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .rooms()
        .update("room-1", &json!({"name": "renamed"}))
        .expect("update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/video/rooms/room-1");
    assert_eq!(e.matched_route.as_deref(), Some("video.update_room"));
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("renamed"));
}

#[test]
fn test_video_update_room_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.update_room", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .rooms()
        .update("missing", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("video.update_room"));
}

#[test]
fn test_video_delete_room_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.video().rooms().delete("room-del").expect("delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/video/rooms/room-del");
    assert_eq!(e.matched_route.as_deref(), Some("video.delete_room"));
}

#[test]
fn test_video_delete_room_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.delete_room", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .rooms()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("video.delete_room"));
}

#[test]
fn test_video_list_room_streams_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .rooms()
        .list_streams("room-1", &HashMap::new())
        .expect("list_streams");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/rooms/room-1/streams");
    assert_eq!(e.matched_route.as_deref(), Some("video.list_room_streams"));
}

#[test]
fn test_video_list_room_streams_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.list_room_streams", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .rooms()
        .list_streams("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("video.list_room_streams"));
}

#[test]
fn test_video_create_room_stream_success() {
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
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/video/rooms/room-1/streams");
    assert_eq!(e.matched_route.as_deref(), Some("video.create_room_stream"));
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("url").and_then(Value::as_str),
        Some("rtmp://example.com/live")
    );
}

#[test]
fn test_video_create_room_stream_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.create_room_stream", 422, json!({"error": "bad"}));
    let err = c
        .video()
        .rooms()
        .create_stream("room-1", video_gen::VideoRoomsCreateStreamRequest::new(""))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("video.create_room_stream"));
}

// ---------------------------------------------------------------------------
// Room Tokens
// ---------------------------------------------------------------------------

#[test]
fn test_video_create_room_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_tokens()
        .create(video_gen::VideoRoomTokensCreateRequest::new("my-room"))
        .expect("create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/video/room_tokens");
    assert_eq!(e.matched_route.as_deref(), Some("video.create_room_token"));
}

#[test]
fn test_video_create_room_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.create_room_token", 422, json!({"error": "bad"}));
    let err = c
        .video()
        .room_tokens()
        .create(video_gen::VideoRoomTokensCreateRequest::new(""))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(e.matched_route.as_deref(), Some("video.create_room_token"));
}

// ---------------------------------------------------------------------------
// Room Sessions
// ---------------------------------------------------------------------------

#[test]
fn test_video_list_room_sessions_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_sessions()
        .list(&HashMap::new())
        .expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/room_sessions");
    assert_eq!(e.matched_route.as_deref(), Some("video.list_room_sessions"));
}

#[test]
fn test_video_list_room_sessions_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.list_room_sessions", 500, json!({"error": "boom"}));
    let err = c
        .video()
        .room_sessions()
        .list(&HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(e.matched_route.as_deref(), Some("video.list_room_sessions"));
}

#[test]
fn test_video_get_room_session_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.video().room_sessions().get("sess-abc").expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/room_sessions/sess-abc");
    assert_eq!(e.matched_route.as_deref(), Some("video.get_room_session"));
}

#[test]
fn test_video_get_room_session_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.get_room_session", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .room_sessions()
        .get("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("video.get_room_session"));
}

#[test]
fn test_video_list_room_session_events_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_sessions()
        .list_events("sess-1", &HashMap::new())
        .expect("list_events");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/room_sessions/sess-1/events");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_session_events")
    );
}

#[test]
fn test_video_list_room_session_events_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "video.list_room_session_events",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .video()
        .room_sessions()
        .list_events("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_session_events")
    );
}

#[test]
fn test_video_list_room_session_members_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_sessions()
        .list_members("sess-1", &HashMap::new())
        .expect("list_members");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/room_sessions/sess-1/members");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_session_members")
    );
}

#[test]
fn test_video_list_room_session_members_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "video.list_room_session_members",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .video()
        .room_sessions()
        .list_members("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_session_members")
    );
}

#[test]
fn test_video_list_room_session_recordings_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_sessions()
        .list_recordings("sess-2", &HashMap::new())
        .expect("list_recordings");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/room_sessions/sess-2/recordings");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_session_recordings")
    );
}

#[test]
fn test_video_list_room_session_recordings_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "video.list_room_session_recordings",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .video()
        .room_sessions()
        .list_recordings("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_session_recordings")
    );
}

// ---------------------------------------------------------------------------
// Room Recordings
// ---------------------------------------------------------------------------

#[test]
fn test_video_list_room_recordings_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_recordings()
        .list(&HashMap::new())
        .expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/room_recordings");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_recordings")
    );
}

#[test]
fn test_video_list_room_recordings_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.list_room_recordings", 500, json!({"error": "boom"}));
    let err = c
        .video()
        .room_recordings()
        .list(&HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_recordings")
    );
}

#[test]
fn test_video_get_room_recording_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_recordings()
        .get("rec-xyz", &HashMap::new())
        .expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/room_recordings/rec-xyz");
    assert_eq!(e.matched_route.as_deref(), Some("video.get_room_recording"));
}

#[test]
fn test_video_get_room_recording_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.get_room_recording", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .room_recordings()
        .get("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("video.get_room_recording"));
}

#[test]
fn test_video_delete_room_recording_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_recordings()
        .delete("rec-del")
        .expect("delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/video/room_recordings/rec-del");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.delete_room_recording")
    );
}

#[test]
fn test_video_delete_room_recording_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.delete_room_recording", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .room_recordings()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.delete_room_recording")
    );
}

#[test]
fn test_video_list_room_recording_events_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .room_recordings()
        .list_events("rec-1", &HashMap::new())
        .expect("list_events");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/room_recordings/rec-1/events");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_recording_events")
    );
}

#[test]
fn test_video_list_room_recording_events_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "video.list_room_recording_events",
        404,
        json!({"error": "nf"}),
    );
    let err = c
        .video()
        .room_recordings()
        .list_events("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_room_recording_events")
    );
}

// ---------------------------------------------------------------------------
// Conferences
// ---------------------------------------------------------------------------

#[test]
fn test_video_create_video_conference_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conferences()
        .create(&json!({"name": "conf"}))
        .expect("create");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/video/conferences");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.create_video_conference")
    );
}

#[test]
fn test_video_create_video_conference_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "video.create_video_conference",
        422,
        json!({"error": "bad"}),
    );
    let err = c
        .video()
        .conferences()
        .create(&json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.create_video_conference")
    );
}

#[test]
fn test_video_list_video_conferences_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.video().conferences().list(&HashMap::new()).expect("list");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/conferences");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_video_conferences")
    );
}

#[test]
fn test_video_list_video_conferences_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "video.list_video_conferences",
        500,
        json!({"error": "boom"}),
    );
    let err = c
        .video()
        .conferences()
        .list(&HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 500);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(500));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_video_conferences")
    );
}

#[test]
fn test_video_get_video_conference_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.video().conferences().get("conf-1").expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/conferences/conf-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.get_video_conference")
    );
}

#[test]
fn test_video_get_video_conference_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.get_video_conference", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .conferences()
        .get("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.get_video_conference")
    );
}

#[test]
fn test_video_update_video_conference_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conferences()
        .update("conf-1", &json!({"name": "renamed"}))
        .expect("update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/video/conferences/conf-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.update_video_conference")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(sent.get("name").and_then(Value::as_str), Some("renamed"));
}

#[test]
fn test_video_update_video_conference_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.update_video_conference", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .conferences()
        .update("missing", &json!({}))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.update_video_conference")
    );
}

#[test]
fn test_video_delete_video_conference_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.video().conferences().delete("conf-del").expect("delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/video/conferences/conf-del");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.delete_video_conference")
    );
}

#[test]
fn test_video_delete_video_conference_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.delete_video_conference", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .conferences()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.delete_video_conference")
    );
}

#[test]
fn test_video_list_conference_tokens_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conferences()
        .list_conference_tokens("conf-1", &HashMap::new())
        .expect("list_conference_tokens");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/conferences/conf-1/conference_tokens");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_conference_tokens")
    );
}

#[test]
fn test_video_list_conference_tokens_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.list_conference_tokens", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .conferences()
        .list_conference_tokens("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_conference_tokens")
    );
}

#[test]
fn test_video_list_conference_streams_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conferences()
        .list_streams("conf-2", &HashMap::new())
        .expect("list_streams");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/conferences/conf-2/streams");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_conference_streams")
    );
}

#[test]
fn test_video_list_conference_streams_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.list_conference_streams", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .conferences()
        .list_streams("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.list_conference_streams")
    );
}

#[test]
fn test_video_create_conference_stream_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conferences()
        .create_stream(
            "conf-1",
            video_gen::VideoConferencesCreateStreamRequest::new("rtmp://example.com/live"),
        )
        .expect("create_stream");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/video/conferences/conf-1/streams");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.create_conference_stream")
    );
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("url").and_then(Value::as_str),
        Some("rtmp://example.com/live")
    );
}

#[test]
fn test_video_create_conference_stream_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set(
        "video.create_conference_stream",
        422,
        json!({"error": "bad"}),
    );
    let err = c
        .video()
        .conferences()
        .create_stream(
            "conf-1",
            video_gen::VideoConferencesCreateStreamRequest::new(""),
        )
        .expect_err("should fail");
    assert_eq!(err.status_code(), 422);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(422));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.create_conference_stream")
    );
}

// ---------------------------------------------------------------------------
// Conference Tokens
// ---------------------------------------------------------------------------

#[test]
fn test_video_get_conference_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .conference_tokens()
        .get("tok-1", &HashMap::new())
        .expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/conference_tokens/tok-1");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.get_conference_token")
    );
}

#[test]
fn test_video_get_conference_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.get_conference_token", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .conference_tokens()
        .get("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.get_conference_token")
    );
}

#[test]
fn test_video_reset_conference_token_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.video().conference_tokens().reset("tok-2").expect("reset");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "POST");
    assert_eq!(e.path, "/api/video/conference_tokens/tok-2/reset");
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.reset_conference_token")
    );
}

#[test]
fn test_video_reset_conference_token_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.reset_conference_token", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .conference_tokens()
        .reset("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(
        e.matched_route.as_deref(),
        Some("video.reset_conference_token")
    );
}

// ---------------------------------------------------------------------------
// Streams (top-level)
// ---------------------------------------------------------------------------

#[test]
fn test_video_get_stream_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .streams()
        .get("stream-1", &HashMap::new())
        .expect("get");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "GET");
    assert_eq!(e.path, "/api/video/streams/stream-1");
    assert_eq!(e.matched_route.as_deref(), Some("video.get_stream"));
}

#[test]
fn test_video_get_stream_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.get_stream", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .streams()
        .get("missing", &HashMap::new())
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("video.get_stream"));
}

#[test]
fn test_video_update_stream_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c
        .video()
        .streams()
        .update(
            "stream-2",
            video_gen::VideoStreamsUpdateRequest::new("rtmp://example.com/new"),
        )
        .expect("update");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "PUT");
    assert_eq!(e.path, "/api/video/streams/stream-2");
    assert_eq!(e.matched_route.as_deref(), Some("video.update_stream"));
    let sent = e.body_object().expect("body");
    assert_eq!(
        sent.get("url").and_then(Value::as_str),
        Some("rtmp://example.com/new")
    );
}

#[test]
fn test_video_update_stream_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.update_stream", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .streams()
        .update("missing", video_gen::VideoStreamsUpdateRequest::new(""))
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("video.update_stream"));
}

#[test]
fn test_video_delete_stream_success() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    let body = c.video().streams().delete("stream-3").expect("delete");
    assert!(body.is_object());
    let e = common::mocktest::journal_last();
    assert_eq!(e.method, "DELETE");
    assert_eq!(e.path, "/api/video/streams/stream-3");
    assert_eq!(e.matched_route.as_deref(), Some("video.delete_stream"));
}

#[test]
fn test_video_delete_stream_error() {
    let _g = common::mocktest::begin();
    let c = common::mocktest::client();
    common::mocktest::scenario_set("video.delete_stream", 404, json!({"error": "nf"}));
    let err = c
        .video()
        .streams()
        .delete("missing")
        .expect_err("should fail");
    assert_eq!(err.status_code(), 404);
    let e = common::mocktest::journal_last();
    assert_eq!(e.response_status, Some(404));
    assert_eq!(e.matched_route.as_deref(), Some("video.delete_stream"));
}
