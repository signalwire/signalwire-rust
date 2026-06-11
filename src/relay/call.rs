use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::action::*;
use super::constants;
use super::event::Event;
use crate::logging::Logger;

/// Callback type for call-level event listeners.
pub type CallEventCallback = Arc<dyn Fn(&Event, &Call) + Send + Sync>;

/// Represents a RELAY voice call.
///
/// Holds call-level state, dispatches server events to registered listeners
/// and to in-flight Action objects, and exposes every calling.* RPC method
/// as a first-class Rust method.
pub struct Call {
    // ── identity ──────────────────────────────────────────────────────
    pub call_id: Option<String>,
    pub node_id: Option<String>,
    pub tag: Option<String>,

    // ── state ─────────────────────────────────────────────────────────
    pub state: Mutex<String>,
    pub device: Mutex<Value>,
    pub peer: Mutex<Value>,
    pub end_reason: Mutex<Option<String>>,
    pub context: Option<String>,
    pub dial_winner: Mutex<bool>,

    // ── in-flight actions (control_id -> Action) ──────────────────────
    pub actions: Mutex<HashMap<String, Arc<Action>>>,

    // ── event listeners ───────────────────────────────────────────────
    on_event_callbacks: Mutex<Vec<CallEventCallback>>,

    // ── commands sent (for testing without a real client) ─────────────
    pub sent_commands: Mutex<Vec<(String, Value)>>,

    logger: Logger,
}

impl Call {
    /// Construct a Call from a server params object.
    pub fn new(params: &Value) -> Self {
        Call {
            call_id: params
                .get("call_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            node_id: params
                .get("node_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            tag: params
                .get("tag")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            state: Mutex::new(
                params
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("created")
                    .to_string(),
            ),
            device: Mutex::new(
                params.get("device").cloned().unwrap_or(Value::Object(Default::default())),
            ),
            peer: Mutex::new(
                params.get("peer").cloned().unwrap_or(Value::Object(Default::default())),
            ),
            end_reason: Mutex::new(None),
            context: params
                .get("context")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            dial_winner: Mutex::new(false),
            actions: Mutex::new(HashMap::new()),
            on_event_callbacks: Mutex::new(Vec::new()),
            sent_commands: Mutex::new(Vec::new()),
            logger: Logger::new("relay.call"),
        }
    }

    /// Current call state.
    pub fn current_state(&self) -> String {
        self.state.lock().unwrap().clone()
    }

    /// Current call state as a typed [`CallState`].
    ///
    /// The typed counterpart to [`current_state`](Call::current_state): both
    /// read the same underlying string, so
    /// `call.call_state().as_str() == call.current_state()` always holds. An
    /// unrecognised server value parses to [`CallState::Other`] rather than
    /// panicking. Enables `match call.call_state() { CallState::Ended => …, … }`
    /// and `call.call_state().is_terminal()` instead of stringly comparisons.
    #[must_use]
    pub fn call_state(&self) -> super::state_enums::CallState {
        super::state_enums::CallState::from_str(&self.state.lock().unwrap())
    }

    /// Python-style `__repr__` (mirrors `Call.__repr__` in the
    /// signalwire-python reference). Returns a string of the form
    /// `Call(call_id=..., state=...)`.
    pub fn repr(&self) -> String {
        format!(
            "Call(call_id={:?}, state={:?})",
            self.call_id, self.current_state()
        )
    }

    // ------------------------------------------------------------------
    // Event dispatch
    // ------------------------------------------------------------------

    /// Central event router invoked by the Client whenever a server event
    /// targets this call.
    pub fn dispatch_event(&self, event: &Event) {
        let event_type = event.event_type();
        let params = event.params();

        self.logger.debug(&format!("dispatchEvent: {}", event_type));

        // ── call-level state events ──────────────────────────────────
        if event_type == "calling.call.state" {
            if let Some(s) = params.get("state").and_then(|v| v.as_str()) {
                *self.state.lock().unwrap() = s.to_string();
            }
            if let Some(r) = params.get("end_reason").and_then(|v| v.as_str()) {
                *self.end_reason.lock().unwrap() = Some(r.to_string());
            }
            if let Some(p) = params.get("peer") {
                *self.peer.lock().unwrap() = p.clone();
            }

            // Terminal state -- resolve every in-flight action
            if constants::is_call_terminal(&self.state.lock().unwrap()) {
                self.resolve_all_actions();
            }
        }

        // ── connect events carry peer info ───────────────────────────
        if event_type == "calling.call.connect"
            && let Some(p) = params.get("peer") {
                *self.peer.lock().unwrap() = p.clone();
            }

        // ── route by control_id to the owning Action ─────────────────
        if let Some(control_id) = event.control_id() {
            let action = {
                let actions = self.actions.lock().unwrap();
                actions.get(control_id).cloned()
            };
            if let Some(action) = action {
                action.handle_event(event);

                // Check whether the action has reached a terminal state
                if let Some(action_state) = params.get("state").and_then(|v| v.as_str())
                    && constants::is_action_terminal(event_type, action_state) {
                        action.resolve(None);
                        self.actions
                            .lock()
                            .unwrap()
                            .remove(control_id);
                    }
            }
        }

        // ── fire user-registered callbacks ───────────────────────────
        let callbacks = self.on_event_callbacks.lock().unwrap().clone();
        for cb in &callbacks {
            cb(event, self);
        }
    }

    /// Register a generic event listener on this call.
    pub fn on<F: Fn(&Event, &Call) + Send + Sync + 'static>(&self, cb: F) {
        self.on_event_callbacks
            .lock()
            .unwrap()
            .push(Arc::new(cb));
    }

    /// Mark every outstanding action as completed.
    pub fn resolve_all_actions(&self) {
        let mut actions = self.actions.lock().unwrap();
        for (_id, action) in actions.drain() {
            action.resolve(None);
        }
    }

    // ------------------------------------------------------------------
    // Simple RPC methods (fire-and-return)
    // ------------------------------------------------------------------

    pub fn answer(&self) -> Value {
        self.execute("calling.answer", Value::Object(Default::default()))
    }

    pub fn hangup(&self) -> Value {
        self.execute("calling.hangup", Value::Object(Default::default()))
    }

    pub fn pass(&self) -> Value {
        self.execute("calling.pass", Value::Object(Default::default()))
    }

    pub fn connect(&self, params: Value) -> Value {
        self.execute("calling.connect", params)
    }

    pub fn disconnect(&self) -> Value {
        self.execute("calling.disconnect", Value::Object(Default::default()))
    }

    pub fn hold(&self) -> Value {
        self.execute("calling.hold", Value::Object(Default::default()))
    }

    pub fn unhold(&self) -> Value {
        self.execute("calling.unhold", Value::Object(Default::default()))
    }

    pub fn denoise(&self) -> Value {
        self.execute("calling.denoise", Value::Object(Default::default()))
    }

    pub fn denoise_stop(&self) -> Value {
        self.execute("calling.denoise.stop", Value::Object(Default::default()))
    }

    pub fn transfer(&self, params: Value) -> Value {
        self.execute("calling.transfer", params)
    }

    pub fn join_conference(&self, params: Value) -> Value {
        self.execute("calling.conference.join", params)
    }

    pub fn leave_conference(&self) -> Value {
        self.execute("calling.conference.leave", Value::Object(Default::default()))
    }

    pub fn echo_call(&self) -> Value {
        self.execute("calling.echo", Value::Object(Default::default()))
    }

    pub fn bind_digit(&self, params: Value) -> Value {
        self.execute("calling.bind_digit", params)
    }

    pub fn clear_digit_bindings(&self) -> Value {
        self.execute("calling.clear_digit_bindings", Value::Object(Default::default()))
    }

    pub fn live_transcribe(&self, params: Value) -> Value {
        self.execute("calling.live_transcribe", params)
    }

    pub fn live_translate(&self, params: Value) -> Value {
        self.execute("calling.live_translate", params)
    }

    pub fn join_room(&self, params: Value) -> Value {
        self.execute("calling.room.join", params)
    }

    pub fn leave_room(&self) -> Value {
        self.execute("calling.room.leave", Value::Object(Default::default()))
    }

    pub fn amazon_bedrock(&self, params: Value) -> Value {
        self.execute("calling.amazon_bedrock", params)
    }

    pub fn ai_message(&self, params: Value) -> Value {
        self.execute("calling.ai.message", params)
    }

    pub fn ai_hold(&self) -> Value {
        self.execute("calling.ai.hold", Value::Object(Default::default()))
    }

    pub fn ai_unhold(&self) -> Value {
        self.execute("calling.ai.unhold", Value::Object(Default::default()))
    }

    pub fn user_event(&self, params: Value) -> Value {
        self.execute("calling.user_event", params)
    }

    pub fn queue_enter(&self, params: Value) -> Value {
        self.execute("calling.queue.enter", params)
    }

    pub fn queue_leave(&self) -> Value {
        self.execute("calling.queue.leave", Value::Object(Default::default()))
    }

    pub fn refer_call(&self, params: Value) -> Value {
        self.execute("calling.refer", params)
    }

    pub fn send_digits(&self, params: Value) -> Value {
        self.execute("calling.send_digits", params)
    }

    // ------------------------------------------------------------------
    // Action methods (return Action objects tracked by control_id)
    // ------------------------------------------------------------------

    pub fn play(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.play", "calling.play.stop", params)
    }

    pub fn record(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.record", "calling.record.stop", params)
    }

    pub fn collect(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.collect", "calling.collect.stop", params)
    }

    pub fn play_and_collect(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.play_and_collect", "calling.collect.stop", params)
    }

    pub fn detect(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.detect", "calling.detect.stop", params)
    }

    pub fn send_fax(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.send_fax", "calling.send_fax.stop", params)
    }

    pub fn receive_fax(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.receive_fax", "calling.receive_fax.stop", params)
    }

    pub fn tap(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.tap", "calling.tap.stop", params)
    }

    pub fn stream(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.stream", "calling.stream.stop", params)
    }

    pub fn pay(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.pay", "calling.pay.stop", params)
    }

    pub fn transcribe(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.transcribe", "calling.transcribe.stop", params)
    }

    pub fn ai(&self, params: Value) -> Arc<Action> {
        self.start_action("calling.ai", "calling.ai.stop", params)
    }

    // ------------------------------------------------------------------
    // Typed audio / detect / prompt convenience wrappers
    //
    // Thin typed wrappers over the generic `play` / `detect` /
    // `play_and_collect` actions above, mirroring the Python reference's
    // `call.play_tts` / `call.detect_digit` / `call.prompt_tts` family.
    // Each builds the EXACT RELAY media/params shape with serde_json and
    // delegates to the matching generic so the emitted wire frame is
    // byte-identical to hand-building the media dict.
    //
    // Required arguments are typed Rust parameters; the keyword-optional
    // arguments (Python's `*, language=None, volume=None, ...`) ride in a
    // trailing `opts: Value` JSON object — the Rust SDK's idiomatic
    // **kwargs stand-in, consistent with every other Call command method.
    // Only keys the caller actually supplies are copied onto the wire, so
    // the only-provided-keys behavior matches Python exactly.
    // ------------------------------------------------------------------

    /// Play text-to-speech. Typed convenience over [`Call::play`].
    ///
    /// Wire shape: `play [{"type":"tts","params":{"text":...,language?,
    /// gender?,voice?}}]` with an optional top-level `volume`.
    /// `opts` may carry `language`, `gender`, `voice` (strings) and
    /// `volume` (number).
    pub fn play_tts(&self, text: &str, opts: Value) -> Arc<Action> {
        let mut tts = serde_json::Map::new();
        tts.insert("text".to_string(), Value::String(text.to_string()));
        for key in ["language", "gender", "voice"] {
            if let Some(v) = opts.get(key) {
                tts.insert(key.to_string(), v.clone());
            }
        }
        let mut params = serde_json::Map::new();
        params.insert(
            "play".to_string(),
            serde_json::json!([{ "type": "tts", "params": Value::Object(tts) }]),
        );
        if let Some(vol) = opts.get("volume") {
            params.insert("volume".to_string(), vol.clone());
        }
        self.play(Value::Object(params))
    }

    /// Play an audio file from a URL. Typed convenience over [`Call::play`].
    ///
    /// Wire shape: `play [{"type":"audio","params":{"url":...}}]` with an
    /// optional top-level `volume` (read from `opts`).
    pub fn play_audio(&self, url: &str, opts: Value) -> Arc<Action> {
        let mut params = serde_json::Map::new();
        params.insert(
            "play".to_string(),
            serde_json::json!([{ "type": "audio", "params": { "url": url } }]),
        );
        if let Some(vol) = opts.get("volume") {
            params.insert("volume".to_string(), vol.clone());
        }
        self.play(Value::Object(params))
    }

    /// Play silence for `duration` seconds. Typed convenience over
    /// [`Call::play`].
    ///
    /// Wire shape: `play [{"type":"silence","params":{"duration":...}}]`.
    pub fn play_silence(&self, duration: f64) -> Arc<Action> {
        self.play(serde_json::json!({
            "play": [{ "type": "silence", "params": { "duration": duration } }]
        }))
    }

    /// Play a named ringtone by country code. Typed convenience over
    /// [`Call::play`].
    ///
    /// Wire shape: `play [{"type":"ringtone","params":{"name":...,
    /// duration?}}]` with an optional top-level `volume`.
    /// `opts` may carry `duration` and `volume` (numbers).
    pub fn play_ringtone(&self, name: &str, opts: Value) -> Arc<Action> {
        let mut rt = serde_json::Map::new();
        rt.insert("name".to_string(), Value::String(name.to_string()));
        if let Some(d) = opts.get("duration") {
            rt.insert("duration".to_string(), d.clone());
        }
        let mut params = serde_json::Map::new();
        params.insert(
            "play".to_string(),
            serde_json::json!([{ "type": "ringtone", "params": Value::Object(rt) }]),
        );
        if let Some(vol) = opts.get("volume") {
            params.insert("volume".to_string(), vol.clone());
        }
        self.play(Value::Object(params))
    }

    /// Detect DTMF digits. Typed convenience over [`Call::detect`].
    ///
    /// Wire shape: `detect {"type":"digit","params":{digits?}}` with an
    /// optional top-level `timeout`. `opts` may carry `digits` (string)
    /// and `timeout` (number).
    pub fn detect_digit(&self, opts: Value) -> Arc<Action> {
        let mut detect_params = serde_json::Map::new();
        if let Some(d) = opts.get("digits") {
            detect_params.insert("digits".to_string(), d.clone());
        }
        let mut params = serde_json::Map::new();
        params.insert(
            "detect".to_string(),
            serde_json::json!({ "type": "digit", "params": Value::Object(detect_params) }),
        );
        if let Some(t) = opts.get("timeout") {
            params.insert("timeout".to_string(), t.clone());
        }
        self.detect(Value::Object(params))
    }

    /// Detect human vs answering machine (AMD). Typed convenience over
    /// [`Call::detect`].
    ///
    /// Wire shape: `detect {"type":"machine","params":{...only-provided...}}`
    /// with an optional top-level `timeout`. `opts` may carry any of
    /// `initial_timeout`, `end_silence_timeout`, `machine_voice_threshold`,
    /// `machine_words_threshold`, `detect_interruptions`,
    /// `detect_message_end`, and `timeout`.
    pub fn detect_answering_machine(&self, opts: Value) -> Arc<Action> {
        let mut detect_params = serde_json::Map::new();
        for key in [
            "initial_timeout",
            "end_silence_timeout",
            "machine_voice_threshold",
            "machine_words_threshold",
            "detect_interruptions",
            "detect_message_end",
        ] {
            if let Some(v) = opts.get(key) {
                detect_params.insert(key.to_string(), v.clone());
            }
        }
        let mut params = serde_json::Map::new();
        params.insert(
            "detect".to_string(),
            serde_json::json!({ "type": "machine", "params": Value::Object(detect_params) }),
        );
        if let Some(t) = opts.get("timeout") {
            params.insert("timeout".to_string(), t.clone());
        }
        self.detect(Value::Object(params))
    }

    /// Detect a fax tone (CED/CNG). Typed convenience over [`Call::detect`].
    ///
    /// Wire shape: `detect {"type":"fax","params":{tone?}}` with an optional
    /// top-level `timeout`. `opts` may carry `tone` (string) and `timeout`
    /// (number).
    pub fn detect_fax(&self, opts: Value) -> Arc<Action> {
        let mut detect_params = serde_json::Map::new();
        if let Some(tone) = opts.get("tone") {
            detect_params.insert("tone".to_string(), tone.clone());
        }
        let mut params = serde_json::Map::new();
        params.insert(
            "detect".to_string(),
            serde_json::json!({ "type": "fax", "params": Value::Object(detect_params) }),
        );
        if let Some(t) = opts.get("timeout") {
            params.insert("timeout".to_string(), t.clone());
        }
        self.detect(Value::Object(params))
    }

    /// Play TTS then collect input. Typed media over
    /// [`Call::play_and_collect`].
    ///
    /// Wire shape: `play_and_collect [{"type":"tts","params":{"text":...,
    /// language?,gender?,voice?}}]` with the given `collect` object and an
    /// optional top-level `volume`. `opts` may carry `language`, `gender`,
    /// `voice` (strings) and `volume` (number).
    pub fn prompt_tts(&self, text: &str, collect: Value, opts: Value) -> Arc<Action> {
        let mut tts = serde_json::Map::new();
        tts.insert("text".to_string(), Value::String(text.to_string()));
        for key in ["language", "gender", "voice"] {
            if let Some(v) = opts.get(key) {
                tts.insert(key.to_string(), v.clone());
            }
        }
        let mut params = serde_json::Map::new();
        params.insert(
            "play".to_string(),
            serde_json::json!([{ "type": "tts", "params": Value::Object(tts) }]),
        );
        params.insert("collect".to_string(), collect);
        if let Some(vol) = opts.get("volume") {
            params.insert("volume".to_string(), vol.clone());
        }
        self.play_and_collect(Value::Object(params))
    }

    /// Play an audio file then collect input. Typed media over
    /// [`Call::play_and_collect`].
    ///
    /// Wire shape: `play_and_collect [{"type":"audio","params":{"url":...}}]`
    /// with the given `collect` object and an optional top-level `volume`.
    /// `opts` may carry `volume` (number).
    pub fn prompt_audio(&self, url: &str, collect: Value, opts: Value) -> Arc<Action> {
        let mut params = serde_json::Map::new();
        params.insert(
            "play".to_string(),
            serde_json::json!([{ "type": "audio", "params": { "url": url } }]),
        );
        params.insert("collect".to_string(), collect);
        if let Some(vol) = opts.get("volume") {
            params.insert("volume".to_string(), vol.clone());
        }
        self.play_and_collect(Value::Object(params))
    }

    // ------------------------------------------------------------------
    // State-wait convenience (wait_for_answered / wait_for_ringing /
    // wait_for_ending) is intentionally NOT implemented.
    //
    // Python's wait_for_* helpers are built on a generic async
    // `Call.wait_for(event_type, predicate)` event-bus primitive that
    // suspends a coroutine until a matching `calling.call.state` event
    // arrives. The Rust Call deliberately has no such async wait
    // primitive (see PORT_OMISSIONS.md for `Call.wait_for` /
    // `Call.wait_for_ended`): it is a synchronous command surface, and
    // state observation is done through `Call::on` callbacks
    // (register_event_callback) which fire as state events are
    // dispatched. With no `wait_for` to short-circuit against, the three
    // state waiters have nothing to delegate to, so they are documented
    // as surface omissions in PORT_OMISSIONS.md rather than stubbed.
    // ------------------------------------------------------------------

    // ------------------------------------------------------------------
    // Private helpers
    // ------------------------------------------------------------------

    fn base_params(&self) -> Value {
        serde_json::json!({
            "node_id": self.node_id,
            "call_id": self.call_id,
        })
    }

    /// Send a simple (non-action) RPC call.
    fn execute(&self, method: &str, extra: Value) -> Value {
        let mut base = self.base_params();
        if let (Some(base_map), Some(extra_map)) =
            (base.as_object_mut(), extra.as_object())
        {
            for (k, v) in extra_map {
                base_map.insert(k.clone(), v.clone());
            }
        }
        self.sent_commands
            .lock()
            .unwrap()
            .push((method.to_string(), base.clone()));
        base
    }

    /// Spin up a long-running action tracked by a unique control_id.
    fn start_action(
        &self,
        method: &str,
        stop_method: &str,
        extra: Value,
    ) -> Arc<Action> {
        let control_id = generate_uuid();
        let call_id = self.call_id.as_deref().unwrap_or("");
        let node_id = self.node_id.as_deref().unwrap_or("");

        let action = Arc::new(Action::with_stop_method(
            &control_id, call_id, node_id, stop_method,
        ));

        self.actions
            .lock()
            .unwrap()
            .insert(control_id.clone(), action.clone());

        let mut base = self.base_params();
        if let Some(base_map) = base.as_object_mut() {
            base_map.insert("control_id".to_string(), Value::String(control_id));
            if let Some(extra_map) = extra.as_object() {
                for (k, v) in extra_map {
                    base_map.insert(k.clone(), v.clone());
                }
            }
        }
        self.sent_commands
            .lock()
            .unwrap()
            .push((method.to_string(), base));

        action
    }
}

/// Generate a simple UUID v4.
fn generate_uuid() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    let mut data = [0u8; 16];
    rng.fill(&mut data);
    data[6] = (data[6] & 0x0f) | 0x40; // version 4
    data[8] = (data[8] & 0x3f) | 0x80; // variant RFC 4122
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
        u16::from_be_bytes([data[4], data[5]]),
        u16::from_be_bytes([data[6], data[7]]),
        u16::from_be_bytes([data[8], data[9]]),
        // 6 bytes -> 48-bit integer
        ((data[10] as u64) << 40)
            | ((data[11] as u64) << 32)
            | ((data[12] as u64) << 24)
            | ((data[13] as u64) << 16)
            | ((data[14] as u64) << 8)
            | (data[15] as u64),
    )
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_call() -> Call {
        Call::new(&json!({
            "call_id": "call-1",
            "node_id": "node-1",
            "tag": "tag-1",
            "context": "default",
            "state": "created",
        }))
    }

    fn make_event(event_type: &str, params: Value) -> Event {
        Event::parse(event_type, &params)
    }

    #[test]
    fn test_call_new() {
        let call = make_call();
        assert_eq!(call.call_id, Some("call-1".to_string()));
        assert_eq!(call.node_id, Some("node-1".to_string()));
        assert_eq!(call.tag, Some("tag-1".to_string()));
        assert_eq!(call.context, Some("default".to_string()));
        assert_eq!(call.current_state(), "created");
    }

    #[test]
    fn test_call_minimal() {
        let call = Call::new(&json!({}));
        assert!(call.call_id.is_none());
        assert_eq!(call.current_state(), "created");
    }

    #[test]
    fn test_dispatch_state_event() {
        let call = make_call();
        let ev = make_event("calling.call.state", json!({"state": "ringing"}));
        call.dispatch_event(&ev);
        assert_eq!(call.current_state(), "ringing");
    }

    #[test]
    fn test_dispatch_ended_resolves_actions() {
        let call = make_call();
        let action = call.play(json!({}));
        assert!(!action.is_done());

        let ev = make_event("calling.call.state", json!({"state": "ended"}));
        call.dispatch_event(&ev);
        assert!(action.is_done());
        assert!(call.actions.lock().unwrap().is_empty());
    }

    #[test]
    fn test_dispatch_end_reason() {
        let call = make_call();
        let ev = make_event(
            "calling.call.state",
            json!({"state": "ended", "end_reason": "hangup"}),
        );
        call.dispatch_event(&ev);
        assert_eq!(
            *call.end_reason.lock().unwrap(),
            Some("hangup".to_string())
        );
    }

    #[test]
    fn test_dispatch_connect_peer() {
        let call = make_call();
        let ev = make_event(
            "calling.call.connect",
            json!({"peer": {"call_id": "peer-1"}}),
        );
        call.dispatch_event(&ev);
        assert_eq!(call.peer.lock().unwrap()["call_id"], "peer-1");
    }

    #[test]
    fn test_dispatch_action_event() {
        let call = make_call();
        let action = call.play(json!({}));
        let control_id = action.control_id().to_string();

        let ev = make_event(
            "calling.call.play",
            json!({"control_id": control_id, "state": "playing"}),
        );
        call.dispatch_event(&ev);
        assert_eq!(action.state(), Some("playing".to_string()));
    }

    #[test]
    fn test_dispatch_action_terminal() {
        let call = make_call();
        let action = call.play(json!({}));
        let control_id = action.control_id().to_string();

        let ev = make_event(
            "calling.call.play",
            json!({"control_id": control_id, "state": "finished"}),
        );
        call.dispatch_event(&ev);
        assert!(action.is_done());
        assert!(call.actions.lock().unwrap().is_empty());
    }

    #[test]
    fn test_on_event_listener() {
        let call = make_call();
        let count = Arc::new(Mutex::new(0u32));
        let count2 = count.clone();
        call.on(move |_, _| {
            *count2.lock().unwrap() += 1;
        });

        let ev = make_event("calling.call.state", json!({"state": "ringing"}));
        call.dispatch_event(&ev);
        assert_eq!(*count.lock().unwrap(), 1);
    }

    // -- Simple method tests --

    #[test]
    fn test_simple_methods_send_commands() {
        let call = make_call();

        call.answer();
        call.hangup();
        call.pass();
        call.hold();
        call.unhold();
        call.denoise();
        call.denoise_stop();
        call.disconnect();
        call.echo_call();
        call.leave_conference();
        call.leave_room();
        call.ai_hold();
        call.ai_unhold();
        call.queue_leave();
        call.clear_digit_bindings();

        let cmds = call.sent_commands.lock().unwrap();
        assert_eq!(cmds.len(), 15);
        assert_eq!(cmds[0].0, "calling.answer");
        assert_eq!(cmds[1].0, "calling.hangup");
        assert_eq!(cmds[2].0, "calling.pass");
        assert_eq!(cmds[3].0, "calling.hold");
        assert_eq!(cmds[4].0, "calling.unhold");
    }

    #[test]
    fn test_parameterized_methods() {
        let call = make_call();

        call.connect(json!({"to": "+15551001000"}));
        call.transfer(json!({"dest": "sip:foo@bar"}));
        call.join_conference(json!({"name": "room1"}));
        call.bind_digit(json!({"digits": "*"}));
        call.send_digits(json!({"digits": "1234"}));

        let cmds = call.sent_commands.lock().unwrap();
        assert_eq!(cmds.len(), 5);
        assert_eq!(cmds[0].0, "calling.connect");
        assert_eq!(cmds[0].1["to"], "+15551001000");
    }

    // -- Action method tests --

    #[test]
    fn test_play_creates_action() {
        let call = make_call();
        let action = call.play(json!({"url": "http://example.com/audio.mp3"}));
        assert!(!action.is_done());
        assert_eq!(action.stop_method(), "calling.play.stop");
        assert_eq!(call.actions.lock().unwrap().len(), 1);

        let cmds = call.sent_commands.lock().unwrap();
        assert_eq!(cmds[0].0, "calling.play");
        assert!(cmds[0].1.get("control_id").is_some());
    }

    #[test]
    fn test_record_creates_action() {
        let call = make_call();
        let action = call.record(json!({}));
        assert_eq!(action.stop_method(), "calling.record.stop");
    }

    #[test]
    fn test_collect_creates_action() {
        let call = make_call();
        let action = call.collect(json!({}));
        assert_eq!(action.stop_method(), "calling.collect.stop");
    }

    #[test]
    fn test_detect_creates_action() {
        let call = make_call();
        let action = call.detect(json!({}));
        assert_eq!(action.stop_method(), "calling.detect.stop");
    }

    #[test]
    fn test_tap_creates_action() {
        let call = make_call();
        let action = call.tap(json!({}));
        assert_eq!(action.stop_method(), "calling.tap.stop");
    }

    #[test]
    fn test_stream_creates_action() {
        let call = make_call();
        let action = call.stream(json!({}));
        assert_eq!(action.stop_method(), "calling.stream.stop");
    }

    #[test]
    fn test_pay_creates_action() {
        let call = make_call();
        let action = call.pay(json!({}));
        assert_eq!(action.stop_method(), "calling.pay.stop");
    }

    #[test]
    fn test_transcribe_creates_action() {
        let call = make_call();
        let action = call.transcribe(json!({}));
        assert_eq!(action.stop_method(), "calling.transcribe.stop");
    }

    #[test]
    fn test_ai_creates_action() {
        let call = make_call();
        let action = call.ai(json!({}));
        assert_eq!(action.stop_method(), "calling.ai.stop");
    }

    #[test]
    fn test_send_fax_creates_action() {
        let call = make_call();
        let action = call.send_fax(json!({}));
        assert_eq!(action.stop_method(), "calling.send_fax.stop");
    }

    #[test]
    fn test_receive_fax_creates_action() {
        let call = make_call();
        let action = call.receive_fax(json!({}));
        assert_eq!(action.stop_method(), "calling.receive_fax.stop");
    }

    #[test]
    fn test_play_and_collect_creates_action() {
        let call = make_call();
        let action = call.play_and_collect(json!({}));
        assert_eq!(action.stop_method(), "calling.collect.stop");
    }

    // -- Typed convenience wrapper tests (built media/params shapes) --

    /// Pull the single command a convenience wrapper recorded.
    fn one_built(call: &Call, expect_method: &str) -> Value {
        let cmds = call.sent_commands.lock().unwrap();
        assert_eq!(cmds.len(), 1, "expected exactly one built command");
        assert_eq!(cmds[0].0, expect_method);
        cmds[0].1.clone()
    }

    #[test]
    fn test_play_tts_builds_tts_media() {
        let call = make_call();
        let action = call.play_tts(
            "Hi there",
            json!({"language": "en-US", "gender": "male", "voice": "spore", "volume": -2.0}),
        );
        assert_eq!(action.stop_method(), "calling.play.stop");
        let p = one_built(&call, "calling.play");
        let media = &p["play"][0];
        assert_eq!(media["type"], "tts");
        assert_eq!(media["params"]["text"], "Hi there");
        assert_eq!(media["params"]["language"], "en-US");
        assert_eq!(media["params"]["gender"], "male");
        assert_eq!(media["params"]["voice"], "spore");
        // volume is top-level, not inside the tts params.
        assert_eq!(p["volume"], -2.0);
        assert!(media["params"].get("volume").is_none());
    }

    #[test]
    fn test_play_tts_omits_unset_optionals() {
        let call = make_call();
        call.play_tts("Bare", json!({}));
        let p = one_built(&call, "calling.play");
        let params = &p["play"][0]["params"];
        assert_eq!(params["text"], "Bare");
        assert!(params.get("language").is_none());
        assert!(params.get("gender").is_none());
        assert!(params.get("voice").is_none());
        assert!(p.get("volume").is_none());
    }

    #[test]
    fn test_play_audio_builds_audio_media() {
        let call = make_call();
        call.play_audio("https://x/a.mp3", json!({"volume": 1.5}));
        let p = one_built(&call, "calling.play");
        assert_eq!(p["play"][0]["type"], "audio");
        assert_eq!(p["play"][0]["params"]["url"], "https://x/a.mp3");
        assert_eq!(p["volume"], 1.5);
    }

    #[test]
    fn test_play_silence_builds_silence_media() {
        let call = make_call();
        call.play_silence(3.0);
        let p = one_built(&call, "calling.play");
        assert_eq!(p["play"][0]["type"], "silence");
        assert_eq!(p["play"][0]["params"]["duration"], 3.0);
    }

    #[test]
    fn test_play_ringtone_builds_ringtone_media() {
        let call = make_call();
        call.play_ringtone("us", json!({"duration": 5.0, "volume": -1.0}));
        let p = one_built(&call, "calling.play");
        assert_eq!(p["play"][0]["type"], "ringtone");
        assert_eq!(p["play"][0]["params"]["name"], "us");
        assert_eq!(p["play"][0]["params"]["duration"], 5.0);
        assert_eq!(p["volume"], -1.0);
    }

    #[test]
    fn test_play_ringtone_omits_unset_duration() {
        let call = make_call();
        call.play_ringtone("de", json!({}));
        let p = one_built(&call, "calling.play");
        assert_eq!(p["play"][0]["params"]["name"], "de");
        assert!(p["play"][0]["params"].get("duration").is_none());
        assert!(p.get("volume").is_none());
    }

    #[test]
    fn test_detect_digit_builds_digit_detect() {
        let call = make_call();
        let action = call.detect_digit(json!({"digits": "42", "timeout": 10.0}));
        assert_eq!(action.stop_method(), "calling.detect.stop");
        let p = one_built(&call, "calling.detect");
        assert_eq!(p["detect"]["type"], "digit");
        assert_eq!(p["detect"]["params"]["digits"], "42");
        assert_eq!(p["timeout"], 10.0);
    }

    #[test]
    fn test_detect_digit_empty_params() {
        let call = make_call();
        call.detect_digit(json!({}));
        let p = one_built(&call, "calling.detect");
        assert_eq!(p["detect"]["type"], "digit");
        assert!(p["detect"]["params"].as_object().unwrap().is_empty());
        assert!(p.get("timeout").is_none());
    }

    #[test]
    fn test_detect_answering_machine_only_provided_keys() {
        let call = make_call();
        call.detect_answering_machine(json!({
            "initial_timeout": 5.0,
            "machine_words_threshold": 6,
            "detect_message_end": false,
            "timeout": 25.0,
        }));
        let p = one_built(&call, "calling.detect");
        assert_eq!(p["detect"]["type"], "machine");
        let dp = p["detect"]["params"].as_object().unwrap();
        assert_eq!(dp["initial_timeout"], 5.0);
        assert_eq!(dp["machine_words_threshold"], 6);
        assert_eq!(dp["detect_message_end"], false);
        assert!(!dp.contains_key("end_silence_timeout"));
        assert!(!dp.contains_key("machine_voice_threshold"));
        assert!(!dp.contains_key("detect_interruptions"));
        // timeout is the top-level detect() arg, not an AMD param.
        assert!(!dp.contains_key("timeout"));
        assert_eq!(p["timeout"], 25.0);
    }

    #[test]
    fn test_detect_fax_builds_fax_detect() {
        let call = make_call();
        call.detect_fax(json!({"tone": "CNG"}));
        let p = one_built(&call, "calling.detect");
        assert_eq!(p["detect"]["type"], "fax");
        assert_eq!(p["detect"]["params"]["tone"], "CNG");
        assert!(p.get("timeout").is_none());
    }

    #[test]
    fn test_prompt_tts_builds_tts_media_plus_collect() {
        let call = make_call();
        let action = call.prompt_tts(
            "Pick one",
            json!({"digits": {"max": 1}}),
            json!({"voice": "spore", "volume": 0.5}),
        );
        assert_eq!(action.stop_method(), "calling.collect.stop");
        let p = one_built(&call, "calling.play_and_collect");
        assert_eq!(p["play"][0]["type"], "tts");
        assert_eq!(p["play"][0]["params"]["text"], "Pick one");
        assert_eq!(p["play"][0]["params"]["voice"], "spore");
        assert_eq!(p["collect"]["digits"]["max"], 1);
        assert_eq!(p["volume"], 0.5);
    }

    #[test]
    fn test_prompt_audio_builds_audio_media_plus_collect() {
        let call = make_call();
        call.prompt_audio(
            "https://x/p.wav",
            json!({"speech": {"end_silence_timeout": 1}}),
            json!({}),
        );
        let p = one_built(&call, "calling.play_and_collect");
        assert_eq!(p["play"][0]["type"], "audio");
        assert_eq!(p["play"][0]["params"]["url"], "https://x/p.wav");
        assert_eq!(p["collect"]["speech"]["end_silence_timeout"], 1);
        // volume omitted when not supplied.
        assert!(p.get("volume").is_none());
    }

    #[test]
    fn test_resolve_all_actions() {
        let call = make_call();
        let a1 = call.play(json!({}));
        let a2 = call.record(json!({}));
        assert!(!a1.is_done());
        assert!(!a2.is_done());

        call.resolve_all_actions();
        assert!(a1.is_done());
        assert!(a2.is_done());
        assert!(call.actions.lock().unwrap().is_empty());
    }

    #[test]
    fn test_generate_uuid_format() {
        let uuid = generate_uuid();
        // Should be 8-4-4-4-12 format
        let parts: Vec<&str> = uuid.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }

    #[test]
    fn test_generate_uuid_uniqueness() {
        let a = generate_uuid();
        let b = generate_uuid();
        assert_ne!(a, b);
    }
}
