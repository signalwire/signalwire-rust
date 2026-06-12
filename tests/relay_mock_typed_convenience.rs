// Mock-backed integration tests for the typed audio/detect/prompt
// convenience wrappers on `relay::Call` (play_tts / play_audio /
// play_silence / play_ringtone / detect_digit / detect_answering_machine /
// detect_fax / prompt_tts / prompt_audio), mirroring the Python reference's
// `call.play_tts` / `call.detect_*` / `call.prompt_*` family.
//
// Each wrapper is a thin typed convenience over the generic Call.play /
// Call.detect / Call.play_and_collect actions: it builds the EXACT RELAY
// media/params shape with serde_json and delegates. The Rust Call records
// every command it builds into `sent_commands` (it does not transmit on its
// own), so the test drives the REAL built frame: it invokes the wrapper,
// pulls the exact params the wrapper assembled, sends that frame over the
// wire through a connected client, and asserts the shared mock_relay server
// journaled the identical media shape. No transport mock — the assertion is
// against the real mock journal, paired with a behavioral assertion on the
// shape the wrapper produced.

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

use common::relay_mocktest;

fn wait_until<F: Fn() -> bool>(budget_ms: u64, f: F) -> bool {
    use std::time::{Duration, Instant};
    let deadline = Instant::now() + Duration::from_millis(budget_ms);
    while Instant::now() < deadline {
        if f() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn answered_inbound_call(
    client: &Arc<signalwire::relay::Client>,
    call_id: &str,
) -> Arc<signalwire::relay::Call> {
    let captured: Arc<Mutex<Option<Arc<signalwire::relay::Call>>>> =
        Arc::new(Mutex::new(None));
    let cap2 = captured.clone();
    let client2 = client.clone();
    client.on_call(move |call, _ev| {
        let id = call.call_id.clone().unwrap_or_default();
        let frame = json!({
            "jsonrpc": "2.0",
            "id": format!("ans-{}", id),
            "method": "calling.answer",
            "params": {"call_id": id, "node_id": call.node_id.clone().unwrap_or_default()},
        });
        client2.send(&frame);
        *cap2.lock().unwrap() = Some(call);
    });
    relay_mocktest::inbound_call(json!({
        "call_id": call_id,
        "auto_states": ["created"],
    }));
    assert!(
        wait_until(3000, || captured.lock().unwrap().is_some()),
        "on_call did not fire for {call_id}"
    );
    let call = captured.lock().unwrap().clone().unwrap();
    *call.state.lock().unwrap() = "answered".to_string();
    call
}

/// Pull the params of the single command the wrapper recorded (it builds
/// exactly one `calling.<verb>` command), asserting the verb matches.
fn built_params(call: &Arc<signalwire::relay::Call>, expect_method: &str) -> Value {
    let cmds = call.sent_commands.lock().unwrap();
    assert_eq!(cmds.len(), 1, "wrapper should build exactly one command");
    assert_eq!(cmds[0].0, expect_method, "wrapper built the wrong verb");
    cmds[0].1.clone()
}

/// Send the wrapper's built params over the wire (so the shared mock
/// journals them), preserving the exact media shape the wrapper assembled.
fn send_built(
    client: &Arc<signalwire::relay::Client>,
    method: &str,
    params: &Value,
) -> String {
    let id = format!("rpc-{method}");
    let frame = json!({
        "jsonrpc": "2.0",
        "id": id.clone(),
        "method": method,
        "params": params.clone(),
    });
    client.send(&frame);
    id
}

/// Journal the wrapper's built frame and return the inner params the mock
/// recorded. Bridges the behavioral build with a real wire assertion.
fn journal_built(
    client: &Arc<signalwire::relay::Client>,
    call: &Arc<signalwire::relay::Call>,
    method: &str,
) -> Value {
    let params = built_params(call, method);
    send_built(client, method, &params);
    std::thread::sleep(std::time::Duration::from_millis(150));
    let entry = relay_mocktest::journal_recv(Some(method))
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("expected one {method} frame in mock journal"));
    entry.inner_params().clone()
}

// ---------------------------------------------------------------------------
// play_tts
// ---------------------------------------------------------------------------

#[test]
fn test_play_tts_builds_and_journals_tts_media() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-ptts");

    let action = call.play_tts(
        "Hello world",
        json!({"language": "en-US", "gender": "female", "voice": "spore", "volume": -2.5}),
    );
    assert!(!action.is_done());

    let p = journal_built(&client, &call, "calling.play");
    let media = p.get("play").and_then(Value::as_array).expect("play array");
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].get("type").and_then(Value::as_str), Some("tts"));
    let mp = media[0].get("params").unwrap();
    assert_eq!(mp.get("text").and_then(Value::as_str), Some("Hello world"));
    assert_eq!(mp.get("language").and_then(Value::as_str), Some("en-US"));
    assert_eq!(mp.get("gender").and_then(Value::as_str), Some("female"));
    assert_eq!(mp.get("voice").and_then(Value::as_str), Some("spore"));
    // volume rides at the top level, not inside the tts params.
    assert_eq!(p.get("volume").and_then(Value::as_f64), Some(-2.5));
    assert!(mp.get("volume").is_none());
    client.disconnect();
}

#[test]
fn test_play_tts_omits_unset_optionals() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-ptts-min");

    call.play_tts("Just text", json!({}));

    let p = journal_built(&client, &call, "calling.play");
    let mp = p.get("play").and_then(Value::as_array).unwrap()[0]
        .get("params")
        .unwrap();
    assert_eq!(mp.get("text").and_then(Value::as_str), Some("Just text"));
    // Only-provided-keys: no language/gender/voice keys when caller omits them.
    assert!(mp.get("language").is_none());
    assert!(mp.get("gender").is_none());
    assert!(mp.get("voice").is_none());
    assert!(p.get("volume").is_none());
    client.disconnect();
}

// ---------------------------------------------------------------------------
// play_audio
// ---------------------------------------------------------------------------

#[test]
fn test_play_audio_builds_and_journals_audio_media() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-paudio");

    call.play_audio("https://cdn.example/clip.mp3", json!({"volume": 3.0}));

    let p = journal_built(&client, &call, "calling.play");
    let media = p.get("play").and_then(Value::as_array).unwrap();
    assert_eq!(media[0].get("type").and_then(Value::as_str), Some("audio"));
    assert_eq!(
        media[0]
            .get("params")
            .and_then(|mp| mp.get("url"))
            .and_then(Value::as_str),
        Some("https://cdn.example/clip.mp3")
    );
    assert_eq!(p.get("volume").and_then(Value::as_f64), Some(3.0));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// play_silence
// ---------------------------------------------------------------------------

#[test]
fn test_play_silence_builds_and_journals_silence_media() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-psil");

    call.play_silence(4.5);

    let p = journal_built(&client, &call, "calling.play");
    let media = p.get("play").and_then(Value::as_array).unwrap();
    assert_eq!(media[0].get("type").and_then(Value::as_str), Some("silence"));
    assert_eq!(
        media[0]
            .get("params")
            .and_then(|mp| mp.get("duration"))
            .and_then(Value::as_f64),
        Some(4.5)
    );
    client.disconnect();
}

// ---------------------------------------------------------------------------
// play_ringtone
// ---------------------------------------------------------------------------

#[test]
fn test_play_ringtone_builds_and_journals_ringtone_media() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-prt");

    call.play_ringtone("us", json!({"duration": 8.0, "volume": -1.0}));

    let p = journal_built(&client, &call, "calling.play");
    let media = p.get("play").and_then(Value::as_array).unwrap();
    assert_eq!(media[0].get("type").and_then(Value::as_str), Some("ringtone"));
    let mp = media[0].get("params").unwrap();
    assert_eq!(mp.get("name").and_then(Value::as_str), Some("us"));
    assert_eq!(mp.get("duration").and_then(Value::as_f64), Some(8.0));
    assert_eq!(p.get("volume").and_then(Value::as_f64), Some(-1.0));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// detect_digit
// ---------------------------------------------------------------------------

#[test]
fn test_detect_digit_builds_and_journals_digit_detect() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-ddig");

    call.detect_digit(json!({"digits": "123", "timeout": 12.0}));

    let p = journal_built(&client, &call, "calling.detect");
    let detect = p.get("detect").expect("detect object");
    assert_eq!(detect.get("type").and_then(Value::as_str), Some("digit"));
    assert_eq!(
        detect
            .get("params")
            .and_then(|dp| dp.get("digits"))
            .and_then(Value::as_str),
        Some("123")
    );
    // timeout rides at the top level, not inside detect.params.
    assert_eq!(p.get("timeout").and_then(Value::as_f64), Some(12.0));
    client.disconnect();
}

#[test]
fn test_detect_digit_empty_params_when_unset() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-ddig-min");

    call.detect_digit(json!({}));

    let p = journal_built(&client, &call, "calling.detect");
    let detect = p.get("detect").unwrap();
    assert_eq!(detect.get("type").and_then(Value::as_str), Some("digit"));
    assert!(detect
        .get("params")
        .and_then(Value::as_object)
        .unwrap()
        .is_empty());
    assert!(p.get("timeout").is_none());
    client.disconnect();
}

// ---------------------------------------------------------------------------
// detect_answering_machine
// ---------------------------------------------------------------------------

#[test]
fn test_detect_answering_machine_builds_only_provided_keys() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-amd");

    call.detect_answering_machine(json!({
        "initial_timeout": 5.0,
        "machine_words_threshold": 6,
        "detect_interruptions": true,
        "timeout": 30.0,
    }));

    let p = journal_built(&client, &call, "calling.detect");
    let detect = p.get("detect").unwrap();
    assert_eq!(detect.get("type").and_then(Value::as_str), Some("machine"));
    let dp = detect.get("params").and_then(Value::as_object).unwrap();
    assert_eq!(dp.get("initial_timeout").and_then(Value::as_f64), Some(5.0));
    assert_eq!(
        dp.get("machine_words_threshold").and_then(Value::as_u64),
        Some(6)
    );
    assert_eq!(
        dp.get("detect_interruptions").and_then(Value::as_bool),
        Some(true)
    );
    // Only-provided-keys: the keys the caller didn't pass are absent.
    assert!(!dp.contains_key("end_silence_timeout"));
    assert!(!dp.contains_key("machine_voice_threshold"));
    assert!(!dp.contains_key("detect_message_end"));
    // timeout is top-level (a detect() arg), not an AMD param.
    assert!(!dp.contains_key("timeout"));
    assert_eq!(p.get("timeout").and_then(Value::as_f64), Some(30.0));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// detect_fax
// ---------------------------------------------------------------------------

#[test]
fn test_detect_fax_builds_and_journals_fax_detect() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-dfax");

    call.detect_fax(json!({"tone": "CED", "timeout": 20.0}));

    let p = journal_built(&client, &call, "calling.detect");
    let detect = p.get("detect").unwrap();
    assert_eq!(detect.get("type").and_then(Value::as_str), Some("fax"));
    assert_eq!(
        detect
            .get("params")
            .and_then(|dp| dp.get("tone"))
            .and_then(Value::as_str),
        Some("CED")
    );
    assert_eq!(p.get("timeout").and_then(Value::as_f64), Some(20.0));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// prompt_tts
// ---------------------------------------------------------------------------

#[test]
fn test_prompt_tts_builds_tts_media_plus_collect() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-prtts");

    call.prompt_tts(
        "Press a digit",
        json!({"digits": {"max": 1}}),
        json!({"voice": "spore", "volume": 0.5}),
    );

    let p = journal_built(&client, &call, "calling.play_and_collect");
    let media = p.get("play").and_then(Value::as_array).unwrap();
    assert_eq!(media[0].get("type").and_then(Value::as_str), Some("tts"));
    let mp = media[0].get("params").unwrap();
    assert_eq!(mp.get("text").and_then(Value::as_str), Some("Press a digit"));
    assert_eq!(mp.get("voice").and_then(Value::as_str), Some("spore"));
    // collect object passes through verbatim.
    assert_eq!(
        p.get("collect")
            .and_then(|c| c.get("digits"))
            .and_then(|d| d.get("max"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(p.get("volume").and_then(Value::as_f64), Some(0.5));
    client.disconnect();
}

// ---------------------------------------------------------------------------
// prompt_audio
// ---------------------------------------------------------------------------

#[test]
fn test_prompt_audio_builds_audio_media_plus_collect() {
    let _g = relay_mocktest::begin();
    let client = relay_mocktest::connected_client(&["default"]);
    let call = answered_inbound_call(&client, "call-praudio");

    call.prompt_audio(
        "https://cdn.example/prompt.wav",
        json!({"speech": {"end_silence_timeout": 1}}),
        json!({"volume": -4.0}),
    );

    let p = journal_built(&client, &call, "calling.play_and_collect");
    let media = p.get("play").and_then(Value::as_array).unwrap();
    assert_eq!(media[0].get("type").and_then(Value::as_str), Some("audio"));
    assert_eq!(
        media[0]
            .get("params")
            .and_then(|mp| mp.get("url"))
            .and_then(Value::as_str),
        Some("https://cdn.example/prompt.wav")
    );
    assert_eq!(
        p.get("collect")
            .and_then(|c| c.get("speech"))
            .and_then(|s| s.get("end_silence_timeout"))
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(p.get("volume").and_then(Value::as_f64), Some(-4.0));
    client.disconnect();
}
