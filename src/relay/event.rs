use std::collections::HashMap;

/// A RELAY event received from the SignalWire server.
///
/// Events carry an `event_type` (e.g. `"calling.call.state"`), a
/// timestamp, and a bag of string-keyed parameters.
// Field names (event_type, …) mirror the RELAY wire / Python field names 1:1;
// `event_type` is also a JSON key. struct_field_names would have us drop the
// `event_` prefix, which would diverge from the wire shape.
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
        self.params.get("call_id").and_then(|v| v.as_str())
    }

    pub fn node_id(&self) -> Option<&str> {
        self.params.get("node_id").and_then(|v| v.as_str())
    }

    pub fn control_id(&self) -> Option<&str> {
        self.params.get("control_id").and_then(|v| v.as_str())
    }

    pub fn tag(&self) -> Option<&str> {
        self.params.get("tag").and_then(|v| v.as_str())
    }

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

/// Base RELAY event — a typed view over the generic [`Event`].
///
/// Parity with Python's `signalwire.relay.event.RelayEvent`. Concrete
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
}

/// `calling.call.receive` — inbound call received.
///
/// Parity with Python's `signalwire.relay.event.CallReceiveEvent`; built via
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
}

/// `calling.call.state` — call state transition.
///
/// Parity with Python's `signalwire.relay.event.CallStateEvent`; built via
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

    /// The `call_id` from the event params (Python parity:
    /// `CallStateEvent.call_id`).
    #[must_use]
    pub fn call_id(&self) -> String {
        str_field(self.base.event().params(), "call_id")
    }

    /// The `call_state` from the event params (Python parity:
    /// `CallStateEvent.call_state`).
    #[must_use]
    pub fn call_state(&self) -> String {
        str_field(self.base.event().params(), "call_state")
    }

    /// The `direction` from the event params (Python parity:
    /// `CallStateEvent.direction`).
    #[must_use]
    pub fn direction(&self) -> String {
        str_field(self.base.event().params(), "direction")
    }
}

/// A `calling.*` error notification.
///
/// Parity with Python's `signalwire.relay.event.CallingErrorEvent`; built via
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
}

/// `calling.call.collect` — digit/speech collection result.
///
/// Parity with Python's `signalwire.relay.event.CollectEvent`; built via
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

    /// The `control_id` (Python parity: `CollectEvent.control_id`).
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The collection `state` (Python parity: `CollectEvent.state`).
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }

    /// The `result` object (Python parity: `CollectEvent.result`, default
    /// `{}`).
    #[must_use]
    pub fn result(&self) -> Value {
        self.base
            .event()
            .params()
            .get("result")
            .cloned()
            .unwrap_or_else(|| Value::Object(serde_json::Map::new()))
    }

    /// The tri-state `final` flag (Python parity: `CollectEvent.final`,
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
/// Parity with Python's `signalwire.relay.event.ConferenceEvent`; built via
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
}

/// `calling.call.connect` — call connect result.
///
/// Parity with Python's `signalwire.relay.event.ConnectEvent`; built via
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
}

/// `calling.call.denoise` — noise-reduction state.
///
/// Parity with Python's `signalwire.relay.event.DenoiseEvent`; built via
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
}

/// `calling.call.detect` — detector result.
///
/// Parity with Python's `signalwire.relay.event.DetectEvent`; built via
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
}

/// A dial result notification.
///
/// Parity with Python's `signalwire.relay.event.DialEvent`; built via
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
}

/// `calling.call.echo` — echo-command notification.
///
/// Parity with Python's `signalwire.relay.event.EchoEvent`; built via
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
}

/// `calling.call.fax` — fax send/receive notification.
///
/// Parity with Python's `signalwire.relay.event.FaxEvent`; built via
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
}

/// A hold/unhold notification.
///
/// Parity with Python's `signalwire.relay.event.HoldEvent`; built via
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
}

/// `messaging.receive` — inbound message received.
///
/// Parity with Python's `signalwire.relay.event.MessageReceiveEvent`; built via
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
}

/// `messaging.state` — message state transition.
///
/// Parity with Python's `signalwire.relay.event.MessageStateEvent`; built via
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
}

/// `calling.call.pay` — pay-command notification.
///
/// Parity with Python's `signalwire.relay.event.PayEvent`; built via
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
}

/// `calling.call.play` — playback notification.
///
/// Parity with Python's `signalwire.relay.event.PlayEvent`; built via
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
}

/// A queue notification.
///
/// Parity with Python's `signalwire.relay.event.QueueEvent`; built via
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

    /// The `control_id` (Python parity: `QueueEvent.control_id`).
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The queue `status` (Python parity: `QueueEvent.status`).
    #[must_use]
    pub fn status(&self) -> String {
        str_field(self.base.event().params(), "status")
    }

    /// The queue id (Python parity: `QueueEvent.queue_id`, RENAMED from the
    /// wire `id` field).
    #[must_use]
    pub fn queue_id(&self) -> String {
        str_field(self.base.event().params(), "id")
    }

    /// The queue name (Python parity: `QueueEvent.queue_name`, RENAMED from
    /// the wire `name` field).
    #[must_use]
    pub fn queue_name(&self) -> String {
        str_field(self.base.event().params(), "name")
    }

    /// The caller's position in the queue (Python parity:
    /// `QueueEvent.position`, default `0`).
    #[must_use]
    pub fn position(&self) -> i64 {
        self.base
            .event()
            .params()
            .get("position")
            .and_then(Value::as_i64)
            .unwrap_or(0)
    }

    /// The queue size (Python parity: `QueueEvent.size`, default `0`).
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
/// Parity with Python's `signalwire.relay.event.RecordEvent`; built via
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

    /// The `control_id` (Python parity: `RecordEvent.control_id`).
    #[must_use]
    pub fn control_id(&self) -> String {
        str_field(self.base.event().params(), "control_id")
    }

    /// The recording `state` (Python parity: `RecordEvent.state`).
    #[must_use]
    pub fn state(&self) -> String {
        str_field(self.base.event().params(), "state")
    }

    /// The recording `url` (Python parity: `RecordEvent.url` — nested
    /// `record.url` first, then a flat `url`, else `""`).
    #[must_use]
    pub fn url(&self) -> String {
        self.record_field("url")
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    }

    /// The recording `duration` (Python parity: `RecordEvent.duration` —
    /// nested `record.duration` first, then flat `duration`, else `0.0`).
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.record_field("duration")
            .as_ref()
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    }

    /// The recording `size` in bytes (Python parity: `RecordEvent.size` —
    /// nested `record.size` first, then flat `size`, else `0`).
    #[must_use]
    pub fn size(&self) -> i64 {
        self.record_field("size")
            .as_ref()
            .and_then(Value::as_i64)
            .unwrap_or(0)
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
/// Parity with Python's `signalwire.relay.event.ReferEvent`; built via
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
}

/// `calling.call.send_digits` — send-digits notification.
///
/// Parity with Python's `signalwire.relay.event.SendDigitsEvent`; built via
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
}

/// `calling.call.stream` — media-stream notification.
///
/// Parity with Python's `signalwire.relay.event.StreamEvent`; built via
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
}

/// `calling.call.tap` — media-tap notification.
///
/// Parity with Python's `signalwire.relay.event.TapEvent`; built via
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
}

/// `calling.call.transcribe` — live-transcription notification.
///
/// Parity with Python's `signalwire.relay.event.TranscribeEvent`; built via
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
}

/// Parse a raw RELAY notification payload into a [`RelayEvent`].
///
/// Parity with Python's module-level `signalwire.relay.event.parse_event`.
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
