use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use super::event::Event;

/// Callback type for completion notifications.
pub type CompletedCallback = Box<dyn FnOnce(&Action) + Send>;

/// Sender half of a oneshot channel for async callers waiting on resolution.
/// We box this so Action stays object-safe.
type OneshotSender = std::sync::mpsc::SyncSender<()>;

/// Base class for all RELAY call actions (play, record, collect, etc.).
///
/// An Action is the client-side handle returned when you start an
/// asynchronous operation on a call.  It accumulates events, tracks
/// state, and resolves once the operation reaches a terminal state.
pub struct Action {
    control_id: String,
    call_id: String,
    node_id: String,
    state: Mutex<Option<String>>,
    completed: Mutex<bool>,
    result: Mutex<Option<Value>>,
    events: Mutex<Vec<Event>>,
    payload: Mutex<HashMap<String, Value>>,
    on_completed: Mutex<Option<CompletedCallback>>,
    callback_fired: Mutex<bool>,
    /// A sender to notify async waiters when the action resolves.
    notify_tx: Mutex<Option<OneshotSender>>,
    /// The RPC method used to stop this action (empty = not stoppable).
    stop_method: String,
    /// Commands recorded during tests (method, params).
    pub(crate) sent_commands: Mutex<Vec<(String, HashMap<String, Value>)>>,
}

impl Action {
    pub fn new(control_id: &str, call_id: &str, node_id: &str) -> Self {
        Self::with_stop_method(control_id, call_id, node_id, "")
    }

    pub fn with_stop_method(
        control_id: &str,
        call_id: &str,
        node_id: &str,
        stop_method: &str,
    ) -> Self {
        Action {
            control_id: control_id.to_string(),
            call_id: call_id.to_string(),
            node_id: node_id.to_string(),
            state: Mutex::new(None),
            completed: Mutex::new(false),
            result: Mutex::new(None),
            events: Mutex::new(Vec::new()),
            payload: Mutex::new(HashMap::new()),
            on_completed: Mutex::new(None),
            callback_fired: Mutex::new(false),
            notify_tx: Mutex::new(None),
            stop_method: stop_method.to_string(),
            sent_commands: Mutex::new(Vec::new()),
        }
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    pub fn control_id(&self) -> &str {
        &self.control_id
    }

    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    pub fn node_id(&self) -> &str {
        &self.node_id
    }

    pub fn is_done(&self) -> bool {
        *self.completed.lock().unwrap()
    }

    pub fn state(&self) -> Option<String> {
        self.state.lock().unwrap().clone()
    }

    pub fn result(&self) -> Option<Value> {
        self.result.lock().unwrap().clone()
    }

    pub fn payload(&self) -> HashMap<String, Value> {
        self.payload.lock().unwrap().clone()
    }

    pub fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    pub fn stop_method(&self) -> &str {
        &self.stop_method
    }

    // ------------------------------------------------------------------
    // Async wait support
    // ------------------------------------------------------------------

    /// Install a channel sender so that `wait_sync()` can block until
    /// the action resolves.
    pub fn set_notify_sender(&self, tx: OneshotSender) {
        *self.notify_tx.lock().unwrap() = Some(tx);
    }

    // ------------------------------------------------------------------
    // Callback registration
    // ------------------------------------------------------------------

    /// Register a callback to fire when the action completes.
    /// If the action is already done, the callback fires immediately.
    pub fn on_completed<F: FnOnce(&Action) + Send + 'static>(&self, cb: F) {
        let mut guard = self.on_completed.lock().unwrap();
        *guard = Some(Box::new(cb));
        drop(guard);

        if self.is_done() && !*self.callback_fired.lock().unwrap() {
            self.fire_callback();
        }
    }

    // ------------------------------------------------------------------
    // Event handling
    // ------------------------------------------------------------------

    /// Append an incoming event and update local state / payload.
    /// Subclasses override `should_handle_event` to filter.
    pub fn handle_event(&self, event: &Event) {
        if !self.should_handle_event(event) {
            return;
        }

        self.events.lock().unwrap().push(event.clone());

        // Merge params into payload
        let mut payload = self.payload.lock().unwrap();
        for (k, v) in event.params() {
            payload.insert(k.clone(), v.clone());
        }

        if let Some(s) = event.state() {
            *self.state.lock().unwrap() = Some(s.to_string());
        }
    }

    /// Override point for subclasses that need to filter events.
    /// Default: accept all events.
    fn should_handle_event(&self, _event: &Event) -> bool {
        true
    }

    // ------------------------------------------------------------------
    // Resolution
    // ------------------------------------------------------------------

    /// Mark this action as completed.
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

        // Notify async waiters
        if let Some(tx) = self.notify_tx.lock().unwrap().take() {
            let _ = tx.try_send(());
        }
    }

    // ------------------------------------------------------------------
    // Sub-command helpers
    // ------------------------------------------------------------------

    /// Stop the running action by sending its stop sub-command.
    pub fn stop(&self) {
        if !self.stop_method.is_empty() {
            self.execute_subcommand(&self.stop_method.clone(), HashMap::new());
        }
    }

    /// Send a sub-command RPC through the client.
    pub fn execute_subcommand(&self, method: &str, extra: HashMap<String, Value>) {
        let mut params = HashMap::new();
        params.insert(
            "control_id".to_string(),
            Value::String(self.control_id.clone()),
        );
        params.insert(
            "call_id".to_string(),
            Value::String(self.call_id.clone()),
        );
        params.insert(
            "node_id".to_string(),
            Value::String(self.node_id.clone()),
        );
        for (k, v) in extra {
            params.insert(k, v);
        }
        self.sent_commands
            .lock()
            .unwrap()
            .push((method.to_string(), params));
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

// ======================================================================
// Concrete action subclasses
//
// In Rust we cannot use OOP inheritance, so each "subclass" is a
// newtype wrapper around an Arc<Action> that configures stop_method
// and may add extra accessors.
// ======================================================================

macro_rules! action_subclass {
    ($name:ident, $stop:expr) => {
        pub struct $name {
            pub inner: Arc<Action>,
        }

        impl $name {
            pub fn new(control_id: &str, call_id: &str, node_id: &str) -> Self {
                $name {
                    inner: Arc::new(Action::with_stop_method(
                        control_id, call_id, node_id, $stop,
                    )),
                }
            }

            pub fn action(&self) -> &Action {
                &self.inner
            }
        }

        impl std::ops::Deref for $name {
            type Target = Action;
            fn deref(&self) -> &Action {
                &self.inner
            }
        }
    };
}

// -- PlayAction --------------------------------------------------------

action_subclass!(PlayAction, "calling.play.stop");

impl PlayAction {
    pub fn pause(&self) {
        self.execute_subcommand("calling.play.pause", HashMap::new());
    }

    pub fn resume(&self) {
        self.execute_subcommand("calling.play.resume", HashMap::new());
    }

    pub fn volume(&self, db: f64) {
        let mut extra = HashMap::new();
        extra.insert("volume".to_string(), serde_json::json!(db));
        self.execute_subcommand("calling.play.volume", extra);
    }
}

// -- RecordAction ------------------------------------------------------

action_subclass!(RecordAction, "calling.record.stop");

impl RecordAction {
    pub fn pause(&self) {
        self.execute_subcommand("calling.record.pause", HashMap::new());
    }

    pub fn resume(&self) {
        self.execute_subcommand("calling.record.resume", HashMap::new());
    }

    pub fn url(&self) -> Option<String> {
        self.payload()
            .get("url")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn duration(&self) -> Option<f64> {
        self.payload().get("duration").and_then(|v| v.as_f64())
    }

    pub fn size(&self) -> Option<u64> {
        self.payload().get("size").and_then(|v| v.as_u64())
    }
}

// -- CollectAction (ignores play events) -------------------------------

pub struct CollectAction {
    pub inner: Arc<Action>,
}

impl CollectAction {
    pub fn new(control_id: &str, call_id: &str, node_id: &str) -> Self {
        CollectAction {
            inner: Arc::new(Action::with_stop_method(
                control_id,
                call_id,
                node_id,
                "calling.collect.stop",
            )),
        }
    }

    pub fn action(&self) -> &Action {
        &self.inner
    }

    pub fn start_input_timers(&self) {
        self.execute_subcommand("calling.collect.start_input_timers", HashMap::new());
    }

    pub fn collect_result(&self) -> Option<Value> {
        self.payload().get("result").cloned()
    }

    /// Handle an event, silently ignoring intermediate play events.
    pub fn handle_event_filtered(&self, event: &Event) {
        if event.event_type() == "calling.call.play" {
            return;
        }
        self.inner.handle_event(event);
    }
}

impl std::ops::Deref for CollectAction {
    type Target = Action;
    fn deref(&self) -> &Action {
        &self.inner
    }
}

// -- DetectAction ------------------------------------------------------

action_subclass!(DetectAction, "calling.detect.stop");

impl DetectAction {
    pub fn detect_result(&self) -> Option<Value> {
        let p = self.payload();
        p.get("detect").or_else(|| p.get("result")).cloned()
    }
}

// -- FaxAction ---------------------------------------------------------

pub struct FaxAction {
    pub inner: Arc<Action>,
    pub fax_type: String,
}

impl FaxAction {
    pub fn new(control_id: &str, call_id: &str, node_id: &str, fax_type: &str) -> Self {
        let stop = if fax_type == "receive" {
            "calling.receive_fax.stop"
        } else {
            "calling.send_fax.stop"
        };
        FaxAction {
            inner: Arc::new(Action::with_stop_method(control_id, call_id, node_id, stop)),
            fax_type: fax_type.to_string(),
        }
    }

    pub fn action(&self) -> &Action {
        &self.inner
    }

    pub fn fax_type(&self) -> &str {
        &self.fax_type
    }
}

impl std::ops::Deref for FaxAction {
    type Target = Action;
    fn deref(&self) -> &Action {
        &self.inner
    }
}

// -- Simple action wrappers --------------------------------------------

action_subclass!(TapAction, "calling.tap.stop");
action_subclass!(StreamAction, "calling.stream.stop");
action_subclass!(PayAction, "calling.pay.stop");
action_subclass!(TranscribeAction, "calling.transcribe.stop");
action_subclass!(AIAction, "calling.ai.stop");

// ======================================================================
// Tests
// ======================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_event(event_type: &str, state: &str, control_id: &str) -> Event {
        let mut params = HashMap::new();
        params.insert("state".to_string(), json!(state));
        params.insert("control_id".to_string(), json!(control_id));
        params.insert("call_id".to_string(), json!("c1"));
        Event::new(event_type, params, 1.0)
    }

    #[test]
    fn test_action_new() {
        let a = Action::new("ctrl-1", "call-1", "node-1");
        assert_eq!(a.control_id(), "ctrl-1");
        assert_eq!(a.call_id(), "call-1");
        assert_eq!(a.node_id(), "node-1");
        assert!(!a.is_done());
        assert!(a.state().is_none());
        assert!(a.result().is_none());
    }

    #[test]
    fn test_handle_event_updates_state() {
        let a = Action::new("ctrl", "call", "node");
        let ev = make_event("calling.call.play", "playing", "ctrl");
        a.handle_event(&ev);
        assert_eq!(a.state(), Some("playing".to_string()));
        assert_eq!(a.events().len(), 1);
    }

    #[test]
    fn test_handle_event_merges_payload() {
        let a = Action::new("ctrl", "call", "node");
        let mut params = HashMap::new();
        params.insert("url".to_string(), json!("http://example.com/rec.wav"));
        params.insert("duration".to_string(), json!(10.5));
        let ev = Event::new("calling.call.record", params, 1.0);
        a.handle_event(&ev);

        let p = a.payload();
        assert_eq!(p.get("url").unwrap(), "http://example.com/rec.wav");
        assert_eq!(p.get("duration").unwrap().as_f64(), Some(10.5));
    }

    #[test]
    fn test_resolve() {
        let a = Action::new("ctrl", "call", "node");
        assert!(!a.is_done());
        a.resolve(Some(json!({"ok": true})));
        assert!(a.is_done());
        assert_eq!(a.result().unwrap()["ok"], true);
    }

    #[test]
    fn test_resolve_idempotent() {
        let a = Action::new("ctrl", "call", "node");
        a.resolve(Some(json!(1)));
        a.resolve(Some(json!(2)));
        assert_eq!(a.result().unwrap(), json!(1));
    }

    #[test]
    fn test_on_completed_fires() {
        let a = Arc::new(Action::new("ctrl", "call", "node"));
        let flag = Arc::new(Mutex::new(false));
        let flag2 = flag.clone();
        a.on_completed(move |_action| {
            *flag2.lock().unwrap() = true;
        });
        a.resolve(None);
        assert!(*flag.lock().unwrap());
    }

    #[test]
    fn test_on_completed_fires_if_already_done() {
        let a = Arc::new(Action::new("ctrl", "call", "node"));
        a.resolve(None);
        let flag = Arc::new(Mutex::new(false));
        let flag2 = flag.clone();
        a.on_completed(move |_| {
            *flag2.lock().unwrap() = true;
        });
        assert!(*flag.lock().unwrap());
    }

    #[test]
    fn test_stop_sends_command() {
        let a = Action::with_stop_method("ctrl", "call", "node", "calling.play.stop");
        a.stop();
        let cmds = a.sent_commands.lock().unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].0, "calling.play.stop");
        assert_eq!(cmds[0].1["control_id"], "ctrl");
    }

    #[test]
    fn test_stop_noop_when_empty() {
        let a = Action::new("ctrl", "call", "node");
        a.stop();
        assert!(a.sent_commands.lock().unwrap().is_empty());
    }

    #[test]
    fn test_execute_subcommand() {
        let a = Action::new("ctrl", "call", "node");
        let mut extra = HashMap::new();
        extra.insert("volume".to_string(), json!(-4.0));
        a.execute_subcommand("calling.play.volume", extra);
        let cmds = a.sent_commands.lock().unwrap();
        assert_eq!(cmds[0].1["volume"], json!(-4.0));
    }

    #[test]
    fn test_notify_sender() {
        let a = Action::new("ctrl", "call", "node");
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        a.set_notify_sender(tx);
        a.resolve(None);
        assert!(rx.recv().is_ok());
    }

    // -- PlayAction tests --

    #[test]
    fn test_play_action_stop_method() {
        let pa = PlayAction::new("ctrl", "call", "node");
        assert_eq!(pa.stop_method(), "calling.play.stop");
    }

    #[test]
    fn test_play_action_pause_resume_volume() {
        let pa = PlayAction::new("ctrl", "call", "node");
        pa.pause();
        pa.resume();
        pa.volume(-3.5);
        let cmds = pa.sent_commands.lock().unwrap();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].0, "calling.play.pause");
        assert_eq!(cmds[1].0, "calling.play.resume");
        assert_eq!(cmds[2].0, "calling.play.volume");
        assert_eq!(cmds[2].1["volume"], json!(-3.5));
    }

    // -- RecordAction tests --

    #[test]
    fn test_record_action_accessors() {
        let ra = RecordAction::new("ctrl", "call", "node");
        let mut params = HashMap::new();
        params.insert("url".to_string(), json!("http://rec.wav"));
        params.insert("duration".to_string(), json!(12.5));
        params.insert("size".to_string(), json!(1024));
        let ev = Event::new("calling.call.record", params, 1.0);
        ra.handle_event(&ev);

        assert_eq!(ra.url(), Some("http://rec.wav".to_string()));
        assert_eq!(ra.duration(), Some(12.5));
        assert_eq!(ra.size(), Some(1024));
    }

    #[test]
    fn test_record_action_pause_resume() {
        let ra = RecordAction::new("ctrl", "call", "node");
        ra.pause();
        ra.resume();
        let cmds = ra.sent_commands.lock().unwrap();
        assert_eq!(cmds[0].0, "calling.record.pause");
        assert_eq!(cmds[1].0, "calling.record.resume");
    }

    // -- CollectAction tests --

    #[test]
    fn test_collect_action_ignores_play_events() {
        let ca = CollectAction::new("ctrl", "call", "node");
        let play_ev = make_event("calling.call.play", "playing", "ctrl");
        ca.handle_event_filtered(&play_ev);
        assert!(ca.events().is_empty());

        let collect_ev = make_event("calling.call.collect", "collecting", "ctrl");
        ca.handle_event_filtered(&collect_ev);
        assert_eq!(ca.events().len(), 1);
    }

    #[test]
    fn test_collect_action_start_input_timers() {
        let ca = CollectAction::new("ctrl", "call", "node");
        ca.start_input_timers();
        let cmds = ca.sent_commands.lock().unwrap();
        assert_eq!(cmds[0].0, "calling.collect.start_input_timers");
    }

    #[test]
    fn test_collect_result() {
        let ca = CollectAction::new("ctrl", "call", "node");
        let mut params = HashMap::new();
        params.insert("result".to_string(), json!({"digits": "1234"}));
        let ev = Event::new("calling.call.collect", params, 1.0);
        ca.handle_event(&ev);
        assert_eq!(ca.collect_result().unwrap()["digits"], "1234");
    }

    // -- DetectAction tests --

    #[test]
    fn test_detect_action_result() {
        let da = DetectAction::new("ctrl", "call", "node");
        let mut params = HashMap::new();
        params.insert("detect".to_string(), json!({"type": "machine"}));
        let ev = Event::new("calling.call.detect", params, 1.0);
        da.handle_event(&ev);
        assert_eq!(da.detect_result().unwrap()["type"], "machine");
    }

    // -- FaxAction tests --

    #[test]
    fn test_fax_action_send() {
        let fa = FaxAction::new("ctrl", "call", "node", "send");
        assert_eq!(fa.fax_type(), "send");
        assert_eq!(fa.stop_method(), "calling.send_fax.stop");
    }

    #[test]
    fn test_fax_action_receive() {
        let fa = FaxAction::new("ctrl", "call", "node", "receive");
        assert_eq!(fa.fax_type(), "receive");
        assert_eq!(fa.stop_method(), "calling.receive_fax.stop");
    }

    // -- Other action subclasses --

    #[test]
    fn test_tap_action() {
        let a = TapAction::new("ctrl", "call", "node");
        assert_eq!(a.stop_method(), "calling.tap.stop");
    }

    #[test]
    fn test_stream_action() {
        let a = StreamAction::new("ctrl", "call", "node");
        assert_eq!(a.stop_method(), "calling.stream.stop");
    }

    #[test]
    fn test_pay_action() {
        let a = PayAction::new("ctrl", "call", "node");
        assert_eq!(a.stop_method(), "calling.pay.stop");
    }

    #[test]
    fn test_transcribe_action() {
        let a = TranscribeAction::new("ctrl", "call", "node");
        assert_eq!(a.stop_method(), "calling.transcribe.stop");
    }

    #[test]
    fn test_ai_action() {
        let a = AIAction::new("ctrl", "call", "node");
        assert_eq!(a.stop_method(), "calling.ai.stop");
    }

    #[test]
    fn test_deref_on_subclass() {
        let pa = PlayAction::new("ctrl", "call", "node");
        // Access Action methods through Deref
        assert_eq!(pa.control_id(), "ctrl");
        assert!(!pa.is_done());
        pa.resolve(None);
        assert!(pa.is_done());
    }
}
