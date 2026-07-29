use std::collections::HashMap;

/// A RELAY event received from the SignalWire server.
///
/// Events carry an `event_type` (e.g. `"calling.call.state"`), a
/// timestamp, and a bag of string-keyed parameters.
///
/// Field names (`event_type`, …) mirror the RELAY wire / Python field names
/// 1:1 — `event_type` is itself a JSON key — so the `struct_field_names`
/// lint is suppressed rather than dropping the `event_` prefix and diverging
/// from the wire shape.
#[allow(clippy::struct_field_names)]
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
            // Python parity: event timestamps are float seconds. The i64
            // millisecond count only loses precision past 2^52 ms (year ~144000),
            // so the f64 cast is exact for any real timestamp.
            #[allow(clippy::cast_precision_loss)]
            let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
            now
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
            Some(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            None => HashMap::new(),
        };
        Self::new(event_type, params, 0.0)
    }

    // ------------------------------------------------------------------
    // Accessors
    // ------------------------------------------------------------------

    /// The RELAY event type this event was dispatched under, e.g.
    /// `"calling.call.state"` or `"messaging.state"`. This is the key the
    /// client's handler registry routes on.
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    /// Event time as float seconds since the Unix epoch, matching Python's
    /// float-seconds timestamps. Set from the wire when the server supplied
    /// one, otherwise stamped locally at construction.
    pub fn timestamp(&self) -> f64 {
        self.timestamp
    }

    /// The raw `params` object from the wire, unmodified. The typed
    /// accessors below read well-known keys out of this map; anything the
    /// SDK does not model is still reachable here.
    pub fn params(&self) -> &HashMap<String, serde_json::Value> {
        &self.params
    }

    /// The `call_id` this event pertains to, if the params carry one.
    ///
    /// `call_id` is one of RELAY's four correlation mechanisms: it ties an
    /// event to a specific call leg for the life of that leg. `None` for
    /// events that are not call-scoped (or if the key is not a string).
    pub fn call_id(&self) -> Option<&str> {
        self.params.get("call_id").and_then(|v| v.as_str())
    }

    /// The `node_id` of the media node handling this call, if present.
    ///
    /// The node id must be echoed back on subsequent call-control requests
    /// so the request is routed to the node that owns the leg.
    pub fn node_id(&self) -> Option<&str> {
        self.params.get("node_id").and_then(|v| v.as_str())
    }

    /// The `control_id` of the in-flight action this event reports on, if
    /// present.
    ///
    /// A `control_id` is generated per call-control command (play, record,
    /// collect, …) and correlates every subsequent update for that one
    /// action, distinguishing concurrent actions on the same call.
    pub fn control_id(&self) -> Option<&str> {
        self.params.get("control_id").and_then(|v| v.as_str())
    }

    /// The caller-supplied `tag` echoed back by the server, if present.
    ///
    /// A tag correlates a server response with the local request that
    /// caused it before a server-assigned id is known.
    pub fn tag(&self) -> Option<&str> {
        self.params.get("tag").and_then(|v| v.as_str())
    }

    /// The `state` string carried by this event, if present.
    ///
    /// Which vocabulary applies depends on [`event_type`](Event::event_type)
    /// — call lifecycle, dial outcome, or message delivery. See
    /// [`constants`](super::constants) for the raw values and
    /// [`state_enums`](super::state_enums) for the typed views.
    pub fn state(&self) -> Option<&str> {
        self.params.get("state").and_then(|v| v.as_str())
    }

    /// Serialize back to a JSON-compatible map.
    #[must_use]
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::json!({
            "event_type": self.event_type,
            "timestamp": self.timestamp,
            "params": self.params,
        })
    }
}

// ------------------------------------------------------------------
// Typed RELAY event layer (parity with Python's per-event subclasses)
// ------------------------------------------------------------------
//
// Python models each RELAY notification as a `RelayEvent` subclass
// (CallStateEvent, PlayEvent, …) built via a `from_payload(payload)`
// classmethod. Rust mirrors this with a base `RelayEvent` newtype over
// the generic `Event` plus one thin typed wrapper per event family. Each
// `from_payload` extracts the notification's `params` object and wraps it,
// carrying the wire `event_type` so downstream dispatch stays type-agnostic.

use serde_json::Value;

/// Extract `(event_type, params)` from a RELAY notification payload.
///
/// RELAY notifications arrive as `{ "event_type": "...", "params": {...} }`
/// (sometimes nested under `params.params`). This normalizes both shapes to
/// the inner params object plus the declared `event_type`.
fn split_payload(payload: &Value) -> (String, HashMap<String, Value>) {
    let event_type = payload
        .get("event_type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let params_value = payload.get("params").unwrap_or(payload);
    let params = params_value
        .as_object()
        .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    (event_type, params)
}

/// Read a string field from an event's params map, defaulting to `""`
/// (Python's `p.get(<key>, "")` on the typed-event decoders).
fn str_field(params: &HashMap<String, Value>, key: &str) -> String {
    params
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Read an integer field from an event's params map, defaulting to `0`
/// (Python's `p.get(<key>, 0)`).
fn int_field(params: &HashMap<String, Value>, key: &str) -> i64 {
    params.get(key).and_then(Value::as_i64).unwrap_or(0)
}

/// Read a float field from an event's params map, defaulting to `0.0`
/// (Python's `p.get(<key>, 0.0)`).
fn float_field(params: &HashMap<String, Value>, key: &str) -> f64 {
    params.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

/// Read a boolean field from an event's params map, defaulting to `false`
/// (Python's `p.get(<key>, False)`).
fn bool_field(params: &HashMap<String, Value>, key: &str) -> bool {
    params.get(key).and_then(Value::as_bool).unwrap_or(false)
}

/// Read an object field from an event's params map as a JSON `Value`,
/// defaulting to an empty object (Python's `p.get(<key>, {})`).
fn dict_field(params: &HashMap<String, Value>, key: &str) -> Value {
    params
        .get(key)
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()))
}

/// Read a string-array field from an event's params map, defaulting to an
/// empty list (Python's `p.get(<key>, [])`). Non-string array elements are
/// skipped.
fn str_list_field(params: &HashMap<String, Value>, key: &str) -> Vec<String> {
    params
        .get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Base RELAY event — a typed view over the generic [`Event`].
///
/// Concrete
/// event families ([`CallStateEvent`], [`PlayEvent`], …) wrap this and are
/// produced by their `from_payload` constructor.
#[derive(Debug, Clone)]
pub struct RelayEvent {
    inner: Event,
}

impl RelayEvent {
    /// Build a [`RelayEvent`] from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        let (event_type, params) = split_payload(payload);
        RelayEvent {
            inner: Event::new(&event_type, params, 0.0),
        }
    }

    /// The underlying generic [`Event`].
    #[must_use]
    pub fn event(&self) -> &Event {
        &self.inner
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.inner.event_type()
    }

    /// The typed-event class name this payload dispatches to, mirroring
    /// Python's `EVENT_CLASS_MAP` (`type(parse_event(payload)).__name__`).
    /// Unmapped event types fall back to `"RelayEvent"`.
    #[must_use]
    pub fn class_name(&self) -> &'static str {
        match self.inner.event_type() {
            "calling.call.state" => "CallStateEvent",
            "calling.call.receive" => "CallReceiveEvent",
            "calling.call.play" => "PlayEvent",
            "calling.call.record" => "RecordEvent",
            "calling.call.collect" => "CollectEvent",
            "calling.call.connect" => "ConnectEvent",
            "calling.call.detect" => "DetectEvent",
            "calling.call.fax" => "FaxEvent",
            "calling.call.tap" => "TapEvent",
            "calling.call.stream" => "StreamEvent",
            "calling.call.send_digits" => "SendDigitsEvent",
            "calling.call.dial" => "DialEvent",
            "calling.call.refer" => "ReferEvent",
            "calling.call.denoise" => "DenoiseEvent",
            "calling.call.pay" => "PayEvent",
            "calling.call.queue" => "QueueEvent",
            "calling.call.echo" => "EchoEvent",
            "calling.call.transcribe" => "TranscribeEvent",
            "calling.call.hold" => "HoldEvent",
            "calling.conference" => "ConferenceEvent",
            "calling.error" => "CallingErrorEvent",
            "messaging.receive" => "MessageReceiveEvent",
            "messaging.state" => "MessageStateEvent",
            _ => "RelayEvent",
        }
    }

    /// The `call_id` from params (default `""`).
    #[must_use]
    pub fn call_id(&self) -> String {
        str_field(self.inner.params(), "call_id")
    }

    /// The `call_state` from params (default `""`), for `calling.call.state`.
    #[must_use]
    pub fn call_state(&self) -> String {
        str_field(self.inner.params(), "call_state")
    }

    /// The `direction` from params (default `""`).
    #[must_use]
    pub fn direction(&self) -> String {
        str_field(self.inner.params(), "direction")
    }

    /// The raw event params map (Python `RelayEvent.params`).
    #[must_use]
    pub fn params(&self) -> &HashMap<String, Value> {
        self.inner.params()
    }

    /// The event `timestamp` in float seconds (Python `RelayEvent.timestamp`).
    #[must_use]
    pub fn timestamp(&self) -> f64 {
        self.inner.timestamp()
    }
}

/// `calling.call.receive` — inbound call received.
///
/// Built via
/// [`CallReceiveEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct CallReceiveEvent {
    base: RelayEvent,
}

impl CallReceiveEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        CallReceiveEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }

    /// The `call_state`.
    #[must_use]
    pub fn call_state(&self) -> String {
        str_field(self.base.event().params(), "call_state")
    }

    /// The call `direction`.
    #[must_use]
    pub fn direction(&self) -> String {
        str_field(self.base.event().params(), "direction")
    }

    /// The `device` object.
    #[must_use]
    pub fn device(&self) -> Value {
        dict_field(self.base.event().params(), "device")
    }

    /// The `node_id`.
    #[must_use]
    pub fn node_id(&self) -> String {
        str_field(self.base.event().params(), "node_id")
    }

    /// The `project_id`.
    #[must_use]
    pub fn project_id(&self) -> String {
        str_field(self.base.event().params(), "project_id")
    }

    /// The `context` (falling back to the wire `protocol` field, per Python).
    #[must_use]
    pub fn context(&self) -> String {
        let params = self.base.event().params();
        params
            .get("context")
            .or_else(|| params.get("protocol"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    /// The `segment_id`.
    #[must_use]
    pub fn segment_id(&self) -> String {
        str_field(self.base.event().params(), "segment_id")
    }

    /// The `tag`.
    #[must_use]
    pub fn tag(&self) -> String {
        str_field(self.base.event().params(), "tag")
    }
}

/// `calling.call.state` — call state transition.
///
/// Built via
/// [`CallStateEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct CallStateEvent {
    base: RelayEvent,
}

impl CallStateEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        CallStateEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }

    /// The `call_id` from the event params.
    #[must_use]
    pub fn call_id(&self) -> String {
        str_field(self.base.event().params(), "call_id")
    }

    /// The `call_state` from the event params.
    #[must_use]
    pub fn call_state(&self) -> String {
        str_field(self.base.event().params(), "call_state")
    }

    /// The `direction` from the event params.
    #[must_use]
    pub fn direction(&self) -> String {
        str_field(self.base.event().params(), "direction")
    }

    /// The `end_reason` from the event params.
    #[must_use]
    pub fn end_reason(&self) -> String {
        str_field(self.base.event().params(), "end_reason")
    }

    /// The `device` object.
    #[must_use]
    pub fn device(&self) -> Value {
        dict_field(self.base.event().params(), "device")
    }
}

/// A `calling.*` error notification.
///
/// Built via
/// [`CallingErrorEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct CallingErrorEvent {
    base: RelayEvent,
}

impl CallingErrorEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        CallingErrorEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }

    /// The error `code`.
    #[must_use]
    pub fn code(&self) -> String {
        str_field(self.base.event().params(), "code")
    }

    /// The error `message`.
    #[must_use]
    pub fn message(&self) -> String {
        str_field(self.base.event().params(), "message")
    }
}

/// `calling.call.collect` — digit/speech collection result.
///
/// Built via
/// [`CollectEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct CollectEvent {
    base: RelayEvent,
}

impl CollectEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        CollectEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }

    /// The `control_id`.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The collection `state`.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }

    /// The `result` object.
    #[must_use]
    pub fn result(&self) -> Value {
        self.base
            .event()
            .params()
            .get("result")
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()))
    }

    /// The tri-state `final` flag (
    /// `bool | None`). `None` when absent.
    #[must_use]
    pub fn is_final(&self) -> Option<bool> {
        self.base
            .event()
            .params()
            .get("final")
            .and_then(Value::as_bool)
    }
}

/// A conference-related notification.
///
/// Built via
/// [`ConferenceEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct ConferenceEvent {
    base: RelayEvent,
}

impl ConferenceEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        ConferenceEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `conference_id` field.
    #[must_use]
    pub fn conference_id(&self) -> String {
        str_field(self.base.event().params(), "conference_id")
    }

    /// The `name` field.
    #[must_use]
    pub fn name(&self) -> String {
        str_field(self.base.event().params(), "name")
    }

    /// The `status` field.
    #[must_use]
    pub fn status(&self) -> String {
        str_field(self.base.event().params(), "status")
    }
}

/// `calling.call.connect` — call connect result.
///
/// Built via
/// [`ConnectEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct ConnectEvent {
    base: RelayEvent,
}

impl ConnectEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        ConnectEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `connect_state` field.
    #[must_use]
    pub fn connect_state(&self) -> String {
        str_field(self.base.event().params(), "connect_state")
    }

    /// The `peer` field.
    #[must_use]
    pub fn peer(&self) -> Value {
        dict_field(self.base.event().params(), "peer")
    }
}

/// `calling.call.denoise` — noise-reduction state.
///
/// Built via
/// [`DenoiseEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct DenoiseEvent {
    base: RelayEvent,
}

impl DenoiseEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        DenoiseEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `denoised` field.
    #[must_use]
    pub fn denoised(&self) -> bool {
        bool_field(self.base.event().params(), "denoised")
    }
}

/// `calling.call.detect` — detector result.
///
/// Built via
/// [`DetectEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct DetectEvent {
    base: RelayEvent,
}

impl DetectEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        DetectEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `control_id` field.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The `detect` field.
    #[must_use]
    pub fn detect(&self) -> Value {
        dict_field(self.base.event().params(), "detect")
    }
}

/// A dial result notification.
///
/// Built via
/// [`DialEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct DialEvent {
    base: RelayEvent,
}

impl DialEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        DialEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `tag` field.
    #[must_use]
    pub fn tag(&self) -> String {
        str_field(self.base.event().params(), "tag")
    }

    /// The `dial_state` field.
    #[must_use]
    pub fn dial_state(&self) -> String {
        str_field(self.base.event().params(), "dial_state")
    }

    /// The `call` field.
    #[must_use]
    pub fn call(&self) -> Value {
        dict_field(self.base.event().params(), "call")
    }
}

/// `calling.call.echo` — echo-command notification.
///
/// Built via
/// [`EchoEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct EchoEvent {
    base: RelayEvent,
}

impl EchoEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        EchoEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `state` field.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }
}

/// `calling.call.fax` — fax send/receive notification.
///
/// Built via
/// [`FaxEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct FaxEvent {
    base: RelayEvent,
}

impl FaxEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        FaxEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `control_id` field.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The `fax` field.
    #[must_use]
    pub fn fax(&self) -> Value {
        dict_field(self.base.event().params(), "fax")
    }
}

/// A hold/unhold notification.
///
/// Built via
/// [`HoldEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct HoldEvent {
    base: RelayEvent,
}

impl HoldEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        HoldEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `state` field.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }
}

/// `messaging.receive` — inbound message received.
///
/// Built via
/// [`MessageReceiveEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct MessageReceiveEvent {
    base: RelayEvent,
}

impl MessageReceiveEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        MessageReceiveEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `message_id` field.
    #[must_use]
    pub fn message_id(&self) -> String {
        str_field(self.base.event().params(), "message_id")
    }

    /// The `context` field.
    #[must_use]
    pub fn context(&self) -> String {
        str_field(self.base.event().params(), "context")
    }

    /// The `direction` field.
    #[must_use]
    pub fn direction(&self) -> String {
        str_field(self.base.event().params(), "direction")
    }

    /// The `from_number` field.
    #[must_use]
    pub fn from_number(&self) -> String {
        str_field(self.base.event().params(), "from_number")
    }

    /// The `to_number` field.
    #[must_use]
    pub fn to_number(&self) -> String {
        str_field(self.base.event().params(), "to_number")
    }

    /// The `body` field.
    #[must_use]
    pub fn body(&self) -> String {
        str_field(self.base.event().params(), "body")
    }

    /// The `media` field.
    #[must_use]
    pub fn media(&self) -> Vec<String> {
        str_list_field(self.base.event().params(), "media")
    }

    /// The `segments` field.
    #[must_use]
    pub fn segments(&self) -> i64 {
        int_field(self.base.event().params(), "segments")
    }

    /// The `message_state` field.
    #[must_use]
    pub fn message_state(&self) -> String {
        str_field(self.base.event().params(), "message_state")
    }

    /// The `tags` field.
    #[must_use]
    pub fn tags(&self) -> Vec<String> {
        str_list_field(self.base.event().params(), "tags")
    }
}

/// `messaging.state` — message state transition.
///
/// Built via
/// [`MessageStateEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct MessageStateEvent {
    base: RelayEvent,
}

impl MessageStateEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        MessageStateEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `message_id` field.
    #[must_use]
    pub fn message_id(&self) -> String {
        str_field(self.base.event().params(), "message_id")
    }

    /// The `context` field.
    #[must_use]
    pub fn context(&self) -> String {
        str_field(self.base.event().params(), "context")
    }

    /// The `direction` field.
    #[must_use]
    pub fn direction(&self) -> String {
        str_field(self.base.event().params(), "direction")
    }

    /// The `from_number` field.
    #[must_use]
    pub fn from_number(&self) -> String {
        str_field(self.base.event().params(), "from_number")
    }

    /// The `to_number` field.
    #[must_use]
    pub fn to_number(&self) -> String {
        str_field(self.base.event().params(), "to_number")
    }

    /// The `body` field.
    #[must_use]
    pub fn body(&self) -> String {
        str_field(self.base.event().params(), "body")
    }

    /// The `media` field.
    #[must_use]
    pub fn media(&self) -> Vec<String> {
        str_list_field(self.base.event().params(), "media")
    }

    /// The `segments` field.
    #[must_use]
    pub fn segments(&self) -> i64 {
        int_field(self.base.event().params(), "segments")
    }

    /// The `message_state` field.
    #[must_use]
    pub fn message_state(&self) -> String {
        str_field(self.base.event().params(), "message_state")
    }

    /// The `reason` field.
    #[must_use]
    pub fn reason(&self) -> String {
        str_field(self.base.event().params(), "reason")
    }

    /// The `tags` field.
    #[must_use]
    pub fn tags(&self) -> Vec<String> {
        str_list_field(self.base.event().params(), "tags")
    }
}

/// `calling.call.pay` — pay-command notification.
///
/// Built via
/// [`PayEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct PayEvent {
    base: RelayEvent,
}

impl PayEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        PayEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `control_id` field.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The `state` field.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }
}

/// `calling.call.play` — playback notification.
///
/// Built via
/// [`PlayEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct PlayEvent {
    base: RelayEvent,
}

impl PlayEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        PlayEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `control_id` field.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The `state` field.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }
}

/// A queue notification.
///
/// Built via
/// [`QueueEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct QueueEvent {
    base: RelayEvent,
}

impl QueueEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        QueueEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }

    /// The `control_id`.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The queue `status`.
    #[must_use]
    pub fn status(&self) -> String {
        str_field(self.base.event().params(), "status")
    }

    /// The queue id.
    #[must_use]
    pub fn queue_id(&self) -> String {
        str_field(self.base.event().params(), "id")
    }

    /// The queue name.
    #[must_use]
    pub fn queue_name(&self) -> String {
        str_field(self.base.event().params(), "name")
    }

    /// The caller's position in the queue.
    #[must_use]
    pub fn position(&self) -> i64 {
        self.base
            .event()
            .params()
            .get("position")
            .and_then(Value::as_i64)
            .unwrap_or(0)
    }

    /// The queue size.
    #[must_use]
    pub fn size(&self) -> i64 {
        self.base
            .event()
            .params()
            .get("size")
            .and_then(Value::as_i64)
            .unwrap_or(0)
    }
}

/// `calling.call.record` — recording notification.
///
/// Built via
/// [`RecordEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct RecordEvent {
    base: RelayEvent,
}

impl RecordEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        RecordEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }

    /// The `control_id`.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The recording `state`.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }

    /// The recording `url`.
    #[must_use]
    pub fn url(&self) -> String {
        self.record_field("url")
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    /// The recording `duration`.
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.record_field("duration")
            .as_ref()
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    }

    /// The recording `size` in bytes.
    #[must_use]
    pub fn size(&self) -> i64 {
        self.record_field("size")
            .as_ref()
            .and_then(Value::as_i64)
            .unwrap_or(0)
    }

    /// The nested `record` object.
    #[must_use]
    pub fn record(&self) -> Value {
        dict_field(self.base.event().params(), "record")
    }

    /// Resolve a field with the `RecordEvent` fallback: the nested `record`
    /// object's key, else the flat top-level key.
    fn record_field(&self, key: &str) -> Option<Value> {
        let params = self.base.event().params();
        params
            .get("record")
            .and_then(|r| r.get(key))
            .or_else(|| params.get(key))
            .cloned()
    }
}

/// `calling.call.refer` — refer notification.
///
/// Built via
/// [`ReferEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct ReferEvent {
    base: RelayEvent,
}

impl ReferEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        ReferEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `state` field.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }

    /// The `sip_refer_to` field.
    #[must_use]
    pub fn sip_refer_to(&self) -> String {
        str_field(self.base.event().params(), "sip_refer_to")
    }

    /// The `sip_refer_response_code` field.
    #[must_use]
    pub fn sip_refer_response_code(&self) -> String {
        str_field(self.base.event().params(), "sip_refer_response_code")
    }

    /// The `sip_notify_response_code` field.
    #[must_use]
    pub fn sip_notify_response_code(&self) -> String {
        str_field(self.base.event().params(), "sip_notify_response_code")
    }
}

/// `calling.call.send_digits` — send-digits notification.
///
/// Built via
/// [`SendDigitsEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct SendDigitsEvent {
    base: RelayEvent,
}

impl SendDigitsEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        SendDigitsEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `control_id` field.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The `state` field.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }
}

/// `calling.call.stream` — media-stream notification.
///
/// Built via
/// [`StreamEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct StreamEvent {
    base: RelayEvent,
}

impl StreamEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        StreamEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `control_id` field.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The `state` field.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }

    /// The `url` field.
    #[must_use]
    pub fn url(&self) -> String {
        str_field(self.base.event().params(), "url")
    }

    /// The `name` field.
    #[must_use]
    pub fn name(&self) -> String {
        str_field(self.base.event().params(), "name")
    }
}

/// `calling.call.tap` — media-tap notification.
///
/// Built via
/// [`TapEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct TapEvent {
    base: RelayEvent,
}

impl TapEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        TapEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `control_id` field.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The `state` field.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }

    /// The `tap` field.
    #[must_use]
    pub fn tap(&self) -> Value {
        dict_field(self.base.event().params(), "tap")
    }

    /// The `device` field.
    #[must_use]
    pub fn device(&self) -> Value {
        dict_field(self.base.event().params(), "device")
    }
}

/// `calling.call.transcribe` — live-transcription notification.
///
/// Built via
/// [`TranscribeEvent::from_payload`].
#[derive(Debug, Clone)]
pub struct TranscribeEvent {
    base: RelayEvent,
}

impl TranscribeEvent {
    /// Build this typed event from a raw RELAY notification payload.
    #[must_use]
    pub fn from_payload(payload: &Value) -> Self {
        TranscribeEvent {
            base: RelayEvent::from_payload(payload),
        }
    }

    /// The base [`RelayEvent`] view.
    #[must_use]
    pub fn base(&self) -> &RelayEvent {
        &self.base
    }

    /// The wire `event_type` string.
    #[must_use]
    pub fn event_type(&self) -> &str {
        self.base.event_type()
    }
    /// The `control_id` field.
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The `state` field.
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }

    /// The `url` field.
    #[must_use]
    pub fn url(&self) -> String {
        str_field(self.base.event().params(), "url")
    }

    /// The `recording_id` field.
    #[must_use]
    pub fn recording_id(&self) -> String {
        str_field(self.base.event().params(), "recording_id")
    }

    /// The `duration` field.
    #[must_use]
    pub fn duration(&self) -> f64 {
        float_field(self.base.event().params(), "duration")
    }

    /// The `size` field.
    #[must_use]
    pub fn size(&self) -> i64 {
        int_field(self.base.event().params(), "size")
    }
}

/// Parse a raw RELAY notification payload into a [`RelayEvent`].
///
/// The concrete event family is determined by the wire `event_type`; the
/// returned base carries it for downstream dispatch.
#[must_use]
pub fn parse_event(payload: &Value) -> RelayEvent {
    RelayEvent::from_payload(payload)
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
        let dbg = format!("{ev:?}");
        assert!(dbg.contains("test.event"));
    }

    #[test]
    fn test_relay_event_from_payload_nested_params() {
        let payload = json!({
            "event_type": "calling.call.state",
            "params": {"call_id": "c1", "state": "answered"}
        });
        let ev = RelayEvent::from_payload(&payload);
        assert_eq!(ev.event_type(), "calling.call.state");
        assert_eq!(ev.event().call_id(), Some("c1"));
        assert_eq!(ev.event().state(), Some("answered"));
    }

    #[test]
    fn test_relay_event_from_payload_flat() {
        let payload = json!({"event_type": "x", "call_id": "c2"});
        let ev = RelayEvent::from_payload(&payload);
        // Flat payload with no "params" key falls back to the payload itself.
        assert_eq!(ev.event().call_id(), Some("c2"));
    }

    #[test]
    fn test_typed_events_from_payload() {
        let payload = json!({
            "event_type": "calling.call.play",
            "params": {"call_id": "cid", "control_id": "pl-1"}
        });
        let ev = PlayEvent::from_payload(&payload);
        assert_eq!(ev.event_type(), "calling.call.play");
        assert_eq!(ev.base().event().control_id(), Some("pl-1"));

        let st = CallStateEvent::from_payload(&payload);
        assert_eq!(st.base().event().call_id(), Some("cid"));
    }

    #[test]
    fn test_parse_event_function() {
        let payload = json!({"event_type": "messaging.receive", "params": {"tag": "t9"}});
        let ev = parse_event(&payload);
        assert_eq!(ev.event_type(), "messaging.receive");
        assert_eq!(ev.event().tag(), Some("t9"));
    }
}
