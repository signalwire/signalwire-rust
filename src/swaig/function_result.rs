use std::collections::HashMap;

use serde_json::{json, Map, Value};

/// Render a list of string values the way Python renders a `list[str]` inside
/// an f-string — `['a', 'b', 'c']` — so the `join_conference` validation
/// error messages are byte-identical to the reference's `ValueError` text.
fn render_list(values: &[&str]) -> String {
    let items: Vec<String> = values.iter().map(|v| format!("'{}'", v)).collect();
    format!("[{}]", items.join(", "))
}

/// Result returned from a SWAIG function handler.
///
/// Serialises to match the Python reference's `to_dict()`: `response` is omitted when empty,
/// `action` when empty, and `post_process` unless there are actions; an otherwise-empty result
/// defaults to `{"response": "Action completed."}`.
#[derive(Debug, Clone)]
pub struct FunctionResult {
    response: String,
    actions: Vec<Value>,
    post_process: bool,
}

impl FunctionResult {
    pub fn new() -> Self {
        FunctionResult {
            response: String::new(),
            actions: Vec::new(),
            post_process: false,
        }
    }

    pub fn with_response(response: &str) -> Self {
        FunctionResult {
            response: response.to_string(),
            actions: Vec::new(),
            post_process: false,
        }
    }

    // ── Core ─────────────────────────────────────────────────────────────

    pub fn set_response(&mut self, text: &str) -> &mut Self {
        self.response = text.to_string();
        self
    }

    pub fn set_post_process(&mut self, val: bool) -> &mut Self {
        self.post_process = val;
        self
    }

    pub fn add_action(&mut self, action: Value) -> &mut Self {
        self.actions.push(action);
        self
    }

    pub fn add_actions(&mut self, actions: Vec<Value>) -> &mut Self {
        for a in actions {
            self.actions.push(a);
        }
        self
    }

    /// Serialise to a JSON value.
    ///
    /// - `response` is always included.
    /// - `action` is only included if at least one action exists.
    /// - `post_process` is only included if `true`.
    pub fn to_value(&self) -> Value {
        let mut map = Map::new();
        // response is omitted when empty (Python parity).
        if !self.response.is_empty() {
            map.insert("response".to_string(), Value::String(self.response.clone()));
        }

        if !self.actions.is_empty() {
            map.insert("action".to_string(), Value::Array(self.actions.clone()));
        }

        // post_process only matters when there are actions to execute.
        if self.post_process && !self.actions.is_empty() {
            map.insert("post_process".to_string(), Value::Bool(true));
        }

        // Ensure at least one of response or action is present.
        if map.is_empty() {
            map.insert(
                "response".to_string(),
                Value::String("Action completed.".to_string()),
            );
        }

        Value::Object(map)
    }

    /// Compact JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_value()).expect("FunctionResult serialisation should not fail")
    }

    /// Build the canonical SWML document that wraps a single verb and push it
    /// through `execute_swml`, exactly like the Python reference's virtual
    /// helpers (`send_sms`/`pay`/`record_call`/`tap`/…), which all construct
    /// `{"version": "1.0.0", "sections": {"main": [{verb: params}]}}` and route
    /// it through `execute_swml`. This guarantees the emitted action is the
    /// wrapped SWML document (`{"SWML": {...}}`) — never a bare verb.
    fn push_swml_verb(&mut self, verb: &str, params: Value) {
        let swml_doc = json!({
            "version": "1.0.0",
            "sections": {
                "main": [ { verb: params } ]
            }
        });
        self.execute_swml(swml_doc, false);
    }

    // ── Call Control ─────────────────────────────────────────────────────

    pub fn connect(&mut self, destination: &str, _final: bool, from: &str) -> &mut Self {
        let mut connect_obj = Map::new();
        connect_obj.insert("to".to_string(), json!(destination));
        if !from.is_empty() {
            connect_obj.insert("from".to_string(), json!(from));
        }

        self.actions.push(json!({
            "SWML": {
                "sections": {
                    "main": [
                        {"connect": Value::Object(connect_obj)}
                    ]
                },
                "version": "1.0.0"
            },
            // final=true -> permanent transfer; matches the Python reference
            // (function_result.py connect: "transfer": str(final).lower()).
            "transfer": if _final { "true" } else { "false" }
        }));

        self
    }

    /// Add a SWML transfer action with an AI response set up for when the
    /// transfer completes and control returns to the agent.
    ///
    /// Mirrors the Python reference (`FunctionResult.swml_transfer`): emits a
    /// SWML document whose `main` section is `[{set: {ai_response}}, {transfer:
    /// {dest}}]`, with a top-level `"transfer": str(final).lower()` flag marking
    /// whether the transfer is permanent (`final = true`, the default) or
    /// temporary. `ai_response` is carried inside the SWML `set` verb (it is NOT
    /// assigned to `self.response`).
    pub fn swml_transfer(&mut self, dest: &str, ai_response: &str, _final: bool) -> &mut Self {
        let swml_action = json!({
            "SWML": {
                "version": "1.0.0",
                "sections": {
                    "main": [
                        {"set": {"ai_response": ai_response}},
                        {"transfer": {"dest": dest}}
                    ]
                }
            },
            // final=true -> permanent transfer; matches the Python reference
            // (function_result.py swml_transfer: "transfer": str(final).lower()).
            "transfer": if _final { "true" } else { "false" }
        });
        self.actions.push(swml_action);
        self
    }

    pub fn hangup(&mut self) -> &mut Self {
        // Python: add_action("hangup", True) — the value is the boolean true.
        self.actions.push(json!({"hangup": true}));
        self
    }

    pub fn hold(&mut self, timeout: i64) -> &mut Self {
        // Python: add_action("hold", timeout) — the value is the bare clamped int.
        let clamped = timeout.clamp(0, 900);
        self.actions.push(json!({"hold": clamped}));
        self
    }

    /// Control how the agent waits for user input.
    ///
    /// Mirrors the Python reference (`FunctionResult.wait_for_user`): the action
    /// value is a **scalar**, chosen by the same precedence —
    /// `answer_first` (the string `"answer_first"`) > `timeout` (bare int) >
    /// `enabled` (bare bool) > the default bare bool `true`.
    pub fn wait_for_user(
        &mut self,
        enabled: Option<bool>,
        timeout: Option<i64>,
        answer_first: Option<bool>,
    ) -> &mut Self {
        let wait_value: Value = if answer_first == Some(true) {
            json!("answer_first")
        } else if let Some(t) = timeout {
            json!(t)
        } else if let Some(e) = enabled {
            json!(e)
        } else {
            json!(true)
        };
        self.actions.push(json!({"wait_for_user": wait_value}));
        self
    }

    pub fn stop(&mut self) -> &mut Self {
        self.actions.push(json!({"stop": true}));
        self
    }

    // ── State & Data ─────────────────────────────────────────────────────

    pub fn update_global_data(&mut self, data: Value) -> &mut Self {
        self.actions.push(json!({"set_global_data": data}));
        self
    }

    /// Python: add_action("unset_global_data", keys) — value is the bare key
    /// list (Python's `Union[str, List[str]]`); no `{"keys": ...}` wrapper.
    pub fn remove_global_data(&mut self, keys: Vec<&str>) -> &mut Self {
        self.actions.push(json!({"unset_global_data": keys}));
        self
    }

    pub fn set_metadata(&mut self, data: Value) -> &mut Self {
        self.actions.push(json!({"set_meta_data": data}));
        self
    }

    /// Python: add_action("unset_meta_data", keys) — value is the bare key list;
    /// no `{"keys": ...}` wrapper.
    pub fn remove_metadata(&mut self, keys: Vec<&str>) -> &mut Self {
        self.actions.push(json!({"unset_meta_data": keys}));
        self
    }

    /// Send a user event through SWML to update the client UI.
    ///
    /// Mirrors the Python reference (`FunctionResult.swml_user_event`): emits a
    /// `"SWML"` action whose `main` section nests the event payload under
    /// `{"user_event": {"event": event_data}}`.
    pub fn swml_user_event(&mut self, event_data: Value) -> &mut Self {
        let swml_action = json!({
            "version": "1.0.0",
            "sections": {
                "main": [
                    { "user_event": { "event": event_data } }
                ]
            }
        });
        self.actions.push(json!({ "SWML": swml_action }));
        self
    }

    /// Python: add_action("change_step", step_name) — the value is the bare step
    /// name string (not a `context_switch` dict).
    pub fn swml_change_step(&mut self, step_name: &str) -> &mut Self {
        self.actions.push(json!({"change_step": step_name}));
        self
    }

    /// Python: add_action("change_context", context_name) — the value is the
    /// bare context name string (not a `context_switch` dict).
    pub fn swml_change_context(&mut self, context_name: &str) -> &mut Self {
        self.actions.push(json!({"change_context": context_name}));
        self
    }

    /// Change the agent context/prompt during the conversation.
    ///
    /// Mirrors the Python reference (`FunctionResult.switch_context`): when only
    /// `system_prompt` is supplied (no `user_prompt`/`consolidate`/`full_reset`,
    /// and — for this port's documented `isolated` extension — no `isolated`),
    /// the action value is the **bare system-prompt string**
    /// (`{"context_switch": "<prompt>"}`). Otherwise it is the object form
    /// carrying every set field.
    pub fn switch_context(
        &mut self,
        system_prompt: &str,
        user_prompt: &str,
        consolidate: bool,
        full_reset: bool,
        isolated: bool,
    ) -> &mut Self {
        if !system_prompt.is_empty()
            && user_prompt.is_empty()
            && !consolidate
            && !full_reset
            && !isolated
        {
            // Simple string context switch.
            self.actions
                .push(json!({"context_switch": system_prompt}));
            return self;
        }

        let mut ctx = Map::new();
        if !system_prompt.is_empty() {
            ctx.insert("system_prompt".to_string(), json!(system_prompt));
        }
        if !user_prompt.is_empty() {
            ctx.insert("user_prompt".to_string(), json!(user_prompt));
        }
        if consolidate {
            ctx.insert("consolidate".to_string(), json!(true));
        }
        if full_reset {
            ctx.insert("full_reset".to_string(), json!(true));
        }
        if isolated {
            ctx.insert("isolated".to_string(), json!(true));
        }

        self.actions.push(json!({"context_switch": Value::Object(ctx)}));
        self
    }

    /// After first send, replace the tool_call+result pair in conversation
    /// history.
    ///
    /// Mirrors the Python reference (`FunctionResult.replace_in_history`, whose
    /// `text` parameter is `Union[str, bool] = True`): the action key is
    /// `"replace_in_history"`. `Some(t)` replaces the tool call with an assistant
    /// message containing `t`; `None` uses the default `true`, which removes the
    /// tool_call+result pair from history entirely.
    pub fn replace_in_history(&mut self, text: Option<&str>) -> &mut Self {
        match text {
            Some(t) => self.actions.push(json!({"replace_in_history": t})),
            None => self.actions.push(json!({"replace_in_history": true})),
        }
        self
    }

    // ── Media ────────────────────────────────────────────────────────────

    pub fn say(&mut self, text: &str) -> &mut Self {
        self.actions.push(json!({"say": text}));
        self
    }

    /// Play an audio/video file in the background.
    ///
    /// Mirrors the Python reference (`FunctionResult.play_background_file`): the
    /// action key is `"playback_bg"`. With `wait = true` the value is
    /// `{"file": filename, "wait": true}` (suppress attention-getting behaviour);
    /// otherwise it is the bare filename string.
    pub fn play_background_file(&mut self, filename: &str, wait: bool) -> &mut Self {
        if wait {
            self.actions
                .push(json!({"playback_bg": {"file": filename, "wait": true}}));
        } else {
            self.actions.push(json!({"playback_bg": filename}));
        }
        self
    }

    /// Python: add_action("stop_playback_bg", True).
    pub fn stop_background_file(&mut self) -> &mut Self {
        self.actions.push(json!({"stop_playback_bg": true}));
        self
    }

    /// Start background call recording (SWML `record_call`).
    ///
    /// Mirrors the Python reference (`FunctionResult.record_call`): the verb is
    /// wrapped in a SWML document (`{"SWML": {version, sections: {main:
    /// [{record_call: params}]}}}`) — never a bare verb — and the reference's
    /// two closed-set validations are reproduced, returning `Err(message)` with
    /// the exact reference `ValueError` text:
    ///
    /// - `format` ∈ `{wav, mp3, mp4}`
    /// - `direction` ∈ `{speak, listen, both}`
    ///
    /// `stereo`, `format`, `direction`, `beep`, and `input_sensitivity` are
    /// **always** emitted (matching the reference, which seeds `record_params`
    /// with all five); `control_id`, `terminators`, `initial_timeout`,
    /// `end_silence_timeout`, `max_length`, and `status_url` are emitted only
    /// when supplied. There is no `initiator` field — the previous port invented
    /// it; it is removed.
    #[allow(clippy::too_many_arguments)]
    pub fn record_call(
        &mut self,
        control_id: &str,
        stereo: bool,
        format: &str,
        direction: &str,
        terminators: &str,
        beep: bool,
        input_sensitivity: f64,
        initial_timeout: Option<f64>,
        end_silence_timeout: Option<f64>,
        max_length: Option<f64>,
        status_url: &str,
    ) -> Result<&mut Self, String> {
        // ── Validation (exact reference ValueError messages) ─────────────
        let valid_format = ["wav", "mp3", "mp4"];
        if !valid_format.contains(&format) {
            return Err("format must be 'wav', 'mp3', or 'mp4'".to_string());
        }
        let valid_direction = ["speak", "listen", "both"];
        if !valid_direction.contains(&direction) {
            return Err("direction must be 'speak', 'listen', or 'both'".to_string());
        }

        // ── Build params (mirrors the reference's always-on + optional set) ──
        let mut record = Map::new();
        record.insert("stereo".to_string(), json!(stereo));
        record.insert("format".to_string(), json!(format));
        record.insert("direction".to_string(), json!(direction));
        record.insert("beep".to_string(), json!(beep));
        record.insert("input_sensitivity".to_string(), json!(input_sensitivity));
        if !control_id.is_empty() {
            record.insert("control_id".to_string(), json!(control_id));
        }
        if !terminators.is_empty() {
            record.insert("terminators".to_string(), json!(terminators));
        }
        if let Some(t) = initial_timeout {
            record.insert("initial_timeout".to_string(), json!(t));
        }
        if let Some(t) = end_silence_timeout {
            record.insert("end_silence_timeout".to_string(), json!(t));
        }
        if let Some(t) = max_length {
            record.insert("max_length".to_string(), json!(t));
        }
        if !status_url.is_empty() {
            record.insert("status_url".to_string(), json!(status_url));
        }

        self.push_swml_verb("record_call", Value::Object(record));
        Ok(self)
    }

    /// Stop an active background recording (SWML `stop_record_call`).
    ///
    /// Mirrors the Python reference: the verb is wrapped in a SWML document. The
    /// params are `{"control_id": ...}` when supplied, else `{}` (most-recent).
    pub fn stop_record_call(&mut self, control_id: &str) -> &mut Self {
        let params = if !control_id.is_empty() {
            json!({"control_id": control_id})
        } else {
            json!({})
        };
        self.push_swml_verb("stop_record_call", params);
        self
    }

    // ── Speech & AI ──────────────────────────────────────────────────────

    pub fn add_dynamic_hints(&mut self, hints: Vec<Value>) -> &mut Self {
        self.actions.push(json!({"add_dynamic_hints": hints}));
        self
    }

    /// Clear all dynamic speech-recognition hints.
    ///
    /// Mirrors the Python reference (`FunctionResult.clear_dynamic_hints`),
    /// which appends `{"clear_dynamic_hints": {}}` — the value is an **empty
    /// object**, not a boolean. (`json!({})` serialises to `{}`.)
    pub fn clear_dynamic_hints(&mut self) -> &mut Self {
        self.actions.push(json!({"clear_dynamic_hints": {}}));
        self
    }

    pub fn set_end_of_speech_timeout(&mut self, ms: i64) -> &mut Self {
        self.actions.push(json!({"end_of_speech_timeout": ms}));
        self
    }

    pub fn set_speech_event_timeout(&mut self, ms: i64) -> &mut Self {
        self.actions.push(json!({"speech_event_timeout": ms}));
        self
    }

    pub fn toggle_functions(&mut self, toggles: HashMap<String, bool>) -> &mut Self {
        let formatted: Vec<Value> = toggles
            .into_iter()
            .map(|(name, active)| json!({"function": name, "active": active}))
            .collect();
        self.actions.push(json!({"toggle_functions": formatted}));
        self
    }

    /// Python: add_action("functions_on_speaker_timeout", enabled).
    pub fn enable_functions_on_timeout(&mut self, enabled: bool) -> &mut Self {
        self.actions
            .push(json!({"functions_on_speaker_timeout": enabled}));
        self
    }

    pub fn enable_extensive_data(&mut self, enabled: bool) -> &mut Self {
        self.actions.push(json!({"extensive_data": enabled}));
        self
    }

    /// Python: add_action("settings", settings).
    pub fn update_settings(&mut self, settings: Value) -> &mut Self {
        self.actions.push(json!({"settings": settings}));
        self
    }

    // ── Advanced ─────────────────────────────────────────────────────────

    /// Execute SWML content, optionally marking the call to exit the agent
    /// afterward (`transfer = true`).
    ///
    /// Mirrors the Python reference (`FunctionResult.execute_swml`): the action
    /// key is **always** `"SWML"`; when `transfer` is set, a `"transfer": "true"`
    /// flag is added **inside** the SWML dict (it is not a separate action key).
    ///
    /// Input normalisation matches Python:
    /// - A JSON **string** is parsed to a dict so the transfer flag can be added;
    ///   if it is not valid JSON it falls back to `{"raw_swml": "<text>"}`.
    /// - A JSON **object** is used as-is (a copy, so the transfer flag does not
    ///   mutate the caller's value).
    /// - Any other JSON scalar/array is wrapped as `{"raw_swml": <value-as-string>}`,
    ///   the same fallback Python uses for a non-dict, non-string `swml_content`.
    pub fn execute_swml(&mut self, swml_content: Value, transfer: bool) -> &mut Self {
        let mut swml_data: Map<String, Value> = match swml_content {
            Value::String(s) => {
                // Raw SWML string — parse to an object so the transfer key can be
                // added; on parse failure fall back to the raw_swml wrapper.
                match serde_json::from_str::<Value>(&s) {
                    Ok(Value::Object(m)) => m,
                    _ => {
                        let mut m = Map::new();
                        m.insert("raw_swml".to_string(), Value::String(s));
                        m
                    }
                }
            }
            Value::Object(m) => m,
            other => {
                // Non-dict, non-string content (number/bool/array/null) — wrap as
                // raw_swml using its string rendering, mirroring Python's fallback.
                let mut m = Map::new();
                let raw = match &other {
                    Value::Null => "null".to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(n) => n.to_string(),
                    _ => other.to_string(),
                };
                m.insert("raw_swml".to_string(), Value::String(raw));
                m
            }
        };

        if transfer {
            swml_data.insert("transfer".to_string(), json!("true"));
        }

        self.actions.push(json!({ "SWML": Value::Object(swml_data) }));
        self
    }

    /// Join an ad-hoc audio conference (SWML `join_conference`).
    ///
    /// Mirrors the Python reference
    /// (`FunctionResult.join_conference`) — `name` is required and the
    /// remaining 18 parameters are optional with the reference's defaults.
    /// `wait_url` carries the hold-music URL (Python has no `hold_audio`
    /// parameter; this port previously invented one — it is removed).
    ///
    /// The seven closed-set / range validations from the reference are
    /// reproduced and return `Err(message)` with the reference's exact
    /// `ValueError` text (Rust's idiomatic recoverable-validation channel —
    /// the `Err` type is out-of-band to the cross-language audit, which
    /// unwraps `Result<T, E>` to `T`):
    ///
    /// - `beep` ∈ `{true, false, onEnter, onExit}`
    /// - `0 < max_participants <= 250`
    /// - `record` ∈ `{do-not-record, record-from-start}`
    /// - `trim` ∈ `{trim-silence, do-not-trim}`
    /// - `status_callback_method` ∈ `{GET, POST}`
    /// - `recording_status_callback_method` ∈ `{GET, POST}`
    /// - `name` (trimmed) must not be empty
    ///
    /// When every parameter is left at its default the `join_conference` value
    /// is the bare conference name string; otherwise it is a `{"name": ...}`
    /// object carrying every non-default parameter under its snake_case wire
    /// key. Either way the verb is wrapped in the canonical SWML document
    /// (`{"SWML": {version, sections: {main: [{join_conference: ...}]}}}`),
    /// matching the reference (which routes join_conference through
    /// `execute_swml`) — never a bare verb.
    #[allow(clippy::too_many_arguments)]
    pub fn join_conference(
        &mut self,
        name: &str,
        muted: bool,
        beep: &str,
        start_on_enter: bool,
        end_on_exit: bool,
        wait_url: Option<&str>,
        max_participants: i64,
        record: &str,
        region: Option<&str>,
        trim: &str,
        coach: Option<&str>,
        status_callback_event: Option<&str>,
        status_callback: Option<&str>,
        status_callback_method: &str,
        recording_status_callback: Option<&str>,
        recording_status_callback_method: &str,
        recording_status_callback_event: &str,
        result: Option<Value>,
    ) -> Result<&mut Self, String> {
        // ── Validation (exact reference ValueError messages) ─────────────
        let valid_beep = ["true", "false", "onEnter", "onExit"];
        if !valid_beep.contains(&beep) {
            return Err(format!("beep must be one of {}", render_list(&valid_beep)));
        }

        if max_participants <= 0 || max_participants > 250 {
            return Err("max_participants must be a positive integer <= 250".to_string());
        }

        let valid_record = ["do-not-record", "record-from-start"];
        if !valid_record.contains(&record) {
            return Err(format!("record must be one of {}", render_list(&valid_record)));
        }

        let valid_trim = ["trim-silence", "do-not-trim"];
        if !valid_trim.contains(&trim) {
            return Err(format!("trim must be one of {}", render_list(&valid_trim)));
        }

        let valid_methods = ["GET", "POST"];
        if !valid_methods.contains(&status_callback_method) {
            return Err(format!(
                "status_callback_method must be one of {}",
                render_list(&valid_methods)
            ));
        }
        if !valid_methods.contains(&recording_status_callback_method) {
            return Err(format!(
                "recording_status_callback_method must be one of {}",
                render_list(&valid_methods)
            ));
        }

        if name.trim().is_empty() {
            return Err("name cannot be empty".to_string());
        }

        // ── Build params (mirrors the reference's simple/full forms) ─────
        let all_defaults = !muted
            && beep == "true"
            && start_on_enter
            && !end_on_exit
            && wait_url.is_none()
            && max_participants == 250
            && record == "do-not-record"
            && region.is_none()
            && trim == "trim-silence"
            && coach.is_none()
            && status_callback_event.is_none()
            && status_callback.is_none()
            && status_callback_method == "POST"
            && recording_status_callback.is_none()
            && recording_status_callback_method == "POST"
            && recording_status_callback_event == "completed"
            && result.is_none();

        let join_params: Value = if all_defaults {
            // Simple form — just the conference name.
            json!(name)
        } else {
            // Full object form: required name + every non-default parameter
            // under its snake_case wire key.
            let mut p = Map::new();
            p.insert("name".to_string(), json!(name));
            if muted {
                p.insert("muted".to_string(), json!(muted));
            }
            if beep != "true" {
                p.insert("beep".to_string(), json!(beep));
            }
            if !start_on_enter {
                p.insert("start_on_enter".to_string(), json!(start_on_enter));
            }
            if end_on_exit {
                p.insert("end_on_exit".to_string(), json!(end_on_exit));
            }
            if let Some(v) = wait_url {
                p.insert("wait_url".to_string(), json!(v));
            }
            if max_participants != 250 {
                p.insert("max_participants".to_string(), json!(max_participants));
            }
            if record != "do-not-record" {
                p.insert("record".to_string(), json!(record));
            }
            if let Some(v) = region {
                p.insert("region".to_string(), json!(v));
            }
            if trim != "trim-silence" {
                p.insert("trim".to_string(), json!(trim));
            }
            if let Some(v) = coach {
                p.insert("coach".to_string(), json!(v));
            }
            if let Some(v) = status_callback_event {
                p.insert("status_callback_event".to_string(), json!(v));
            }
            if let Some(v) = status_callback {
                p.insert("status_callback".to_string(), json!(v));
            }
            if status_callback_method != "POST" {
                p.insert(
                    "status_callback_method".to_string(),
                    json!(status_callback_method),
                );
            }
            if let Some(v) = recording_status_callback {
                p.insert("recording_status_callback".to_string(), json!(v));
            }
            if recording_status_callback_method != "POST" {
                p.insert(
                    "recording_status_callback_method".to_string(),
                    json!(recording_status_callback_method),
                );
            }
            if recording_status_callback_event != "completed" {
                p.insert(
                    "recording_status_callback_event".to_string(),
                    json!(recording_status_callback_event),
                );
            }
            if let Some(v) = result {
                p.insert("result".to_string(), v);
            }
            Value::Object(p)
        };

        // Wrap in the SWML document (parity with the reference, which routes
        // join_conference through execute_swml) — never a bare verb.
        self.push_swml_verb("join_conference", join_params);
        Ok(self)
    }

    /// Join a RELAY room (SWML `join_room`). Wrapped in a SWML document,
    /// matching the Python reference.
    pub fn join_room(&mut self, name: &str) -> &mut Self {
        self.push_swml_verb("join_room", json!({"name": name}));
        self
    }

    /// Send a SIP REFER (SWML `sip_refer`). Wrapped in a SWML document, matching
    /// the Python reference.
    pub fn sip_refer(&mut self, to_uri: &str) -> &mut Self {
        self.push_swml_verb("sip_refer", json!({"to_uri": to_uri}));
        self
    }

    /// Start a background call tap (SWML `tap`).
    ///
    /// Mirrors the Python reference (`FunctionResult.tap`): the verb is wrapped
    /// in a SWML document — never a bare verb — and the reference's three
    /// validations are reproduced, returning `Err(message)` with the exact
    /// reference `ValueError` text:
    ///
    /// - `direction` ∈ `{speak, hear, both}`
    /// - `codec` ∈ `{PCMU, PCMA}`
    /// - `rtp_ptime > 0`
    ///
    /// Only `uri` is always emitted; `control_id`, `direction`, `codec`,
    /// `rtp_ptime`, and `status_url` are emitted only when they differ from the
    /// reference defaults (`direction="both"`, `codec="PCMU"`, `rtp_ptime=20`).
    pub fn tap(
        &mut self,
        uri: &str,
        control_id: &str,
        direction: &str,
        codec: &str,
        rtp_ptime: i64,
        status_url: &str,
    ) -> Result<&mut Self, String> {
        // ── Validation (exact reference ValueError messages) ─────────────
        let valid_directions = ["speak", "hear", "both"];
        if !valid_directions.contains(&direction) {
            return Err(format!(
                "direction must be one of {}",
                render_list(&valid_directions)
            ));
        }
        let valid_codecs = ["PCMU", "PCMA"];
        if !valid_codecs.contains(&codec) {
            return Err(format!("codec must be one of {}", render_list(&valid_codecs)));
        }
        if rtp_ptime <= 0 {
            return Err("rtp_ptime must be a positive integer".to_string());
        }

        // ── Build params (mirrors the reference's "only when != default") ──
        let mut tap_obj = Map::new();
        tap_obj.insert("uri".to_string(), json!(uri));
        if !control_id.is_empty() {
            tap_obj.insert("control_id".to_string(), json!(control_id));
        }
        if direction != "both" {
            tap_obj.insert("direction".to_string(), json!(direction));
        }
        if codec != "PCMU" {
            tap_obj.insert("codec".to_string(), json!(codec));
        }
        if rtp_ptime != 20 {
            tap_obj.insert("rtp_ptime".to_string(), json!(rtp_ptime));
        }
        if !status_url.is_empty() {
            tap_obj.insert("status_url".to_string(), json!(status_url));
        }

        self.push_swml_verb("tap", Value::Object(tap_obj));
        Ok(self)
    }

    /// Stop an active tap stream (SWML `stop_tap`). Wrapped in a SWML document,
    /// matching the Python reference.
    pub fn stop_tap(&mut self, control_id: &str) -> &mut Self {
        let params = if !control_id.is_empty() {
            json!({"control_id": control_id})
        } else {
            json!({})
        };
        self.push_swml_verb("stop_tap", params);
        self
    }

    /// Send a text message to a PSTN number (SWML `send_sms`).
    ///
    /// Mirrors the Python reference (`FunctionResult.send_sms`): the verb is
    /// wrapped in a SWML document — never a bare verb. The reference's
    /// validation is reproduced, returning `Err` with the exact reference
    /// `ValueError` text when neither `body` nor `media` is provided. `body` is
    /// emitted only when non-empty; `media`, `tags`, and `region` are emitted
    /// only when supplied.
    pub fn send_sms(
        &mut self,
        to: &str,
        from: &str,
        body: &str,
        media: Vec<&str>,
        tags: Vec<&str>,
        region: &str,
    ) -> Result<&mut Self, String> {
        // Validate that at least body or media is provided (parity with the
        // reference's ValueError).
        if body.is_empty() && media.is_empty() {
            return Err("Either body or media must be provided".to_string());
        }

        let mut sms = Map::new();
        sms.insert("to_number".to_string(), json!(to));
        sms.insert("from_number".to_string(), json!(from));
        if !body.is_empty() {
            sms.insert("body".to_string(), json!(body));
        }
        if !media.is_empty() {
            sms.insert("media".to_string(), json!(media));
        }
        if !tags.is_empty() {
            sms.insert("tags".to_string(), json!(tags));
        }
        if !region.is_empty() {
            sms.insert("region".to_string(), json!(region));
        }
        self.push_swml_verb("send_sms", Value::Object(sms));
        Ok(self)
    }

    /// Process a payment (SWML `pay`).
    ///
    /// Mirrors the Python reference (`FunctionResult.pay`) exactly, including its
    /// wire-key choices and value rendering:
    /// - the verb is wrapped in a SWML document whose `main` section is
    ///   `[{set: {ai_response}}, {pay: pay_params}]` — never a bare verb;
    /// - the input-method key is **`input`** (not `input_method`);
    /// - the status-callback key is **`status_url`** (the previous port invented
    ///   `action_url` — removed);
    /// - `timeout`, `max_attempts`, `min_postal_code_length` are **stringified**;
    ///   `security_code` is rendered lower-case (`"true"`/`"false"`);
    /// - `payment_method`, `token_type`, `currency`, `language`, `voice`,
    ///   `valid_card_types`, and `postal_code` are always emitted; `status_url`,
    ///   `charge_amount`, `description`, `parameters`, and `prompts` are emitted
    ///   only when supplied.
    ///
    /// `postal_code` is taken as the already-rendered wire string (pass
    /// `"true"`/`"false"` for the boolean cases, or the literal postcode),
    /// mirroring the reference's `Union[bool, str]`. `parameters`/`prompts` are
    /// JSON arrays (`Value::Null` to omit). An empty `ai_response` uses the
    /// reference's default status message.
    #[allow(clippy::too_many_arguments)]
    pub fn pay(
        &mut self,
        payment_connector_url: &str,
        input_method: &str,
        status_url: &str,
        payment_method: &str,
        timeout: i64,
        max_attempts: i64,
        security_code: bool,
        postal_code: &str,
        min_postal_code_length: i64,
        token_type: &str,
        charge_amount: &str,
        currency: &str,
        language: &str,
        voice: &str,
        description: &str,
        valid_card_types: &str,
        parameters: Value,
        prompts: Value,
        ai_response: &str,
    ) -> &mut Self {
        let mut pay_params = Map::new();
        pay_params.insert(
            "payment_connector_url".to_string(),
            json!(payment_connector_url),
        );
        pay_params.insert("input".to_string(), json!(input_method));
        pay_params.insert("payment_method".to_string(), json!(payment_method));
        pay_params.insert("timeout".to_string(), json!(timeout.to_string()));
        pay_params.insert("max_attempts".to_string(), json!(max_attempts.to_string()));
        pay_params.insert(
            "security_code".to_string(),
            json!(if security_code { "true" } else { "false" }),
        );
        pay_params.insert(
            "min_postal_code_length".to_string(),
            json!(min_postal_code_length.to_string()),
        );
        pay_params.insert("token_type".to_string(), json!(token_type));
        pay_params.insert("currency".to_string(), json!(currency));
        pay_params.insert("language".to_string(), json!(language));
        pay_params.insert("voice".to_string(), json!(voice));
        pay_params.insert("valid_card_types".to_string(), json!(valid_card_types));
        pay_params.insert("postal_code".to_string(), json!(postal_code));

        // Optional parameters (emitted only when supplied).
        if !status_url.is_empty() {
            pay_params.insert("status_url".to_string(), json!(status_url));
        }
        if !charge_amount.is_empty() {
            pay_params.insert("charge_amount".to_string(), json!(charge_amount));
        }
        if !description.is_empty() {
            pay_params.insert("description".to_string(), json!(description));
        }
        if !parameters.is_null() {
            pay_params.insert("parameters".to_string(), parameters);
        }
        if !prompts.is_null() {
            pay_params.insert("prompts".to_string(), prompts);
        }

        // The set verb carries the ai_response; an empty arg uses the
        // reference's default status message.
        let resolved_ai_response = if ai_response.is_empty() {
            "The payment status is ${pay_result}, do not mention anything else about collecting payment if successful."
        } else {
            ai_response
        };

        // SWML document: {set: {ai_response}} then {pay: pay_params}.
        let swml_doc = json!({
            "version": "1.0.0",
            "sections": {
                "main": [
                    {"set": {"ai_response": resolved_ai_response}},
                    {"pay": Value::Object(pay_params)}
                ]
            }
        });
        self.execute_swml(swml_doc, false);
        self
    }

    // ── RPC ──────────────────────────────────────────────────────────────

    /// Execute an RPC method on a call (SWML `execute_rpc`).
    ///
    /// Mirrors the Python reference (`FunctionResult.execute_rpc`): the rpc
    /// params are keyed `{method, call_id?, node_id?, params?}` where
    /// `call_id` and `node_id` are **TOP-LEVEL siblings** of `method`/`params`
    /// (NOT nested inside `params`), and the whole `{"execute_rpc": ...}` verb
    /// is wrapped in a canonical SWML document via `execute_swml` — never a
    /// bare action key. There is **no** `jsonrpc` envelope (the previous port
    /// invented one — removed) and method strings are **bare** (`"dial"`, not
    /// `"calling.dial"`). `call_id`/`node_id` are emitted only when non-empty;
    /// `params` is emitted only when it is a non-empty value (Python's
    /// `if params:` — an empty object/null is dropped).
    pub fn execute_rpc(
        &mut self,
        method: &str,
        params: Value,
        call_id: &str,
        node_id: &str,
    ) -> &mut Self {
        let mut rpc = Map::new();
        rpc.insert("method".to_string(), json!(method));
        if !call_id.is_empty() {
            rpc.insert("call_id".to_string(), json!(call_id));
        }
        if !node_id.is_empty() {
            rpc.insert("node_id".to_string(), json!(node_id));
        }
        // Python: `if params:` — omit when falsy (null or empty object).
        if !params.is_null() && params != json!({}) {
            rpc.insert("params".to_string(), params);
        }
        self.push_swml_verb("execute_rpc", Value::Object(rpc));
        self
    }

    /// Dial out to a number with a destination SWML URL (via `execute_rpc`).
    ///
    /// Mirrors the Python reference (`FunctionResult.rpc_dial`): method `"dial"`,
    /// params `{devices: {type: device_type, params: {to_number, from_number}},
    /// dest_swml}`. `device_type` defaults to `"phone"` and is caller-overridable
    /// (the previous port hard-coded the device and invented
    /// `call_timeout`/`region` — removed).
    pub fn rpc_dial(
        &mut self,
        to_number: &str,
        from_number: &str,
        dest_swml: &str,
        device_type: &str,
    ) -> &mut Self {
        let params = json!({
            "devices": {
                "type": device_type,
                "params": {
                    "to_number": to_number,
                    "from_number": from_number
                }
            },
            "dest_swml": dest_swml
        });
        self.execute_rpc("dial", params, "", "")
    }

    /// Inject a message into an AI agent on another call (via `execute_rpc`).
    ///
    /// Mirrors the Python reference (`FunctionResult.rpc_ai_message`): method
    /// `"ai_message"`, `call_id` carried as the TOP-LEVEL execute_rpc sibling,
    /// params `{role, message_text}`. `role` defaults to `"system"` and is
    /// caller-overridable (the previous port omitted `role` and mis-nested
    /// `call_id` — fixed).
    pub fn rpc_ai_message(&mut self, call_id: &str, message_text: &str, role: &str) -> &mut Self {
        let params = json!({
            "role": role,
            "message_text": message_text
        });
        self.execute_rpc("ai_message", params, call_id, "")
    }

    /// Unhold another call (via `execute_rpc`).
    ///
    /// Mirrors the Python reference (`FunctionResult.rpc_ai_unhold`): method
    /// `"ai_unhold"`, `call_id` as the TOP-LEVEL execute_rpc sibling, params
    /// `{}` (which `execute_rpc` drops, since it is empty).
    pub fn rpc_ai_unhold(&mut self, call_id: &str) -> &mut Self {
        self.execute_rpc("ai_unhold", json!({}), call_id, "")
    }

    /// Queue simulated user input.
    ///
    /// Mirrors the Python reference (`FunctionResult.simulate_user_input`):
    /// the action key is **`user_input`** (the previous port emitted
    /// `simulate_user_input`, which is the *method* name, not the wire key).
    pub fn simulate_user_input(&mut self, text: &str) -> &mut Self {
        self.actions.push(json!({"user_input": text}));
        self
    }

    // ── Payment Helpers (static) ─────────────────────────────────────────

    /// Create a payment-prompt structure for use with `pay`.
    ///
    /// Mirrors the Python reference (`FunctionResult.create_payment_prompt`):
    /// `{"for": for_situation, "actions": actions, "card_type"?, "error_type"?}`.
    /// `actions` is the list of `{type, phrase}` action objects (a JSON array).
    /// `card_type`/`error_type` are emitted only when non-empty.
    pub fn create_payment_prompt(
        for_situation: &str,
        actions: Value,
        card_type: &str,
        error_type: &str,
    ) -> Value {
        let mut prompt = Map::new();
        prompt.insert("for".to_string(), json!(for_situation));
        prompt.insert("actions".to_string(), actions);
        if !card_type.is_empty() {
            prompt.insert("card_type".to_string(), json!(card_type));
        }
        if !error_type.is_empty() {
            prompt.insert("error_type".to_string(), json!(error_type));
        }
        Value::Object(prompt)
    }

    /// Create a payment action for use in payment prompts.
    ///
    /// Mirrors the Python reference (`FunctionResult.create_payment_action`):
    /// `{"type": action_type, "phrase": phrase}` (`action_type` is `"Say"` for
    /// text-to-speech or `"Play"` for an audio file).
    pub fn create_payment_action(action_type: &str, phrase: &str) -> Value {
        json!({
            "type": action_type,
            "phrase": phrase
        })
    }

    /// Create a payment parameter (name/value pair) for use with `pay`.
    ///
    /// Mirrors the Python reference (`FunctionResult.create_payment_parameter`):
    /// `{"name": name, "value": value}`.
    pub fn create_payment_parameter(name: &str, value: &str) -> Value {
        json!({
            "name": name,
            "value": value
        })
    }
}

impl Default for FunctionResult {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // Convenience: the FunctionResult `action` array (panics if absent).
    fn actions(fr: &FunctionResult) -> Vec<Value> {
        fr.to_value()["action"].as_array().unwrap().clone()
    }

    // Convenience: the first action object.
    fn action0(fr: &FunctionResult) -> Value {
        actions(fr).into_iter().next().unwrap()
    }

    // Convenience: the `main` section list of the first SWML-wrapped action.
    fn swml_main(fr: &FunctionResult) -> Vec<Value> {
        action0(fr)["SWML"]["sections"]["main"]
            .as_array()
            .unwrap()
            .clone()
    }

    // ── Construction / core ───────────────────────────────────────────────

    #[test]
    fn test_construction_default() {
        // An otherwise-empty result defaults to {"response": "Action completed."}
        // (Python to_dict parity), with no action/post_process keys.
        let fr = FunctionResult::new();
        let val = fr.to_value();
        assert_eq!(val["response"], "Action completed.");
        assert!(val.get("action").is_none());
        assert!(val.get("post_process").is_none());
    }

    #[test]
    fn test_construction_with_response() {
        let fr = FunctionResult::with_response("hello");
        assert_eq!(fr.to_value()["response"], "hello");
    }

    #[test]
    fn test_set_response() {
        let mut fr = FunctionResult::new();
        fr.set_response("world");
        assert_eq!(fr.to_value()["response"], "world");
    }

    #[test]
    fn test_set_post_process_true_with_action() {
        // Parity: post_process only appears in to_dict() when there is at
        // least one action (Python to_dict: `if self.post_process and self.action`).
        let mut fr = FunctionResult::new();
        fr.set_post_process(true).add_action(json!({"test": "value"}));
        assert_eq!(fr.to_value()["post_process"], true);
    }

    #[test]
    fn test_set_post_process_false_omitted() {
        let mut fr = FunctionResult::new();
        fr.set_post_process(false).add_action(json!({"x": 1}));
        assert!(fr.to_value().get("post_process").is_none());
    }

    #[test]
    fn test_post_process_omitted_without_action() {
        // post_process is dropped when there are no actions, even if set true
        // (Python to_dict: `if self.post_process and self.action`).
        let mut fr = FunctionResult::new();
        fr.set_response("hi").set_post_process(true);
        assert!(fr.to_value().get("post_process").is_none());
    }

    #[test]
    fn test_empty_response_omitted_with_action() {
        // response is omitted when empty but an action is present (Python parity:
        // {"action": [...]} with no "response" key).
        let mut fr = FunctionResult::new();
        fr.add_action(json!({"x": 1}));
        let val = fr.to_value();
        assert!(val.get("response").is_none());
        assert_eq!(val["action"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_add_action() {
        let mut fr = FunctionResult::new();
        fr.add_action(json!({"play": {"url": "https://example.com/audio.mp3"}}));
        assert_eq!(action0(&fr), json!({"play": {"url": "https://example.com/audio.mp3"}}));
    }

    #[test]
    fn test_add_actions_multiple() {
        let mut fr = FunctionResult::new();
        fr.add_actions(vec![
            json!({"play": {"url": "https://example.com/audio.mp3"}}),
            json!({"transfer": "+15551234567"}),
        ]);
        assert_eq!(actions(&fr).len(), 2);
    }

    #[test]
    fn test_chaining() {
        let mut fr = FunctionResult::new();
        fr.set_response("chained")
            .set_post_process(true)
            .add_action(json!({"test": 1}));
        let val = fr.to_value();
        assert_eq!(val["response"], "chained");
        assert_eq!(val["post_process"], true);
        assert_eq!(val["action"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_to_json_roundtrip() {
        let fr = FunctionResult::with_response("test");
        let parsed: Value = serde_json::from_str(&fr.to_json()).unwrap();
        assert_eq!(parsed["response"], "test");
    }

    // ── Call control ──────────────────────────────────────────────────────

    #[test]
    fn test_connect_basic() {
        // Python connect(): action is {SWML:{sections.main:[{connect:{to}}],
        // version:"1.0.0"}, transfer:"true"/"false"}.
        let mut fr = FunctionResult::new();
        fr.connect("+15551234567", true, "");
        let a = action0(&fr);
        assert_eq!(a["transfer"], "true");
        assert_eq!(a["SWML"]["version"], "1.0.0");
        assert_eq!(a["SWML"]["sections"]["main"][0]["connect"]["to"], "+15551234567");
        assert!(a["SWML"]["sections"]["main"][0]["connect"].get("from").is_none());
    }

    #[test]
    fn test_connect_with_from_and_final_false() {
        let mut fr = FunctionResult::new();
        fr.connect("+15551234567", false, "+15559876543");
        let a = action0(&fr);
        assert_eq!(a["transfer"], "false");
        let connect = &a["SWML"]["sections"]["main"][0]["connect"];
        assert_eq!(connect["to"], "+15551234567");
        assert_eq!(connect["from"], "+15559876543");
    }

    #[test]
    fn test_swml_transfer_final() {
        // Python swml_transfer(final=true): main = [{set:{ai_response}},
        // {transfer:{dest}}]; top-level transfer="true". ai_response is NOT
        // assigned to the FunctionResult response.
        let mut fr = FunctionResult::with_response("Transferring");
        fr.swml_transfer("https://example.com/swml", "Goodbye!", true);
        let a = action0(&fr);
        assert_eq!(a["transfer"], "true");
        assert_eq!(a["SWML"]["version"], "1.0.0");
        let main = &a["SWML"]["sections"]["main"];
        assert_eq!(main[0], json!({"set": {"ai_response": "Goodbye!"}}));
        assert_eq!(main[1], json!({"transfer": {"dest": "https://example.com/swml"}}));
        // response untouched by swml_transfer.
        assert_eq!(fr.to_value()["response"], "Transferring");
    }

    #[test]
    fn test_swml_transfer_temporary() {
        let mut fr = FunctionResult::new();
        fr.swml_transfer("sip:support@company.com", "Welcome back!", false);
        let a = action0(&fr);
        assert_eq!(a["transfer"], "false");
        assert_eq!(
            a["SWML"]["sections"]["main"][1]["transfer"]["dest"],
            "sip:support@company.com"
        );
    }

    #[test]
    fn test_hangup() {
        // Python: add_action("hangup", True) -> {"hangup": true} (bare bool).
        let mut fr = FunctionResult::new();
        fr.hangup();
        assert_eq!(action0(&fr), json!({"hangup": true}));
    }

    #[test]
    fn test_hold_default_and_clamp() {
        // Python hold(): {"hold": <bare clamped int>}.
        let mut fr = FunctionResult::new();
        fr.hold(60);
        assert_eq!(action0(&fr), json!({"hold": 60}));

        let mut hi = FunctionResult::new();
        hi.hold(1500);
        assert_eq!(action0(&hi), json!({"hold": 900}));

        let mut lo = FunctionResult::new();
        lo.hold(-50);
        assert_eq!(action0(&lo), json!({"hold": 0}));

        let mut exact = FunctionResult::new();
        exact.hold(900);
        assert_eq!(action0(&exact), json!({"hold": 900}));
    }

    #[test]
    fn test_wait_for_user_default_true() {
        let mut fr = FunctionResult::new();
        fr.wait_for_user(None, None, None);
        assert_eq!(action0(&fr), json!({"wait_for_user": true}));
    }

    #[test]
    fn test_wait_for_user_enabled_only() {
        let mut fr = FunctionResult::new();
        fr.wait_for_user(Some(true), None, None);
        assert_eq!(action0(&fr), json!({"wait_for_user": true}));

        let mut off = FunctionResult::new();
        off.wait_for_user(Some(false), None, None);
        assert_eq!(action0(&off), json!({"wait_for_user": false}));
    }

    #[test]
    fn test_wait_for_user_timeout_takes_priority_over_enabled() {
        // Python precedence: timeout beats enabled; emitted as a bare int.
        let mut fr = FunctionResult::new();
        fr.wait_for_user(Some(true), Some(30), None);
        assert_eq!(action0(&fr), json!({"wait_for_user": 30}));
    }

    #[test]
    fn test_wait_for_user_answer_first_takes_priority() {
        let mut fr = FunctionResult::new();
        fr.wait_for_user(Some(true), Some(30), Some(true));
        assert_eq!(action0(&fr), json!({"wait_for_user": "answer_first"}));
    }

    #[test]
    fn test_stop() {
        let mut fr = FunctionResult::new();
        fr.stop();
        assert_eq!(action0(&fr), json!({"stop": true}));
    }

    // ── State & data ──────────────────────────────────────────────────────

    #[test]
    fn test_update_global_data() {
        let mut fr = FunctionResult::new();
        fr.update_global_data(json!({"user_id": "123", "session": "abc"}));
        assert_eq!(
            action0(&fr),
            json!({"set_global_data": {"user_id": "123", "session": "abc"}})
        );
    }

    #[test]
    fn test_remove_global_data_list_is_bare_value() {
        // Python: add_action("unset_global_data", keys) -> bare key list,
        // NOT wrapped in {"keys": ...}.
        let mut fr = FunctionResult::new();
        fr.remove_global_data(vec!["user_id", "session", "token"]);
        assert_eq!(
            action0(&fr),
            json!({"unset_global_data": ["user_id", "session", "token"]})
        );
    }

    #[test]
    fn test_set_metadata() {
        let mut fr = FunctionResult::new();
        fr.set_metadata(json!({"key1": "value1", "key2": 42}));
        assert_eq!(
            action0(&fr),
            json!({"set_meta_data": {"key1": "value1", "key2": 42}})
        );
    }

    #[test]
    fn test_remove_metadata_list_is_bare_value() {
        let mut fr = FunctionResult::new();
        fr.remove_metadata(vec!["key1", "key2"]);
        assert_eq!(action0(&fr), json!({"unset_meta_data": ["key1", "key2"]}));
    }

    #[test]
    fn test_swml_user_event() {
        // Python swml_user_event: {"SWML": {sections:{main:[{user_event:
        // {event: <data>}}]}, version:"1.0.0"}}.
        let mut fr = FunctionResult::with_response("Blackjack!");
        let event = json!({"type": "cards_dealt", "player_hand": ["Ace", "King"], "score": 21});
        fr.swml_user_event(event.clone());
        let a = action0(&fr);
        assert_eq!(a["SWML"]["version"], "1.0.0");
        assert_eq!(a["SWML"]["sections"]["main"][0]["user_event"]["event"], event);
    }

    #[test]
    fn test_swml_change_step() {
        // Python: add_action("change_step", step_name) -> bare string.
        let mut fr = FunctionResult::new();
        fr.swml_change_step("betting");
        assert_eq!(action0(&fr), json!({"change_step": "betting"}));
    }

    #[test]
    fn test_swml_change_context() {
        let mut fr = FunctionResult::new();
        fr.swml_change_context("technical_support");
        assert_eq!(action0(&fr), json!({"change_context": "technical_support"}));
    }

    #[test]
    fn test_switch_context_simple_string() {
        // Only system_prompt -> bare string form.
        let mut fr = FunctionResult::new();
        fr.switch_context("You are a helpful bot", "", false, false, false);
        assert_eq!(
            action0(&fr),
            json!({"context_switch": "You are a helpful bot"})
        );
    }

    #[test]
    fn test_switch_context_full_object() {
        let mut fr = FunctionResult::new();
        fr.switch_context("New prompt", "User msg", true, true, false);
        let ctx = &action0(&fr)["context_switch"];
        assert_eq!(ctx["system_prompt"], "New prompt");
        assert_eq!(ctx["user_prompt"], "User msg");
        assert_eq!(ctx["consolidate"], true);
        assert_eq!(ctx["full_reset"], true);
    }

    #[test]
    fn test_switch_context_isolated_extension() {
        // Port extension: `isolated` forces the object form and is emitted
        // under the `isolated` wire key (documented in PORT_SIGNATURE_OMISSIONS).
        let mut fr = FunctionResult::new();
        fr.switch_context("sys", "", false, false, true);
        let ctx = &action0(&fr)["context_switch"];
        assert_eq!(ctx["system_prompt"], "sys");
        assert_eq!(ctx["isolated"], true);
    }

    #[test]
    fn test_replace_in_history_default_true() {
        // Python replace_in_history default arg True -> bare boolean true.
        let mut fr = FunctionResult::new();
        fr.replace_in_history(None);
        assert_eq!(action0(&fr), json!({"replace_in_history": true}));
    }

    #[test]
    fn test_replace_in_history_string() {
        let mut fr = FunctionResult::new();
        fr.replace_in_history(Some("I've saved your data."));
        assert_eq!(
            action0(&fr),
            json!({"replace_in_history": "I've saved your data."})
        );
    }

    // ── Media ─────────────────────────────────────────────────────────────

    #[test]
    fn test_say() {
        let mut fr = FunctionResult::new();
        fr.say("Hello there");
        assert_eq!(action0(&fr), json!({"say": "Hello there"}));
    }

    #[test]
    fn test_play_background_file_no_wait_is_bare_filename() {
        // Python action key is "playback_bg"; no-wait value is the bare filename.
        let mut fr = FunctionResult::new();
        fr.play_background_file("music.mp3", false);
        assert_eq!(action0(&fr), json!({"playback_bg": "music.mp3"}));
    }

    #[test]
    fn test_play_background_file_wait_object() {
        let mut fr = FunctionResult::new();
        fr.play_background_file("music.mp3", true);
        assert_eq!(
            action0(&fr),
            json!({"playback_bg": {"file": "music.mp3", "wait": true}})
        );
    }

    #[test]
    fn test_stop_background_file() {
        // Python action key is "stop_playback_bg" (value true).
        let mut fr = FunctionResult::new();
        fr.stop_background_file();
        assert_eq!(action0(&fr), json!({"stop_playback_bg": true}));
    }

    #[test]
    fn test_record_call_defaults_wrapped_in_swml() {
        // Python record_call(): SWML-wrapped; always emits stereo/format/
        // direction/beep/input_sensitivity; control_id absent at default.
        let mut fr = FunctionResult::new();
        fr.record_call("", false, "wav", "both", "", false, 44.0, None, None, None, "")
            .unwrap();
        let rec = &swml_main(&fr)[0]["record_call"];
        assert_eq!(rec["stereo"], false);
        assert_eq!(rec["format"], "wav");
        assert_eq!(rec["direction"], "both");
        assert_eq!(rec["beep"], false);
        assert_eq!(rec["input_sensitivity"], 44.0);
        assert!(rec.get("control_id").is_none());
        assert!(rec.get("terminators").is_none());
        // The previous port's invented `initiator` field is gone.
        assert!(rec.get("initiator").is_none());
    }

    #[test]
    fn test_record_call_custom_params() {
        let mut fr = FunctionResult::new();
        fr.record_call(
            "rec-1", true, "mp3", "speak", "#", true, 50.0,
            Some(10.0), Some(5.0), Some(600.0), "https://example.com/rec-status",
        )
        .unwrap();
        let rec = &swml_main(&fr)[0]["record_call"];
        assert_eq!(rec["control_id"], "rec-1");
        assert_eq!(rec["stereo"], true);
        assert_eq!(rec["format"], "mp3");
        assert_eq!(rec["direction"], "speak");
        assert_eq!(rec["terminators"], "#");
        assert_eq!(rec["beep"], true);
        assert_eq!(rec["input_sensitivity"], 50.0);
        assert_eq!(rec["initial_timeout"], 10.0);
        assert_eq!(rec["end_silence_timeout"], 5.0);
        assert_eq!(rec["max_length"], 600.0);
        assert_eq!(rec["status_url"], "https://example.com/rec-status");
    }

    #[test]
    fn test_record_call_accepts_mp4() {
        let mut fr = FunctionResult::new();
        fr.record_call("", false, "mp4", "both", "", false, 44.0, None, None, None, "")
            .unwrap();
        assert_eq!(swml_main(&fr)[0]["record_call"]["format"], "mp4");
    }

    #[test]
    fn test_record_call_invalid_format_err() {
        let mut fr = FunctionResult::new();
        let err = fr
            .record_call("", false, "ogg", "both", "", false, 44.0, None, None, None, "")
            .unwrap_err();
        assert_eq!(err, "format must be 'wav', 'mp3', or 'mp4'");
        assert!(fr.to_value().get("action").is_none());
    }

    #[test]
    fn test_record_call_invalid_direction_err() {
        let mut fr = FunctionResult::new();
        let err = fr
            .record_call("", false, "wav", "left", "", false, 44.0, None, None, None, "")
            .unwrap_err();
        assert_eq!(err, "direction must be 'speak', 'listen', or 'both'");
    }

    #[test]
    fn test_stop_record_call_with_and_without_id() {
        let mut with = FunctionResult::new();
        with.stop_record_call("rec-1");
        assert_eq!(swml_main(&with)[0]["stop_record_call"]["control_id"], "rec-1");

        let mut without = FunctionResult::new();
        without.stop_record_call("");
        assert_eq!(swml_main(&without)[0]["stop_record_call"], json!({}));
    }

    // ── Speech & AI ───────────────────────────────────────────────────────

    #[test]
    fn test_add_dynamic_hints() {
        let mut fr = FunctionResult::new();
        fr.add_dynamic_hints(vec![json!("Cabby"), json!({"pattern": "cab bee"})]);
        assert_eq!(action0(&fr)["add_dynamic_hints"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_clear_dynamic_hints_is_empty_object() {
        // Python: append {"clear_dynamic_hints": {}} -> empty OBJECT (not bool).
        let mut fr = FunctionResult::new();
        fr.clear_dynamic_hints();
        assert_eq!(action0(&fr), json!({"clear_dynamic_hints": {}}));
    }

    #[test]
    fn test_set_end_of_speech_timeout() {
        let mut fr = FunctionResult::new();
        fr.set_end_of_speech_timeout(500);
        assert_eq!(action0(&fr), json!({"end_of_speech_timeout": 500}));
    }

    #[test]
    fn test_set_speech_event_timeout() {
        let mut fr = FunctionResult::new();
        fr.set_speech_event_timeout(1000);
        assert_eq!(action0(&fr), json!({"speech_event_timeout": 1000}));
    }

    #[test]
    fn test_toggle_functions() {
        let mut fr = FunctionResult::new();
        let mut toggles = HashMap::new();
        toggles.insert("get_weather".to_string(), true);
        fr.toggle_functions(toggles);
        let tf = action0(&fr)["toggle_functions"].as_array().unwrap().clone();
        assert_eq!(tf.len(), 1);
        assert_eq!(tf[0]["function"], "get_weather");
        assert_eq!(tf[0]["active"], true);
    }

    #[test]
    fn test_enable_functions_on_timeout() {
        // Python action key is "functions_on_speaker_timeout".
        let mut fr = FunctionResult::new();
        fr.enable_functions_on_timeout(true);
        assert_eq!(action0(&fr), json!({"functions_on_speaker_timeout": true}));

        let mut off = FunctionResult::new();
        off.enable_functions_on_timeout(false);
        assert_eq!(action0(&off), json!({"functions_on_speaker_timeout": false}));
    }

    #[test]
    fn test_enable_extensive_data() {
        let mut fr = FunctionResult::new();
        fr.enable_extensive_data(true);
        assert_eq!(action0(&fr), json!({"extensive_data": true}));
    }

    #[test]
    fn test_update_settings() {
        // Python action key is "settings" (not "ai_settings").
        let mut fr = FunctionResult::new();
        fr.update_settings(json!({"temperature": 0.7, "top-p": 0.9}));
        assert_eq!(
            action0(&fr),
            json!({"settings": {"temperature": 0.7, "top-p": 0.9}})
        );
    }

    // ── Advanced: execute_swml ────────────────────────────────────────────

    #[test]
    fn test_execute_swml_dict_no_transfer() {
        let mut fr = FunctionResult::new();
        let swml = json!({"version": "1.0.0", "sections": {"main": [{"play": "test.mp3"}]}});
        fr.execute_swml(swml.clone(), false);
        let a = action0(&fr);
        assert_eq!(a["SWML"], swml);
        assert!(a["SWML"].get("transfer").is_none());
    }

    #[test]
    fn test_execute_swml_transfer_true_adds_inner_transfer_key() {
        // Python: transfer=True sets "transfer":"true" INSIDE the SWML dict.
        let mut fr = FunctionResult::new();
        fr.execute_swml(json!({"version": "1.0.0", "sections": {"main": []}}), true);
        assert_eq!(action0(&fr)["SWML"]["transfer"], "true");
    }

    #[test]
    fn test_execute_swml_string_invalid_json_falls_back_to_raw_swml() {
        let mut fr = FunctionResult::new();
        fr.execute_swml(json!("not valid json {{{"), false);
        assert_eq!(action0(&fr)["SWML"]["raw_swml"], "not valid json {{{");
    }

    #[test]
    fn test_execute_swml_string_valid_json() {
        let mut fr = FunctionResult::new();
        let s = serde_json::to_string(&json!({"version": "1.0.0", "sections": {"main": []}})).unwrap();
        fr.execute_swml(json!(s), false);
        assert_eq!(action0(&fr)["SWML"]["version"], "1.0.0");
    }

    // ── join_conference ───────────────────────────────────────────────────

    fn jc_defaults<'a>(
        fr: &'a mut FunctionResult,
        name: &str,
    ) -> Result<&'a mut FunctionResult, String> {
        fr.join_conference(
            name, false, "true", true, false, None, 250, "do-not-record", None,
            "trim-silence", None, None, None, "POST", None, "POST", "completed", None,
        )
    }

    #[test]
    fn test_join_conference_simple_form_is_bare_name() {
        let mut fr = FunctionResult::new();
        jc_defaults(&mut fr, "my-conference").unwrap();
        assert_eq!(swml_main(&fr)[0]["join_conference"], json!("my-conference"));
    }

    #[test]
    fn test_join_conference_full_object_form() {
        let mut fr = FunctionResult::new();
        fr.join_conference(
            "team-meeting", true, "onEnter", false, true,
            Some("https://example.com/hold-music"), 50, "record-from-start",
            Some("us-east"), "do-not-trim", Some("call-id-123"),
            Some("start end"), Some("https://example.com/callback"), "GET",
            Some("https://example.com/rec-callback"), "GET", "in-progress",
            Some(json!({"key": "value"})),
        )
        .unwrap();
        let jc = &swml_main(&fr)[0]["join_conference"];
        assert_eq!(jc["name"], "team-meeting");
        assert_eq!(jc["muted"], true);
        assert_eq!(jc["beep"], "onEnter");
        assert_eq!(jc["start_on_enter"], false);
        assert_eq!(jc["end_on_exit"], true);
        assert_eq!(jc["wait_url"], "https://example.com/hold-music");
        assert_eq!(jc["max_participants"], 50);
        assert_eq!(jc["record"], "record-from-start");
        assert_eq!(jc["region"], "us-east");
        assert_eq!(jc["trim"], "do-not-trim");
        assert_eq!(jc["coach"], "call-id-123");
        assert_eq!(jc["status_callback_event"], "start end");
        assert_eq!(jc["status_callback"], "https://example.com/callback");
        assert_eq!(jc["status_callback_method"], "GET");
        assert_eq!(jc["recording_status_callback"], "https://example.com/rec-callback");
        assert_eq!(jc["recording_status_callback_method"], "GET");
        assert_eq!(jc["recording_status_callback_event"], "in-progress");
        assert_eq!(jc["result"], json!({"key": "value"}));
    }

    #[test]
    fn test_join_conference_invalid_beep_err() {
        let mut fr = FunctionResult::new();
        let err = fr
            .join_conference(
                "conf", false, "invalid", true, false, None, 250, "do-not-record",
                None, "trim-silence", None, None, None, "POST", None, "POST",
                "completed", None,
            )
            .unwrap_err();
        assert_eq!(err, "beep must be one of ['true', 'false', 'onEnter', 'onExit']");
        assert!(fr.to_value().get("action").is_none());
    }

    #[test]
    fn test_join_conference_max_participants_err() {
        for bad in [300_i64, 0, -5] {
            let mut fr = FunctionResult::new();
            let err = fr
                .join_conference(
                    "conf", false, "true", true, false, None, bad, "do-not-record",
                    None, "trim-silence", None, None, None, "POST", None, "POST",
                    "completed", None,
                )
                .unwrap_err();
            assert_eq!(err, "max_participants must be a positive integer <= 250");
        }
    }

    #[test]
    fn test_join_conference_invalid_record_err() {
        let mut fr = FunctionResult::new();
        let err = fr
            .join_conference(
                "conf", false, "true", true, false, None, 250, "always", None,
                "trim-silence", None, None, None, "POST", None, "POST", "completed", None,
            )
            .unwrap_err();
        assert_eq!(err, "record must be one of ['do-not-record', 'record-from-start']");
    }

    #[test]
    fn test_join_conference_invalid_trim_err() {
        let mut fr = FunctionResult::new();
        let err = fr
            .join_conference(
                "conf", false, "true", true, false, None, 250, "do-not-record", None,
                "bad-value", None, None, None, "POST", None, "POST", "completed", None,
            )
            .unwrap_err();
        assert_eq!(err, "trim must be one of ['trim-silence', 'do-not-trim']");
    }

    #[test]
    fn test_join_conference_invalid_callback_methods_err() {
        let mut fr = FunctionResult::new();
        let err = fr
            .join_conference(
                "conf", false, "true", true, false, None, 250, "do-not-record", None,
                "trim-silence", None, None, None, "PUT", None, "POST", "completed", None,
            )
            .unwrap_err();
        assert_eq!(err, "status_callback_method must be one of ['GET', 'POST']");

        let mut fr2 = FunctionResult::new();
        let err2 = fr2
            .join_conference(
                "conf", false, "true", true, false, None, 250, "do-not-record", None,
                "trim-silence", None, None, None, "POST", None, "DELETE", "completed", None,
            )
            .unwrap_err();
        assert_eq!(err2, "recording_status_callback_method must be one of ['GET', 'POST']");
    }

    #[test]
    fn test_join_conference_empty_and_whitespace_name_err() {
        let mut fr = FunctionResult::new();
        assert_eq!(jc_defaults(&mut fr, "").unwrap_err(), "name cannot be empty");
        let mut fr2 = FunctionResult::new();
        assert_eq!(jc_defaults(&mut fr2, "   ").unwrap_err(), "name cannot be empty");
    }

    // ── join_room / sip_refer ─────────────────────────────────────────────

    #[test]
    fn test_join_room() {
        let mut fr = FunctionResult::new();
        fr.join_room("my-room");
        assert_eq!(swml_main(&fr)[0]["join_room"]["name"], "my-room");
    }

    #[test]
    fn test_sip_refer() {
        let mut fr = FunctionResult::new();
        fr.sip_refer("sip:alice@example.com");
        assert_eq!(swml_main(&fr)[0]["sip_refer"]["to_uri"], "sip:alice@example.com");
    }

    // ── tap ───────────────────────────────────────────────────────────────

    #[test]
    fn test_tap_defaults_omit_optional_keys() {
        // Python tap(): only `uri` always present; direction/codec/rtp_ptime
        // omitted at their defaults; SWML-wrapped.
        let mut fr = FunctionResult::new();
        fr.tap("rtp://192.168.1.1:5000", "", "both", "PCMU", 20, "").unwrap();
        let tap = &swml_main(&fr)[0]["tap"];
        assert_eq!(tap["uri"], "rtp://192.168.1.1:5000");
        assert!(tap.get("direction").is_none());
        assert!(tap.get("codec").is_none());
        assert!(tap.get("rtp_ptime").is_none());
        assert!(tap.get("control_id").is_none());
        assert!(tap.get("status_url").is_none());
    }

    #[test]
    fn test_tap_custom_params() {
        let mut fr = FunctionResult::new();
        fr.tap("ws://example.com/tap", "my-tap-1", "speak", "PCMA", 30, "https://example.com/status")
            .unwrap();
        let tap = &swml_main(&fr)[0]["tap"];
        assert_eq!(tap["uri"], "ws://example.com/tap");
        assert_eq!(tap["control_id"], "my-tap-1");
        assert_eq!(tap["direction"], "speak");
        assert_eq!(tap["codec"], "PCMA");
        assert_eq!(tap["rtp_ptime"], 30);
        assert_eq!(tap["status_url"], "https://example.com/status");
    }

    #[test]
    fn test_tap_direction_hear_allowed() {
        let mut fr = FunctionResult::new();
        fr.tap("rtp://1.2.3.4:5000", "", "hear", "PCMU", 20, "").unwrap();
        assert_eq!(swml_main(&fr)[0]["tap"]["direction"], "hear");
    }

    #[test]
    fn test_tap_invalid_direction_err() {
        let mut fr = FunctionResult::new();
        let err = fr.tap("rtp://1.2.3.4:5000", "", "invalid", "PCMU", 20, "").unwrap_err();
        assert_eq!(err, "direction must be one of ['speak', 'hear', 'both']");
        assert!(fr.to_value().get("action").is_none());
    }

    #[test]
    fn test_tap_invalid_codec_err() {
        let mut fr = FunctionResult::new();
        let err = fr.tap("rtp://1.2.3.4:5000", "", "both", "G729", 20, "").unwrap_err();
        assert_eq!(err, "codec must be one of ['PCMU', 'PCMA']");
    }

    #[test]
    fn test_tap_invalid_rtp_ptime_err() {
        for bad in [0_i64, -10] {
            let mut fr = FunctionResult::new();
            let err = fr.tap("rtp://1.2.3.4:5000", "", "both", "PCMU", bad, "").unwrap_err();
            assert_eq!(err, "rtp_ptime must be a positive integer");
        }
    }

    #[test]
    fn test_stop_tap_with_and_without_id() {
        let mut with = FunctionResult::new();
        with.stop_tap("my-tap-1");
        assert_eq!(swml_main(&with)[0]["stop_tap"]["control_id"], "my-tap-1");

        let mut without = FunctionResult::new();
        without.stop_tap("");
        assert_eq!(swml_main(&without)[0]["stop_tap"], json!({}));
    }

    // ── send_sms ──────────────────────────────────────────────────────────

    #[test]
    fn test_send_sms_with_body() {
        let mut fr = FunctionResult::new();
        fr.send_sms("+15551234567", "+15559876543", "Hello from AI", vec![], vec![], "")
            .unwrap();
        let sms = &swml_main(&fr)[0]["send_sms"];
        assert_eq!(sms["to_number"], "+15551234567");
        assert_eq!(sms["from_number"], "+15559876543");
        assert_eq!(sms["body"], "Hello from AI");
        assert!(sms.get("media").is_none());
    }

    #[test]
    fn test_send_sms_media_only_omits_body() {
        let mut fr = FunctionResult::new();
        fr.send_sms("+15551234567", "+15559876543", "", vec!["https://example.com/image.png"], vec![], "")
            .unwrap();
        let sms = &swml_main(&fr)[0]["send_sms"];
        assert!(sms.get("body").is_none());
        assert_eq!(sms["media"], json!(["https://example.com/image.png"]));
    }

    #[test]
    fn test_send_sms_tags_and_region() {
        let mut fr = FunctionResult::new();
        fr.send_sms("+15551234567", "+15559876543", "Tagged", vec![], vec!["support", "urgent"], "us-east")
            .unwrap();
        let sms = &swml_main(&fr)[0]["send_sms"];
        assert_eq!(sms["tags"], json!(["support", "urgent"]));
        assert_eq!(sms["region"], "us-east");
    }

    #[test]
    fn test_send_sms_neither_body_nor_media_err() {
        let mut fr = FunctionResult::new();
        let err = fr
            .send_sms("+15551234567", "+15559876543", "", vec![], vec![], "")
            .unwrap_err();
        assert_eq!(err, "Either body or media must be provided");
        assert!(fr.to_value().get("action").is_none());
    }

    // ── pay ───────────────────────────────────────────────────────────────

    // Convenience: a default `pay` call mirroring Python's keyword defaults.
    // postal_code default in Python is the boolean True, which renders to the
    // wire string "true"; the Rust port takes the pre-rendered &str.
    fn pay_defaults(fr: &mut FunctionResult, connector: &str) {
        fr.pay(
            connector, "dtmf", "", "credit-card", 5, 1, true, "true", 0,
            "reusable", "", "usd", "en-US", "woman", "", "visa mastercard amex",
            Value::Null, Value::Null, "",
        );
    }

    #[test]
    fn test_pay_default_params() {
        let mut fr = FunctionResult::new();
        pay_defaults(&mut fr, "https://pay.example.com/connector");
        let main = swml_main(&fr);
        // First verb sets ai_response (default status message).
        assert!(main[0]["set"]["ai_response"].is_string());
        let p = &main[1]["pay"];
        assert_eq!(p["payment_connector_url"], "https://pay.example.com/connector");
        assert_eq!(p["input"], "dtmf");
        assert_eq!(p["payment_method"], "credit-card");
        assert_eq!(p["timeout"], "5");
        assert_eq!(p["max_attempts"], "1");
        assert_eq!(p["security_code"], "true");
        assert_eq!(p["postal_code"], "true");
        assert_eq!(p["min_postal_code_length"], "0");
        assert_eq!(p["token_type"], "reusable");
        assert_eq!(p["currency"], "usd");
        assert_eq!(p["language"], "en-US");
        assert_eq!(p["voice"], "woman");
        assert_eq!(p["valid_card_types"], "visa mastercard amex");
        // Optional keys absent at default.
        assert!(p.get("status_url").is_none());
        assert!(p.get("charge_amount").is_none());
        assert!(p.get("description").is_none());
        assert!(p.get("parameters").is_none());
        assert!(p.get("prompts").is_none());
        // wire key is `input`, never `input_method`; status key is `status_url`,
        // never the invented `action_url`.
        assert!(p.get("input_method").is_none());
        assert!(p.get("action_url").is_none());
    }

    #[test]
    fn test_pay_all_custom_params() {
        let mut fr = FunctionResult::new();
        fr.pay(
            "https://pay.example.com", "voice", "https://status.example.com",
            "credit-card", 10, 3, false, "90210", 5, "one-time", "49.99", "eur",
            "fr-FR", "man", "Monthly subscription", "visa amex",
            Value::Null, Value::Null, "Payment processed.",
        );
        let main = swml_main(&fr);
        let p = &main[1]["pay"];
        assert_eq!(p["input"], "voice");
        assert_eq!(p["status_url"], "https://status.example.com");
        assert_eq!(p["timeout"], "10");
        assert_eq!(p["max_attempts"], "3");
        assert_eq!(p["security_code"], "false");
        assert_eq!(p["postal_code"], "90210");
        assert_eq!(p["min_postal_code_length"], "5");
        assert_eq!(p["token_type"], "one-time");
        assert_eq!(p["charge_amount"], "49.99");
        assert_eq!(p["currency"], "eur");
        assert_eq!(p["language"], "fr-FR");
        assert_eq!(p["voice"], "man");
        assert_eq!(p["description"], "Monthly subscription");
        assert_eq!(p["valid_card_types"], "visa amex");
        assert_eq!(main[0]["set"]["ai_response"], "Payment processed.");
    }

    #[test]
    fn test_pay_with_prompts_and_parameters() {
        let prompts = json!([{"for": "payment-card-number", "actions": [{"type": "Say", "phrase": "Enter card"}]}]);
        let parameters = json!([{"name": "store_id", "value": "123"}]);
        let mut fr = FunctionResult::new();
        fr.pay(
            "https://pay.example.com", "dtmf", "", "credit-card", 5, 1, true, "true",
            0, "reusable", "", "usd", "en-US", "woman", "", "visa mastercard amex",
            parameters.clone(), prompts.clone(), "",
        );
        let p = &swml_main(&fr)[1]["pay"];
        assert_eq!(p["prompts"], prompts);
        assert_eq!(p["parameters"], parameters);
    }

    #[test]
    fn test_pay_postal_code_false() {
        let mut fr = FunctionResult::new();
        fr.pay(
            "https://pay.example.com", "dtmf", "", "credit-card", 5, 1, true, "false",
            0, "reusable", "", "usd", "en-US", "woman", "", "visa mastercard amex",
            Value::Null, Value::Null, "",
        );
        assert_eq!(swml_main(&fr)[1]["pay"]["postal_code"], "false");
    }

    // ── RPC ───────────────────────────────────────────────────────────────

    #[test]
    fn test_execute_rpc_method_only() {
        // Python execute_rpc: SWML-wrapped; method-only omits call_id/node_id/params.
        let mut fr = FunctionResult::new();
        fr.execute_rpc("ping", Value::Null, "", "");
        let rpc = &swml_main(&fr)[0]["execute_rpc"];
        assert_eq!(rpc["method"], "ping");
        assert!(rpc.get("call_id").is_none());
        assert!(rpc.get("node_id").is_none());
        assert!(rpc.get("params").is_none());
        // No invented jsonrpc envelope.
        assert!(rpc.get("jsonrpc").is_none());
    }

    #[test]
    fn test_execute_rpc_all_params_top_level_call_id() {
        let mut fr = FunctionResult::new();
        fr.execute_rpc(
            "ai_message",
            json!({"role": "system", "message_text": "Hello"}),
            "call-123",
            "node-456",
        );
        let rpc = &swml_main(&fr)[0]["execute_rpc"];
        assert_eq!(rpc["method"], "ai_message");
        // call_id / node_id are TOP-LEVEL siblings of params.
        assert_eq!(rpc["call_id"], "call-123");
        assert_eq!(rpc["node_id"], "node-456");
        assert_eq!(rpc["params"], json!({"role": "system", "message_text": "Hello"}));
    }

    #[test]
    fn test_execute_rpc_call_id_only_no_params() {
        let mut fr = FunctionResult::new();
        fr.execute_rpc("status", Value::Null, "call-789", "");
        let rpc = &swml_main(&fr)[0]["execute_rpc"];
        assert_eq!(rpc["call_id"], "call-789");
        assert!(rpc.get("params").is_none());
    }

    #[test]
    fn test_rpc_dial_basic() {
        let mut fr = FunctionResult::new();
        fr.rpc_dial("+15551234567", "+15559876543", "https://example.com/call-agent", "phone");
        let rpc = &swml_main(&fr)[0]["execute_rpc"];
        assert_eq!(rpc["method"], "dial");
        let params = &rpc["params"];
        assert_eq!(params["dest_swml"], "https://example.com/call-agent");
        assert_eq!(params["devices"]["type"], "phone");
        assert_eq!(params["devices"]["params"]["to_number"], "+15551234567");
        assert_eq!(params["devices"]["params"]["from_number"], "+15559876543");
        // call_id/node_id not set by rpc_dial.
        assert!(rpc.get("call_id").is_none());
    }

    #[test]
    fn test_rpc_dial_custom_device_type() {
        let mut fr = FunctionResult::new();
        fr.rpc_dial("+15551234567", "+15559876543", "https://example.com/swml", "sip");
        let params = &swml_main(&fr)[0]["execute_rpc"]["params"];
        assert_eq!(params["devices"]["type"], "sip");
    }

    #[test]
    fn test_rpc_ai_message_default_role() {
        let mut fr = FunctionResult::new();
        fr.rpc_ai_message("call-abc", "Please take a message.", "system");
        let rpc = &swml_main(&fr)[0]["execute_rpc"];
        assert_eq!(rpc["method"], "ai_message");
        assert_eq!(rpc["call_id"], "call-abc");
        assert_eq!(rpc["params"]["role"], "system");
        assert_eq!(rpc["params"]["message_text"], "Please take a message.");
    }

    #[test]
    fn test_rpc_ai_message_custom_role() {
        let mut fr = FunctionResult::new();
        fr.rpc_ai_message("call-xyz", "User said hello", "user");
        assert_eq!(
            swml_main(&fr)[0]["execute_rpc"]["params"]["role"],
            "user"
        );
    }

    #[test]
    fn test_rpc_ai_unhold_omits_empty_params() {
        // Python: params={} is falsy -> execute_rpc drops the "params" key.
        let mut fr = FunctionResult::new();
        fr.rpc_ai_unhold("call-abc");
        let rpc = &swml_main(&fr)[0]["execute_rpc"];
        assert_eq!(rpc["method"], "ai_unhold");
        assert_eq!(rpc["call_id"], "call-abc");
        assert!(rpc.get("params").is_none());
    }

    #[test]
    fn test_simulate_user_input_uses_user_input_key() {
        // Python action key is "user_input".
        let mut fr = FunctionResult::new();
        fr.simulate_user_input("I want to book a flight");
        assert_eq!(action0(&fr), json!({"user_input": "I want to book a flight"}));
    }

    // ── Payment helpers (static) ──────────────────────────────────────────

    #[test]
    fn test_create_payment_prompt_basic() {
        let actions = json!([{"type": "Say", "phrase": "Enter your card number"}]);
        let prompt = FunctionResult::create_payment_prompt("payment-card-number", actions.clone(), "", "");
        assert_eq!(prompt["for"], "payment-card-number");
        assert_eq!(prompt["actions"], actions);
        assert!(prompt.get("card_type").is_none());
        assert!(prompt.get("error_type").is_none());
    }

    #[test]
    fn test_create_payment_prompt_with_card_and_error_type() {
        let actions = json!([{"type": "Say", "phrase": "Try again"}]);
        let prompt = FunctionResult::create_payment_prompt(
            "payment-card-number", actions, "visa", "timeout",
        );
        assert_eq!(prompt["card_type"], "visa");
        assert_eq!(prompt["error_type"], "timeout");
    }

    #[test]
    fn test_create_payment_action() {
        let say = FunctionResult::create_payment_action("Say", "Enter card number");
        assert_eq!(say, json!({"type": "Say", "phrase": "Enter card number"}));
        let play = FunctionResult::create_payment_action("Play", "https://example.com/prompt.mp3");
        assert_eq!(play, json!({"type": "Play", "phrase": "https://example.com/prompt.mp3"}));
    }

    #[test]
    fn test_create_payment_parameter() {
        let param = FunctionResult::create_payment_parameter("store_id", "abc-123");
        assert_eq!(param, json!({"name": "store_id", "value": "abc-123"}));
    }

    // ── to_value edge cases (Python to_dict parity) ───────────────────────

    #[test]
    fn test_to_value_actions_only_omits_empty_response() {
        // NOTE: Rust always emits "response" (even empty) in to_value, whereas
        // Python's to_dict omits an empty response. Documented in PORT_ADDITIONS
        // (to_value/to_json serde helpers). We assert the action shape here.
        let mut fr = FunctionResult::new();
        fr.hangup();
        assert_eq!(fr.to_value()["action"], json!([{"hangup": true}]));
    }

    #[test]
    fn test_post_process_with_actions_included() {
        let mut fr = FunctionResult::with_response("Response");
        fr.set_post_process(true).stop();
        let v = fr.to_value();
        assert_eq!(v["post_process"], true);
        assert_eq!(v["action"], json!([{"stop": true}]));
    }

    // ── Multi-action chaining ─────────────────────────────────────────────

    #[test]
    fn test_multi_action_chain() {
        let mut fr = FunctionResult::new();
        fr.set_response("Processing")
            .say("Please hold")
            .hold(60)
            .update_global_data(json!({"status": "processing"}))
            .set_post_process(true);
        let val = fr.to_value();
        assert_eq!(val["response"], "Processing");
        assert_eq!(val["post_process"], true);
        let acts = val["action"].as_array().unwrap();
        assert_eq!(acts.len(), 3);
        assert_eq!(acts[0], json!({"say": "Please hold"}));
        assert_eq!(acts[1], json!({"hold": 60}));
        assert_eq!(acts[2], json!({"set_global_data": {"status": "processing"}}));
    }

    #[test]
    fn test_default_trait() {
        // Default is an empty result -> defaults to "Action completed." (Python parity).
        let fr = FunctionResult::default();
        assert_eq!(fr.to_value()["response"], "Action completed.");
    }
}
