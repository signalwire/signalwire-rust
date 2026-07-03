//! SWAIG function — a tool the AI model can call.
//!
//! Port of Python `signalwire.core.swaig_function.SWAIGFunction`. Represents a
//! single tool (exactly the OpenAI/Anthropic "function tool" concept): a name,
//! an LLM-facing description, a JSON-Schema parameter object, and a handler
//! invoked when the model calls it. Rendered into the SWAIG array via
//! [`SwaigFunction::to_swaig`].
//!
//! The Rust struct is named `SwaigFunction`; the enumerator's class-rename
//! table folds it onto the reference `SWAIGFunction`.

use std::sync::Arc;

use serde_json::{Map, Value, json};

use crate::swaig::function_result::FunctionResult;

/// Handler invoked when the model calls this tool. Receives `(args,
/// raw_data)` and returns a [`FunctionResult`]. Matches the shape used by
/// [`crate::swml::service::Service::define_tool`].
pub type SwaigHandler =
    Arc<dyn Fn(&Map<String, Value>, &Map<String, Value>) -> FunctionResult + Send + Sync>;

/// A SWAIG function — a tool the AI model can call.
#[derive(Clone)]
pub struct SwaigFunction {
    name: String,
    handler: SwaigHandler,
    description: String,
    parameters: Value,
    secure: bool,
    fillers: Option<Value>,
    wait_file: Option<String>,
    wait_file_loops: Option<i64>,
    webhook_url: Option<String>,
    required: Vec<String>,
    extra_swaig_fields: Map<String, Value>,
}

impl SwaigFunction {
    /// Initialize a new SWAIG function. Python parity:
    /// `SWAIGFunction.__init__`.
    ///
    /// `parameters` is the JSON-Schema parameters object (or the raw
    /// properties map, wrapped on demand by [`SwaigFunction::to_swaig`]).
    #[must_use]
    pub fn new(name: &str, handler: SwaigHandler, description: &str, parameters: Value) -> Self {
        SwaigFunction {
            name: name.to_string(),
            handler,
            description: description.to_string(),
            parameters,
            secure: false,
            fillers: None,
            wait_file: None,
            wait_file_loops: None,
            webhook_url: None,
            required: Vec::new(),
            extra_swaig_fields: Map::new(),
        }
    }

    /// Set whether the function requires SWAIG token validation.
    #[must_use]
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Set the required parameter names.
    #[must_use]
    pub fn required(mut self, required: Vec<String>) -> Self {
        self.required = required;
        self
    }

    /// Set an external webhook URL (marks the function external).
    #[must_use]
    pub fn webhook_url(mut self, url: &str) -> Self {
        self.webhook_url = Some(url.to_string());
        self
    }

    /// Set filler phrases keyed by language code.
    #[must_use]
    pub fn fillers(mut self, fillers: Value) -> Self {
        self.fillers = Some(fillers);
        self
    }

    /// Set a wait-file URL and loop count.
    #[must_use]
    pub fn wait_file(mut self, url: &str, loops: Option<i64>) -> Self {
        self.wait_file = Some(url.to_string());
        self.wait_file_loops = loops;
        self
    }

    /// Add an extra SWAIG-only field (e.g. `meta_data_token`).
    #[must_use]
    pub fn extra_field(mut self, key: &str, value: Value) -> Self {
        self.extra_swaig_fields.insert(key.to_string(), value);
        self
    }

    /// The function name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether an external webhook URL is configured.
    #[must_use]
    pub fn is_external(&self) -> bool {
        self.webhook_url.is_some()
    }

    /// Whether the function requires token validation.
    #[must_use]
    pub fn is_secure(&self) -> bool {
        self.secure
    }

    /// Call the underlying handler directly. Python parity:
    /// `SWAIGFunction.__call__` (the enumerator maps Rust `call` → the
    /// reference `__call__` dunder).
    pub fn call(&self, args: &Map<String, Value>, raw_data: &Map<String, Value>) -> FunctionResult {
        (self.handler)(args, raw_data)
    }

    /// Execute the function and return its result as a value (from
    /// [`FunctionResult::to_value`]). Python parity: `SWAIGFunction.execute`.
    #[must_use]
    pub fn execute(
        &self,
        args: &Map<String, Value>,
        raw_data: Option<&Map<String, Value>>,
    ) -> Value {
        let empty = Map::new();
        let raw = raw_data.unwrap_or(&empty);
        (self.handler)(args, raw).to_value()
    }

    /// Validate `args` against the parameter schema. Returns
    /// `(is_valid, errors)`. The Rust port performs a lightweight
    /// required-key check (there is no runtime JSON-Schema validator wired in);
    /// this mirrors the reference's "skip when no validator is available"
    /// fallback, tightened to enforce the `required` list. Python parity:
    /// `SWAIGFunction.validate_args`.
    #[must_use]
    pub fn validate_args(&self, args: &Map<String, Value>) -> (bool, Vec<String>) {
        let schema = self.ensure_parameter_structure();
        let props = schema.get("properties").and_then(Value::as_object);
        if props.is_none_or(serde_json::Map::is_empty) {
            return (true, Vec::new());
        }
        let mut errors = Vec::new();
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required {
                if let Some(k) = key.as_str()
                    && !args.contains_key(k)
                {
                    errors.push(format!("'{k}' is a required property"));
                }
            }
        }
        (errors.is_empty(), errors)
    }

    /// Convert this function to a SWAIG-compatible value for SWML. Python
    /// parity: `SWAIGFunction.to_swaig`.
    #[must_use]
    pub fn to_swaig(&self, base_url: &str, token: Option<&str>, call_id: Option<&str>) -> Value {
        let mut url = format!("{base_url}/swaig");
        if let (Some(t), Some(c)) = (token, call_id) {
            url = format!("{url}?token={t}&call_id={c}");
        }
        let mut def = Map::new();
        def.insert("function".to_string(), json!(self.name));
        def.insert("description".to_string(), json!(self.description));
        def.insert("parameters".to_string(), self.ensure_parameter_structure());
        def.insert("web_hook_url".to_string(), json!(url));
        if let Some(f) = &self.fillers
            && f.as_object().is_none_or(|o| !o.is_empty())
        {
            def.insert("fillers".to_string(), f.clone());
        }
        for (k, v) in &self.extra_swaig_fields {
            def.insert(k.clone(), v.clone());
        }
        Value::Object(def)
    }

    /// Wrap the raw parameters into the `{type, properties, [required]}`
    /// structure the SWML AI verb expects (Python `_ensure_parameter_structure`).
    fn ensure_parameter_structure(&self) -> Value {
        match &self.parameters {
            Value::Object(obj) if obj.is_empty() => json!({"type": "object", "properties": {}}),
            Value::Object(obj) if obj.contains_key("type") && obj.contains_key("properties") => {
                self.parameters.clone()
            }
            Value::Object(_) => {
                let mut result = Map::new();
                result.insert("type".to_string(), json!("object"));
                result.insert("properties".to_string(), self.parameters.clone());
                if !self.required.is_empty() {
                    result.insert("required".to_string(), json!(self.required));
                }
                Value::Object(result)
            }
            _ => json!({"type": "object", "properties": {}}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn echo_handler() -> SwaigHandler {
        Arc::new(|args: &Map<String, Value>, _raw| {
            let mut fr = FunctionResult::new();
            let text = args
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("done")
                .to_string();
            fr.set_response(&text);
            fr
        })
    }

    #[test]
    fn test_call_invokes_handler() {
        let f = SwaigFunction::new("echo", echo_handler(), "Echo text", json!({}));
        let mut args = Map::new();
        args.insert("text".to_string(), json!("hi"));
        let result = f.call(&args, &Map::new());
        assert_eq!(result.to_value()["response"], "hi");
    }

    #[test]
    fn test_execute_returns_value() {
        let f = SwaigFunction::new("echo", echo_handler(), "Echo", json!({}));
        let mut args = Map::new();
        args.insert("text".to_string(), json!("out"));
        let v = f.execute(&args, None);
        assert_eq!(v["response"], "out");
    }

    #[test]
    fn test_to_swaig_shape() {
        let params = json!({"type": "object", "properties": {"text": {"type": "string"}}});
        let f = SwaigFunction::new("echo", echo_handler(), "Echo text", params);
        let sw = f.to_swaig("https://x", None, None);
        assert_eq!(sw["function"], "echo");
        assert_eq!(sw["description"], "Echo text");
        assert_eq!(sw["web_hook_url"], "https://x/swaig");
        assert_eq!(sw["parameters"]["properties"]["text"]["type"], "string");
    }

    #[test]
    fn test_to_swaig_with_token_and_call_id() {
        let f = SwaigFunction::new("t", echo_handler(), "d", json!({}));
        let sw = f.to_swaig("https://x", Some("tok"), Some("cid"));
        assert_eq!(sw["web_hook_url"], "https://x/swaig?token=tok&call_id=cid");
    }

    #[test]
    fn test_to_swaig_wraps_bare_properties_with_required() {
        let props = json!({"text": {"type": "string"}});
        let f =
            SwaigFunction::new("t", echo_handler(), "d", props).required(vec!["text".to_string()]);
        let sw = f.to_swaig("https://x", None, None);
        assert_eq!(sw["parameters"]["type"], "object");
        assert_eq!(sw["parameters"]["required"], json!(["text"]));
    }

    #[test]
    fn test_validate_args_required_missing() {
        let props = json!({"text": {"type": "string"}});
        let f =
            SwaigFunction::new("t", echo_handler(), "d", props).required(vec!["text".to_string()]);
        let (ok, errs) = f.validate_args(&Map::new());
        assert!(!ok);
        assert!(errs[0].contains("required"));
    }

    #[test]
    fn test_validate_args_ok_when_no_properties() {
        let f = SwaigFunction::new("t", echo_handler(), "d", json!({}));
        let (ok, errs) = f.validate_args(&Map::new());
        assert!(ok);
        assert!(errs.is_empty());
    }

    #[test]
    fn test_external_and_secure_flags() {
        let f = SwaigFunction::new("t", echo_handler(), "d", json!({}))
            .secure(true)
            .webhook_url("https://ext");
        assert!(f.is_secure());
        assert!(f.is_external());
        assert_eq!(f.name(), "t");
    }
}
