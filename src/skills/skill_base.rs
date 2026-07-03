use serde_json::{Map, Value};

use crate::agent::AgentBase;

/// The base JSON-schema documenting the configuration parameters every skill
/// accepts via `SkillBase` (SWAIG-field merge, prompt-skip, tool-name
/// override). Skills that add their own parameters build on top of this.
///
/// `pub(crate)` so per-skill `get_parameter_schema` overrides can reuse it
/// without duplicating the shared controls; crate-internal → not public API.
pub(crate) fn default_parameter_schema() -> Value {
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

/// Trait implemented by all skills (both builtin and custom).
///
/// A skill encapsulates tools, hints, global data, and prompt sections that can
/// be loaded into an `AgentBase` via the `SkillManager`.
pub trait SkillBase: Send + Sync {
    /// Unique `snake_case` name of this skill (e.g. `"datetime"`).
    fn name(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Semantic version string.
    fn version(&self) -> &'static str {
        "1.0.0"
    }

    /// Environment variables that must be set before `setup` is called.
    fn required_env_vars(&self) -> Vec<String> {
        Vec::new()
    }

    /// Packages this skill depends on.
    ///
    /// Mirrors Python's `REQUIRED_PACKAGES` class attribute. In Python the
    /// list drives a runtime `importlib` availability check
    /// (`validate_packages`). Rust is compiled and its dependencies are
    /// resolved by Cargo at build time, so a runtime "is the package
    /// importable" probe has no equivalent — anything a skill needs is
    /// linked into the binary. The declared list is retained so the
    /// surface matches Python and so tooling / GUIs can display a skill's
    /// declared dependencies; [`validate_packages`](Self::validate_packages)
    /// consumes it below.
    fn required_packages(&self) -> Vec<String> {
        Vec::new()
    }

    /// Whether multiple instances of this skill can be loaded simultaneously.
    fn supports_multiple_instances(&self) -> bool {
        false
    }

    /// Instance key used to track loaded skills (allows `tool_name` overrides).
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

    /// Build this skill's SWAIG tool definitions without registering them.
    ///
    /// Mirrors Python's `get_tools()` — the DataMap-style skills
    /// (`api_ninjas_trivia`, `play_background_file`, `weather_api`) override
    /// this to return their fully-formed tool definitions (each a JSON
    /// object with `function`/`argument`/`data_map` keys). `register_tools`
    /// then iterates the returned list, merges `swaig_fields`, and registers
    /// each. Skills that register handler-backed tools directly (via
    /// `AgentBase::define_tool`) leave this at the default empty list.
    fn get_tools(&self) -> Vec<Value> {
        Vec::new()
    }

    /// Define a tool on the agent, automatically merging this skill's
    /// `swaig_fields`.
    ///
    /// Mirrors Python's `SkillBase.define_tool(**kwargs)` wrapper: skills
    /// call this instead of `agent.define_tool(...)` so that any
    /// `swaig_fields` configured for the instance are folded into the tool
    /// definition. `swaig_fields` are applied first; the explicit arguments
    /// take precedence.
    fn define_tool(
        &self,
        agent: &mut AgentBase,
        name: &str,
        description: &str,
        parameters: Value,
        handler: crate::swml::service::FunctionHandler,
        secure: bool,
    ) {
        agent.define_tool(name, description, parameters, handler, secure);
        // swaig_fields are metadata merged into the registered definition.
        let swaig_fields = self.get_swaig_fields();
        if !swaig_fields.is_empty() {
            agent.merge_swaig_fields(name, &swaig_fields);
        }
    }

    /// Namespaced key under which this skill instance stores state in the
    /// agent's `global_data` (mirrors Python's `_get_skill_namespace`).
    ///
    /// Uses the `prefix` param when present, else the instance key, so
    /// multiple instances of a multi-instance skill don't collide.
    fn get_skill_namespace(&self) -> String {
        if let Some(prefix) = self.params().get("prefix").and_then(|v| v.as_str()) {
            format!("skill:{prefix}")
        } else {
            format!("skill:{}", self.get_instance_key())
        }
    }

    /// Read this skill instance's namespaced state out of the `raw_data`
    /// passed to a SWAIG handler (mirrors Python's `get_skill_data`).
    ///
    /// `raw_data` is expected to contain a `global_data` object; the skill's
    /// slice lives under [`get_skill_namespace`](Self::get_skill_namespace).
    /// Returns an empty map when absent.
    fn get_skill_data(&self, raw_data: &Value) -> Map<String, Value> {
        let namespace = self.get_skill_namespace();
        raw_data
            .get("global_data")
            .and_then(|g| g.get(&namespace))
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default()
    }

    /// Write this skill instance's namespaced state into a `FunctionResult`
    /// (mirrors Python's `update_skill_data`).
    ///
    /// Wraps `data` under the skill's namespace key and appends a
    /// `set_global_data` action to `result` for chaining.
    fn update_skill_data(
        &self,
        result: &mut crate::swaig::FunctionResult,
        data: Map<String, Value>,
    ) {
        let namespace = self.get_skill_namespace();
        let mut wrapper = Map::new();
        wrapper.insert(namespace, Value::Object(data));
        result.update_global_data(Value::Object(wrapper));
    }

    /// Validate that this skill's declared packages are available.
    ///
    /// Mirrors Python's `validate_packages`, which runtime-imports each
    /// `REQUIRED_PACKAGES` entry. Rust has no runtime import step — Cargo
    /// resolves and links every dependency at build time, so anything a
    /// skill's [`required_packages`](Self::required_packages) declares is
    /// guaranteed present in a binary that compiled. This therefore always
    /// returns `true`; it exists to keep the surface aligned with Python and
    /// to give skills (e.g. `wikipedia_search`) the same call site.
    fn validate_packages(&self) -> bool {
        true
    }

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
        if self
            .params()
            .get("skip_prompt")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
        {
            return Vec::new();
        }
        Vec::new()
    }

    /// JSON-Schema describing accepted parameters.
    fn get_parameter_schema(&self) -> Value {
        default_parameter_schema()
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
        self.params
            .get(key)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }

    /// Like [`get_bool`](Self::get_bool) but returns `default` when the key is
    /// absent (or not a boolean). Needed for params whose documented default
    /// is `true` — `get_bool` always falls back to `false`.
    pub fn get_bool_or(&self, key: &str, default: bool) -> bool {
        self.params
            .get(key)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(default)
    }

    pub fn get_i64(&self, key: &str, default: i64) -> i64 {
        self.params
            .get(key)
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(default)
    }

    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        self.params
            .get(key)
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(default)
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

    /// Minimal skill for exercising the base-trait helper methods.
    struct TestSkill {
        sp: SkillParams,
    }

    impl SkillBase for TestSkill {
        fn name(&self) -> &'static str {
            "test_skill"
        }
        fn description(&self) -> &'static str {
            "a test skill"
        }
        fn supports_multiple_instances(&self) -> bool {
            true
        }
        fn setup(&mut self) -> bool {
            true
        }
        fn register_tools(&self, _agent: &mut AgentBase) {}
        fn params(&self) -> &Map<String, Value> {
            &self.sp.params
        }
    }

    fn test_skill(params: Value) -> TestSkill {
        TestSkill {
            sp: SkillParams::new(value_to_map(params)),
        }
    }

    #[test]
    fn test_get_tools_default_empty() {
        let s = test_skill(serde_json::json!({}));
        assert!(s.get_tools().is_empty());
    }

    #[test]
    fn test_validate_packages_always_true() {
        let s = test_skill(serde_json::json!({}));
        assert!(s.validate_packages());
    }

    #[test]
    fn test_get_skill_namespace_prefix_and_instance_key() {
        // With an explicit prefix.
        let s = test_skill(serde_json::json!({"prefix": "myns"}));
        assert_eq!(s.get_skill_namespace(), "skill:myns");
        // Without a prefix -> derived from the instance key (name + tool_name).
        let s2 = test_skill(serde_json::json!({"tool_name": "foo"}));
        assert_eq!(s2.get_skill_namespace(), "skill:test_skill_foo");
    }

    #[test]
    fn test_get_skill_data_reads_namespaced_slice() {
        let s = test_skill(serde_json::json!({"prefix": "acct"}));
        let raw = serde_json::json!({
            "global_data": {
                "skill:acct": {"count": 3},
                "other": {"x": 1},
            }
        });
        let data = s.get_skill_data(&raw);
        assert_eq!(data.get("count"), Some(&serde_json::json!(3)));
        // Missing namespace -> empty map.
        let empty = test_skill(serde_json::json!({"prefix": "none"}));
        assert!(empty.get_skill_data(&raw).is_empty());
    }

    #[test]
    fn test_update_skill_data_wraps_under_namespace() {
        let s = test_skill(serde_json::json!({"prefix": "acct"}));
        let mut result = crate::swaig::FunctionResult::new();
        let mut data = Map::new();
        data.insert("count".to_string(), serde_json::json!(7));
        s.update_skill_data(&mut result, data);
        // The set_global_data action nests the data under skill:acct.
        let v = result.to_value();
        let action = &v["action"][0]["set_global_data"];
        assert_eq!(action["skill:acct"]["count"], serde_json::json!(7));
    }

    #[test]
    fn test_define_tool_merges_swaig_fields() {
        use crate::agent::{AgentBase, AgentOptions};
        let s = test_skill(serde_json::json!({
            "swaig_fields": {"fillers": {"en-US": ["one moment"]}}
        }));
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        s.define_tool(
            &mut agent,
            "my_tool",
            "does a thing",
            serde_json::json!({}),
            Box::new(|_a, _r| crate::swaig::FunctionResult::new()),
            false,
        );
        let def = &agent.get_function("my_tool").unwrap().definition;
        // swaig_fields folded into the registered definition.
        assert_eq!(def["fillers"]["en-US"], serde_json::json!(["one moment"]));
        assert_eq!(def["function"], serde_json::json!("my_tool"));
    }
}
