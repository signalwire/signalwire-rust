//! Typed builder for SWAIG tool-parameter JSON schemas.
//!
//! [`AgentBase::define_tool`] / [`Service::define_tool`] take the function's
//! parameter schema as an untyped `serde_json::Value` — a `properties` object
//! hand-written as nested `json!({ ... })`:
//!
//! ```ignore
//! agent.define_tool(
//!     "search_faqs",
//!     "Search the FAQ knowledge base",
//!     json!({
//!         "query": {"type": "string", "description": "The question or keywords"},
//!     }),
//!     handler,
//!     false,
//! );
//! ```
//!
//! That is easy to typo (`"strign"`, `"desciption"`, a forgotten `"type"`) and
//! the compiler can't help. [`ParamsBuilder`] is an *additive*, fully-typed
//! convenience over the **exact same wire output**: it produces the identical
//! `properties` object — so it drops straight into the existing, unchanged
//! `define_tool` path. It is not a new format.
//!
//! ```no_run
//! use signalwire::swaig::ParamsBuilder;
//! use serde_json::json;
//!
//! let params = ParamsBuilder::new()
//!     .string("query", "The question or keywords to search")
//!     .build();
//!
//! // Byte-identical to the hand-written form above:
//! assert_eq!(
//!     params,
//!     json!({"query": {"type": "string", "description": "The question or keywords to search"}})
//! );
//! ```
//!
//! ## Two outputs, both byte-identical to the hand-written forms
//!
//! - [`build`](ParamsBuilder::build) returns the **`properties` object** — what
//!   Rust's `define_tool(parameters)` accepts (it wraps it as
//!   `{"type":"object","properties": <this>}`).
//! - [`build_schema`](ParamsBuilder::build_schema) returns the **full** JSON
//!   schema `{"type":"object","properties":{…},"required":[…]}` — exactly the
//!   shape the Python reference's `_ensure_parameter_structure` emits and the
//!   shape hand-written for `register_swaig_function` / `DataMap` full
//!   definitions. This is the form that carries a top-level
//!   [`required`](ParamsBuilder::required) list.
//!
//! ## Closed-set (enum) properties
//!
//! [`enum_of`](ParamsBuilder::enum_of) renders a schema `"enum": [...]` from any
//! iterator of `impl AsRef<str>`, so the Tier-1 media enums plug straight in via
//! their `all()` slices:
//!
//! ```no_run
//! use signalwire::swaig::{ParamsBuilder, RecordFormat};
//! use serde_json::json;
//!
//! let params = ParamsBuilder::new()
//!     .enum_of("fmt", RecordFormat::all(), "Recording container format")
//!     .build();
//!
//! assert_eq!(
//!     params,
//!     json!({"fmt": {"type": "string", "enum": ["wav", "mp3", "mp4"], "description": "Recording container format"}})
//! );
//! ```
//!
//! [`AgentBase::define_tool`]: crate::agent::AgentBase::define_tool
//! [`Service::define_tool`]: crate::swml::service::Service::define_tool

use serde_json::{Map, Value};

/// The JSON-Schema primitive types a SWAIG parameter property can declare.
///
/// Used by [`ParamsBuilder::array`] (the element kind) and, indirectly, by the
/// per-kind builder methods. The `as_str` value is the literal that lands in
/// the schema's `"type"` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[must_use]
pub enum ParamKind {
    /// `"string"`
    String,
    /// `"number"` (floating point)
    Number,
    /// `"integer"`
    Integer,
    /// `"boolean"`
    Boolean,
    /// `"array"`
    Array,
    /// `"object"`
    Object,
}

impl ParamKind {
    /// The canonical JSON-Schema `"type"` string for this kind.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ParamKind::String => "string",
            ParamKind::Number => "number",
            ParamKind::Integer => "integer",
            ParamKind::Boolean => "boolean",
            ParamKind::Array => "array",
            ParamKind::Object => "object",
        }
    }
}

impl std::fmt::Display for ParamKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single parameter property under construction.
///
/// You rarely build one of these directly — [`ParamsBuilder`]'s per-kind
/// methods create and insert one for you. Reach for [`PropertyBuilder`] only
/// when you need a property whose options (`default`, `format`, a property-local
/// `required` flag, a nested schema) don't fit the one-line helpers, then pass
/// it to [`ParamsBuilder::property`].
///
/// The rendered shape is a JSON object: `{"type": <kind>, "description": <desc>,
/// …optional keys…}`, byte-identical to what you'd hand-write.
#[derive(Debug, Clone)]
#[must_use]
pub struct PropertyBuilder {
    schema: Map<String, Value>,
}

impl PropertyBuilder {
    /// Start a property of `kind` with an LLM-facing `description`.
    ///
    /// `description` is prompt-engineering text the model reads to decide how to
    /// fill the argument — be specific (format, source, constraints), not terse.
    pub fn new(kind: ParamKind, description: &str) -> Self {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::from(kind.as_str()));
        schema.insert("description".to_string(), Value::from(description));
        PropertyBuilder { schema }
    }

    /// Mark this individual property required, emitting `"required": true`
    /// alongside its `type`/`description`.
    ///
    /// This is the per-property flag style some skills use
    /// (e.g. the `datasphere` skill). For the JSON-Schema top-level
    /// `"required": ["a","b"]` array, use [`ParamsBuilder::required`] instead.
    pub fn required(mut self, required: bool) -> Self {
        self.schema
            .insert("required".to_string(), Value::Bool(required));
        self
    }

    /// Attach a `"default"` value to the property.
    pub fn default(mut self, default: impl Into<Value>) -> Self {
        self.schema.insert("default".to_string(), default.into());
        self
    }

    /// Attach a JSON-Schema `"format"` hint (e.g. `"date"`, `"email"`,
    /// `"uri"`). Free-form: the format vocabulary is open, so this stays a
    /// `&str`.
    pub fn format(mut self, format: &str) -> Self {
        self.schema
            .insert("format".to_string(), Value::from(format));
        self
    }

    /// Constrain the property to a closed set, emitting
    /// `"enum": [<variant>, …]`.
    ///
    /// Accepts any iterator of `impl AsRef<str>`, so the Tier-1 media enums plug
    /// in via their `all()` slices (`RecordFormat::all()`, `Codec::all()`, …) —
    /// each variant renders as its canonical wire string.
    pub fn enum_values<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let variants: Vec<Value> = values
            .into_iter()
            .map(|v| Value::from(v.as_ref()))
            .collect();
        self.schema
            .insert("enum".to_string(), Value::Array(variants));
        self
    }

    /// Set the element schema for an array property, emitting
    /// `"items": {"type": <kind>}`.
    pub fn items(mut self, kind: ParamKind) -> Self {
        let mut items = Map::new();
        items.insert("type".to_string(), Value::from(kind.as_str()));
        self.schema
            .insert("items".to_string(), Value::Object(items));
        self
    }

    /// Set the nested `properties` for an object property from another
    /// [`ParamsBuilder`], emitting `"properties": {…}` (and a nested
    /// `"required": [...]` if the inner builder declared one).
    pub fn properties(mut self, inner: ParamsBuilder) -> Self {
        self.schema.insert(
            "properties".to_string(),
            Value::Object(inner.properties_map()),
        );
        if !inner.required.is_empty() {
            self.schema.insert(
                "required".to_string(),
                Value::Array(
                    inner
                        .required
                        .iter()
                        .map(|s| Value::from(s.as_str()))
                        .collect(),
                ),
            );
        }
        self
    }

    /// Insert an arbitrary extra schema key (escape hatch for JSON-Schema
    /// keywords without a dedicated helper, e.g. `"minimum"`, `"pattern"`).
    pub fn extra(mut self, key: &str, value: impl Into<Value>) -> Self {
        self.schema.insert(key.to_string(), value.into());
        self
    }

    /// Finish the property, yielding its rendered schema object.
    #[must_use]
    pub fn build(self) -> Value {
        Value::Object(self.schema)
    }

    fn into_map(self) -> Map<String, Value> {
        self.schema
    }
}

/// Fluent, typed builder for a SWAIG tool's parameter schema.
///
/// Build up the parameters one property at a time, then call
/// [`build`](Self::build) to get the `properties` object for `define_tool`, or
/// [`build_schema`](Self::build_schema) for the full
/// `{"type":"object","properties":{…},"required":[…]}` schema. See the
/// [module docs](self) for the full picture and byte-identity guarantees.
///
/// ```no_run
/// use signalwire::swaig::{ParamsBuilder, RecordFormat};
///
/// let params = ParamsBuilder::new()
///     .string("service", "The service the customer is asking about")
///     .string("date", "Appointment date, YYYY-MM-DD")
///     .integer("party_size", "Number of people")
///     .boolean("confirmed", "Whether the user confirmed")
///     .enum_of("fmt", RecordFormat::all(), "Recording container format")
///     .required(["service", "date"])
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct ParamsBuilder {
    properties: Map<String, Value>,
    required: Vec<String>,
}

impl ParamsBuilder {
    /// Start an empty parameter schema.
    pub fn new() -> Self {
        ParamsBuilder {
            properties: Map::new(),
            required: Vec::new(),
        }
    }

    /// Add a `string` property with `description`.
    pub fn string(self, name: &str, description: &str) -> Self {
        self.property(name, PropertyBuilder::new(ParamKind::String, description))
    }

    /// Add a `number` (floating-point) property with `description`.
    pub fn number(self, name: &str, description: &str) -> Self {
        self.property(name, PropertyBuilder::new(ParamKind::Number, description))
    }

    /// Add an `integer` property with `description`.
    pub fn integer(self, name: &str, description: &str) -> Self {
        self.property(name, PropertyBuilder::new(ParamKind::Integer, description))
    }

    /// Add a `boolean` property with `description`.
    pub fn boolean(self, name: &str, description: &str) -> Self {
        self.property(name, PropertyBuilder::new(ParamKind::Boolean, description))
    }

    /// Add a closed-set (`enum`) property with `description`.
    ///
    /// `variants` is any iterator of `impl AsRef<str>`, so the Tier-1 media
    /// enums plug in directly:
    ///
    /// ```no_run
    /// use signalwire::swaig::{ParamsBuilder, TapDirection};
    /// use serde_json::json;
    ///
    /// let params = ParamsBuilder::new()
    ///     .enum_of("direction", TapDirection::all(), "Audio direction to tap")
    ///     .build();
    /// assert_eq!(
    ///     params["direction"]["enum"],
    ///     json!(["speak", "hear", "both"])
    /// );
    /// ```
    pub fn enum_of<I, S>(self, name: &str, variants: I, description: &str) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.property(
            name,
            PropertyBuilder::new(ParamKind::String, description).enum_values(variants),
        )
    }

    /// Add an `array` property whose elements are `items` (a [`ParamKind`]),
    /// with `description`.
    pub fn array(self, name: &str, items: ParamKind, description: &str) -> Self {
        self.property(
            name,
            PropertyBuilder::new(ParamKind::Array, description).items(items),
        )
    }

    /// Add a nested `object` property whose shape is described by `inner`
    /// (another [`ParamsBuilder`]), with `description`.
    pub fn object(self, name: &str, inner: ParamsBuilder, description: &str) -> Self {
        self.property(
            name,
            PropertyBuilder::new(ParamKind::Object, description).properties(inner),
        )
    }

    /// Add a fully-customised property built via [`PropertyBuilder`] (the
    /// escape hatch for `default`/`format`/per-property `required`/nesting that
    /// the one-line helpers don't cover).
    pub fn property(mut self, name: &str, property: PropertyBuilder) -> Self {
        self.properties
            .insert(name.to_string(), Value::Object(property.into_map()));
        self
    }

    /// Declare the top-level required-parameter list, emitting
    /// `"required": [<name>, …]` in [`build_schema`](Self::build_schema).
    ///
    /// This is the JSON-Schema-style required array (sibling of `properties`),
    /// matching the Python reference's `required=[…]` argument. Calling it more
    /// than once replaces the previous list. For a per-property flag instead,
    /// use [`PropertyBuilder::required`].
    pub fn required<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.required = names.into_iter().map(Into::into).collect();
        self
    }

    /// Render just the **`properties` object** — the value Rust's
    /// `define_tool(parameters)` accepts (it wraps it as
    /// `{"type":"object","properties": <this>}`).
    ///
    /// Byte-identical to the hand-written `json!({ <name>: { … } })` form. The
    /// top-level [`required`](Self::required) list is **not** included here (it
    /// has no slot inside a bare `properties` object); use
    /// [`build_schema`](Self::build_schema) when you need it.
    #[must_use]
    pub fn build(self) -> Value {
        Value::Object(self.properties)
    }

    /// Render the **full JSON schema**:
    /// `{"type":"object","properties":{…}}`, plus `"required":[…]` when a
    /// top-level [`required`](Self::required) list was declared.
    ///
    /// Byte-identical to the Python reference's `_ensure_parameter_structure`
    /// output and to the hand-written full-schema forms used with
    /// `register_swaig_function` / `DataMap` definitions.
    #[must_use]
    pub fn build_schema(self) -> Value {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::from("object"));
        schema.insert("properties".to_string(), Value::Object(self.properties));
        if !self.required.is_empty() {
            schema.insert(
                "required".to_string(),
                Value::Array(
                    self.required
                        .iter()
                        .map(|s| Value::from(s.as_str()))
                        .collect(),
                ),
            );
        }
        Value::Object(schema)
    }

    fn properties_map(&self) -> Map<String, Value> {
        self.properties.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentBase, AgentOptions};
    use crate::swaig::{Codec, FunctionResult, RecordDirection, RecordFormat, TapDirection};
    use serde_json::json;
    use std::collections::HashMap;

    fn default_agent() -> AgentBase {
        AgentBase::new(AgentOptions::new("params_builder_test"))
    }

    // ── (a) byte-identical `properties` output across all property kinds ────

    #[test]
    fn test_builder_properties_byte_identical_to_handwritten_all_kinds() {
        // One property of every supported scalar/array/object kind, plus an
        // enum property — built typed.
        let built = ParamsBuilder::new()
            .string("service", "The service the customer is asking about")
            .number("amount", "Dollar amount, e.g. 19.99")
            .integer("party_size", "Number of people in the party")
            .boolean("confirmed", "Whether the user confirmed the booking")
            .enum_of("fmt", RecordFormat::all(), "Recording container format")
            .array("tags", ParamKind::String, "Free-form labels")
            .object(
                "address",
                ParamsBuilder::new()
                    .string("street", "Street line")
                    .string("zip", "Postal code"),
                "Mailing address",
            )
            .build();

        // The exact same thing, hand-written as nested json! — the form
        // define_tool() takes today.
        let handwritten = json!({
            "service": {"type": "string", "description": "The service the customer is asking about"},
            "amount": {"type": "number", "description": "Dollar amount, e.g. 19.99"},
            "party_size": {"type": "integer", "description": "Number of people in the party"},
            "confirmed": {"type": "boolean", "description": "Whether the user confirmed the booking"},
            "fmt": {
                "type": "string",
                "enum": ["wav", "mp3", "mp4"],
                "description": "Recording container format"
            },
            "tags": {
                "type": "array",
                "description": "Free-form labels",
                "items": {"type": "string"}
            },
            "address": {
                "type": "object",
                "description": "Mailing address",
                "properties": {
                    "street": {"type": "string", "description": "Street line"},
                    "zip": {"type": "string", "description": "Postal code"}
                }
            }
        });

        assert_eq!(built, handwritten);
    }

    #[test]
    fn test_builder_full_schema_byte_identical_with_top_level_required() {
        // build_schema() carries the top-level required array — byte-identical
        // to the hand-written full schema used with register_swaig_function /
        // DataMap (e.g. the datasphere_serverless skill's argument block).
        let built = ParamsBuilder::new()
            .string("service", "The service to book")
            .string("date", "Appointment date, YYYY-MM-DD")
            .required(["service", "date"])
            .build_schema();

        let handwritten = json!({
            "type": "object",
            "properties": {
                "service": {"type": "string", "description": "The service to book"},
                "date": {"type": "string", "description": "Appointment date, YYYY-MM-DD"}
            },
            "required": ["service", "date"]
        });

        assert_eq!(built, handwritten);
    }

    #[test]
    fn test_enum_property_byte_identical_for_every_tier1_media_enum() {
        // Each Tier-1 enum's all() slice renders to the exact wire-string list,
        // proving the enum integration is byte-for-byte what a hand-written
        // "enum": [...] would be — and that the three direction vocabularies
        // stay distinct (listen ≠ hear).
        let rec_fmt = ParamsBuilder::new()
            .enum_of("fmt", RecordFormat::all(), "format")
            .build();
        assert_eq!(
            rec_fmt,
            json!({"fmt": {"type": "string", "enum": ["wav", "mp3", "mp4"], "description": "format"}})
        );

        let rec_dir = ParamsBuilder::new()
            .enum_of("dir", RecordDirection::all(), "direction")
            .build();
        assert_eq!(
            rec_dir,
            json!({"dir": {"type": "string", "enum": ["speak", "listen", "both"], "description": "direction"}})
        );

        let tap_dir = ParamsBuilder::new()
            .enum_of("dir", TapDirection::all(), "direction")
            .build();
        assert_eq!(
            tap_dir,
            json!({"dir": {"type": "string", "enum": ["speak", "hear", "both"], "description": "direction"}})
        );

        let codec = ParamsBuilder::new()
            .enum_of("codec", Codec::all(), "codec")
            .build();
        assert_eq!(
            codec,
            json!({"codec": {"type": "string", "enum": ["PCMU", "PCMA"], "description": "codec"}})
        );

        // The record vocabulary's `listen` is NOT the tap vocabulary's `hear`.
        assert_ne!(rec_dir["dir"]["enum"], tap_dir["dir"]["enum"]);
    }

    #[test]
    fn test_property_builder_options_byte_identical() {
        // default / format / per-property required / array-items / nested
        // object via the PropertyBuilder escape hatch.
        let built = ParamsBuilder::new()
            .property(
                "format",
                PropertyBuilder::new(ParamKind::String, "Container format")
                    .enum_values(RecordFormat::all())
                    .default("wav"),
            )
            .property(
                "date",
                PropertyBuilder::new(ParamKind::String, "ISO date")
                    .format("date")
                    .required(true),
            )
            .build();

        let handwritten = json!({
            "format": {
                "type": "string",
                "description": "Container format",
                "enum": ["wav", "mp3", "mp4"],
                "default": "wav"
            },
            "date": {
                "type": "string",
                "description": "ISO date",
                "format": "date",
                "required": true
            }
        });

        assert_eq!(built, handwritten);
    }

    #[test]
    fn test_empty_builder_matches_handwritten_empty() {
        // The info_gatherer "start_questions" tool passes json!({}); the
        // builder's empty output is byte-identical.
        assert_eq!(ParamsBuilder::new().build(), json!({}));
        assert_eq!(
            ParamsBuilder::new().build_schema(),
            json!({"type": "object", "properties": {}})
        );
    }

    // ── (b) a REAL define_tool() with builder params → render → invoke ──────

    #[test]
    fn test_define_tool_with_builder_params_renders_into_swaig_json() {
        // Drive the REAL define_tool path with builder-built params, then
        // render the SWAIG block and assert the parameters appear verbatim.
        let built = ParamsBuilder::new()
            .string("query", "The question or keywords to search")
            .enum_of("fmt", RecordFormat::all(), "Recording container format")
            .integer("limit", "Max results")
            .build();

        // Capture the equivalent hand-written params for a side-by-side wire
        // comparison after rendering.
        let handwritten = json!({
            "query": {"type": "string", "description": "The question or keywords to search"},
            "fmt": {
                "type": "string",
                "enum": ["wav", "mp3", "mp4"],
                "description": "Recording container format"
            },
            "limit": {"type": "integer", "description": "Max results"}
        });
        assert_eq!(built, handwritten);

        let mut agent = default_agent();
        agent.manual_set_proxy_url("https://proxy.example.com");
        agent.define_tool(
            "search_faqs",
            "Search the FAQ knowledge base by keyword",
            built.clone(),
            Box::new(|args, _raw| {
                let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                FunctionResult::with_response(&format!("searched: {q}"))
            }),
            false,
        );

        // Render the AI verb and locate our function's argument schema.
        let ai = agent.build_ai_verb(&HashMap::new());
        let funcs = ai["SWAIG"]["functions"]
            .as_array()
            .expect("functions array");
        let func = funcs
            .iter()
            .find(|f| f["function"] == "search_faqs")
            .expect("search_faqs registered");

        // define_tool wraps `parameters` as argument.{type:object, properties}.
        assert_eq!(func["argument"]["type"], "object");
        // The builder-produced properties appear verbatim in the SWAIG JSON.
        assert_eq!(func["argument"]["properties"], built);
        // Spot-check the enum survived end-to-end into the rendered schema.
        assert_eq!(
            func["argument"]["properties"]["fmt"]["enum"],
            json!(["wav", "mp3", "mp4"])
        );
        assert_eq!(func["argument"]["properties"]["query"]["type"], "string");
        assert_eq!(func["argument"]["properties"]["limit"]["type"], "integer");

        // And it is byte-identical to rendering the hand-written params the
        // same way — typed builder is a pure convenience over identical wire.
        let mut agent_hw = default_agent();
        agent_hw.manual_set_proxy_url("https://proxy.example.com");
        agent_hw.define_tool(
            "search_faqs",
            "Search the FAQ knowledge base by keyword",
            handwritten,
            Box::new(|_args, _raw| FunctionResult::with_response("x")),
            false,
        );
        let ai_hw = agent_hw.build_ai_verb(&HashMap::new());
        let func_hw = ai_hw["SWAIG"]["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["function"] == "search_faqs")
            .unwrap();
        assert_eq!(func["argument"], func_hw["argument"]);
    }

    #[test]
    fn test_define_tool_with_builder_params_is_invocable() {
        // The registered tool actually dispatches — builder params don't
        // disturb handler wiring.
        let built = ParamsBuilder::new().string("query", "search text").build();

        let mut agent = default_agent();
        agent.define_tool(
            "search",
            "Search",
            built,
            Box::new(|args, _raw| {
                let q = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                FunctionResult::with_response(&format!("hit: {q}"))
            }),
            false,
        );

        let mut args = serde_json::Map::new();
        args.insert("query".to_string(), Value::from("pricing"));
        let raw = serde_json::Map::new();
        let result = agent
            .on_function_call("search", &args, &raw)
            .expect("handler dispatched");
        let v = result.to_value();
        assert_eq!(v["response"], "hit: pricing");
    }

    #[test]
    fn test_must_use_kinds_render_type_string() {
        assert_eq!(ParamKind::String.as_str(), "string");
        assert_eq!(ParamKind::Number.as_str(), "number");
        assert_eq!(ParamKind::Integer.as_str(), "integer");
        assert_eq!(ParamKind::Boolean.as_str(), "boolean");
        assert_eq!(ParamKind::Array.as_str(), "array");
        assert_eq!(ParamKind::Object.as_str(), "object");
        assert_eq!(ParamKind::Integer.to_string(), "integer");
    }
}
