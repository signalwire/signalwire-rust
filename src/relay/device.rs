//! Typed RELAY device object.
//!
//! The `{ "type": <string>, "params": <object> }` device object recurs as a
//! raw JSON blob across `calling.dial` / `calling.connect` / `calling.refer` /
//! `calling.tap` (and inside the serial/parallel device *matrix*
//! `[[device, …], …]`). [`Device`] types the **shape** of that blob while
//! deliberately keeping `type` a plain [`String`]: the discriminant
//! (`phone` / `sip` / `webrtc` / `rtp` / …) is **not** schema-enumerated in
//! `relay-protocol/calling.{dial,connect,refer,tap}.params.json` (each pins
//! only `{type: string, params}`), so an enum would be inventing a closed set
//! the contract does not declare.
//!
//! This is **additive**: every existing raw-`serde_json::Value` entry point
//! (`Call::connect`, `Call::refer_call`, `Call::tap`,
//! `Client::dial_blocking`) is unchanged. [`Device::to_value`] serialises to
//! the **byte-identical** wire object the hand-written `json!({...})` produces
//! (field order `type` then `params`, `params` always an object), so a typed
//! device drops straight into any of those raw-`Value` parameters.

use serde_json::{Map, Value};

/// A typed RELAY device descriptor (`{type, params}`).
///
/// `type` is the device-kind discriminant (kept as a `String` — the wire
/// contract does not enumerate it). `params` is the device-specific
/// parameter object (e.g. `{to_number, from_number}` for a `phone`).
///
/// Build one with [`Device::new`] (raw params) or [`Device::phone`] /
/// [`Device::sip`] convenience constructors, then hand its
/// [`to_value`](Device::to_value) to any device-taking call:
///
/// ```ignore
/// let dev = Device::phone("+15551112222", "+15553334444");
/// // single parallel leg = one inner list
/// let devices = Device::matrix(&[&[dev]]);
/// client.dial_blocking(devices, None, None, timeout);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    /// Device kind discriminant (`phone`, `sip`, `webrtc`, `rtp`, …). A
    /// plain string: the wire schema does not enumerate it.
    pub device_type: String,
    /// Device-specific parameters object.
    pub params: Value,
}

impl Device {
    /// Build a device from its `type` discriminant and a `params` object.
    ///
    /// A non-object `params` (or [`Value::Null`]) is normalised to an empty
    /// object on the wire, matching the hand-written `"params": {}` form.
    #[must_use]
    pub fn new(device_type: impl Into<String>, params: Value) -> Self {
        Device {
            device_type: device_type.into(),
            params,
        }
    }

    /// A `phone` device with `to_number` / `from_number` params.
    ///
    /// Emits `{"type":"phone","params":{"to_number":…,"from_number":…}}` —
    /// the shape `Client::dial_blocking` and `Call::connect` expect for PSTN
    /// legs.
    #[must_use]
    pub fn phone(to_number: impl Into<String>, from_number: impl Into<String>) -> Self {
        Device::new(
            "phone",
            Value::Object({
                let mut m = Map::new();
                m.insert("to_number".to_string(), Value::String(to_number.into()));
                m.insert("from_number".to_string(), Value::String(from_number.into()));
                m
            }),
        )
    }

    /// A `sip` device addressed to a SIP URI.
    ///
    /// Emits `{"type":"sip","params":{"to":…,"from":…}}`.
    #[must_use]
    pub fn sip(to: impl Into<String>, from: impl Into<String>) -> Self {
        Device::new(
            "sip",
            Value::Object({
                let mut m = Map::new();
                m.insert("to".to_string(), Value::String(to.into()));
                m.insert("from".to_string(), Value::String(from.into()));
                m
            }),
        )
    }

    /// Serialise to the wire device object: `{"type": …, "params": {…}}`.
    ///
    /// Field order is `type` then `params` and `params` is always an object,
    /// so the output is byte-identical to the hand-written
    /// `json!({"type": t, "params": p})` that every existing device call
    /// site uses. (`serde_json` is built with `preserve_order`, so insertion
    /// order is the serialised order.)
    #[must_use]
    pub fn to_value(&self) -> Value {
        let params = match &self.params {
            Value::Object(_) => self.params.clone(),
            _ => Value::Object(Map::new()),
        };
        let mut obj = Map::new();
        obj.insert("type".to_string(), Value::String(self.device_type.clone()));
        obj.insert("params".to_string(), params);
        Value::Object(obj)
    }

    /// Build the serial/parallel device **matrix** (`[[device, …], …]`) that
    /// `dial` / `connect` take, from rows of devices.
    ///
    /// Each inner slice is one *serial* attempt; the outer slice runs its
    /// rows in *parallel*. `Device::matrix(&[&[a, b]])` = try `a` then `b`
    /// in one parallel leg; `Device::matrix(&[&[a], &[b]])` = ring `a` and
    /// `b` in parallel.
    #[must_use]
    pub fn matrix(rows: &[&[Device]]) -> Value {
        Value::Array(
            rows.iter()
                .map(|row| Value::Array(row.iter().map(Device::to_value).collect()))
                .collect(),
        )
    }
}

// ----------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn to_value_is_byte_identical_to_handwritten() {
        let dev = Device::phone("+15551112222", "+15553334444");
        let hand = json!({
            "type": "phone",
            "params": {"to_number": "+15551112222", "from_number": "+15553334444"},
        });
        // Structural equality.
        assert_eq!(dev.to_value(), hand);
        // Byte-identical serialisation (field order preserved).
        assert_eq!(
            serde_json::to_string(&dev.to_value()).unwrap(),
            serde_json::to_string(&hand).unwrap(),
        );
        assert_eq!(
            serde_json::to_string(&dev.to_value()).unwrap(),
            r#"{"type":"phone","params":{"to_number":"+15551112222","from_number":"+15553334444"}}"#,
        );
    }

    #[test]
    fn sip_shape() {
        let dev = Device::sip("sip:bob@example.com", "sip:alice@example.com");
        assert_eq!(
            dev.to_value(),
            json!({
                "type": "sip",
                "params": {"to": "sip:bob@example.com", "from": "sip:alice@example.com"},
            }),
        );
    }

    #[test]
    fn new_with_raw_params_round_trips() {
        let dev = Device::new("rtp", json!({"addr": "203.0.113.1", "port": 4000}));
        let hand = json!({"type": "rtp", "params": {"addr": "203.0.113.1", "port": 4000}});
        assert_eq!(dev.to_value(), hand);
        assert_eq!(
            serde_json::to_string(&dev.to_value()).unwrap(),
            serde_json::to_string(&hand).unwrap(),
        );
    }

    #[test]
    fn non_object_params_normalised_to_empty_object() {
        // A device with no params must still emit "params": {} (the form the
        // tap/connect schemas accept and existing call sites hand-write).
        let dev = Device::new("phone", Value::Null);
        assert_eq!(dev.to_value(), json!({"type": "phone", "params": {}}));
    }

    #[test]
    fn matrix_single_parallel_leg() {
        let a = Device::phone("+1", "+2");
        let m = Device::matrix(&[std::slice::from_ref(&a)]);
        let hand = json!([[{"type": "phone", "params": {"to_number": "+1", "from_number": "+2"}}]]);
        assert_eq!(m, hand);
        assert_eq!(
            serde_json::to_string(&m).unwrap(),
            serde_json::to_string(&hand).unwrap(),
        );
    }

    #[test]
    fn matrix_serial_and_parallel() {
        let a = Device::phone("+1", "+2");
        let b = Device::sip("sip:b", "sip:a");
        // one parallel leg, two serial devices
        let serial = Device::matrix(&[&[a.clone(), b.clone()]]);
        assert_eq!(serial.as_array().unwrap().len(), 1);
        assert_eq!(serial[0].as_array().unwrap().len(), 2);
        // two parallel legs, one device each
        let parallel = Device::matrix(&[&[a], &[b]]);
        assert_eq!(parallel.as_array().unwrap().len(), 2);
        assert_eq!(parallel[0].as_array().unwrap().len(), 1);
    }
}
