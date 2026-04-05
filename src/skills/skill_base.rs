use serde_json::{Map, Value};

use crate::agent::AgentBase;

/// Trait implemented by all skills (both builtin and custom).
///
/// A skill encapsulates tools, hints, global data, and prompt sections that can
/// be loaded into an `AgentBase` via the `SkillManager`.
pub trait SkillBase: Send + Sync {
    /// Unique snake_case name of this skill (e.g. `"datetime"`).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Semantic version string.
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// Environment variables that must be set before `setup` is called.
    fn required_env_vars(&self) -> Vec<String> {
        Vec::new()
    }

    /// Whether multiple instances of this skill can be loaded simultaneously.
    fn supports_multiple_instances(&self) -> bool {
        false
    }

    /// Instance key used to track loaded skills (allows tool_name overrides).
    fn get_instance_key(&self) -> String {
        let mut key = self.name().to_string();
        if let Some(tn) = self.params().get("tool_name").and_then(|v| v.as_str()) {
            key.push('_');
            key.push_str(tn);
        }
        key
    }

    /// One-time setup. Return `true` on success.
    fn setup(&mut self) -> bool;

    /// Register tools on the agent.
    fn register_tools(&self, agent: &mut AgentBase);

    /// Speech recognition hints.
    fn get_hints(&self) -> Vec<String> {
        Vec::new()
    }

    /// Key/value pairs merged into the agent's global data.
    fn get_global_data(&self) -> Map<String, Value> {
        Map::new()
    }

    /// POM sections merged into the agent's prompt.
    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.params().get("skip_prompt").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Vec::new();
        }
        Vec::new()
    }

    /// JSON-Schema describing accepted parameters.
    fn get_parameter_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "swaig_fields": {
                    "type": "array",
                    "description": "Additional SWAIG fields to merge into tool definitions",
                    "default": [],
                },
                "skip_prompt": {
                    "type": "boolean",
                    "description": "If true, skip adding prompt sections for this skill",
                    "default": false,
                },
                "tool_name": {
                    "type": "string",
                    "description": "Custom tool name override for this skill instance",
                },
            },
        })
    }

    /// Called when the skill is unloaded.
    fn cleanup(&mut self) {}

    /// Access the skill's configuration parameters.
    fn params(&self) -> &Map<String, Value>;

    /// Validate that all required env vars are set. Returns missing var names.
    fn validate_env_vars(&self) -> Vec<String> {
        let mut missing = Vec::new();
        for var in self.required_env_vars() {
            if std::env::var(&var).unwrap_or_default().is_empty() {
                missing.push(var);
            }
        }
        missing
    }

    /// Get the tool name, falling back to `default` if no override is set.
    fn get_tool_name(&self, default: &str) -> String {
        self.params()
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    }

    /// Get the SWAIG fields to merge into tool definitions.
    fn get_swaig_fields(&self) -> Map<String, Value> {
        self.params()
            .get("swaig_fields")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default()
    }
}

/// Parameters holder used by the default `SkillBase` implementations.
#[derive(Debug, Clone)]
pub struct SkillParams {
    pub params: Map<String, Value>,
}

impl SkillParams {
    pub fn new(params: Map<String, Value>) -> Self {
        SkillParams { params }
    }

    pub fn empty() -> Self {
        SkillParams { params: Map::new() }
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.params.get(key).and_then(|v| v.as_str())
    }

    pub fn get_str_or(&self, key: &str, default: &str) -> String {
        self.get_str(key).unwrap_or(default).to_string()
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.params.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
    }

    pub fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.params.get(key).and_then(|v| v.as_i64()).unwrap_or(default)
    }

    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.params.get(key).and_then(|v| v.as_f64()).unwrap_or(default)
    }

    pub fn get_array(&self, key: &str) -> Vec<Value> {
        self.params
            .get(key)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_object(&self, key: &str) -> Map<String, Value> {
        self.params
            .get(key)
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default()
    }
}

/// Convert a `Value` (expected to be an object) into a `Map<String, Value>`.
pub fn value_to_map(val: Value) -> Map<String, Value> {
    match val {
        Value::Object(m) => m,
        _ => Map::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_params_get_str() {
        let mut m = Map::new();
        m.insert("name".to_string(), Value::String("test".to_string()));
        let p = SkillParams::new(m);
        assert_eq!(p.get_str("name"), Some("test"));
        assert_eq!(p.get_str("missing"), None);
    }

    #[test]
    fn test_skill_params_get_str_or() {
        let p = SkillParams::empty();
        assert_eq!(p.get_str_or("key", "default"), "default");
    }

    #[test]
    fn test_skill_params_get_bool() {
        let mut m = Map::new();
        m.insert("flag".to_string(), Value::Bool(true));
        let p = SkillParams::new(m);
        assert!(p.get_bool("flag"));
        assert!(!p.get_bool("missing"));
    }

    #[test]
    fn test_skill_params_get_i64() {
        let mut m = Map::new();
        m.insert("count".to_string(), serde_json::json!(42));
        let p = SkillParams::new(m);
        assert_eq!(p.get_i64("count", 0), 42);
        assert_eq!(p.get_i64("missing", 5), 5);
    }

    #[test]
    fn test_value_to_map_object() {
        let val = serde_json::json!({"a": 1});
        let map = value_to_map(val);
        assert_eq!(map.get("a").unwrap(), &serde_json::json!(1));
    }

    #[test]
    fn test_value_to_map_non_object() {
        let val = serde_json::json!(42);
        let map = value_to_map(val);
        assert!(map.is_empty());
    }
}
