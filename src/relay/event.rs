use std::collections::HashMap;

/// A RELAY event received from the SignalWire server.
///
/// Events carry an `event_type` (e.g. `"calling.call.state"`), a
/// timestamp, and a bag of string-keyed parameters.
#[derive(Debug, Clone)]
pub struct Event {
    event_type: String,
    timestamp: f64,
    params: HashMap<String, serde_json::Value>,
}

impl Event {
    /// Create a new event.  If `timestamp` is `0.0`, the current time is used.
    pub fn new(
        event_type: &str,
        params: HashMap<String, serde_json::Value>,
        timestamp: f64,
    ) -> Self {
        let ts = if timestamp == 0.0 {
            chrono::Utc::now().timestamp_millis() as f64 / 1000.0
        } else {
            timestamp
        };
        Event {
            event_type: event_type.to_string(),
            timestamp: ts,
            params,
        }
    }

    /// Convenience constructor from a `serde_json::Value` params object.
    pub fn parse(event_type: &str, params_value: &serde_json::Value) -> Self {
        let params = match params_value.as_object() {
            Some(map) => map
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            None => HashMap::new(),
        };
        Self::new(event_type, params, 0.0)
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    pub fn params(&self) -> &HashMap<String, serde_json::Value> {
        &self.params
    }

    pub fn call_id(&self) -> Option<&str> {
        self.params
            .get("call_id")
            .and_then(|v| v.as_str())
    }

    pub fn node_id(&self) -> Option<&str> {
        self.params
            .get("node_id")
            .and_then(|v| v.as_str())
    }

    pub fn control_id(&self) -> Option<&str> {
        self.params
            .get("control_id")
            .and_then(|v| v.as_str())
    }

    pub fn tag(&self) -> Option<&str> {
        self.params.get("tag").and_then(|v| v.as_str())
    }

    pub fn state(&self) -> Option<&str> {
        self.params.get("state").and_then(|v| v.as_str())
    }

    /// Serialize back to a JSON-compatible map.
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "event_type": self.event_type,
            "timestamp": self.timestamp,
            "params": self.params,
        })
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_params() -> HashMap<String, serde_json::Value> {
        let mut m = HashMap::new();
        m.insert("call_id".to_string(), json!("call-1"));
        m.insert("node_id".to_string(), json!("node-1"));
        m.insert("control_id".to_string(), json!("ctrl-1"));
        m.insert("tag".to_string(), json!("tag-1"));
        m.insert("state".to_string(), json!("ringing"));
        m
    }

    #[test]
    fn test_new_with_explicit_timestamp() {
        let ev = Event::new("calling.call.state", sample_params(), 123.456);
        assert_eq!(ev.event_type(), "calling.call.state");
        assert!((ev.timestamp() - 123.456).abs() < f64::EPSILON);
    }

    #[test]
    fn test_new_with_auto_timestamp() {
        let ev = Event::new("calling.call.state", HashMap::new(), 0.0);
        assert!(ev.timestamp() > 0.0);
    }

    #[test]
    fn test_accessors() {
        let ev = Event::new("calling.call.state", sample_params(), 1.0);
        assert_eq!(ev.call_id(), Some("call-1"));
        assert_eq!(ev.node_id(), Some("node-1"));
        assert_eq!(ev.control_id(), Some("ctrl-1"));
        assert_eq!(ev.tag(), Some("tag-1"));
        assert_eq!(ev.state(), Some("ringing"));
    }

    #[test]
    fn test_accessors_missing() {
        let ev = Event::new("x", HashMap::new(), 1.0);
        assert!(ev.call_id().is_none());
        assert!(ev.node_id().is_none());
        assert!(ev.control_id().is_none());
        assert!(ev.tag().is_none());
        assert!(ev.state().is_none());
    }

    #[test]
    fn test_parse_from_value() {
        let val = json!({"call_id": "c1", "state": "answered"});
        let ev = Event::parse("calling.call.state", &val);
        assert_eq!(ev.event_type(), "calling.call.state");
        assert_eq!(ev.call_id(), Some("c1"));
        assert_eq!(ev.state(), Some("answered"));
    }

    #[test]
    fn test_parse_from_non_object() {
        let val = json!("not-an-object");
        let ev = Event::parse("test", &val);
        assert!(ev.params().is_empty());
    }

    #[test]
    fn test_to_value() {
        let ev = Event::new("ev", sample_params(), 5.0);
        let v = ev.to_value();
        assert_eq!(v["event_type"], "ev");
        assert_eq!(v["timestamp"], 5.0);
        assert_eq!(v["params"]["call_id"], "call-1");
    }

    #[test]
    fn test_clone() {
        let ev = Event::new("ev", sample_params(), 1.0);
        let ev2 = ev.clone();
        assert_eq!(ev.event_type(), ev2.event_type());
        assert_eq!(ev.call_id(), ev2.call_id());
    }

    #[test]
    fn test_debug_format() {
        let ev = Event::new("test.event", HashMap::new(), 1.0);
        let dbg = format!("{:?}", ev);
        assert!(dbg.contains("test.event"));
    }
}
