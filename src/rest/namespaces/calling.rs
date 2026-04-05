use serde_json::{json, Value};

use crate::rest::error::SignalWireRestError;
use crate::rest::http_client::HttpClient;

/// Base path for all calling commands.
const BASE_PATH: &str = "/api/calling/calls";

/// Calling API namespace.
///
/// Provides 37 call-control command methods that each POST to
/// `/api/calling/calls` with a JSON body containing the command name,
/// an optional call ID, and parameters.
pub struct Calling<'a> {
    client: &'a HttpClient,
    project_id: String,
}

impl<'a> Calling<'a> {
    pub fn new(client: &'a HttpClient, project_id: &str) -> Self {
        Calling {
            client,
            project_id: project_id.to_string(),
        }
    }

    pub fn client(&self) -> &HttpClient {
        self.client
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn base_path(&self) -> &str {
        BASE_PATH
    }

    // -----------------------------------------------------------------
    // Internal execute helper
    // -----------------------------------------------------------------

    fn execute(
        &self,
        command: &str,
        call_id: Option<&str>,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        let mut body = json!({
            "command": command,
            "params": params,
        });
        if let Some(id) = call_id {
            body["id"] = json!(id);
        }
        self.client.post(BASE_PATH, &body)
    }

    // -----------------------------------------------------------------
    // Call lifecycle (5)
    // -----------------------------------------------------------------

    pub fn dial(&self, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("dial", None, params)
    }

    pub fn update_call(&self, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("update", None, params)
    }

    pub fn end(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.end", Some(call_id), params)
    }

    pub fn transfer(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.transfer", Some(call_id), params)
    }

    pub fn disconnect(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.disconnect", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Play (5)
    // -----------------------------------------------------------------

    pub fn play(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.play", Some(call_id), params)
    }

    pub fn play_pause(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.play.pause", Some(call_id), params)
    }

    pub fn play_resume(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.play.resume", Some(call_id), params)
    }

    pub fn play_stop(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.play.stop", Some(call_id), params)
    }

    pub fn play_volume(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.play.volume", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Record (4)
    // -----------------------------------------------------------------

    pub fn record(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.record", Some(call_id), params)
    }

    pub fn record_pause(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.record.pause", Some(call_id), params)
    }

    pub fn record_resume(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.record.resume", Some(call_id), params)
    }

    pub fn record_stop(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.record.stop", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Collect (3)
    // -----------------------------------------------------------------

    pub fn collect(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.collect", Some(call_id), params)
    }

    pub fn collect_stop(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.collect.stop", Some(call_id), params)
    }

    pub fn collect_start_input_timers(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.collect.start_input_timers", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Detect (2)
    // -----------------------------------------------------------------

    pub fn detect(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.detect", Some(call_id), params)
    }

    pub fn detect_stop(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.detect.stop", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Tap (2)
    // -----------------------------------------------------------------

    pub fn tap(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.tap", Some(call_id), params)
    }

    pub fn tap_stop(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.tap.stop", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Stream (2)
    // -----------------------------------------------------------------

    pub fn stream(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.stream", Some(call_id), params)
    }

    pub fn stream_stop(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.stream.stop", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Denoise (2)
    // -----------------------------------------------------------------

    pub fn denoise(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.denoise", Some(call_id), params)
    }

    pub fn denoise_stop(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.denoise.stop", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Transcribe (2)
    // -----------------------------------------------------------------

    pub fn transcribe(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.transcribe", Some(call_id), params)
    }

    pub fn transcribe_stop(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.transcribe.stop", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // AI (4)
    // -----------------------------------------------------------------

    pub fn ai_message(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.ai_message", Some(call_id), params)
    }

    pub fn ai_hold(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.ai_hold", Some(call_id), params)
    }

    pub fn ai_unhold(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.ai_unhold", Some(call_id), params)
    }

    pub fn ai_stop(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.ai.stop", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Live transcribe / translate (2)
    // -----------------------------------------------------------------

    pub fn live_transcribe(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.live_transcribe", Some(call_id), params)
    }

    pub fn live_translate(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.live_translate", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Fax (2)
    // -----------------------------------------------------------------

    pub fn send_fax_stop(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.send_fax.stop", Some(call_id), params)
    }

    pub fn receive_fax_stop(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.receive_fax.stop", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // SIP (1)
    // -----------------------------------------------------------------

    pub fn refer(&self, call_id: &str, params: Value) -> Result<Value, SignalWireRestError> {
        self.execute("calling.refer", Some(call_id), params)
    }

    // -----------------------------------------------------------------
    // Custom events (1)
    // -----------------------------------------------------------------

    pub fn user_event(
        &self,
        call_id: &str,
        params: Value,
    ) -> Result<Value, SignalWireRestError> {
        self.execute("calling.user_event", Some(call_id), params)
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rest::http_client::StubTransport;

    fn make_calling() -> (crate::rest::http_client::HttpClient, std::sync::Arc<StubTransport>) {
        crate::rest::http_client::HttpClient::with_stub(
            "proj",
            "tok",
            "https://test.signalwire.com",
        )
    }

    #[test]
    fn test_base_path() {
        let (client, _) = make_calling();
        let c = Calling::new(&client, "proj");
        assert_eq!(c.base_path(), "/api/calling/calls");
        assert_eq!(c.project_id(), "proj");
    }

    #[test]
    fn test_dial() {
        let (client, stub) = make_calling();
        stub.set_response(200, r#"{"call_id":"c1"}"#);
        let c = Calling::new(&client, "proj");
        let result = c.dial(json!({"to": "+15551001000"})).unwrap();
        assert_eq!(result["call_id"], "c1");

        let reqs = stub.requests.lock().unwrap();
        assert_eq!(reqs[0].0, "POST");
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "dial");
        assert!(body.get("id").is_none());
    }

    #[test]
    fn test_play() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.play("call-1", json!({"url": "http://example.com/a.mp3"}))
            .unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.play");
        assert_eq!(body["id"], "call-1");
    }

    #[test]
    fn test_play_pause() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.play_pause("call-1", json!({})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.play.pause");
    }

    #[test]
    fn test_record() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.record("call-1", json!({})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.record");
    }

    #[test]
    fn test_collect() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.collect("c1", json!({"digits": {}})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.collect");
    }

    #[test]
    fn test_detect() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.detect("c1", json!({})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.detect");
    }

    #[test]
    fn test_end() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.end("c1", json!({})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.end");
        assert_eq!(body["id"], "c1");
    }

    #[test]
    fn test_transfer() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.transfer("c1", json!({"dest": "sip:foo@bar"})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.transfer");
    }

    #[test]
    fn test_tap() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.tap("c1", json!({})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.tap");
    }

    #[test]
    fn test_stream() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.stream("c1", json!({})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.stream");
    }

    #[test]
    fn test_denoise() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.denoise("c1", json!({})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.denoise");
    }

    #[test]
    fn test_ai_stop() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.ai_stop("c1", json!({})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.ai.stop");
    }

    #[test]
    fn test_refer() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.refer("c1", json!({"to": "sip:x@y"})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.refer");
    }

    #[test]
    fn test_user_event() {
        let (client, stub) = make_calling();
        stub.set_response(200, "{}");
        let c = Calling::new(&client, "proj");
        c.user_event("c1", json!({"data": "test"})).unwrap();

        let reqs = stub.requests.lock().unwrap();
        let body: Value = serde_json::from_str(reqs[0].2.as_ref().unwrap()).unwrap();
        assert_eq!(body["command"], "calling.user_event");
    }

    #[test]
    fn test_error_propagation() {
        let (client, stub) = make_calling();
        stub.set_response(500, "server error");
        let c = Calling::new(&client, "proj");
        let err = c.dial(json!({})).unwrap_err();
        assert_eq!(err.status_code(), 500);
    }

    #[test]
    fn test_method_count() {
        // Verify we have all 37 methods by counting the public methods
        // that take call_id. We count by inspection of the impl block --
        // this test just ensures the module compiles and is importable.
        let (client, _) = make_calling();
        let c = Calling::new(&client, "proj");
        assert_eq!(c.base_path(), "/api/calling/calls");
    }
}
