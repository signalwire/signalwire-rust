use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::call::Call;
use super::constants;
use super::event::Event;
use super::message::Message;
use crate::logging::Logger;

/// Callback type for inbound call handler.
pub type OnCallHandler = Box<dyn Fn(Arc<Call>, &Event) + Send + Sync>;

/// Callback type for inbound message handler.
pub type OnMessageHandler = Box<dyn Fn(&Event, &Value) + Send + Sync>;

/// Callback type for generic events.
pub type OnEventHandler = Box<dyn Fn(&Event, &Value) + Send + Sync>;

/// Resolve callback for a pending RPC request.
type ResolveCallback = Box<dyn FnOnce(Value) + Send>;

/// Reject callback for a pending RPC request.
type RejectCallback = Box<dyn FnOnce(Value) + Send>;

/// Pending RPC request slot.
struct PendingRequest {
    resolve: Option<ResolveCallback>,
    reject: Option<RejectCallback>,
}

/// Pending dial slot.
struct PendingDial {
    resolve: Box<dyn FnOnce(Arc<Call>) + Send>,
    #[allow(dead_code)]
    tag: String,
}

/// RELAY Client -- manages the WebSocket connection to SignalWire, sends
/// JSON-RPC requests, and dispatches inbound events to the correct Call
/// or Message objects.
///
/// The transport layer (WebSocket send/receive) is abstracted so that
/// unit tests can inject messages without needing a real WebSocket.
pub struct Client {
    // ── identity / auth ───────────────────────────────────────────────
    pub project: String,
    pub token: String,
    pub host: String,
    pub contexts: Mutex<Vec<String>>,
    pub connected: Mutex<bool>,
    pub session_id: Mutex<Option<String>>,
    pub protocol: Mutex<Option<String>>,
    pub authorization_state: Mutex<Option<String>>,
    pub agent: String,

    // ── correlation maps ──────────────────────────────────────────────
    pending: Mutex<HashMap<String, PendingRequest>>,
    pub calls: Mutex<HashMap<String, Arc<Call>>>,
    pending_dials: Mutex<HashMap<String, PendingDial>>,
    pub messages: Mutex<HashMap<String, Arc<Message>>>,

    // ── event handlers ────────────────────────────────────────────────
    on_call_handler: Mutex<Option<OnCallHandler>>,
    on_message_handler: Mutex<Option<OnMessageHandler>>,
    on_event_handler: Mutex<Option<OnEventHandler>>,

    // ── internals ─────────────────────────────────────────────────────
    reconnect_delay: Mutex<u64>,
    running: Mutex<bool>,

    /// Messages sent through the transport (for testing).
    pub sent_messages: Mutex<Vec<Value>>,

    logger: Logger,
}

impl Client {
    pub fn new(project: &str, token: &str, host: &str) -> Self {
        Client {
            project: project.to_string(),
            token: token.to_string(),
            host: host.to_string(),
            contexts: Mutex::new(Vec::new()),
            connected: Mutex::new(false),
            session_id: Mutex::new(None),
            protocol: Mutex::new(None),
            authorization_state: Mutex::new(None),
            agent: "signalwire-agents-rust/1.0".to_string(),
            pending: Mutex::new(HashMap::new()),
            calls: Mutex::new(HashMap::new()),
            pending_dials: Mutex::new(HashMap::new()),
            messages: Mutex::new(HashMap::new()),
            on_call_handler: Mutex::new(None),
            on_message_handler: Mutex::new(None),
            on_event_handler: Mutex::new(None),
            reconnect_delay: Mutex::new(1),
            running: Mutex::new(false),
            sent_messages: Mutex::new(Vec::new()),
            logger: Logger::new("relay.client"),
        }
    }

    /// Create from env vars SIGNALWIRE_PROJECT_ID, SIGNALWIRE_API_TOKEN, SIGNALWIRE_SPACE.
    pub fn from_env() -> Result<Self, String> {
        let project =
            std::env::var("SIGNALWIRE_PROJECT_ID").map_err(|_| "SIGNALWIRE_PROJECT_ID not set")?;
        let token =
            std::env::var("SIGNALWIRE_API_TOKEN").map_err(|_| "SIGNALWIRE_API_TOKEN not set")?;
        let host = std::env::var("SIGNALWIRE_SPACE").map_err(|_| "SIGNALWIRE_SPACE not set")?;
        Ok(Self::new(&project, &token, &host))
    }

    // ══════════════════════════════════════════════════════════════════
    //  Connection lifecycle
    // ══════════════════════════════════════════════════════════════════

    /// Establish the WebSocket connection and authenticate.
    /// (Stub: production would open WSS to wss://{host}/api/relay/ws)
    ///
    /// Resets the reconnect delay on a fresh connection.  The delay is
    /// *not* reset when called from `reconnect()` because the backoff
    /// has already been bumped before the call.
    pub fn connect(&self) {
        self.logger
            .info(&format!("Connecting to {}", self.host));
        *self.connected.lock().unwrap() = true;
    }

    /// Initial connect -- resets reconnect delay and connects.
    pub fn connect_fresh(&self) {
        *self.reconnect_delay.lock().unwrap() = 1;
        self.connect();
    }

    /// Send the signalwire.connect RPC to authenticate and bind a session.
    pub fn authenticate(&self) {
        self.logger.info("Authenticating");

        let msg = json!({
            "jsonrpc": "2.0",
            "id": generate_uuid(),
            "method": "signalwire.connect",
            "params": {
                "version": {
                    "major": constants::PROTOCOL_VERSION_MAJOR,
                    "minor": constants::PROTOCOL_VERSION_MINOR,
                    "revision": constants::PROTOCOL_VERSION_REVISION,
                },
                "authentication": {
                    "project": self.project,
                    "token": self.token,
                },
                "agent": self.agent,
            },
        });

        self.send(&msg);
    }

    /// Gracefully close the connection.
    pub fn disconnect(&self) {
        self.logger.info("Disconnecting");
        *self.running.lock().unwrap() = false;
        *self.connected.lock().unwrap() = false;
    }

    /// Reconnect with exponential back-off (1 s -> 30 s cap).
    pub fn reconnect(&self) {
        *self.connected.lock().unwrap() = false;

        let delay = *self.reconnect_delay.lock().unwrap();
        self.logger
            .warn(&format!("Reconnecting in {}s", delay));

        // In a real implementation we would sleep here.

        {
            let mut rd = self.reconnect_delay.lock().unwrap();
            *rd = (*rd * 2).min(30);
        }

        self.connect();
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }

    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    // ══════════════════════════════════════════════════════════════════
    //  JSON-RPC transport
    // ══════════════════════════════════════════════════════════════════

    /// Build and send a JSON-RPC request. Returns the message ID.
    pub fn send_request(&self, method: &str, params: Value) -> String {
        let id = generate_uuid();
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send(&msg);
        id
    }

    /// Register a pending-response slot for a request ID.
    pub fn register_pending<R, E>(&self, id: &str, resolve: R, reject: E)
    where
        R: FnOnce(Value) + Send + 'static,
        E: FnOnce(Value) + Send + 'static,
    {
        self.pending.lock().unwrap().insert(
            id.to_string(),
            PendingRequest {
                resolve: Some(Box::new(resolve)),
                reject: Some(Box::new(reject)),
            },
        );
    }

    /// Send a raw JSON message through the transport.
    pub fn send(&self, msg: &Value) {
        self.logger
            .debug(&format!(">> {}", msg));
        self.sent_messages.lock().unwrap().push(msg.clone());
    }

    /// Send an acknowledgement for a server-initiated request.
    pub fn send_ack(&self, id: &str) {
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {},
        }));
    }

    // ══════════════════════════════════════════════════════════════════
    //  Inbound message handling
    // ══════════════════════════════════════════════════════════════════

    /// Parse a raw JSON string from the server and route it.
    pub fn handle_message(&self, raw: &str) {
        self.logger.debug(&format!("<< {}", raw));

        let data: Value = match serde_json::from_str(raw) {
            Ok(d) => d,
            Err(_) => {
                self.logger.warn("Received unparseable message");
                return;
            }
        };

        // ── response to a pending request ────────────────────────────
        if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
            let mut pending = self.pending.lock().unwrap();
            if let Some(mut slot) = pending.remove(id) {
                if data.get("error").is_some() {
                    if let Some(reject) = slot.reject.take() {
                        reject(data["error"].clone());
                    }
                } else {
                    if let Some(resolve) = slot.resolve.take() {
                        resolve(data.get("result").cloned().unwrap_or(json!({})));
                    }
                }
                return;
            }
        }

        // ── server-initiated request ─────────────────────────────────
        let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match method {
            "signalwire.ping" => {
                self.send_ack(id);
            }
            "signalwire.disconnect" => {
                self.handle_disconnect();
            }
            "signalwire.event" => {
                self.send_ack(id);
                let outer_params = data.get("params").cloned().unwrap_or(json!({}));
                self.handle_event(&outer_params);
            }
            _ => {
                self.logger
                    .debug(&format!("Unhandled method: {}", method));
            }
        }
    }

    /// Route a signalwire.event payload to the appropriate handler.
    pub fn handle_event(&self, outer_params: &Value) {
        let event_type = outer_params
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let params = outer_params.get("params").cloned().unwrap_or(json!({}));

        let event = Event::parse(event_type, &params);

        // ── authorization state ──────────────────────────────────────
        if event_type == "signalwire.authorization.state" {
            let auth_state = params
                .get("authorization_state")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            *self.authorization_state.lock().unwrap() = auth_state.clone();
            self.logger
                .info(&format!("Authorization state: {:?}", auth_state));
            return;
        }

        // ── inbound call ─────────────────────────────────────────────
        if event_type == "calling.call.receive" {
            self.handle_inbound_call(&event, &params);
            return;
        }

        // ── inbound message ──────────────────────────────────────────
        if event_type == "messaging.receive" {
            if let Some(handler) = self.on_message_handler.lock().unwrap().as_ref() {
                handler(&event, &params);
            }
            return;
        }

        // ── message state updates ────────────────────────────────────
        if event_type == "messaging.state" {
            if let Some(msg_id) = params.get("message_id").and_then(|v| v.as_str()) {
                let msg = self
                    .messages
                    .lock()
                    .unwrap()
                    .get(msg_id)
                    .cloned();
                if let Some(msg) = msg {
                    msg.dispatch_event(&event);
                    if let Some(s) = params.get("state").and_then(|v| v.as_str()) {
                        if constants::is_message_terminal(s) {
                            self.messages.lock().unwrap().remove(msg_id);
                        }
                    }
                }
            }
            return;
        }

        // ── call state with a pending dial tag ───────────────────────
        if event_type == "calling.call.state" {
            if let Some(tag) = params.get("tag").and_then(|v| v.as_str()) {
                let has_dial = self.pending_dials.lock().unwrap().contains_key(tag);
                if has_dial {
                    if let Some(call_id) = params.get("call_id").and_then(|v| v.as_str()) {
                        let mut calls = self.calls.lock().unwrap();
                        if !calls.contains_key(call_id) {
                            let call = Arc::new(Call::new(&params));
                            calls.insert(call_id.to_string(), call);
                        }
                    }
                }
            }
        }

        // ── dial completion event ────────────────────────────────────
        if event_type == "calling.call.dial" {
            self.handle_dial_event(&event, &params);
            return;
        }

        // ── default: route to the Call by call_id ────────────────────
        if let Some(call_id) = params
            .get("call_id")
            .and_then(|v| v.as_str())
        {
            let call = self.calls.lock().unwrap().get(call_id).cloned();
            if let Some(call) = call {
                call.dispatch_event(&event);

                if call.current_state() == constants::CALL_STATE_ENDED {
                    self.calls.lock().unwrap().remove(call_id);
                }
                return;
            }
        }

        // Fire generic event handler if nothing else matched.
        if let Some(handler) = self.on_event_handler.lock().unwrap().as_ref() {
            handler(&event, outer_params);
        }
    }

    // ══════════════════════════════════════════════════════════════════
    //  Public API methods
    // ══════════════════════════════════════════════════════════════════

    /// Subscribe to one or more inbound contexts.
    pub fn receive(&self, contexts: &[String]) {
        {
            let mut ctx = self.contexts.lock().unwrap();
            for c in contexts {
                if !ctx.contains(c) {
                    ctx.push(c.clone());
                }
            }
        }

        self.send_request("signalwire.receive", json!({"contexts": contexts}));
    }

    /// Unsubscribe from one or more contexts.
    pub fn unreceive(&self, contexts: &[String]) {
        {
            let mut ctx = self.contexts.lock().unwrap();
            ctx.retain(|c| !contexts.contains(c));
        }

        self.send_request("signalwire.unreceive", json!({"contexts": contexts}));
    }

    /// Register a handler for inbound calls.
    pub fn on_call<F: Fn(Arc<Call>, &Event) + Send + Sync + 'static>(&self, cb: F) {
        *self.on_call_handler.lock().unwrap() = Some(Box::new(cb));
    }

    /// Register a handler for inbound messages.
    pub fn on_message<F: Fn(&Event, &Value) + Send + Sync + 'static>(&self, cb: F) {
        *self.on_message_handler.lock().unwrap() = Some(Box::new(cb));
    }

    /// Register a generic event handler.
    pub fn on_event<F: Fn(&Event, &Value) + Send + Sync + 'static>(&self, cb: F) {
        *self.on_event_handler.lock().unwrap() = Some(Box::new(cb));
    }

    /// Get a call by ID.
    pub fn get_call(&self, call_id: &str) -> Option<Arc<Call>> {
        self.calls.lock().unwrap().get(call_id).cloned()
    }

    /// Get a message by ID.
    pub fn get_message(&self, message_id: &str) -> Option<Arc<Message>> {
        self.messages.lock().unwrap().get(message_id).cloned()
    }

    /// Track a new message.
    pub fn track_message(&self, message_id: &str, msg: Arc<Message>) {
        self.messages
            .lock()
            .unwrap()
            .insert(message_id.to_string(), msg);
    }

    /// Register a pending dial.
    pub fn register_dial<F: FnOnce(Arc<Call>) + Send + 'static>(
        &self,
        tag: &str,
        resolve: F,
    ) {
        self.pending_dials.lock().unwrap().insert(
            tag.to_string(),
            PendingDial {
                resolve: Box::new(resolve),
                tag: tag.to_string(),
            },
        );
    }

    /// Remove a pending dial.
    pub fn remove_pending_dial(&self, tag: &str) {
        self.pending_dials.lock().unwrap().remove(tag);
    }

    // ══════════════════════════════════════════════════════════════════
    //  Private helpers
    // ══════════════════════════════════════════════════════════════════

    fn handle_inbound_call(&self, event: &Event, params: &Value) {
        let call_id = match params.get("call_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => {
                self.logger.warn("Inbound call event missing call_id");
                return;
            }
        };

        let call = Arc::new(Call::new(params));
        self.calls
            .lock()
            .unwrap()
            .insert(call_id.to_string(), call.clone());

        self.logger
            .info(&format!("Inbound call {}", call_id));

        if let Some(handler) = self.on_call_handler.lock().unwrap().as_ref() {
            handler(call, event);
        }
    }

    fn handle_dial_event(&self, _event: &Event, params: &Value) {
        let tag = match params.get("tag").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return,
        };
        let call_id = params.get("call_id").and_then(|v| v.as_str());

        // Ensure we have a Call object
        let call = if let Some(cid) = call_id {
            let mut calls = self.calls.lock().unwrap();
            if let Some(existing) = calls.get(cid) {
                existing.clone()
            } else {
                let call = Arc::new(Call::new(params));
                calls.insert(cid.to_string(), call.clone());
                call
            }
        } else {
            return;
        };

        // Resolve the pending dial
        let pending = self.pending_dials.lock().unwrap().remove(&tag);
        if let Some(dial) = pending {
            *call.dial_winner.lock().unwrap() = true;
            (dial.resolve)(call);
        }
    }

    fn handle_disconnect(&self) {
        self.logger.warn("Server sent disconnect");
        *self.connected.lock().unwrap() = false;
    }
}

/// Generate a simple UUID v4.
fn generate_uuid() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut data = [0u8; 16];
    rng.fill(&mut data);
    data[6] = (data[6] & 0x0f) | 0x40;
    data[8] = (data[8] & 0x3f) | 0x80;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
        u16::from_be_bytes([data[4], data[5]]),
        u16::from_be_bytes([data[6], data[7]]),
        u16::from_be_bytes([data[8], data[9]]),
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

    fn make_client() -> Client {
        Client::new("test-project", "test-token", "test.signalwire.com")
    }

    #[test]
    fn test_client_new() {
        let c = make_client();
        assert_eq!(c.project, "test-project");
        assert_eq!(c.token, "test-token");
        assert_eq!(c.host, "test.signalwire.com");
        assert!(!c.is_connected());
    }

    #[test]
    fn test_connect_disconnect() {
        let c = make_client();
        c.connect();
        assert!(c.is_connected());
        c.disconnect();
        assert!(!c.is_connected());
    }

    #[test]
    fn test_reconnect_backoff() {
        let c = make_client();
        c.connect();
        c.reconnect();
        assert_eq!(*c.reconnect_delay.lock().unwrap(), 2);
        c.reconnect();
        assert_eq!(*c.reconnect_delay.lock().unwrap(), 4);
        // Verify cap
        for _ in 0..10 {
            c.reconnect();
        }
        assert!(*c.reconnect_delay.lock().unwrap() <= 30);
    }

    #[test]
    fn test_authenticate_sends_message() {
        let c = make_client();
        c.authenticate();
        let msgs = c.sent_messages.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0]["method"], "signalwire.connect");
        assert_eq!(msgs[0]["params"]["authentication"]["project"], "test-project");
    }

    #[test]
    fn test_send_request() {
        let c = make_client();
        let id = c.send_request("calling.dial", json!({"to": "+1555"}));
        assert!(!id.is_empty());
        let msgs = c.sent_messages.lock().unwrap();
        assert_eq!(msgs[0]["method"], "calling.dial");
    }

    #[test]
    fn test_send_ack() {
        let c = make_client();
        c.send_ack("req-123");
        let msgs = c.sent_messages.lock().unwrap();
        assert_eq!(msgs[0]["id"], "req-123");
        assert!(msgs[0]["result"].is_object());
    }

    #[test]
    fn test_handle_message_response_resolve() {
        let c = make_client();
        let result = Arc::new(Mutex::new(None));
        let result2 = result.clone();

        let id = c.send_request("test.method", json!({}));
        c.register_pending(
            &id,
            move |v| {
                *result2.lock().unwrap() = Some(v);
            },
            |_| {},
        );

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {"session_id": "sess-1"},
        });
        c.handle_message(&response.to_string());

        let r = result.lock().unwrap();
        assert!(r.is_some());
        assert_eq!(r.as_ref().unwrap()["session_id"], "sess-1");
    }

    #[test]
    fn test_handle_message_response_reject() {
        let c = make_client();
        let error = Arc::new(Mutex::new(None));
        let error2 = error.clone();

        let id = c.send_request("test.method", json!({}));
        c.register_pending(
            &id,
            |_| {},
            move |v| {
                *error2.lock().unwrap() = Some(v);
            },
        );

        let response = json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {"code": -32000, "message": "fail"},
        });
        c.handle_message(&response.to_string());

        let e = error.lock().unwrap();
        assert!(e.is_some());
        assert_eq!(e.as_ref().unwrap()["code"], -32000);
    }

    #[test]
    fn test_handle_ping() {
        let c = make_client();
        let msg = json!({
            "jsonrpc": "2.0",
            "id": "ping-1",
            "method": "signalwire.ping",
        });
        c.handle_message(&msg.to_string());

        let msgs = c.sent_messages.lock().unwrap();
        // Should have sent an ack
        let ack = msgs.iter().find(|m| m["id"] == "ping-1");
        assert!(ack.is_some());
    }

    #[test]
    fn test_handle_disconnect() {
        let c = make_client();
        c.connect();
        assert!(c.is_connected());

        let msg = json!({
            "jsonrpc": "2.0",
            "id": "dc-1",
            "method": "signalwire.disconnect",
            "params": {},
        });
        c.handle_message(&msg.to_string());
        assert!(!c.is_connected());
    }

    #[test]
    fn test_handle_inbound_call() {
        let c = make_client();
        let received = Arc::new(Mutex::new(false));
        let received2 = received.clone();
        c.on_call(move |_call, _ev| {
            *received2.lock().unwrap() = true;
        });

        c.handle_event(&json!({
            "event_type": "calling.call.receive",
            "params": {
                "call_id": "call-1",
                "node_id": "node-1",
                "context": "default",
            },
        }));

        assert!(*received.lock().unwrap());
        assert!(c.calls.lock().unwrap().contains_key("call-1"));
    }

    #[test]
    fn test_handle_call_state_event() {
        let c = make_client();

        // Create a call first
        c.handle_event(&json!({
            "event_type": "calling.call.receive",
            "params": {"call_id": "call-1", "node_id": "node-1"},
        }));

        // Send state event
        c.handle_event(&json!({
            "event_type": "calling.call.state",
            "params": {"call_id": "call-1", "state": "ringing"},
        }));

        let call = c.get_call("call-1").unwrap();
        assert_eq!(call.current_state(), "ringing");
    }

    #[test]
    fn test_handle_call_ended_removes_call() {
        let c = make_client();

        c.handle_event(&json!({
            "event_type": "calling.call.receive",
            "params": {"call_id": "call-1", "node_id": "node-1"},
        }));

        c.handle_event(&json!({
            "event_type": "calling.call.state",
            "params": {"call_id": "call-1", "state": "ended"},
        }));

        assert!(c.get_call("call-1").is_none());
    }

    #[test]
    fn test_handle_message_state() {
        let c = make_client();
        let msg = Arc::new(Message::new(&json!({"message_id": "msg-1"})));
        c.track_message("msg-1", msg.clone());

        c.handle_event(&json!({
            "event_type": "messaging.state",
            "params": {"message_id": "msg-1", "state": "sent"},
        }));

        assert_eq!(msg.state(), Some("sent".to_string()));
        // Not terminal, should still be tracked
        assert!(c.get_message("msg-1").is_some());
    }

    #[test]
    fn test_handle_message_terminal_removes() {
        let c = make_client();
        let msg = Arc::new(Message::new(&json!({"message_id": "msg-1"})));
        c.track_message("msg-1", msg.clone());

        c.handle_event(&json!({
            "event_type": "messaging.state",
            "params": {"message_id": "msg-1", "state": "delivered"},
        }));

        assert!(msg.is_done());
        assert!(c.get_message("msg-1").is_none());
    }

    #[test]
    fn test_handle_inbound_message() {
        let c = make_client();
        let received = Arc::new(Mutex::new(false));
        let received2 = received.clone();
        c.on_message(move |_ev, _params| {
            *received2.lock().unwrap() = true;
        });

        c.handle_event(&json!({
            "event_type": "messaging.receive",
            "params": {"message_id": "msg-1", "body": "Hello"},
        }));

        assert!(*received.lock().unwrap());
    }

    #[test]
    fn test_handle_dial_event() {
        let c = make_client();
        let resolved_call = Arc::new(Mutex::new(None));
        let resolved2 = resolved_call.clone();

        c.register_dial("tag-dial-1", move |call| {
            *resolved2.lock().unwrap() = Some(call);
        });

        // First create call via state event with tag
        c.handle_event(&json!({
            "event_type": "calling.call.state",
            "params": {"call_id": "call-dial-1", "tag": "tag-dial-1", "state": "created"},
        }));

        // Then the dial event resolves it
        c.handle_event(&json!({
            "event_type": "calling.call.dial",
            "params": {"call_id": "call-dial-1", "tag": "tag-dial-1", "state": "answered"},
        }));

        let r = resolved_call.lock().unwrap();
        assert!(r.is_some());
        assert!(*r.as_ref().unwrap().dial_winner.lock().unwrap());
    }

    #[test]
    fn test_handle_authorization_state() {
        let c = make_client();
        c.handle_event(&json!({
            "event_type": "signalwire.authorization.state",
            "params": {"authorization_state": "authorized"},
        }));
        assert_eq!(
            *c.authorization_state.lock().unwrap(),
            Some("authorized".to_string())
        );
    }

    #[test]
    fn test_receive_contexts() {
        let c = make_client();
        c.receive(&["default".to_string(), "support".to_string()]);
        let ctx = c.contexts.lock().unwrap();
        assert_eq!(ctx.len(), 2);
        assert!(ctx.contains(&"default".to_string()));

        let msgs = c.sent_messages.lock().unwrap();
        assert!(msgs.iter().any(|m| m["method"] == "signalwire.receive"));
    }

    #[test]
    fn test_receive_no_duplicates() {
        let c = make_client();
        c.receive(&["default".to_string()]);
        c.receive(&["default".to_string(), "other".to_string()]);
        let ctx = c.contexts.lock().unwrap();
        assert_eq!(ctx.len(), 2);
    }

    #[test]
    fn test_unreceive() {
        let c = make_client();
        c.receive(&["a".to_string(), "b".to_string(), "c".to_string()]);
        c.unreceive(&["b".to_string()]);
        let ctx = c.contexts.lock().unwrap();
        assert_eq!(ctx.len(), 2);
        assert!(!ctx.contains(&"b".to_string()));
    }

    #[test]
    fn test_on_event_handler() {
        let c = make_client();
        let received = Arc::new(Mutex::new(false));
        let received2 = received.clone();
        c.on_event(move |_ev, _params| {
            *received2.lock().unwrap() = true;
        });

        // An unrecognized event should fall through to the generic handler
        c.handle_event(&json!({
            "event_type": "unknown.event",
            "params": {},
        }));

        assert!(*received.lock().unwrap());
    }

    #[test]
    fn test_handle_unparseable_message() {
        let c = make_client();
        // Should not panic
        c.handle_message("not-json{{{");
    }

    #[test]
    fn test_handle_event_signalwire_event_method() {
        let c = make_client();
        let received = Arc::new(Mutex::new(false));
        let received2 = received.clone();
        c.on_call(move |_call, _ev| {
            *received2.lock().unwrap() = true;
        });

        let msg = json!({
            "jsonrpc": "2.0",
            "id": "evt-1",
            "method": "signalwire.event",
            "params": {
                "event_type": "calling.call.receive",
                "params": {"call_id": "c1", "node_id": "n1"},
            },
        });
        c.handle_message(&msg.to_string());

        assert!(*received.lock().unwrap());
        // Should have sent an ack
        let msgs = c.sent_messages.lock().unwrap();
        assert!(msgs.iter().any(|m| m["id"] == "evt-1"));
    }
}
