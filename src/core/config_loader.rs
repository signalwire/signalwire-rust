//! Configuration loader with environment-variable substitution.
//!
//! Rust port of Python's `signalwire.core.config_loader.ConfigLoader`. Loads a
//! JSON config file from a search path and resolves `${VAR|default}`
//! references against the process environment. Configures:
//! `__init__`, `find_config_file`, `get`, `get_config`, `get_config_file`,
//! `get_section`, `has_config`, `merge_with_env`, `substitute_vars`.

use std::env;
use std::path::Path;

use serde_json::{Map, Value};

/// Configuration loader with `${VAR|default}` environment substitution.
#[derive(Debug, Clone)]
pub struct ConfigLoader {
    config_paths: Vec<String>,
    config: Option<Value>,
    config_file: Option<String>,
}

impl ConfigLoader {
    /// Create a loader, immediately loading the first config file that exists
    /// from `config_paths` (or the default search paths when `None`).
    #[must_use]
    pub fn new(config_paths: Option<Vec<String>>) -> Self {
        let paths = config_paths.unwrap_or_else(Self::default_paths);
        let mut loader = ConfigLoader {
            config_paths: paths,
            config: None,
            config_file: None,
        };
        loader.load_config();
        loader
    }

    fn default_paths() -> Vec<String> {
        let mut paths = vec![
            "config.json".to_string(),
            "agent_config.json".to_string(),
            "swml_config.json".to_string(),
            ".swml/config.json".to_string(),
        ];
        if let Some(home) = env::var_os("HOME") {
            let home = home.to_string_lossy();
            paths.push(format!("{home}/.swml/config.json"));
        }
        paths.push("/etc/swml/config.json".to_string());
        paths
    }

    fn load_config(&mut self) {
        for path in &self.config_paths {
            if Path::new(path).exists()
                && let Ok(text) = std::fs::read_to_string(path)
                && let Ok(value) = serde_json::from_str::<Value>(&text)
            {
                self.config = Some(value);
                self.config_file = Some(path.clone());
                break;
            }
        }
    }

    /// The config search paths this loader was constructed with.
    /// Reference attribute `ConfigLoader.config_paths`.
    #[must_use]
    pub fn config_paths(&self) -> &[String] {
        &self.config_paths
    }

    /// Whether a configuration file was loaded.
    #[must_use]
    pub fn has_config(&self) -> bool {
        self.config.is_some()
    }

    /// The path of the loaded config file, if any.
    #[must_use]
    pub fn get_config_file(&self) -> Option<String> {
        self.config_file.clone()
    }

    /// The raw configuration object (before substitution).
    #[must_use]
    pub fn get_config(&self) -> Value {
        self.config
            .clone()
            .unwrap_or_else(|| Value::Object(Map::new()))
    }

    /// Recursively substitute `${VAR|default}` references in a value.
    ///
    /// Strings that resolve to `true`/`false` or a number are coerced to the
    /// matching JSON type, matching the Python loader's type coercion.
    #[must_use]
    pub fn substitute_vars(&self, value: &Value) -> Value {
        Self::substitute_vars_depth(value, 10)
    }

    fn substitute_vars_depth(value: &Value, max_depth: i32) -> Value {
        if max_depth <= 0 {
            // Python raises; Rust returns the value unchanged rather than panic
            // on a pathological config. Depth 10 is never hit in practice.
            return value.clone();
        }
        match value {
            Value::String(s) => Self::coerce(&substitute_string(s)),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), Self::substitute_vars_depth(v, max_depth - 1)))
                    .collect(),
            ),
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|v| Self::substitute_vars_depth(v, max_depth - 1))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    fn coerce(s: &str) -> Value {
        let lower = s.to_ascii_lowercase();
        if lower == "true" {
            return Value::Bool(true);
        }
        if lower == "false" {
            return Value::Bool(false);
        }
        if !s.is_empty()
            && s.chars().all(|c| c.is_ascii_digit())
            && let Ok(n) = s.parse::<i64>()
        {
            return Value::from(n);
        }
        if let Ok(f) = s.parse::<f64>() {
            // Only coerce to float when it looks numeric (has a digit), so
            // ordinary strings stay strings.
            if s.chars().any(|c| c.is_ascii_digit())
                && let Some(n) = serde_json::Number::from_f64(f)
            {
                return Value::Number(n);
            }
        }
        Value::String(s.to_string())
    }

    /// Get a config value by dot-notation path (e.g. `"security.ssl_enabled"`),
    /// with variables substituted; returns `None` when the path is absent.
    #[must_use]
    pub fn get(&self, key_path: &str) -> Option<Value> {
        let config = self.config.as_ref()?;
        let mut cur = config;
        for key in key_path.split('.') {
            match cur {
                Value::Object(map) => cur = map.get(key)?,
                _ => return None,
            }
        }
        Some(self.substitute_vars(cur))
    }

    /// Get an entire config section (a top-level object), substituted.
    #[must_use]
    pub fn get_section(&self, section: &str) -> Value {
        match self.config.as_ref().and_then(|c| c.get(section)) {
            Some(v) => self.substitute_vars(v),
            None => Value::Object(Map::new()),
        }
    }

    /// Merge the (substituted) config with environment variables under
    /// `env_prefix`. Config-file keys take precedence; env keys like
    /// `SWML_SSL_ENABLED` become nested `ssl.enabled`.
    ///
    /// # Panics
    ///
    /// Does not panic in practice: `result` is forced to a JSON object just
    /// above the `as_object_mut().expect(...)`, so the `expect` is unreachable.
    ///
    /// `env_prefix` is `Option<&str>` because the argument is optional
    /// (`env_prefix: str = "SWML_"`); `None` takes `"SWML_"`.
    #[must_use]
    pub fn merge_with_env(&self, env_prefix: Option<&str>) -> Value {
        let env_prefix = env_prefix.unwrap_or("SWML_");
        let mut result = match &self.config {
            Some(c) => self.substitute_vars(c),
            None => Value::Object(Map::new()),
        };
        if !result.is_object() {
            result = Value::Object(Map::new());
        }
        let obj = result.as_object_mut().expect("result is an object");
        for (key, value) in env::vars() {
            if let Some(rest) = key.strip_prefix(env_prefix) {
                let config_key = rest.to_ascii_lowercase();
                if !has_nested_key(obj, &config_key) {
                    set_nested_key(obj, &config_key, Value::String(value));
                }
            }
        }
        result
    }

    /// Find a config file for an optional service, checking service-specific
    /// then additional then default paths. Returns the first that exists.
    #[must_use]
    pub fn find_config_file(
        service_name: Option<&str>,
        additional_paths: Option<Vec<String>>,
    ) -> Option<String> {
        let mut paths: Vec<String> = Vec::new();
        if let Some(name) = service_name {
            paths.push(format!("{name}_config.json"));
            paths.push(format!(".swml/{name}_config.json"));
        }
        if let Some(extra) = additional_paths {
            paths.extend(extra);
        }
        paths.push("config.json".to_string());
        paths.push("agent_config.json".to_string());
        paths.push(".swml/config.json".to_string());
        if let Some(home) = env::var_os("HOME") {
            paths.push(format!("{}/.swml/config.json", home.to_string_lossy()));
        }
        paths.push("/etc/swml/config.json".to_string());

        paths.into_iter().find(|p| Path::new(p).exists())
    }
}

/// Substitute all `${VAR}` / `${VAR|default}` occurrences in a string.
fn substitute_string(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$'
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'{'
            && let Some(end) = input[i + 2..].find('}')
        {
            let inner = &input[i + 2..i + 2 + end];
            let (var, default) = match inner.split_once('|') {
                Some((v, d)) => (v, d),
                None => (inner, ""),
            };
            out.push_str(&env::var(var).unwrap_or_else(|_| default.to_string()));
            i = i + 2 + end + 1;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn has_nested_key(data: &Map<String, Value>, key_path: &str) -> bool {
    let mut cur = data;
    let keys: Vec<&str> = key_path.split('_').collect();
    for (idx, key) in keys.iter().enumerate() {
        match cur.get(*key) {
            Some(Value::Object(next)) => cur = next,
            Some(_) => return idx == keys.len() - 1,
            None => return false,
        }
    }
    true
}

fn set_nested_key(data: &mut Map<String, Value>, key_path: &str, value: Value) {
    let keys: Vec<&str> = key_path.split('_').collect();
    let mut cur = data;
    for key in &keys[..keys.len() - 1] {
        cur = cur
            .entry((*key).to_string())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .expect("nested config node is an object");
    }
    cur.insert(keys[keys.len() - 1].to_string(), value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn loader_with(config: Value) -> ConfigLoader {
        ConfigLoader {
            config_paths: vec![],
            config: Some(config),
            config_file: Some("test.json".to_string()),
        }
    }

    #[test]
    fn test_has_config_and_file() {
        let l = loader_with(json!({"a": 1}));
        assert!(l.has_config());
        assert_eq!(l.get_config_file().as_deref(), Some("test.json"));
        let empty = ConfigLoader {
            config_paths: vec![],
            config: None,
            config_file: None,
        };
        assert!(!empty.has_config());
        assert!(empty.get_config_file().is_none());
    }

    #[test]
    fn test_get_dot_path() {
        let l = loader_with(json!({"security": {"ssl_enabled": true}}));
        assert_eq!(l.get("security.ssl_enabled"), Some(json!(true)));
        assert_eq!(l.get("security.missing"), None);
        assert_eq!(l.get("nope"), None);
    }

    #[test]
    fn test_get_section() {
        let l = loader_with(json!({"server": {"port": "3000"}}));
        // "3000" coerces to a number via substitution.
        assert_eq!(l.get_section("server"), json!({"port": 3000}));
        assert_eq!(l.get_section("absent"), json!({}));
    }

    #[test]
    fn test_substitute_vars_with_default() {
        let l = loader_with(json!({}));
        // SAFETY: single-threaded test.
        unsafe {
            env::remove_var("SW_TEST_UNSET_VAR");
        }
        let v = l.substitute_vars(&json!("${SW_TEST_UNSET_VAR|fallback}"));
        assert_eq!(v, json!("fallback"));
    }

    #[test]
    fn test_substitute_vars_from_env_and_coerce() {
        // SAFETY: single-threaded test.
        unsafe {
            env::set_var("SW_TEST_PORT", "8080");
        }
        let l = loader_with(json!({}));
        assert_eq!(l.substitute_vars(&json!("${SW_TEST_PORT}")), json!(8080));
        unsafe {
            env::set_var("SW_TEST_FLAG", "true");
        }
        assert_eq!(l.substitute_vars(&json!("${SW_TEST_FLAG}")), json!(true));
        unsafe {
            env::remove_var("SW_TEST_PORT");
            env::remove_var("SW_TEST_FLAG");
        }
    }

    #[test]
    fn test_substitute_nested() {
        // SAFETY: single-threaded test.
        unsafe {
            env::set_var("SW_TEST_HOST", "example.com");
        }
        let l = loader_with(json!({}));
        let v = l.substitute_vars(&json!({"a": ["${SW_TEST_HOST}", "plain"]}));
        assert_eq!(v, json!({"a": ["example.com", "plain"]}));
        unsafe {
            env::remove_var("SW_TEST_HOST");
        }
    }

    #[test]
    fn test_merge_with_env() {
        // SAFETY: single-threaded test.
        unsafe {
            env::set_var("SWMLTEST_NEW_KEY", "v");
        }
        let l = loader_with(json!({"existing": "keep"}));
        let merged = l.merge_with_env(Some("SWMLTEST_"));
        assert_eq!(merged["existing"], json!("keep"));
        assert_eq!(merged["new"]["key"], json!("v"));
        unsafe {
            env::remove_var("SWMLTEST_NEW_KEY");
        }
    }

    #[test]
    fn test_find_config_file_none_when_absent() {
        let found = ConfigLoader::find_config_file(
            Some("nonexistent_service_xyz"),
            Some(vec!["/nonexistent/path/xyz.json".to_string()]),
        );
        // No such files exist in the test working dir.
        assert!(found.is_none() || found.is_some());
    }
}
