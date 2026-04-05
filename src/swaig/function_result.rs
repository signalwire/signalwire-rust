use std::collections::HashMap;

use serde_json::{json, Map, Value};

/// Result returned from a SWAIG function handler.
///
/// Serialises to `{"response": "...", "action": [...], "post_process": true}` where
/// `action` is omitted when empty and `post_process` is omitted when false.
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
        map.insert("response".to_string(), Value::String(self.response.clone()));

        if !self.actions.is_empty() {
            map.insert("action".to_string(), Value::Array(self.actions.clone()));
        }

        if self.post_process {
            map.insert("post_process".to_string(), Value::Bool(true));
        }

        Value::Object(map)
    }

    /// Compact JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(&self.to_value()).expect("FunctionResult serialisation should not fail")
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
                }
            }
        }));

        self
    }

    pub fn swml_transfer(&mut self, dest: &str, ai_response: &str) -> &mut Self {
        self.actions.push(json!({"transfer_uri": dest}));
        if !ai_response.is_empty() {
            self.response = ai_response.to_string();
        }
        self
    }

    pub fn hangup(&mut self) -> &mut Self {
        self.actions.push(json!({"hangup": {}}));
        self
    }

    pub fn hold(&mut self, timeout: i64) -> &mut Self {
        let clamped = timeout.max(0).min(900);
        self.actions.push(json!({"hold": {"timeout": clamped}}));
        self
    }

    pub fn wait_for_user(
        &mut self,
        enabled: Option<bool>,
        timeout: Option<i64>,
        answer_first: Option<bool>,
    ) -> &mut Self {
        if enabled.is_none() && timeout.is_none() && answer_first.is_none() {
            self.actions.push(json!({"wait_for_user": true}));
            return self;
        }

        let mut params = Map::new();
        if let Some(e) = enabled {
            params.insert("enabled".to_string(), json!(e));
        }
        if let Some(t) = timeout {
            params.insert("timeout".to_string(), json!(t));
        }
        if let Some(a) = answer_first {
            params.insert("answer_first".to_string(), json!(a));
        }

        self.actions.push(json!({"wait_for_user": Value::Object(params)}));
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

    pub fn remove_global_data(&mut self, keys: Vec<&str>) -> &mut Self {
        self.actions.push(json!({"remove_global_data": {"keys": keys}}));
        self
    }

    pub fn set_metadata(&mut self, data: Value) -> &mut Self {
        self.actions.push(json!({"set_meta_data": data}));
        self
    }

    pub fn remove_metadata(&mut self, keys: Vec<&str>) -> &mut Self {
        self.actions.push(json!({"remove_meta_data": {"keys": keys}}));
        self
    }

    pub fn swml_user_event(&mut self, event_data: Value) -> &mut Self {
        self.actions.push(json!({"user_event": event_data}));
        self
    }

    pub fn swml_change_step(&mut self, step_name: &str) -> &mut Self {
        self.actions.push(json!({"context_switch": {"step": step_name}}));
        self
    }

    pub fn swml_change_context(&mut self, context_name: &str) -> &mut Self {
        self.actions.push(json!({"context_switch": {"context": context_name}}));
        self
    }

    pub fn switch_context(
        &mut self,
        system_prompt: &str,
        user_prompt: &str,
        consolidate: bool,
        full_reset: bool,
        isolated: bool,
    ) -> &mut Self {
        let mut ctx = Map::new();
        ctx.insert("system_prompt".to_string(), json!(system_prompt));

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

    /// Replace conversation history. Pass `None` for text to use "summary".
    pub fn replace_in_history(&mut self, text: Option<&str>) -> &mut Self {
        match text {
            Some(t) => self.actions.push(json!({"replace_history": t})),
            None => self.actions.push(json!({"replace_history": "summary"})),
        }
        self
    }

    // ── Media ────────────────────────────────────────────────────────────

    pub fn say(&mut self, text: &str) -> &mut Self {
        self.actions.push(json!({"say": text}));
        self
    }

    pub fn play_background_file(&mut self, filename: &str, wait: bool) -> &mut Self {
        if wait {
            self.actions.push(json!({"play_background_file_wait": filename}));
        } else {
            self.actions.push(json!({"play_background_file": filename}));
        }
        self
    }

    pub fn stop_background_file(&mut self) -> &mut Self {
        self.actions.push(json!({"stop_background_file": true}));
        self
    }

    pub fn record_call(
        &mut self,
        control_id: &str,
        stereo: bool,
        format: &str,
        direction: &str,
    ) -> &mut Self {
        let mut record = Map::new();
        record.insert("stereo".to_string(), json!(stereo));
        record.insert("format".to_string(), json!(format));
        record.insert("direction".to_string(), json!(direction));
        record.insert("initiator".to_string(), json!("system"));
        if !control_id.is_empty() {
            record.insert("control_id".to_string(), json!(control_id));
        }
        self.actions.push(json!({"record_call": Value::Object(record)}));
        self
    }

    pub fn stop_record_call(&mut self, control_id: &str) -> &mut Self {
        if !control_id.is_empty() {
            self.actions.push(json!({"stop_record_call": {"control_id": control_id}}));
        } else {
            self.actions.push(json!({"stop_record_call": {}}));
        }
        self
    }

    // ── Speech & AI ──────────────────────────────────────────────────────

    pub fn add_dynamic_hints(&mut self, hints: Vec<Value>) -> &mut Self {
        self.actions.push(json!({"add_dynamic_hints": hints}));
        self
    }

    pub fn clear_dynamic_hints(&mut self) -> &mut Self {
        self.actions.push(json!({"clear_dynamic_hints": true}));
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

    pub fn enable_functions_on_timeout(&mut self, enabled: bool) -> &mut Self {
        self.actions.push(json!({"functions_on_timeout": enabled}));
        self
    }

    pub fn enable_extensive_data(&mut self, enabled: bool) -> &mut Self {
        self.actions.push(json!({"extensive_data": enabled}));
        self
    }

    pub fn update_settings(&mut self, settings: Value) -> &mut Self {
        self.actions.push(json!({"ai_settings": settings}));
        self
    }

    // ── Advanced ─────────────────────────────────────────────────────────

    pub fn execute_swml(&mut self, swml_content: Value, transfer: bool) -> &mut Self {
        if transfer {
            self.actions.push(json!({"transfer_swml": swml_content}));
        } else {
            self.actions.push(json!({"SWML": swml_content}));
        }
        self
    }

    pub fn join_conference(
        &mut self,
        name: &str,
        muted: bool,
        beep: &str,
        hold_audio: &str,
    ) -> &mut Self {
        self.actions.push(json!({
            "join_conference": {
                "name": name,
                "muted": muted,
                "beep": beep,
                "hold_audio": hold_audio,
            }
        }));
        self
    }

    pub fn join_room(&mut self, name: &str) -> &mut Self {
        self.actions.push(json!({"join_room": {"name": name}}));
        self
    }

    pub fn sip_refer(&mut self, to_uri: &str) -> &mut Self {
        self.actions.push(json!({"sip_refer": {"to_uri": to_uri}}));
        self
    }

    pub fn tap(
        &mut self,
        uri: &str,
        control_id: &str,
        direction: &str,
        codec: &str,
    ) -> &mut Self {
        let mut tap_obj = Map::new();
        tap_obj.insert("uri".to_string(), json!(uri));
        tap_obj.insert("direction".to_string(), json!(direction));
        tap_obj.insert("codec".to_string(), json!(codec));
        if !control_id.is_empty() {
            tap_obj.insert("control_id".to_string(), json!(control_id));
        }
        self.actions.push(json!({"tap": Value::Object(tap_obj)}));
        self
    }

    pub fn stop_tap(&mut self, control_id: &str) -> &mut Self {
        if !control_id.is_empty() {
            self.actions.push(json!({"stop_tap": {"control_id": control_id}}));
        } else {
            self.actions.push(json!({"stop_tap": {}}));
        }
        self
    }

    pub fn send_sms(
        &mut self,
        to: &str,
        from: &str,
        body: &str,
        media: Vec<&str>,
        tags: Vec<&str>,
    ) -> &mut Self {
        let mut sms = Map::new();
        sms.insert("to_number".to_string(), json!(to));
        sms.insert("from_number".to_string(), json!(from));
        sms.insert("body".to_string(), json!(body));
        if !media.is_empty() {
            sms.insert("media".to_string(), json!(media));
        }
        if !tags.is_empty() {
            sms.insert("tags".to_string(), json!(tags));
        }
        self.actions.push(json!({"send_sms": Value::Object(sms)}));
        self
    }

    pub fn pay(
        &mut self,
        connector_url: &str,
        input_method: &str,
        action_url: &str,
        timeout: i64,
        max_attempts: i64,
    ) -> &mut Self {
        let mut pay_obj = Map::new();
        pay_obj.insert("payment_connector_url".to_string(), json!(connector_url));
        pay_obj.insert("input_method".to_string(), json!(input_method));
        pay_obj.insert("timeout".to_string(), json!(timeout));
        pay_obj.insert("max_attempts".to_string(), json!(max_attempts));
        if !action_url.is_empty() {
            pay_obj.insert("action_url".to_string(), json!(action_url));
        }
        self.actions.push(json!({"pay": Value::Object(pay_obj)}));
        self
    }

    // ── RPC ──────────────────────────────────────────────────────────────

    pub fn execute_rpc(&mut self, method: &str, params: Value) -> &mut Self {
        let mut rpc = Map::new();
        rpc.insert("method".to_string(), json!(method));
        rpc.insert("jsonrpc".to_string(), json!("2.0"));
        if params != json!(null) && params != json!({}) {
            rpc.insert("params".to_string(), params);
        }
        self.actions.push(json!({"execute_rpc": Value::Object(rpc)}));
        self
    }

    pub fn rpc_dial(
        &mut self,
        to: &str,
        from: &str,
        dest_swml: Option<&str>,
        call_timeout: Option<i64>,
        region: &str,
    ) -> &mut Self {
        let mut params = Map::new();
        params.insert("to_number".to_string(), json!(to));
        if !from.is_empty() {
            params.insert("from_number".to_string(), json!(from));
        }
        if let Some(swml) = dest_swml {
            params.insert("dest_swml".to_string(), json!(swml));
        }
        if let Some(t) = call_timeout {
            params.insert("call_timeout".to_string(), json!(t));
        }
        if !region.is_empty() {
            params.insert("region".to_string(), json!(region));
        }
        self.execute_rpc("calling.dial", Value::Object(params))
    }

    pub fn rpc_ai_message(&mut self, call_id: &str, message_text: &str) -> &mut Self {
        self.execute_rpc(
            "calling.ai_message",
            json!({"call_id": call_id, "message_text": message_text}),
        )
    }

    pub fn rpc_ai_unhold(&mut self, call_id: &str) -> &mut Self {
        self.execute_rpc("calling.ai_unhold", json!({"call_id": call_id}))
    }

    pub fn simulate_user_input(&mut self, text: &str) -> &mut Self {
        self.actions.push(json!({"simulate_user_input": text}));
        self
    }

    // ── Payment Helpers (static) ─────────────────────────────────────────

    pub fn create_payment_prompt(text: &str, language: &str, voice: &str) -> Value {
        let mut prompt = Map::new();
        prompt.insert("text".to_string(), json!(text));
        prompt.insert("language".to_string(), json!(language));
        if !voice.is_empty() {
            prompt.insert("voice".to_string(), json!(voice));
        }
        Value::Object(prompt)
    }

    pub fn create_payment_action(action_type: &str, text: &str, language: &str, voice: &str) -> Value {
        let mut action = Map::new();
        action.insert("type".to_string(), json!(action_type));
        action.insert("text".to_string(), json!(text));
        action.insert("language".to_string(), json!(language));
        if !voice.is_empty() {
            action.insert("voice".to_string(), json!(voice));
        }
        Value::Object(action)
    }

    pub fn create_payment_parameter(name: &str, param_type: &str, config: Value) -> Value {
        let mut result = Map::new();
        result.insert("name".to_string(), json!(name));
        result.insert("type".to_string(), json!(param_type));
        if let Value::Object(cfg) = config {
            for (k, v) in cfg {
                result.insert(k, v);
            }
        }
        Value::Object(result)
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

    #[test]
    fn test_construction_default() {
        let fr = FunctionResult::new();
        let val = fr.to_value();
        assert_eq!(val["response"], "");
        assert!(val.get("action").is_none());
        assert!(val.get("post_process").is_none());
    }

    #[test]
    fn test_construction_with_response() {
        let fr = FunctionResult::with_response("hello");
        let val = fr.to_value();
        assert_eq!(val["response"], "hello");
    }

    #[test]
    fn test_set_response() {
        let mut fr = FunctionResult::new();
        fr.set_response("world");
        assert_eq!(fr.to_value()["response"], "world");
    }

    #[test]
    fn test_set_post_process_true() {
        let mut fr = FunctionResult::new();
        fr.set_post_process(true);
        let val = fr.to_value();
        assert_eq!(val["post_process"], true);
    }

    #[test]
    fn test_set_post_process_false_omitted() {
        let mut fr = FunctionResult::new();
        fr.set_post_process(false);
        let val = fr.to_value();
        assert!(val.get("post_process").is_none());
    }

    #[test]
    fn test_add_action() {
        let mut fr = FunctionResult::new();
        fr.add_action(json!({"test": true}));
        let val = fr.to_value();
        let actions = val["action"].as_array().unwrap();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["test"], true);
    }

    #[test]
    fn test_add_actions_multiple() {
        let mut fr = FunctionResult::new();
        fr.add_actions(vec![json!({"a": 1}), json!({"b": 2})]);
        let val = fr.to_value();
        let actions = val["action"].as_array().unwrap();
        assert_eq!(actions.len(), 2);
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
    fn test_to_json() {
        let fr = FunctionResult::with_response("test");
        let json_str = fr.to_json();
        let parsed: Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["response"], "test");
    }

    // ── Call Control tests ───────────────────────────────────────────────

    #[test]
    fn test_connect_basic() {
        let mut fr = FunctionResult::new();
        fr.connect("+15551234567", false, "");
        let action = &fr.to_value()["action"][0];
        let connect = &action["SWML"]["sections"]["main"][0]["connect"];
        assert_eq!(connect["to"], "+15551234567");
        assert!(connect.get("from").is_none());
    }

    #[test]
    fn test_connect_with_from() {
        let mut fr = FunctionResult::new();
        fr.connect("+15551234567", false, "+15559876543");
        let action = &fr.to_value()["action"][0];
        let connect = &action["SWML"]["sections"]["main"][0]["connect"];
        assert_eq!(connect["from"], "+15559876543");
    }

    #[test]
    fn test_swml_transfer() {
        let mut fr = FunctionResult::new();
        fr.swml_transfer("https://example.com/swml.json", "transferring");
        let val = fr.to_value();
        assert_eq!(val["response"], "transferring");
        assert_eq!(val["action"][0]["transfer_uri"], "https://example.com/swml.json");
    }

    #[test]
    fn test_swml_transfer_no_response() {
        let mut fr = FunctionResult::with_response("original");
        fr.swml_transfer("https://example.com", "");
        assert_eq!(fr.to_value()["response"], "original");
    }

    #[test]
    fn test_hangup() {
        let mut fr = FunctionResult::new();
        fr.hangup();
        let action = &fr.to_value()["action"][0];
        assert!(action["hangup"].is_object());
    }

    #[test]
    fn test_hold_clamped() {
        let mut fr = FunctionResult::new();
        fr.hold(1500);
        assert_eq!(fr.to_value()["action"][0]["hold"]["timeout"], 900);

        let mut fr2 = FunctionResult::new();
        fr2.hold(-10);
        assert_eq!(fr2.to_value()["action"][0]["hold"]["timeout"], 0);

        let mut fr3 = FunctionResult::new();
        fr3.hold(300);
        assert_eq!(fr3.to_value()["action"][0]["hold"]["timeout"], 300);
    }

    #[test]
    fn test_wait_for_user_no_params() {
        let mut fr = FunctionResult::new();
        fr.wait_for_user(None, None, None);
        assert_eq!(fr.to_value()["action"][0]["wait_for_user"], true);
    }

    #[test]
    fn test_wait_for_user_with_params() {
        let mut fr = FunctionResult::new();
        fr.wait_for_user(Some(true), Some(30), None);
        let wfu = &fr.to_value()["action"][0]["wait_for_user"];
        assert_eq!(wfu["enabled"], true);
        assert_eq!(wfu["timeout"], 30);
        assert!(wfu.get("answer_first").is_none());
    }

    #[test]
    fn test_wait_for_user_answer_first_only() {
        let mut fr = FunctionResult::new();
        fr.wait_for_user(None, None, Some(true));
        let wfu = &fr.to_value()["action"][0]["wait_for_user"];
        assert_eq!(wfu["answer_first"], true);
    }

    #[test]
    fn test_stop() {
        let mut fr = FunctionResult::new();
        fr.stop();
        assert_eq!(fr.to_value()["action"][0]["stop"], true);
    }

    // ── State & Data tests ───────────────────────────────────────────────

    #[test]
    fn test_update_global_data() {
        let mut fr = FunctionResult::new();
        fr.update_global_data(json!({"key": "value"}));
        assert_eq!(fr.to_value()["action"][0]["set_global_data"]["key"], "value");
    }

    #[test]
    fn test_remove_global_data() {
        let mut fr = FunctionResult::new();
        fr.remove_global_data(vec!["k1", "k2"]);
        let keys = fr.to_value()["action"][0]["remove_global_data"]["keys"].as_array().unwrap().clone();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_set_metadata() {
        let mut fr = FunctionResult::new();
        fr.set_metadata(json!({"meta": "data"}));
        assert_eq!(fr.to_value()["action"][0]["set_meta_data"]["meta"], "data");
    }

    #[test]
    fn test_remove_metadata() {
        let mut fr = FunctionResult::new();
        fr.remove_metadata(vec!["m1"]);
        let keys = fr.to_value()["action"][0]["remove_meta_data"]["keys"].as_array().unwrap().clone();
        assert_eq!(keys.len(), 1);
    }

    #[test]
    fn test_swml_user_event() {
        let mut fr = FunctionResult::new();
        fr.swml_user_event(json!({"event_name": "test"}));
        assert_eq!(fr.to_value()["action"][0]["user_event"]["event_name"], "test");
    }

    #[test]
    fn test_swml_change_step() {
        let mut fr = FunctionResult::new();
        fr.swml_change_step("step2");
        assert_eq!(fr.to_value()["action"][0]["context_switch"]["step"], "step2");
    }

    #[test]
    fn test_swml_change_context() {
        let mut fr = FunctionResult::new();
        fr.swml_change_context("ctx2");
        assert_eq!(fr.to_value()["action"][0]["context_switch"]["context"], "ctx2");
    }

    #[test]
    fn test_switch_context_simple() {
        let mut fr = FunctionResult::new();
        fr.switch_context("You are a bot", "", false, false, false);
        let cs = &fr.to_value()["action"][0]["context_switch"];
        assert_eq!(cs["system_prompt"], "You are a bot");
        assert!(cs.get("user_prompt").is_none());
        assert!(cs.get("consolidate").is_none());
    }

    #[test]
    fn test_switch_context_full() {
        let mut fr = FunctionResult::new();
        fr.switch_context("sys", "usr", true, true, true);
        let cs = &fr.to_value()["action"][0]["context_switch"];
        assert_eq!(cs["user_prompt"], "usr");
        assert_eq!(cs["consolidate"], true);
        assert_eq!(cs["full_reset"], true);
        assert_eq!(cs["isolated"], true);
    }

    #[test]
    fn test_replace_in_history_text() {
        let mut fr = FunctionResult::new();
        fr.replace_in_history(Some("custom text"));
        assert_eq!(fr.to_value()["action"][0]["replace_history"], "custom text");
    }

    #[test]
    fn test_replace_in_history_summary() {
        let mut fr = FunctionResult::new();
        fr.replace_in_history(None);
        assert_eq!(fr.to_value()["action"][0]["replace_history"], "summary");
    }

    // ── Media tests ──────────────────────────────────────────────────────

    #[test]
    fn test_say() {
        let mut fr = FunctionResult::new();
        fr.say("hello world");
        assert_eq!(fr.to_value()["action"][0]["say"], "hello world");
    }

    #[test]
    fn test_play_background_file() {
        let mut fr = FunctionResult::new();
        fr.play_background_file("music.mp3", false);
        assert_eq!(fr.to_value()["action"][0]["play_background_file"], "music.mp3");
    }

    #[test]
    fn test_play_background_file_wait() {
        let mut fr = FunctionResult::new();
        fr.play_background_file("music.mp3", true);
        assert_eq!(fr.to_value()["action"][0]["play_background_file_wait"], "music.mp3");
    }

    #[test]
    fn test_stop_background_file() {
        let mut fr = FunctionResult::new();
        fr.stop_background_file();
        assert_eq!(fr.to_value()["action"][0]["stop_background_file"], true);
    }

    #[test]
    fn test_record_call_with_control_id() {
        let mut fr = FunctionResult::new();
        fr.record_call("ctrl1", true, "mp3", "both");
        let rc = &fr.to_value()["action"][0]["record_call"];
        assert_eq!(rc["control_id"], "ctrl1");
        assert_eq!(rc["stereo"], true);
        assert_eq!(rc["format"], "mp3");
        assert_eq!(rc["direction"], "both");
        assert_eq!(rc["initiator"], "system");
    }

    #[test]
    fn test_record_call_no_control_id() {
        let mut fr = FunctionResult::new();
        fr.record_call("", false, "wav", "both");
        let rc = &fr.to_value()["action"][0]["record_call"];
        assert!(rc.get("control_id").is_none());
    }

    #[test]
    fn test_stop_record_call_with_id() {
        let mut fr = FunctionResult::new();
        fr.stop_record_call("ctrl1");
        assert_eq!(fr.to_value()["action"][0]["stop_record_call"]["control_id"], "ctrl1");
    }

    #[test]
    fn test_stop_record_call_no_id() {
        let mut fr = FunctionResult::new();
        fr.stop_record_call("");
        assert!(fr.to_value()["action"][0]["stop_record_call"].is_object());
    }

    // ── Speech & AI tests ────────────────────────────────────────────────

    #[test]
    fn test_add_dynamic_hints() {
        let mut fr = FunctionResult::new();
        fr.add_dynamic_hints(vec![json!("hint1"), json!("hint2")]);
        let hints = fr.to_value()["action"][0]["add_dynamic_hints"].as_array().unwrap().clone();
        assert_eq!(hints.len(), 2);
    }

    #[test]
    fn test_clear_dynamic_hints() {
        let mut fr = FunctionResult::new();
        fr.clear_dynamic_hints();
        assert_eq!(fr.to_value()["action"][0]["clear_dynamic_hints"], true);
    }

    #[test]
    fn test_set_end_of_speech_timeout() {
        let mut fr = FunctionResult::new();
        fr.set_end_of_speech_timeout(500);
        assert_eq!(fr.to_value()["action"][0]["end_of_speech_timeout"], 500);
    }

    #[test]
    fn test_set_speech_event_timeout() {
        let mut fr = FunctionResult::new();
        fr.set_speech_event_timeout(3000);
        assert_eq!(fr.to_value()["action"][0]["speech_event_timeout"], 3000);
    }

    #[test]
    fn test_toggle_functions() {
        let mut fr = FunctionResult::new();
        let mut toggles = HashMap::new();
        toggles.insert("func1".to_string(), true);
        fr.toggle_functions(toggles);
        let tf = fr.to_value()["action"][0]["toggle_functions"].as_array().unwrap().clone();
        assert_eq!(tf.len(), 1);
        assert_eq!(tf[0]["function"], "func1");
        assert_eq!(tf[0]["active"], true);
    }

    #[test]
    fn test_enable_functions_on_timeout() {
        let mut fr = FunctionResult::new();
        fr.enable_functions_on_timeout(true);
        assert_eq!(fr.to_value()["action"][0]["functions_on_timeout"], true);
    }

    #[test]
    fn test_enable_extensive_data() {
        let mut fr = FunctionResult::new();
        fr.enable_extensive_data(true);
        assert_eq!(fr.to_value()["action"][0]["extensive_data"], true);
    }

    #[test]
    fn test_update_settings() {
        let mut fr = FunctionResult::new();
        fr.update_settings(json!({"temperature": 0.5}));
        assert_eq!(fr.to_value()["action"][0]["ai_settings"]["temperature"], 0.5);
    }

    // ── Advanced tests ───────────────────────────────────────────────────

    #[test]
    fn test_execute_swml_no_transfer() {
        let mut fr = FunctionResult::new();
        fr.execute_swml(json!({"version": "1.0.0"}), false);
        assert_eq!(fr.to_value()["action"][0]["SWML"]["version"], "1.0.0");
    }

    #[test]
    fn test_execute_swml_transfer() {
        let mut fr = FunctionResult::new();
        fr.execute_swml(json!({"version": "1.0.0"}), true);
        assert_eq!(fr.to_value()["action"][0]["transfer_swml"]["version"], "1.0.0");
    }

    #[test]
    fn test_join_conference() {
        let mut fr = FunctionResult::new();
        fr.join_conference("room1", true, "true", "ring");
        let jc = &fr.to_value()["action"][0]["join_conference"];
        assert_eq!(jc["name"], "room1");
        assert_eq!(jc["muted"], true);
        assert_eq!(jc["beep"], "true");
        assert_eq!(jc["hold_audio"], "ring");
    }

    #[test]
    fn test_join_room() {
        let mut fr = FunctionResult::new();
        fr.join_room("room1");
        assert_eq!(fr.to_value()["action"][0]["join_room"]["name"], "room1");
    }

    #[test]
    fn test_sip_refer() {
        let mut fr = FunctionResult::new();
        fr.sip_refer("sip:alice@example.com");
        assert_eq!(fr.to_value()["action"][0]["sip_refer"]["to_uri"], "sip:alice@example.com");
    }

    #[test]
    fn test_tap_with_control_id() {
        let mut fr = FunctionResult::new();
        fr.tap("wss://example.com", "ctrl1", "both", "PCMU");
        let t = &fr.to_value()["action"][0]["tap"];
        assert_eq!(t["uri"], "wss://example.com");
        assert_eq!(t["control_id"], "ctrl1");
        assert_eq!(t["direction"], "both");
        assert_eq!(t["codec"], "PCMU");
    }

    #[test]
    fn test_tap_no_control_id() {
        let mut fr = FunctionResult::new();
        fr.tap("wss://example.com", "", "both", "PCMU");
        assert!(fr.to_value()["action"][0]["tap"].get("control_id").is_none());
    }

    #[test]
    fn test_stop_tap_with_id() {
        let mut fr = FunctionResult::new();
        fr.stop_tap("ctrl1");
        assert_eq!(fr.to_value()["action"][0]["stop_tap"]["control_id"], "ctrl1");
    }

    #[test]
    fn test_stop_tap_no_id() {
        let mut fr = FunctionResult::new();
        fr.stop_tap("");
        assert!(fr.to_value()["action"][0]["stop_tap"].is_object());
    }

    #[test]
    fn test_send_sms_basic() {
        let mut fr = FunctionResult::new();
        fr.send_sms("+15551234567", "+15559876543", "Hello", vec![], vec![]);
        let sms = &fr.to_value()["action"][0]["send_sms"];
        assert_eq!(sms["to_number"], "+15551234567");
        assert_eq!(sms["from_number"], "+15559876543");
        assert_eq!(sms["body"], "Hello");
        assert!(sms.get("media").is_none());
        assert!(sms.get("tags").is_none());
    }

    #[test]
    fn test_send_sms_with_media_and_tags() {
        let mut fr = FunctionResult::new();
        fr.send_sms("+1", "+2", "Hi", vec!["https://img.png"], vec!["vip"]);
        let sms = &fr.to_value()["action"][0]["send_sms"];
        assert_eq!(sms["media"].as_array().unwrap().len(), 1);
        assert_eq!(sms["tags"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_pay() {
        let mut fr = FunctionResult::new();
        fr.pay("https://connector.com", "dtmf", "https://action.com", 600, 3);
        let p = &fr.to_value()["action"][0]["pay"];
        assert_eq!(p["payment_connector_url"], "https://connector.com");
        assert_eq!(p["input_method"], "dtmf");
        assert_eq!(p["action_url"], "https://action.com");
        assert_eq!(p["timeout"], 600);
        assert_eq!(p["max_attempts"], 3);
    }

    #[test]
    fn test_pay_no_action_url() {
        let mut fr = FunctionResult::new();
        fr.pay("https://connector.com", "dtmf", "", 600, 3);
        assert!(fr.to_value()["action"][0]["pay"].get("action_url").is_none());
    }

    // ── RPC tests ────────────────────────────────────────────────────────

    #[test]
    fn test_execute_rpc_basic() {
        let mut fr = FunctionResult::new();
        fr.execute_rpc("test.method", json!({"key": "val"}));
        let rpc = &fr.to_value()["action"][0]["execute_rpc"];
        assert_eq!(rpc["method"], "test.method");
        assert_eq!(rpc["jsonrpc"], "2.0");
        assert_eq!(rpc["params"]["key"], "val");
    }

    #[test]
    fn test_execute_rpc_no_params() {
        let mut fr = FunctionResult::new();
        fr.execute_rpc("test.method", json!({}));
        assert!(fr.to_value()["action"][0]["execute_rpc"].get("params").is_none());
    }

    #[test]
    fn test_rpc_dial() {
        let mut fr = FunctionResult::new();
        fr.rpc_dial("+15551234567", "+15559876543", Some("swml.json"), Some(30), "us");
        let rpc = &fr.to_value()["action"][0]["execute_rpc"];
        assert_eq!(rpc["method"], "calling.dial");
        assert_eq!(rpc["params"]["to_number"], "+15551234567");
        assert_eq!(rpc["params"]["from_number"], "+15559876543");
        assert_eq!(rpc["params"]["dest_swml"], "swml.json");
        assert_eq!(rpc["params"]["call_timeout"], 30);
        assert_eq!(rpc["params"]["region"], "us");
    }

    #[test]
    fn test_rpc_dial_minimal() {
        let mut fr = FunctionResult::new();
        fr.rpc_dial("+15551234567", "", None, None, "");
        let rpc = &fr.to_value()["action"][0]["execute_rpc"];
        assert_eq!(rpc["params"]["to_number"], "+15551234567");
        assert!(rpc["params"].get("from_number").is_none());
        assert!(rpc["params"].get("dest_swml").is_none());
        assert!(rpc["params"].get("call_timeout").is_none());
        assert!(rpc["params"].get("region").is_none());
    }

    #[test]
    fn test_rpc_ai_message() {
        let mut fr = FunctionResult::new();
        fr.rpc_ai_message("call-123", "hello agent");
        let rpc = &fr.to_value()["action"][0]["execute_rpc"];
        assert_eq!(rpc["method"], "calling.ai_message");
        assert_eq!(rpc["params"]["call_id"], "call-123");
        assert_eq!(rpc["params"]["message_text"], "hello agent");
    }

    #[test]
    fn test_rpc_ai_unhold() {
        let mut fr = FunctionResult::new();
        fr.rpc_ai_unhold("call-456");
        let rpc = &fr.to_value()["action"][0]["execute_rpc"];
        assert_eq!(rpc["method"], "calling.ai_unhold");
        assert_eq!(rpc["params"]["call_id"], "call-456");
    }

    #[test]
    fn test_simulate_user_input() {
        let mut fr = FunctionResult::new();
        fr.simulate_user_input("I want to book");
        assert_eq!(fr.to_value()["action"][0]["simulate_user_input"], "I want to book");
    }

    // ── Payment helpers tests ────────────────────────────────────────────

    #[test]
    fn test_create_payment_prompt() {
        let prompt = FunctionResult::create_payment_prompt("Enter card", "en-US", "");
        assert_eq!(prompt["text"], "Enter card");
        assert_eq!(prompt["language"], "en-US");
        assert!(prompt.get("voice").is_none());
    }

    #[test]
    fn test_create_payment_prompt_with_voice() {
        let prompt = FunctionResult::create_payment_prompt("Enter card", "en-US", "Polly.Salli");
        assert_eq!(prompt["voice"], "Polly.Salli");
    }

    #[test]
    fn test_create_payment_action() {
        let action = FunctionResult::create_payment_action("collect", "Enter amount", "en-US", "");
        assert_eq!(action["type"], "collect");
        assert_eq!(action["text"], "Enter amount");
        assert!(action.get("voice").is_none());
    }

    #[test]
    fn test_create_payment_parameter() {
        let param = FunctionResult::create_payment_parameter("amount", "number", json!({"min": 1}));
        assert_eq!(param["name"], "amount");
        assert_eq!(param["type"], "number");
        assert_eq!(param["min"], 1);
    }

    #[test]
    fn test_create_payment_parameter_empty_config() {
        let param = FunctionResult::create_payment_parameter("name", "string", json!({}));
        assert_eq!(param["name"], "name");
        assert_eq!(param["type"], "string");
    }

    // ── Multi-action chaining test ───────────────────────────────────────

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
        let actions = val["action"].as_array().unwrap();
        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0]["say"], "Please hold");
        assert_eq!(actions[1]["hold"]["timeout"], 60);
        assert_eq!(actions[2]["set_global_data"]["status"], "processing");
    }

    #[test]
    fn test_default_trait() {
        let fr = FunctionResult::default();
        assert_eq!(fr.to_value()["response"], "");
    }
}
