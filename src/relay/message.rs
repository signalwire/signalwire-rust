use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::constants;
use super::event::Event;

/// Callback type for completion notifications.
pub type MessageCompletedCallback = Box<dyn FnOnce(&Message) + Send>;

/// Represents a RELAY messaging message (SMS / MMS).
///
/// A Message is created when you send or receive a message through the
/// RELAY messaging namespace.  It accumulates state-change events and
/// resolves once the message reaches a terminal state (delivered,
/// undelivered, or failed).
pub struct Message {
    message_id: Option<String>,
    context: Option<String>,
    direction: Option<String>,
    from_number: Option<String>,
    to_number: Option<String>,
    body: Mutex<Option<String>>,
    media: Mutex<Vec<String>>,
    tags: Mutex<Vec<String>>,
    state: Mutex<Option<String>>,
    reason: Mutex<Option<String>>,
    completed: Mutex<bool>,
    result: Mutex<Option<Value>>,
    on_completed: Mutex<Option<MessageCompletedCallback>>,
    callback_fired: Mutex<bool>,
    on_event_callbacks: Mutex<Vec<Arc<dyn Fn(&Message, &Event) + Send + Sync>>>,
}

impl Message {
    /// Build a Message from a params map.
    pub fn new(params: &Value) -> Self {
        Message {
            message_id: params
                .get("message_id")
                .or_else(|| params.get("id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            context: params
                .get("context")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            direction: params
                .get("direction")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            from_number: params
                .get("from_number")
                .or_else(|| params.get("from"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            to_number: params
                .get("to_number")
                .or_else(|| params.get("to"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            body: Mutex::new(
                params
                    .get("body")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            ),
            media: Mutex::new(
                params
                    .get("media")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            tags: Mutex::new(
                params
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            state: Mutex::new(
                params
                    .get("state")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            ),
            reason: Mutex::new(
                params
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            ),
            completed: Mutex::new(false),
            result: Mutex::new(None),
            on_completed: Mutex::new(None),
            callback_fired: Mutex::new(false),
            on_event_callbacks: Mutex::new(Vec::new()),
        }
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    pub fn message_id(&self) -> Option<&str> {
        self.message_id.as_deref()
    }

    pub fn context(&self) -> Option<&str> {
        self.context.as_deref()
    }

    pub fn direction(&self) -> Option<&str> {
        self.direction.as_deref()
    }

    pub fn from_number(&self) -> Option<&str> {
        self.from_number.as_deref()
    }

    pub fn to_number(&self) -> Option<&str> {
        self.to_number.as_deref()
    }

    pub fn body(&self) -> Option<String> {
        self.body.lock().unwrap().clone()
    }

    pub fn media(&self) -> Vec<String> {
        self.media.lock().unwrap().clone()
    }

    pub fn tags(&self) -> Vec<String> {
        self.tags.lock().unwrap().clone()
    }

    pub fn state(&self) -> Option<String> {
        self.state.lock().unwrap().clone()
    }

    pub fn reason(&self) -> Option<String> {
        self.reason.lock().unwrap().clone()
    }

    pub fn is_done(&self) -> bool {
        *self.completed.lock().unwrap()
    }

    pub fn result(&self) -> Option<Value> {
        self.result.lock().unwrap().clone()
    }

    // ------------------------------------------------------------------
    // Event dispatch
    // ------------------------------------------------------------------

    /// Process an inbound event for this message.
    pub fn dispatch_event(&self, event: &Event) {
        let params = event.params();

        if let Some(s) = params.get("state").and_then(|v| v.as_str()) {
            *self.state.lock().unwrap() = Some(s.to_string());
        }
        if let Some(r) = params.get("reason").and_then(|v| v.as_str()) {
            *self.reason.lock().unwrap() = Some(r.to_string());
        }
        if let Some(b) = params.get("body").and_then(|v| v.as_str()) {
            *self.body.lock().unwrap() = Some(b.to_string());
        }
        if let Some(m) = params.get("media").and_then(|v| v.as_array()) {
            *self.media.lock().unwrap() = m
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
        if let Some(t) = params.get("tags").and_then(|v| v.as_array()) {
            *self.tags.lock().unwrap() = t
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        // Notify event listeners
        let callbacks = self.on_event_callbacks.lock().unwrap().clone();
        for cb in &callbacks {
            cb(self, event);
        }

        // Auto-resolve on terminal state
        let current_state = self.state.lock().unwrap().clone();
        if let Some(ref s) = current_state {
            if constants::is_message_terminal(s) {
                self.resolve(Some(serde_json::json!(s)));
            }
        }
    }

    // ------------------------------------------------------------------
    // Callbacks
    // ------------------------------------------------------------------

    /// Register a listener that fires on every state-change event.
    pub fn on<F: Fn(&Message, &Event) + Send + Sync + 'static>(&self, cb: F) {
        self.on_event_callbacks
            .lock()
            .unwrap()
            .push(Arc::new(cb));
    }

    /// Register a callback to fire when the message reaches a terminal state.
    pub fn on_completed<F: FnOnce(&Message) + Send + 'static>(&self, cb: F) {
        *self.on_completed.lock().unwrap() = Some(Box::new(cb));

        if self.is_done() && !*self.callback_fired.lock().unwrap() {
            self.fire_callback();
        }
    }

    // ------------------------------------------------------------------
    // Resolution
    // ------------------------------------------------------------------

    /// Mark this message as completed.
    pub fn resolve(&self, result: Option<Value>) {
        {
            let mut completed = self.completed.lock().unwrap();
            if *completed {
                return;
            }
            *completed = true;
        }

        *self.result.lock().unwrap() = result;
        self.fire_callback();
    }

    // ------------------------------------------------------------------
    // Internal
    // ------------------------------------------------------------------

    fn fire_callback(&self) {
        let mut fired = self.callback_fired.lock().unwrap();
        if *fired {
            return;
        }
        let cb = self.on_completed.lock().unwrap().take();
        if let Some(callback) = cb {
            *fired = true;
            drop(fired);
            callback(self);
        }
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn sample_params() -> Value {
        json!({
            "message_id": "msg-1",
            "context": "default",
            "direction": "outbound",
            "from_number": "+15551001000",
            "to_number": "+15552002000",
            "body": "Hello",
            "media": ["http://example.com/pic.jpg"],
            "tags": ["tag1"],
            "state": "queued",
        })
    }

    #[test]
    fn test_new_from_params() {
        let m = Message::new(&sample_params());
        assert_eq!(m.message_id(), Some("msg-1"));
        assert_eq!(m.context(), Some("default"));
        assert_eq!(m.direction(), Some("outbound"));
        assert_eq!(m.from_number(), Some("+15551001000"));
        assert_eq!(m.to_number(), Some("+15552002000"));
        assert_eq!(m.body(), Some("Hello".to_string()));
        assert_eq!(m.media(), vec!["http://example.com/pic.jpg"]);
        assert_eq!(m.tags(), vec!["tag1"]);
        assert_eq!(m.state(), Some("queued".to_string()));
        assert!(!m.is_done());
    }

    #[test]
    fn test_new_minimal() {
        let m = Message::new(&json!({}));
        assert!(m.message_id().is_none());
        assert!(m.context().is_none());
        assert!(m.body().is_none());
        assert!(m.media().is_empty());
    }

    #[test]
    fn test_new_with_alt_keys() {
        let m = Message::new(&json!({"id": "alt-id", "from": "+1", "to": "+2"}));
        assert_eq!(m.message_id(), Some("alt-id"));
        assert_eq!(m.from_number(), Some("+1"));
        assert_eq!(m.to_number(), Some("+2"));
    }

    #[test]
    fn test_dispatch_event_updates_state() {
        let m = Message::new(&sample_params());
        let mut params = HashMap::new();
        params.insert("state".to_string(), json!("sent"));
        let ev = super::super::event::Event::new("messaging.state", params, 1.0);
        m.dispatch_event(&ev);
        assert_eq!(m.state(), Some("sent".to_string()));
    }

    #[test]
    fn test_dispatch_event_updates_body() {
        let m = Message::new(&json!({}));
        let mut params = HashMap::new();
        params.insert("body".to_string(), json!("Updated body"));
        let ev = super::super::event::Event::new("messaging.state", params, 1.0);
        m.dispatch_event(&ev);
        assert_eq!(m.body(), Some("Updated body".to_string()));
    }

    #[test]
    fn test_auto_resolve_on_terminal_delivered() {
        let m = Message::new(&json!({}));
        let mut params = HashMap::new();
        params.insert("state".to_string(), json!("delivered"));
        let ev = super::super::event::Event::new("messaging.state", params, 1.0);
        m.dispatch_event(&ev);
        assert!(m.is_done());
        assert_eq!(m.result().unwrap(), json!("delivered"));
    }

    #[test]
    fn test_auto_resolve_on_terminal_failed() {
        let m = Message::new(&json!({}));
        let mut params = HashMap::new();
        params.insert("state".to_string(), json!("failed"));
        params.insert("reason".to_string(), json!("invalid number"));
        let ev = super::super::event::Event::new("messaging.state", params, 1.0);
        m.dispatch_event(&ev);
        assert!(m.is_done());
        assert_eq!(m.reason(), Some("invalid number".to_string()));
    }

    #[test]
    fn test_no_auto_resolve_on_non_terminal() {
        let m = Message::new(&json!({}));
        let mut params = HashMap::new();
        params.insert("state".to_string(), json!("sent"));
        let ev = super::super::event::Event::new("messaging.state", params, 1.0);
        m.dispatch_event(&ev);
        assert!(!m.is_done());
    }

    #[test]
    fn test_resolve_idempotent() {
        let m = Message::new(&json!({}));
        m.resolve(Some(json!("first")));
        m.resolve(Some(json!("second")));
        assert_eq!(m.result().unwrap(), json!("first"));
    }

    #[test]
    fn test_on_completed_fires() {
        let m = Arc::new(Message::new(&json!({})));
        let flag = Arc::new(Mutex::new(false));
        let flag2 = flag.clone();
        m.on_completed(move |_| {
            *flag2.lock().unwrap() = true;
        });
        m.resolve(None);
        assert!(*flag.lock().unwrap());
    }

    #[test]
    fn test_on_completed_fires_if_already_done() {
        let m = Arc::new(Message::new(&json!({})));
        m.resolve(None);
        let flag = Arc::new(Mutex::new(false));
        let flag2 = flag.clone();
        m.on_completed(move |_| {
            *flag2.lock().unwrap() = true;
        });
        assert!(*flag.lock().unwrap());
    }

    #[test]
    fn test_on_event_listener() {
        let m = Message::new(&json!({}));
        let count = Arc::new(Mutex::new(0u32));
        let count2 = count.clone();
        m.on(move |_, _| {
            *count2.lock().unwrap() += 1;
        });

        let mut params = HashMap::new();
        params.insert("state".to_string(), json!("sent"));
        let ev = super::super::event::Event::new("messaging.state", params, 1.0);
        m.dispatch_event(&ev);
        assert_eq!(*count.lock().unwrap(), 1);
    }
}
