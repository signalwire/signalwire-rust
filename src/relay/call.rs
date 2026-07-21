use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

use serde_json::Value;

use super::action::Action;
use super::client::Client;
use super::constants;
use super::error::RelayError;
use super::event::Event;
use crate::logging::Logger;

/// Callback type for call-level event listeners.
pub type CallEventCallback = Arc<dyn Fn(&Event, &Call) + Send + Sync>;

/// Predicate for [`Call::wait_for`] — returns `true` for an event that
/// satisfies the caller's filter.
pub type EventPredicate = Arc<dyn Fn(&Event) -> bool + Send + Sync>;

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

    // ── commands sent (for wire-frame inspection / tests) ─────────────
    // Internal field so callers can't mutate it; read via
    // `Call::sent_commands`. Bounded at `crate::relay::SENT_LOG_CAP`.
    pub(crate) sent_commands: Mutex<Vec<(String, Value)>>,

    // ── owning client, for transmitting frames to the wire ────────────
    // A `Weak` back-reference (the Client owns the Call via `Arc`, so a
    // strong ref here would leak). When present, every `calling.*` frame a
    // verb builds is transmitted through `Client::send_request` — the same
    // client-send boundary the cross-port relay differ observes. When absent
    // (a bare `Call::new` with no client), frames are only recorded in
    // `sent_commands`. This mirrors Python's `Call._client.execute`.
    client: Mutex<Option<Weak<Client>>>,

    logger: Logger,
}

impl Call {
    /// Construct a Call from a server params object.
    pub fn new(params: &Value) -> Self {
        Call {
            call_id: params
                .get("call_id")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            node_id: params
                .get("node_id")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            tag: params
                .get("tag")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            state: Mutex::new(
                params
                    .get("call_state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("created")
                    .to_string(),
            ),
            device: Mutex::new(
                params
                    .get("device")
                    .cloned()
                    .unwrap_or(Value::Object(serde_json::Map::new())),
            ),
            peer: Mutex::new(
                params
                    .get("peer")
                    .cloned()
                    .unwrap_or(Value::Object(serde_json::Map::new())),
            ),
            end_reason: Mutex::new(None),
            context: params
                .get("context")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            dial_winner: Mutex::new(false),
            actions: Mutex::new(HashMap::new()),
            on_event_callbacks: Mutex::new(Vec::new()),
            sent_commands: Mutex::new(Vec::new()),
            client: Mutex::new(None),
            logger: Logger::new("relay.call"),
        }
    }

    /// Attach the owning [`Client`] so this Call's verbs transmit their
    /// frames to the wire (via [`Client::send_request`]) instead of only
    /// recording them in `sent_commands`. Stored as a `Weak` — the Client
    /// owns the Call, so a strong ref would form a reference cycle. Called by
    /// the Client immediately after it constructs a Call. Mirrors Python,
    /// where a Call always carries `self._client`.
    ///
    /// # Panics
    /// Panics if the internal mutex is poisoned (another thread panicked while
    /// holding the lock). This does not occur under normal operation.
    pub fn set_client(&self, client: &Arc<Client>) {
        *self.client.lock().unwrap() = Some(Arc::downgrade(client));
        // Actions already created (none yet at attach time) would need the
        // same wiring; new actions inherit it via `start_action`.
    }

    /// The live owning [`Client`], if one is attached and still alive.
    fn client(&self) -> Option<Arc<Client>> {
        self.client.lock().unwrap().as_ref().and_then(Weak::upgrade)
    }

    /// Record a `(method, params)` frame into the bounded `sent_commands` log,
    /// dropping the oldest entry once [`crate::relay::SENT_LOG_CAP`] frames are
    /// retained so a long-running call cannot grow the log without limit.
    /// Private (not `pub`) — internal plumbing, not public surface.
    fn record_command(&self, method: &str, params: Value) {
        let mut cmds = self.sent_commands.lock().unwrap();
        if cmds.len() >= crate::relay::SENT_LOG_CAP {
            cmds.remove(0);
        }
        cmds.push((method.to_string(), params));
    }

    /// Snapshot of the `(method, params)` frames this call has built, for
    /// wire-frame inspection / tests. Bounded to the most recent
    /// [`crate::relay::SENT_LOG_CAP`] frames.
    ///
    /// # Panics
    /// Panics if an internal mutex is poisoned (another thread panicked while
    /// holding the lock). This does not occur under normal operation.
    #[must_use]
    pub fn sent_commands(&self) -> Vec<(String, Value)> {
        self.sent_commands.lock().unwrap().clone()
    }

    /// Current call state.
    ///
    /// # Panics
    /// Panics if an internal mutex is poisoned (i.e. another thread panicked
    /// while holding the lock). This does not occur under normal operation.
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
    ///
    /// # Panics
    /// Panics if an internal mutex is poisoned (i.e. another thread panicked
    /// while holding the lock). This does not occur under normal operation.
    pub fn call_state(&self) -> super::state_enums::CallState {
        super::state_enums::CallState::from_str(&self.state.lock().unwrap())
    }

    /// Python-style `__repr__` (mirrors `Call.__repr__` in the
    /// signalwire-python reference). Returns a string of the form
    /// `Call(call_id=..., state=...)`.
    pub fn repr(&self) -> String {
        format!(
            "Call(call_id={:?}, state={:?})",
            self.call_id,
            self.current_state()
        )
    }

    // ------------------------------------------------------------------
    // Event dispatch
    // ------------------------------------------------------------------

    /// Central event router invoked by the Client whenever a server event
    /// targets this call.
    ///
    /// # Panics
    /// Panics if an internal mutex is poisoned (i.e. another thread panicked
    /// while holding the lock). This does not occur under normal operation.
    pub fn dispatch_event(&self, event: &Event) {
        let event_type = event.event_type();
        let params = event.params();

        self.logger.debug(&format!("dispatchEvent: {event_type}"));

        // ── call-level state events ──────────────────────────────────
        if event_type == "calling.call.state" {
            if let Some(s) = params.get("call_state").and_then(|v| v.as_str()) {
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
            && let Some(p) = params.get("peer")
        {
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
                    && constants::is_action_terminal(event_type, action_state)
                {
                    action.resolve(None);
                    self.actions.lock().unwrap().remove(control_id);
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
    ///
    /// # Panics
    /// Panics if an internal mutex is poisoned (i.e. another thread panicked
    /// while holding the lock). This does not occur under normal operation.
    pub fn on<F: Fn(&Event, &Call) + Send + Sync + 'static>(&self, cb: F) {
        self.on_event_callbacks.lock().unwrap().push(Arc::new(cb));
    }

    /// Mark every outstanding action as completed.
    ///
    /// # Panics
    /// Panics if an internal mutex is poisoned (i.e. another thread panicked
    /// while holding the lock). This does not occur under normal operation.
    pub fn resolve_all_actions(&self) {
        let mut actions = self.actions.lock().unwrap();
        for (_id, action) in actions.drain() {
            action.resolve(None);
        }
    }

    // ------------------------------------------------------------------
    // Simple RPC methods (fire-and-return)
    // ------------------------------------------------------------------

    pub fn answer(&self) -> Result<Value, RelayError> {
        self.execute("calling.answer", Value::Object(serde_json::Map::new()))
    }

    pub fn hangup(&self) -> Result<Value, RelayError> {
        self.execute("calling.hangup", Value::Object(serde_json::Map::new()))
    }

    pub fn pass(&self) -> Result<Value, RelayError> {
        self.execute("calling.pass", Value::Object(serde_json::Map::new()))
    }

    pub fn connect(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.connect", params)
    }

    pub fn disconnect(&self) -> Result<Value, RelayError> {
        self.execute("calling.disconnect", Value::Object(serde_json::Map::new()))
    }

    pub fn hold(&self) -> Result<Value, RelayError> {
        self.execute("calling.hold", Value::Object(serde_json::Map::new()))
    }

    pub fn unhold(&self) -> Result<Value, RelayError> {
        self.execute("calling.unhold", Value::Object(serde_json::Map::new()))
    }

    pub fn denoise(&self) -> Result<Value, RelayError> {
        self.execute("calling.denoise", Value::Object(serde_json::Map::new()))
    }

    pub fn denoise_stop(&self) -> Result<Value, RelayError> {
        self.execute(
            "calling.denoise.stop",
            Value::Object(serde_json::Map::new()),
        )
    }

    pub fn transfer(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.transfer", params)
    }

    pub fn join_conference(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.conference.join", params)
    }

    pub fn leave_conference(&self) -> Result<Value, RelayError> {
        self.execute(
            "calling.conference.leave",
            Value::Object(serde_json::Map::new()),
        )
    }

    pub fn echo_call(&self) -> Result<Value, RelayError> {
        self.execute("calling.echo", Value::Object(serde_json::Map::new()))
    }

    pub fn bind_digit(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.bind_digit", params)
    }

    pub fn clear_digit_bindings(&self) -> Result<Value, RelayError> {
        self.execute(
            "calling.clear_digit_bindings",
            Value::Object(serde_json::Map::new()),
        )
    }

    /// Start or stop live transcription on the call.
    ///
    /// The wire schema (`relay-protocol/calling.live_transcribe.params.json`)
    /// requires `params.action` -- the caller's `action` value must be
    /// wrapped, not forwarded flat.
    pub fn live_transcribe(&self, action: Value) -> Result<Value, RelayError> {
        let params = serde_json::json!({ "action": action });
        self.execute("calling.live_transcribe", params)
    }

    /// Start or stop live translation on the call.
    ///
    /// The wire schema (`relay-protocol/calling.live_translate.params.json`)
    /// requires `params.action` -- the caller's `action` value must be
    /// wrapped, not forwarded flat. `status_url` is an optional sibling param.
    pub fn live_translate(
        &self,
        action: Value,
        status_url: Option<&str>,
    ) -> Result<Value, RelayError> {
        let mut params = serde_json::json!({ "action": action });
        if let Some(url) = status_url {
            params["status_url"] = Value::String(url.to_string());
        }
        self.execute("calling.live_translate", params)
    }

    pub fn join_room(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.room.join", params)
    }

    pub fn leave_room(&self) -> Result<Value, RelayError> {
        self.execute("calling.room.leave", Value::Object(serde_json::Map::new()))
    }

    pub fn amazon_bedrock(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.amazon_bedrock", params)
    }

    pub fn ai_message(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.ai.message", params)
    }

    pub fn ai_hold(&self) -> Result<Value, RelayError> {
        self.execute("calling.ai.hold", Value::Object(serde_json::Map::new()))
    }

    pub fn ai_unhold(&self) -> Result<Value, RelayError> {
        self.execute("calling.ai.unhold", Value::Object(serde_json::Map::new()))
    }

    pub fn user_event(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.user_event", params)
    }

    pub fn queue_enter(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.queue.enter", params)
    }

    pub fn queue_leave(&self) -> Result<Value, RelayError> {
        self.execute("calling.queue.leave", Value::Object(serde_json::Map::new()))
    }

    pub fn refer_call(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.refer", params)
    }

    // ------------------------------------------------------------------
    // Python-parity command names
    //
    // These mirror the exact method names on the Python `Call`
    // (`echo` / `pass_` / `refer`), which collide with Rust reserved or
    // conventional names. They emit the identical wire RPC as their
    // `*_call` / `pass` counterparts above; the two coexist so both the
    // Rust-idiom name and the Python-parity name resolve.
    //   - `pass_` matches Python's own reserved-word escape (Python's
    //     method is literally `pass_`), so no rename-table entry is
    //     needed — the escaped name is identical on both sides.
    // ------------------------------------------------------------------

    /// Echo audio back to the caller (mirrors Python `Call.echo`).
    /// Emits `calling.echo`. Optional `timeout` / `status_url` may be
    /// supplied via `params`; pass `Value::Null` or an empty object for none.
    pub fn echo(&self, params: Value) -> Result<Value, RelayError> {
        let extra = if params.is_object() {
            params
        } else {
            Value::Object(serde_json::Map::new())
        };
        self.execute("calling.echo", extra)
    }

    /// Decline control of an inbound call, returning it to routing
    /// (mirrors Python `Call.pass_`). Emits `calling.pass`.
    pub fn pass_(&self) -> Result<Value, RelayError> {
        self.execute("calling.pass", Value::Object(serde_json::Map::new()))
    }

    /// Transfer a SIP call to an external SIP endpoint via REFER
    /// (mirrors Python `Call.refer`). Emits `calling.refer` with the
    /// supplied `device` (+ optional `status_url`) params.
    pub fn refer(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.refer", params)
    }

    pub fn send_digits(&self, params: Value) -> Result<Value, RelayError> {
        self.execute("calling.send_digits", params)
    }

    // ------------------------------------------------------------------
    // Action methods (return Action objects tracked by control_id)
    // ------------------------------------------------------------------

    pub fn play(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.play", "calling.play.stop", params)
    }

    pub fn record(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.record", "calling.record.stop", params)
    }

    pub fn collect(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.collect", "calling.collect.stop", params)
    }

    pub fn play_and_collect(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action(
            "calling.play_and_collect",
            "calling.play_and_collect.stop",
            params,
        )
    }

    pub fn detect(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.detect", "calling.detect.stop", params)
    }

    pub fn send_fax(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.send_fax", "calling.send_fax.stop", params)
    }

    pub fn receive_fax(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.receive_fax", "calling.receive_fax.stop", params)
    }

    pub fn tap(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.tap", "calling.tap.stop", params)
    }

    pub fn stream(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.stream", "calling.stream.stop", params)
    }

    pub fn pay(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.pay", "calling.pay.stop", params)
    }

    pub fn transcribe(&self, params: Value) -> Result<Arc<Action>, RelayError> {
        self.start_action("calling.transcribe", "calling.transcribe.stop", params)
    }

    pub fn ai(&self, params: Value) -> Result<Arc<Action>, RelayError> {
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
    pub fn play_tts(&self, text: &str, opts: Value) -> Result<Arc<Action>, RelayError> {
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
    pub fn play_audio(&self, url: &str, opts: Value) -> Result<Arc<Action>, RelayError> {
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
    pub fn play_silence(&self, duration: f64) -> Result<Arc<Action>, RelayError> {
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
    pub fn play_ringtone(&self, name: &str, opts: Value) -> Result<Arc<Action>, RelayError> {
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
    pub fn detect_digit(&self, opts: Value) -> Result<Arc<Action>, RelayError> {
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
    pub fn detect_answering_machine(&self, opts: Value) -> Result<Arc<Action>, RelayError> {
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
    pub fn detect_fax(&self, opts: Value) -> Result<Arc<Action>, RelayError> {
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
    pub fn prompt_tts(
        &self,
        text: &str,
        collect: Value,
        opts: Value,
    ) -> Result<Arc<Action>, RelayError> {
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
    pub fn prompt_audio(
        &self,
        url: &str,
        collect: Value,
        opts: Value,
    ) -> Result<Arc<Action>, RelayError> {
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
    // Event-bus waits (mirror Python Call.wait_for / wait_for_* family)
    //
    // Python's Call exposes an async `wait_for(event_type, predicate)`
    // primitive that suspends a coroutine until a matching event arrives,
    // plus typed state waiters built on it. The Rust Call is a synchronous
    // command surface, so these block the calling thread instead of
    // returning an awaitable: `wait_for` registers a one-shot listener via
    // `on` and blocks on a channel until a matching event is dispatched (or
    // the optional timeout elapses).
    // ------------------------------------------------------------------

    /// Block until an event of `event_type` matching `predicate` is
    /// dispatched to this call, then return it. `predicate = None` matches
    /// any event of that type. `timeout = None` blocks indefinitely.
    ///
    /// Mirrors Python `Call.wait_for`. Returns `Some(event)` on a match, or
    /// `None` if `timeout` elapsed before a matching event arrived.
    ///
    /// # Panics
    /// Panics if an internal mutex is poisoned (i.e. another thread panicked
    /// while holding the lock). This does not occur under normal operation.
    #[must_use]
    pub fn wait_for(
        &self,
        event_type: &str,
        predicate: Option<EventPredicate>,
        timeout: Option<std::time::Duration>,
    ) -> Option<Event> {
        let (tx, rx) = std::sync::mpsc::sync_channel::<Event>(1);
        let want = event_type.to_string();
        // One-shot guarded sender: only the first matching event is sent;
        // later matches (the listener stays registered) become no-ops.
        let sender = Mutex::new(Some(tx));
        self.on(move |event: &Event, _call: &Call| {
            if event.event_type() != want {
                return;
            }
            if let Some(pred) = &predicate
                && !pred(event)
            {
                return;
            }
            if let Some(tx) = sender.lock().unwrap().take() {
                let _ = tx.try_send(event.clone());
            }
        });

        match timeout {
            Some(dur) => rx.recv_timeout(dur).ok(),
            None => rx.recv().ok(),
        }
    }

    /// Rank of a call state in the created→ended progression, or `-1` if
    /// unknown. Used by the typed state waiters to short-circuit when the
    /// call is already at or past the target (matching Python).
    fn state_rank(state: &str) -> i32 {
        match state {
            constants::CALL_STATE_CREATED => 0,
            constants::CALL_STATE_RINGING => 1,
            constants::CALL_STATE_ANSWERED => 2,
            constants::CALL_STATE_ENDING => 3,
            constants::CALL_STATE_ENDED => 4,
            _ => -1,
        }
    }

    /// Read the state out of a `calling.call.state` event, accepting either
    /// the real-wire `call_state` key or the internal `state` key.
    fn event_call_state(event: &Event) -> Option<String> {
        event
            .params()
            .get("call_state")
            .or_else(|| event.params().get("state"))
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string)
    }

    /// Wait until the call reaches `target` (or a later state). Returns
    /// immediately with a synthesized event if already at/past `target`.
    fn wait_for_state(&self, target: &str, timeout: Option<std::time::Duration>) -> Option<Event> {
        if Self::state_rank(&self.current_state()) >= Self::state_rank(target) {
            let mut params = HashMap::new();
            params.insert(
                "call_state".to_string(),
                Value::String(self.current_state()),
            );
            return Some(Event::new("calling.call.state", params, 0.0));
        }
        let target = target.to_string();
        let pred: EventPredicate =
            Arc::new(move |e: &Event| Self::event_call_state(e).as_deref() == Some(&target));
        self.wait_for("calling.call.state", Some(pred), timeout)
    }

    /// Wait until the call is answered (immediate if already at/past it).
    #[must_use]
    pub fn wait_for_answered(&self, timeout: Option<std::time::Duration>) -> Option<Event> {
        self.wait_for_state(constants::CALL_STATE_ANSWERED, timeout)
    }

    /// Wait until the call is ringing (immediate if already at/past it).
    #[must_use]
    pub fn wait_for_ringing(&self, timeout: Option<std::time::Duration>) -> Option<Event> {
        self.wait_for_state(constants::CALL_STATE_RINGING, timeout)
    }

    /// Wait until the call is ending (immediate if already at/past it).
    #[must_use]
    pub fn wait_for_ending(&self, timeout: Option<std::time::Duration>) -> Option<Event> {
        self.wait_for_state(constants::CALL_STATE_ENDING, timeout)
    }

    /// Wait until the call reaches the ended state (immediate if already ended).
    #[must_use]
    pub fn wait_for_ended(&self, timeout: Option<std::time::Duration>) -> Option<Event> {
        self.wait_for_state(constants::CALL_STATE_ENDED, timeout)
    }

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
    ///
    /// Builds the `calling.<x>` frame's params (`node_id` + `call_id` + the
    /// verb's extra params) and, when a [`Client`] is attached, TRANSMITS it
    /// to the wire via [`Client::send_request`] — the client-send boundary.
    /// The params are always also recorded in `sent_commands` for local
    /// inspection/tests. Mirrors Python's `Call._execute`, which calls
    /// `self._client.execute(method, params)`.
    fn execute(&self, method: &str, extra: Value) -> Result<Value, RelayError> {
        let mut base = self.base_params();
        if let (Some(base_map), Some(extra_map)) = (base.as_object_mut(), extra.as_object()) {
            for (k, v) in extra_map {
                base_map.insert(k.clone(), v.clone());
            }
        }
        self.record_command(method, base.clone());
        // With a LIVE socket attached, genuinely block for the server's
        // response so a rejected verb surfaces as `Err(RelayError::Rpc)` /
        // `Timeout` (404/410 "call gone" swallowed to `Ok({})`), mirroring
        // Python's `await self._client.execute(...)`. Without a live socket
        // (the in-memory dispatch tests, or a bare `Call::new`) keep the
        // fire-and-record path and return the built frame — EMISSION unchanged.
        match self.client() {
            Some(client) if client.has_live_socket() => client.execute_call_verb(method, base),
            Some(client) => {
                client.send_request(method, base.clone());
                Ok(base)
            }
            None => Ok(base),
        }
    }

    /// Spin up a long-running action tracked by a unique `control_id`.
    ///
    /// Mirrors Python's `Call._start_action`: register the action, execute the
    /// start RPC, and — with a live socket — block for its response. If the RPC
    /// fails the action is removed and the error propagates; if the call is
    /// gone (404/410 → `Ok({})`) the action is immediately resolved so a later
    /// `wait()` returns instead of hanging.
    fn start_action(
        &self,
        method: &str,
        stop_method: &str,
        extra: Value,
    ) -> Result<Arc<Action>, RelayError> {
        let control_id = generate_uuid();
        let call_id = self.call_id.as_deref().unwrap_or("");
        let node_id = self.node_id.as_deref().unwrap_or("");

        let action = Arc::new(Action::with_stop_method(
            &control_id,
            call_id,
            node_id,
            stop_method,
        ));
        // Wire the same client into the Action so its control-op sub-commands
        // (stop/pause/resume/volume) transmit to the wire too.
        if let Some(client) = self.client() {
            action.set_client(&client);
        }

        self.actions
            .lock()
            .unwrap()
            .insert(control_id.clone(), action.clone());

        let mut base = self.base_params();
        if let Some(base_map) = base.as_object_mut() {
            base_map.insert("control_id".to_string(), Value::String(control_id.clone()));
            if let Some(extra_map) = extra.as_object() {
                for (k, v) in extra_map {
                    base_map.insert(k.clone(), v.clone());
                }
            }
        }
        self.record_command(method, base.clone());

        // Transmit the action's start frame (mirrors Python's `_start_action`
        // -> `_execute`).
        match self.client() {
            Some(client) if client.has_live_socket() => {
                match client.execute_call_verb(method, base) {
                    Ok(result) => {
                        // `execute_call_verb` returns `{}` when the call was
                        // gone (404/410) — resolve the action immediately so a
                        // later `wait()` doesn't hang forever.
                        if result.as_object().is_some_and(serde_json::Map::is_empty) {
                            self.actions.lock().unwrap().remove(&control_id);
                            action.resolve(None);
                        }
                        Ok(action)
                    }
                    Err(e) => {
                        // The start RPC failed: drop the action and release any
                        // waiter with the error, then propagate.
                        self.actions.lock().unwrap().remove(&control_id);
                        action.resolve(None);
                        Err(e)
                    }
                }
            }
            Some(client) => {
                client.send_request(method, base);
                Ok(action)
            }
            None => Ok(action),
        }
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
        (u64::from(data[10]) << 40)
            | (u64::from(data[11]) << 32)
            | (u64::from(data[12]) << 24)
            | (u64::from(data[13]) << 16)
            | (u64::from(data[14]) << 8)
            | u64::from(data[15]),
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
        let ev = make_event("calling.call.state", json!({"call_state": "ringing"}));
        call.dispatch_event(&ev);
        assert_eq!(call.current_state(), "ringing");
    }

    #[test]
    fn test_dispatch_ended_resolves_actions() {
        let call = make_call();
        let action = call.play(json!({})).unwrap();
        assert!(!action.is_done());

        let ev = make_event("calling.call.state", json!({"call_state": "ended"}));
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
        assert_eq!(*call.end_reason.lock().unwrap(), Some("hangup".to_string()));
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
        let action = call.play(json!({})).unwrap();
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
        let action = call.play(json!({})).unwrap();
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

        let ev = make_event("calling.call.state", json!({"call_state": "ringing"}));
        call.dispatch_event(&ev);
        assert_eq!(*count.lock().unwrap(), 1);
    }

    // -- Simple method tests --

    #[test]
    fn test_simple_methods_send_commands() {
        let call = make_call();

        call.answer().unwrap();
        call.hangup().unwrap();
        call.pass().unwrap();
        call.hold().unwrap();
        call.unhold().unwrap();
        call.denoise().unwrap();
        call.denoise_stop().unwrap();
        call.disconnect().unwrap();
        call.echo_call().unwrap();
        call.leave_conference().unwrap();
        call.leave_room().unwrap();
        call.ai_hold().unwrap();
        call.ai_unhold().unwrap();
        call.queue_leave().unwrap();
        call.clear_digit_bindings().unwrap();

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

        call.connect(json!({"to": "+15551001000"})).unwrap();
        call.transfer(json!({"dest": "sip:foo@bar"})).unwrap();
        call.join_conference(json!({"name": "room1"})).unwrap();
        call.bind_digit(json!({"digits": "*"})).unwrap();
        call.send_digits(json!({"digits": "1234"})).unwrap();

        let cmds = call.sent_commands.lock().unwrap();
        assert_eq!(cmds.len(), 5);
        assert_eq!(cmds[0].0, "calling.connect");
        assert_eq!(cmds[0].1["to"], "+15551001000");
    }

    // -- Action method tests --

    #[test]
    fn test_play_creates_action() {
        let call = make_call();
        let action = call
            .play(json!({"url": "http://example.com/audio.mp3"}))
            .unwrap();
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
        let action = call.record(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.record.stop");
    }

    #[test]
    fn test_collect_creates_action() {
        let call = make_call();
        let action = call.collect(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.collect.stop");
    }

    #[test]
    fn test_detect_creates_action() {
        let call = make_call();
        let action = call.detect(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.detect.stop");
    }

    #[test]
    fn test_tap_creates_action() {
        let call = make_call();
        let action = call.tap(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.tap.stop");
    }

    #[test]
    fn test_stream_creates_action() {
        let call = make_call();
        let action = call.stream(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.stream.stop");
    }

    #[test]
    fn test_pay_creates_action() {
        let call = make_call();
        let action = call.pay(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.pay.stop");
    }

    #[test]
    fn test_transcribe_creates_action() {
        let call = make_call();
        let action = call.transcribe(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.transcribe.stop");
    }

    #[test]
    fn test_ai_creates_action() {
        let call = make_call();
        let action = call.ai(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.ai.stop");
    }

    #[test]
    fn test_send_fax_creates_action() {
        let call = make_call();
        let action = call.send_fax(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.send_fax.stop");
    }

    #[test]
    fn test_receive_fax_creates_action() {
        let call = make_call();
        let action = call.receive_fax(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.receive_fax.stop");
    }

    #[test]
    fn test_play_and_collect_creates_action() {
        let call = make_call();
        let action = call.play_and_collect(json!({})).unwrap();
        assert_eq!(action.stop_method(), "calling.play_and_collect.stop");
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
        let action = call
            .play_tts(
                "Hi there",
                json!({"language": "en-US", "gender": "male", "voice": "spore", "volume": -2.0}),
            )
            .unwrap();
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
        call.play_tts("Bare", json!({})).unwrap();
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
        call.play_audio("https://x/a.mp3", json!({"volume": 1.5}))
            .unwrap();
        let p = one_built(&call, "calling.play");
        assert_eq!(p["play"][0]["type"], "audio");
        assert_eq!(p["play"][0]["params"]["url"], "https://x/a.mp3");
        assert_eq!(p["volume"], 1.5);
    }

    #[test]
    fn test_play_silence_builds_silence_media() {
        let call = make_call();
        call.play_silence(3.0).unwrap();
        let p = one_built(&call, "calling.play");
        assert_eq!(p["play"][0]["type"], "silence");
        assert_eq!(p["play"][0]["params"]["duration"], 3.0);
    }

    #[test]
    fn test_play_ringtone_builds_ringtone_media() {
        let call = make_call();
        call.play_ringtone("us", json!({"duration": 5.0, "volume": -1.0}))
            .unwrap();
        let p = one_built(&call, "calling.play");
        assert_eq!(p["play"][0]["type"], "ringtone");
        assert_eq!(p["play"][0]["params"]["name"], "us");
        assert_eq!(p["play"][0]["params"]["duration"], 5.0);
        assert_eq!(p["volume"], -1.0);
    }

    #[test]
    fn test_play_ringtone_omits_unset_duration() {
        let call = make_call();
        call.play_ringtone("de", json!({})).unwrap();
        let p = one_built(&call, "calling.play");
        assert_eq!(p["play"][0]["params"]["name"], "de");
        assert!(p["play"][0]["params"].get("duration").is_none());
        assert!(p.get("volume").is_none());
    }

    #[test]
    fn test_detect_digit_builds_digit_detect() {
        let call = make_call();
        let action = call
            .detect_digit(json!({"digits": "42", "timeout": 10.0}))
            .unwrap();
        assert_eq!(action.stop_method(), "calling.detect.stop");
        let p = one_built(&call, "calling.detect");
        assert_eq!(p["detect"]["type"], "digit");
        assert_eq!(p["detect"]["params"]["digits"], "42");
        assert_eq!(p["timeout"], 10.0);
    }

    #[test]
    fn test_detect_digit_empty_params() {
        let call = make_call();
        call.detect_digit(json!({})).unwrap();
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
        }))
        .unwrap();
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
        call.detect_fax(json!({"tone": "CNG"})).unwrap();
        let p = one_built(&call, "calling.detect");
        assert_eq!(p["detect"]["type"], "fax");
        assert_eq!(p["detect"]["params"]["tone"], "CNG");
        assert!(p.get("timeout").is_none());
    }

    #[test]
    fn test_prompt_tts_builds_tts_media_plus_collect() {
        let call = make_call();
        let action = call
            .prompt_tts(
                "Pick one",
                json!({"digits": {"max": 1}}),
                json!({"voice": "spore", "volume": 0.5}),
            )
            .unwrap();
        assert_eq!(action.stop_method(), "calling.play_and_collect.stop");
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
        )
        .unwrap();
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
        let a1 = call.play(json!({})).unwrap();
        let a2 = call.record(json!({})).unwrap();
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

    // -- Python-parity command names (echo / pass_ / refer) --

    #[test]
    fn test_echo_emits_calling_echo() {
        let call = make_call();
        call.echo(Value::Null).unwrap();
        let cmds = call.sent_commands.lock().unwrap();
        assert_eq!(cmds[0].0, "calling.echo");
        assert_eq!(cmds[0].1["call_id"], "call-1");
        assert_eq!(cmds[0].1["node_id"], "node-1");
    }

    #[test]
    fn test_echo_forwards_params() {
        let call = make_call();
        call.echo(json!({"timeout": 30, "status_url": "https://x"}))
            .unwrap();
        let cmds = call.sent_commands.lock().unwrap();
        assert_eq!(cmds[0].0, "calling.echo");
        assert_eq!(cmds[0].1["timeout"], 30);
        assert_eq!(cmds[0].1["status_url"], "https://x");
    }

    #[test]
    fn test_pass_emits_calling_pass() {
        let call = make_call();
        call.pass_().unwrap();
        let cmds = call.sent_commands.lock().unwrap();
        assert_eq!(cmds[0].0, "calling.pass");
        assert_eq!(cmds[0].1["call_id"], "call-1");
    }

    #[test]
    fn test_refer_emits_calling_refer_with_device() {
        let call = make_call();
        call.refer(json!({"device": {"type": "sip", "params": {"to": "sip:x"}}}))
            .unwrap();
        let cmds = call.sent_commands.lock().unwrap();
        assert_eq!(cmds[0].0, "calling.refer");
        assert_eq!(cmds[0].1["device"]["type"], "sip");
    }

    // -- wait_for family --

    #[test]
    fn test_wait_for_state_immediate_when_already_past() {
        // Call already answered; waiting for ringing (an earlier state)
        // returns immediately with a synthesized current-state event.
        let call = Call::new(&json!({
            "call_id": "call-1",
            "node_id": "node-1",
            "call_state": "answered",
        }));
        let ev = call
            .wait_for_ringing(Some(std::time::Duration::from_millis(0)))
            .unwrap();
        assert_eq!(ev.event_type(), "calling.call.state");
        assert_eq!(Call::event_call_state(&ev).as_deref(), Some("answered"));
    }

    #[test]
    fn test_wait_for_state_waits_when_not_yet_reached() {
        // From created, ringing is NOT yet reached, so it must wait (and
        // here time out) rather than return immediately.
        let call = make_call(); // state = created
        let got = call.wait_for_ringing(Some(std::time::Duration::from_millis(20)));
        assert!(got.is_none());
    }

    #[test]
    fn test_wait_for_times_out_when_no_event() {
        let call = make_call();
        let got = call.wait_for_answered(Some(std::time::Duration::from_millis(20)));
        assert!(got.is_none());
    }

    #[test]
    fn test_wait_for_unblocks_on_dispatched_event() {
        let call = Arc::new(make_call());
        let c2 = call.clone();
        let handle = std::thread::spawn(move || {
            c2.wait_for_answered(Some(std::time::Duration::from_secs(2)))
        });
        std::thread::sleep(std::time::Duration::from_millis(30));
        // Real-wire key is call_state.
        let ev = make_event("calling.call.state", json!({"call_state": "answered"}));
        call.dispatch_event(&ev);
        let got = handle.join().unwrap().unwrap();
        assert_eq!(Call::event_call_state(&got).as_deref(), Some("answered"));
    }

    #[test]
    fn test_wait_for_generic_with_predicate() {
        let call = Arc::new(make_call());
        let c2 = call.clone();
        let pred: EventPredicate =
            Arc::new(|e: &Event| e.params().get("foo").and_then(|v| v.as_str()) == Some("bar"));
        let handle = std::thread::spawn(move || {
            c2.wait_for(
                "calling.call.custom",
                Some(pred),
                Some(std::time::Duration::from_secs(2)),
            )
        });
        std::thread::sleep(std::time::Duration::from_millis(30));
        // Non-matching event first, then the matching one.
        call.dispatch_event(&make_event("calling.call.custom", json!({"foo": "nope"})));
        call.dispatch_event(&make_event("calling.call.custom", json!({"foo": "bar"})));
        let got = handle.join().unwrap().unwrap();
        assert_eq!(got.params().get("foo").unwrap(), "bar");
    }
}
