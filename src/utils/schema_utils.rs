// schema_utils.rs — Rust port of signalwire.utils.schema_utils.SchemaUtils.
//
// Loads the SWML JSON Schema, extracts verb metadata, and validates
// either a single verb config or a complete SWML document.  Validation
// is lightweight (verb existence + required-property check) by default;
// full JSON Schema validation can be wired in via the `jsonschema`
// crate by extending `init_full_validator`.
//
// Construction rules mirror Python:
//
//   - Pass `schema_path = None` to use the embedded schema.json.
//   - `schema_validation = false` disables validation
//     (`validate_verb` returns `(true, [])` for every call).
//   - The env var `SWML_SKIP_SCHEMA_VALIDATION=1/true/yes` also
//     disables validation regardless of the constructor argument.

use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

/// `SchemaValidationError` — Rust port of
/// `signalwire.utils.schema_utils.SchemaValidationError`.
#[derive(Debug, Clone)]
pub struct SchemaValidationError {
    pub verb_name: String,
    pub errors: Vec<String>,
}

impl SchemaValidationError {
    /// Construct a `SchemaValidationError`. Mirrors Python's
    /// `SchemaValidationError(verb_name, errors)`.
    pub fn new(verb_name: String, errors: Vec<String>) -> Self {
        Self { verb_name, errors }
    }
}

impl std::fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Schema validation failed for '{}': {}",
            self.verb_name,
            self.errors.join("; ")
        )
    }
}

impl std::error::Error for SchemaValidationError {}

/// Verb metadata extracted from the schema.
#[derive(Debug, Clone)]
pub struct VerbDefinition {
    pub name: String,
    pub schema_name: String,
    pub definition: Value,
}

/// `SchemaUtils` — Rust port of
/// `signalwire.utils.schema_utils.SchemaUtils`.
pub struct SchemaUtils {
    schema: Value,
    schema_path: Option<String>,
    validation_enabled: bool,
    verbs: BTreeMap<String, VerbDefinition>,
    full_validator: Option<()>,
}

impl SchemaUtils {
    /// Construct a `SchemaUtils`.  Mirrors Python's
    /// `SchemaUtils(schema_path=None, schema_validation=True)`.
    pub fn new(schema_path: Option<String>, schema_validation: bool) -> Self {
        let env_skip = env_boolish(&env::var("SWML_SKIP_SCHEMA_VALIDATION").unwrap_or_default());
        let mut su = Self {
            schema: Value::Null,
            schema_path,
            validation_enabled: schema_validation && !env_skip,
            verbs: BTreeMap::new(),
            full_validator: None,
        };
        su.schema = su.load_schema();
        su.extract_verbs();
        if su.validation_enabled && !su.schema.is_null() {
            su.init_full_validator();
        }
        su
    }

    /// Whether full JSON Schema validation is wired up.  Mirrors
    /// Python's `full_validation_available` property.
    pub fn full_validation_available(&self) -> bool {
        self.full_validator.is_some()
    }

    /// Read and parse the JSON Schema.  Mirrors Python's
    /// `load_schema()`.
    pub fn load_schema(&self) -> Value {
        if let Some(path) = &self.schema_path {
            return load_from_path(path);
        }
        // Default: embed schema.json bundled with the crate.
        let raw = include_str!("../swml/schema.json");
        serde_json::from_str(raw).unwrap_or(Value::Null)
    }

    /// Sorted list of all known verb names.  Mirrors Python's
    /// `get_all_verb_names()`.
    pub fn get_all_verb_names(&self) -> Vec<String> {
        self.verbs.keys().cloned().collect()
    }

    /// The `properties[verb_name]` block for a verb, or empty when
    /// unknown.  Mirrors Python's `get_verb_properties(verb_name)`.
    pub fn get_verb_properties(&self, verb_name: &str) -> Map<String, Value> {
        let Some(v) = self.verbs.get(verb_name) else {
            return Map::new();
        };
        let outer_props = v.definition.get("properties").and_then(|p| p.as_object());
        let inner = outer_props.and_then(|p| p.get(verb_name));
        match inner.and_then(|i| i.as_object()) {
            Some(o) => o.clone(),
            None => Map::new(),
        }
    }

    /// The `required` list for a verb, or empty when unknown / not
    /// specified.  Mirrors Python's
    /// `get_verb_required_properties(verb_name)`.
    pub fn get_verb_required_properties(&self, verb_name: &str) -> Vec<String> {
        let inner = self.get_verb_properties(verb_name);
        match inner.get("required").and_then(|r| r.as_array()) {
            Some(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(std::string::ToString::to_string))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Parameter-definition block used by code-gen tooling.  Mirrors
    /// Python's `get_verb_parameters(verb_name)`.
    pub fn get_verb_parameters(&self, verb_name: &str) -> Map<String, Value> {
        let inner = self.get_verb_properties(verb_name);
        match inner.get("properties").and_then(|p| p.as_object()) {
            Some(o) => o.clone(),
            None => Map::new(),
        }
    }

    /// Validate a verb config against the schema.  Mirrors Python's
    /// `validate_verb(verb_name, verb_config)`.
    pub fn validate_verb(&self, verb_name: &str, verb_config: &Value) -> (bool, Vec<String>) {
        if !self.validation_enabled {
            return (true, Vec::new());
        }
        if !self.verbs.contains_key(verb_name) {
            return (false, vec![format!("Unknown verb: {}", verb_name)]);
        }
        if self.full_validator.is_some() {
            return self.validate_verb_full(verb_name, verb_config);
        }
        self.validate_verb_lightweight(verb_name, verb_config)
    }

    fn validate_verb_full(&self, verb_name: &str, verb_config: &Value) -> (bool, Vec<String>) {
        // Reserved for full-validator wiring; falls back to lightweight check.
        self.validate_verb_lightweight(verb_name, verb_config)
    }

    fn validate_verb_lightweight(
        &self,
        verb_name: &str,
        verb_config: &Value,
    ) -> (bool, Vec<String>) {
        let mut errors = Vec::new();
        let cfg_obj = verb_config.as_object();
        for prop in self.get_verb_required_properties(verb_name) {
            let present = cfg_obj.is_some_and(|o| o.contains_key(&prop));
            if !present {
                errors.push(format!(
                    "Missing required property '{prop}' for verb '{verb_name}'"
                ));
            }
        }
        (errors.is_empty(), errors)
    }

    /// Validate a complete SWML document.  Mirrors Python's
    /// `validate_document(document)`.  Returns
    /// `(false, ["Schema validator not initialized"])` when no full
    /// validator is wired in.
    pub fn validate_document(&self, _document: &Value) -> (bool, Vec<String>) {
        if self.full_validator.is_none() {
            return (false, vec!["Schema validator not initialized".to_string()]);
        }
        // Reserved for full-validator wiring.
        (true, Vec::new())
    }

    /// Generate a Python-style method signature string for a verb.
    /// Mirrors Python's `generate_method_signature(verb_name)`.
    ///
    /// # Panics
    ///
    /// Does not panic in practice: the internal `params.get(name).unwrap()`
    /// looks up keys taken directly from `params.keys()`, so every lookup is
    /// guaranteed to be present.
    #[must_use]
    pub fn generate_method_signature(&self, verb_name: &str) -> String {
        let params = self.get_verb_parameters(verb_name);
        let required: std::collections::HashSet<String> = self
            .get_verb_required_properties(verb_name)
            .into_iter()
            .collect();
        let mut parts: Vec<String> = vec!["self".to_string()];
        let mut keys: Vec<&String> = params.keys().collect();
        keys.sort();
        for name in &keys {
            let t = python_type_annotation(params.get(*name).unwrap());
            if required.contains(*name) {
                parts.push(format!("{name}: {t}"));
            } else {
                parts.push(format!("{name}: Optional[{t}] = None"));
            }
        }
        parts.push("**kwargs".to_string());
        let mut doc =
            format!("\"\"\"\n        Add the {verb_name} verb to the current document\n        \n");
        for name in &keys {
            let desc = params
                .get(*name)
                .and_then(|d| d.get("description"))
                .and_then(|d| d.as_str())
                .map(|s| s.replace('\n', " ").trim().to_string())
                .unwrap_or_default();
            let _ = write!(doc, "        Args:\n            {name}: {desc}\n");
        }
        doc.push_str(
            "        \n        Returns:\n            True if the verb was added successfully, False otherwise\n        \"\"\"\n",
        );
        format!("def {}({}) -> bool:\n{}", verb_name, parts.join(", "), doc)
    }

    /// Generate a Python-style method body string for a verb.  Mirrors
    /// Python's `generate_method_body(verb_name)`.
    #[must_use]
    pub fn generate_method_body(&self, verb_name: &str) -> String {
        let params = self.get_verb_parameters(verb_name);
        let mut keys: Vec<&String> = params.keys().collect();
        keys.sort();
        let mut lines: Vec<String> = vec![
            "        # Prepare the configuration".to_string(),
            "        config = {}".to_string(),
        ];
        for name in &keys {
            lines.push(format!("        if {name} is not None:"));
            lines.push(format!("            config['{name}'] = {name}"));
        }
        lines.push("        # Add any additional parameters from kwargs".to_string());
        lines.push("        for key, value in kwargs.items():".to_string());
        lines.push("            if value is not None:".to_string());
        lines.push("                config[key] = value".to_string());
        lines.push(String::new());
        lines.push(format!("        # Add the {verb_name} verb"));
        lines.push(format!(
            "        return self.add_verb('{verb_name}', config)"
        ));
        lines.join("\n")
    }

    fn extract_verbs(&mut self) {
        let Some(defs) = self.schema.get("$defs").and_then(|d| d.as_object()) else {
            return;
        };
        let Some(any_of) = defs
            .get("SWMLMethod")
            .and_then(|m| m.get("anyOf"))
            .and_then(|a| a.as_array())
        else {
            return;
        };
        for entry in any_of {
            let Some(ref_str) = entry.get("$ref").and_then(|r| r.as_str()) else {
                continue;
            };
            let prefix = "#/$defs/";
            if !ref_str.starts_with(prefix) {
                continue;
            }
            let schema_name = &ref_str[prefix.len()..];
            let Some(def_schema) = defs.get(schema_name) else {
                continue;
            };
            let props = match def_schema.get("properties").and_then(|p| p.as_object()) {
                Some(p) if !p.is_empty() => p,
                _ => continue,
            };
            let actual_verb = match props.keys().next() {
                Some(k) => k.clone(),
                None => continue,
            };
            self.verbs.insert(
                actual_verb.clone(),
                VerbDefinition {
                    name: actual_verb,
                    schema_name: schema_name.to_string(),
                    definition: def_schema.clone(),
                },
            );
        }
    }

    fn init_full_validator(&mut self) {
        // Reserved for full-validator wiring (`jsonschema` crate).
        self.full_validator = None;
    }
}

fn load_from_path(path: &str) -> Value {
    match fs::read_to_string(Path::new(path)) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn env_boolish(value: &str) -> bool {
    matches!(value.trim().to_lowercase().as_str(), "1" | "true" | "yes")
}

fn python_type_annotation(def: &Value) -> String {
    let Some(obj) = def.as_object() else {
        return "Any".to_string();
    };
    match obj.get("type").and_then(|t| t.as_str()) {
        Some("string") => "str".to_string(),
        Some("integer") => "int".to_string(),
        Some("number") => "float".to_string(),
        Some("boolean") => "bool".to_string(),
        Some("array") => {
            let item = obj
                .get("items")
                .map_or_else(|| "Any".to_string(), python_type_annotation);
            format!("List[{item}]")
        }
        Some("object") => "Dict[str, Any]".to_string(),
        _ => "Any".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Build a `SchemaUtils` after locking the env-mutex and removing
    /// `SWML_SKIP_SCHEMA_VALIDATION`, so this test sees a clean env even
    /// when running in parallel with `env_skip_disables_validation`.
    fn fresh() -> (std::sync::MutexGuard<'static, ()>, SchemaUtils) {
        let g = ENV_MTX.lock().unwrap();
        unsafe { env::remove_var("SWML_SKIP_SCHEMA_VALIDATION") };
        let su = SchemaUtils::new(None, true);
        (g, su)
    }

    #[test]
    fn default_load() {
        let (_g, su) = fresh();
        let names = su.get_all_verb_names();
        assert!(!names.is_empty(), "expected verbs from default schema");
        assert!(names.contains(&"ai".to_string()));
        assert!(names.contains(&"answer".to_string()));
    }

    #[test]
    fn disabled_validation() {
        let su = SchemaUtils::new(None, false);
        assert!(!su.full_validation_available());
        let (valid, errors) = su.validate_verb("ai", &json!({}));
        assert!(valid, "validation skipped should return valid=true");
        assert!(errors.is_empty());
    }

    // Tests that read or mutate SWML_SKIP_SCHEMA_VALIDATION serialize on
    // a single mutex so they don't race each other.  Other tests in the
    // module aren't env-sensitive; they construct SchemaUtils after this
    // mutex is released.
    use std::sync::Mutex;
    static ENV_MTX: Mutex<()> = Mutex::new(());

    #[test]
    fn env_skip_disables_validation() {
        let _g = ENV_MTX.lock().unwrap();
        unsafe { env::set_var("SWML_SKIP_SCHEMA_VALIDATION", "1") };
        let su = SchemaUtils::new(None, true);
        assert!(!su.full_validation_available());
        let (valid, _errors) = su.validate_verb("ai", &json!({}));
        assert!(valid);
        unsafe { env::remove_var("SWML_SKIP_SCHEMA_VALIDATION") };
    }

    #[test]
    fn validate_verb_unknown() {
        let (_g, su) = fresh();
        let (valid, errors) = su.validate_verb("not_a_real_verb", &json!({}));
        assert!(!valid);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Unknown verb"));
    }

    #[test]
    fn get_verb_properties_known() {
        let (_g, su) = fresh();
        let props = su.get_verb_properties("answer");
        assert!(
            !props.is_empty(),
            "expected non-empty properties for 'answer'"
        );
        assert_eq!(props.get("type").and_then(|v| v.as_str()), Some("object"));
    }

    #[test]
    fn get_verb_properties_unknown() {
        let (_g, su) = fresh();
        assert!(su.get_verb_properties("not_a_verb").is_empty());
    }

    #[test]
    fn get_verb_required_properties_unknown() {
        let (_g, su) = fresh();
        assert!(su.get_verb_required_properties("not_a_verb").is_empty());
    }

    #[test]
    fn validate_document_no_full_validator() {
        let (_g, su) = fresh();
        let (valid, errors) = su.validate_document(&json!({
            "version": "1.0.0",
            "sections": {"main": []},
        }));
        assert!(!valid);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("validator not initialized"));
    }

    #[test]
    fn generate_method_signature_shape() {
        let (_g, su) = fresh();
        let sig = su.generate_method_signature("answer");
        assert!(sig.starts_with("def answer("), "got: {sig}");
        assert!(sig.contains("**kwargs"));
    }

    #[test]
    fn generate_method_body_shape() {
        let (_g, su) = fresh();
        let body = su.generate_method_body("answer");
        assert!(body.contains("self.add_verb('answer'"));
        assert!(body.contains("config = {}"));
    }

    #[test]
    fn schema_validation_error_message() {
        let err = SchemaValidationError::new(
            "ai".to_string(),
            vec!["missing prompt".to_string(), "bad type".to_string()],
        );
        let msg = format!("{err}");
        assert!(msg.contains("ai"));
        assert!(msg.contains("missing prompt"));
        assert_eq!(err.verb_name, "ai");
        assert_eq!(err.errors.len(), 2);
    }
}
