use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};

use serde_json::{Map, Value};

use crate::skills::builtin;
use crate::skills::skill_base::SkillBase;

/// Factory function that creates a new skill instance given parameters.
pub type SkillFactory = Box<dyn Fn(Map<String, Value>) -> Box<dyn SkillBase> + Send + Sync>;

/// Shared, cheaply-clonable form of a [`SkillFactory`], used for the
/// registry's internal storage.
///
/// The public [`SkillFactory`] is an owning `Box`, which cannot be cloned out
/// of the registry map. Storing an `Arc` instead lets
/// [`SkillRegistry::get_factory`] clone the factory out and **drop the
/// registry guard before invoking it** — a factory is caller-supplied code,
/// so calling it under the lock would (a) poison `REGISTRY` for the whole
/// process lifetime if it panics, and (b) self-deadlock if it calls back into
/// the registry.
type SharedSkillFactory = Arc<dyn Fn(Map<String, Value>) -> Box<dyn SkillBase> + Send + Sync>;

/// Canonical names of the compiled-in builtin skills, in registration order.
///
/// Used to distinguish `built-in` sources from third-party skills registered
/// at runtime via [`SkillRegistry::register_skill`].
const BUILTIN_NAMES: &[&str] = &[
    "api_ninjas_trivia",
    "claude_skills",
    "custom_skills",
    "datasphere",
    "datasphere_serverless",
    "datetime",
    "google_maps",
    "info_gatherer",
    "joke",
    "math",
    "native_vector_search",
    "play_background_file",
    "spider",
    "swml_transfer",
    "weather_api",
    "web_search",
    "wikipedia_search",
];

/// Thread-safe global registry mapping `snake_case` skill names to factory functions.
///
/// All 18 builtin skills are auto-registered on first access.
static REGISTRY: LazyLock<Mutex<SkillRegistryInner>> = LazyLock::new(|| {
    let mut inner = SkillRegistryInner::new();
    inner.register_builtins();
    Mutex::new(inner)
});

struct SkillRegistryInner {
    skills: HashMap<String, SharedSkillFactory>,
    /// External directories registered via [`SkillRegistry::add_skill_directory`].
    ///
    /// Rust cannot load .rs files at runtime, so the recorded path is
    /// informational: third-party skill crates must call
    /// [`SkillRegistry::register_skill`] at startup with their factory.
    /// This vector keeps the path registration so callers and tooling
    /// can introspect which directories were declared.
    external_paths: Vec<PathBuf>,
}

impl SkillRegistryInner {
    fn new() -> Self {
        SkillRegistryInner {
            skills: HashMap::new(),
            external_paths: Vec::new(),
        }
    }

    fn register_builtins(&mut self) {
        self.skills.insert(
            "api_ninjas_trivia".to_string(),
            Arc::new(|p| Box::new(builtin::api_ninjas_trivia::ApiNinjasTrivia::new(p))),
        );
        self.skills.insert(
            "claude_skills".to_string(),
            Arc::new(|p| Box::new(builtin::claude_skills::ClaudeSkills::new(p))),
        );
        self.skills.insert(
            "custom_skills".to_string(),
            Arc::new(|p| Box::new(builtin::custom_skills::CustomSkills::new(p))),
        );
        self.skills.insert(
            "datasphere".to_string(),
            Arc::new(|p| Box::new(builtin::datasphere::Datasphere::new(p))),
        );
        self.skills.insert(
            "datasphere_serverless".to_string(),
            Arc::new(|p| Box::new(builtin::datasphere_serverless::DatasphereServerless::new(p))),
        );
        self.skills.insert(
            "datetime".to_string(),
            Arc::new(|p| Box::new(builtin::datetime::Datetime::new(p))),
        );
        self.skills.insert(
            "google_maps".to_string(),
            Arc::new(|p| Box::new(builtin::google_maps::GoogleMaps::new(p))),
        );
        self.skills.insert(
            "info_gatherer".to_string(),
            Arc::new(|p| Box::new(builtin::info_gatherer::InfoGatherer::new(p))),
        );
        self.skills.insert(
            "joke".to_string(),
            Arc::new(|p| Box::new(builtin::joke::Joke::new(p))),
        );
        self.skills.insert(
            "math".to_string(),
            Arc::new(|p| Box::new(builtin::math::Math::new(p))),
        );
        self.skills.insert(
            "mcp_gateway".to_string(),
            Arc::new(|p| Box::new(builtin::mcp_gateway::McpGateway::new(p))),
        );
        self.skills.insert(
            "native_vector_search".to_string(),
            Arc::new(|p| Box::new(builtin::native_vector_search::NativeVectorSearch::new(p))),
        );
        self.skills.insert(
            "play_background_file".to_string(),
            Arc::new(|p| Box::new(builtin::play_background_file::PlayBackgroundFile::new(p))),
        );
        self.skills.insert(
            "spider".to_string(),
            Arc::new(|p| Box::new(builtin::spider::Spider::new(p))),
        );
        self.skills.insert(
            "swml_transfer".to_string(),
            Arc::new(|p| Box::new(builtin::swml_transfer::SwmlTransfer::new(p))),
        );
        self.skills.insert(
            "weather_api".to_string(),
            Arc::new(|p| Box::new(builtin::weather_api::WeatherApi::new(p))),
        );
        self.skills.insert(
            "web_search".to_string(),
            Arc::new(|p| Box::new(builtin::web_search::WebSearch::new(p))),
        );
        self.skills.insert(
            "wikipedia_search".to_string(),
            Arc::new(|p| Box::new(builtin::wikipedia_search::WikipediaSearch::new(p))),
        );
    }
}

/// Public interface to the global skill registry.
pub struct SkillRegistry;

impl SkillRegistry {
    /// Register a custom skill factory.
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn register_skill(name: &str, factory: SkillFactory) {
        // Re-home the caller's owning `Box` into the shared `Arc` the map
        // stores, so `get_factory` can clone it out from under the lock.
        let shared: SharedSkillFactory = Arc::new(move |params| factory(params));
        let mut inner = REGISTRY.lock().expect("skill registry poisoned");
        inner.skills.insert(name.to_string(), shared);
    }

    /// Get the factory for a skill by name.
    ///
    /// The returned factory is a snapshot: it holds its own handle to the
    /// registered closure and takes NO lock when invoked. Cloning the factory
    /// out and releasing the registry guard *before* the caller can run it is
    /// deliberate — the factory is caller-supplied code, so invoking it under
    /// the lock would poison `REGISTRY` process-wide on a panic and deadlock
    /// on a factory that re-enters the registry.
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn get_factory(name: &str) -> Option<SkillFactory> {
        let shared: SharedSkillFactory = {
            let inner = REGISTRY.lock().expect("skill registry poisoned");
            Arc::clone(inner.skills.get(name)?)
            // guard dropped here, at the end of this block
        };
        Some(Box::new(
            move |params: Map<String, Value>| -> Box<dyn SkillBase> { shared(params) },
        ))
    }

    /// List all registered skill names (sorted).
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn list_skills() -> Vec<String> {
        let inner = REGISTRY.lock().expect("skill registry poisoned");
        let mut names: Vec<String> = inner.skills.keys().cloned().collect();
        names.sort();
        names
    }

    /// Register an external directory containing third-party skill
    /// factories.
    /// Rust cannot dynamically load `.rs` files the way Python loads
    /// `.py` modules from a directory; in Rust, third-party skills
    /// must call [`SkillRegistry::register_skill`] at startup. The
    /// path is recorded for introspection / logging purposes (matching
    /// the Python `_external_paths` field) so the surface contract
    /// matches.
    ///
    /// # Errors
    /// Returns an error string if the directory does not exist or is
    /// not a directory.
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn add_skill_directory(path: &str) -> Result<(), String> {
        let p = PathBuf::from(path);
        if !p.exists() {
            return Err(format!("Skill directory does not exist: {path}"));
        }
        if !p.is_dir() {
            return Err(format!("Path is not a directory: {path}"));
        }
        let mut inner = REGISTRY.lock().expect("skill registry poisoned");
        let canonical = std::fs::canonicalize(&p).unwrap_or_else(|_| p.clone());
        if !inner
            .external_paths
            .iter()
            .any(|existing| existing == &canonical)
        {
            inner.external_paths.push(canonical);
        }
        Ok(())
    }

    /// Read the list of external skill directories registered via
    /// [`SkillRegistry::add_skill_directory`].
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn external_paths() -> Vec<PathBuf> {
        let inner = REGISTRY.lock().expect("skill registry poisoned");
        inner.external_paths.clone()
    }

    /// Get the factory (Rust analog of Python's skill *class*) for a skill by
    /// name, or `None` when it isn't registered. Python returns a
    /// `type[SkillBase]`; Rust has no runtime class object, so the callable
    /// factory that constructs an instance is the equivalent surface. This
    /// delegates to [`get_factory`](Self::get_factory).
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn get_skill_class(name: &str) -> Option<SkillFactory> {
        Self::get_factory(name)
    }

    /// Discover all available skills and return their metadata. Rust skills are compiled
    /// in, so this
    /// enumerates the statically-registered factories instead. Each entry is
    /// a JSON object with `name`, `description`, `version`,
    /// `required_packages`, `required_env_vars`, and
    /// `supports_multiple_instances` (matching `list_skills`
    /// dicts). Sorted by name for determinism.
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn discover_skills() -> Vec<Value> {
        let names = Self::list_skills();
        let mut out = Vec::with_capacity(names.len());
        for name in names {
            if let Some(factory) = Self::get_factory(&name) {
                let instance = factory(Map::new());
                out.push(Self::skill_metadata(name.as_str(), instance.as_ref()));
            }
        }
        out
    }

    /// Build the complete schema for every available skill, keyed by name. Each value
    /// contains the skill metadata plus its `parameters` schema (from
    /// [`SkillBase::get_parameter_schema`]) and a `source` tag
    /// (`"built-in"` for the compiled-in skills, `"registered"` for skills
    /// added at runtime via [`register_skill`](Self::register_skill)).
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn get_all_skills_schema() -> Map<String, Value> {
        let mut schema = Map::new();
        for name in Self::list_skills() {
            if let Some(factory) = Self::get_factory(&name) {
                let instance = factory(Map::new());
                let source = if BUILTIN_NAMES.contains(&name.as_str()) {
                    "built-in"
                } else {
                    "registered"
                };
                let mut entry = Self::skill_metadata(name.as_str(), instance.as_ref());
                if let Value::Object(ref mut obj) = entry {
                    obj.insert("parameters".to_string(), instance.get_parameter_schema());
                    obj.insert("source".to_string(), Value::String(source.to_string()));
                }
                schema.insert(name, entry);
            }
        }
        schema
    }

    /// List all skill sources and the skills available from each. Returns a map
    /// from source type to the skill names available there:
    /// `built-in`, `external_paths`, `entry_points`, `registered`. Rust has
    /// no Python-style entry points or filesystem-scanned external skills, so
    /// those lists reflect what the Rust registry actually tracks:
    /// `external_paths` holds directories declared via
    /// [`add_skill_directory`](Self::add_skill_directory) (informational),
    /// `entry_points` is always empty, and `registered` holds skills added at
    /// runtime that aren't builtins.
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn list_all_skill_sources() -> HashMap<String, Vec<String>> {
        let mut sources: HashMap<String, Vec<String>> = HashMap::new();

        let mut builtin: Vec<String> = Vec::new();
        let mut registered: Vec<String> = Vec::new();
        for name in Self::list_skills() {
            if BUILTIN_NAMES.contains(&name.as_str()) {
                builtin.push(name);
            } else {
                registered.push(name);
            }
        }

        let external_paths: Vec<String> = Self::external_paths()
            .iter()
            .map(|p| p.display().to_string())
            .collect();

        sources.insert("built-in".to_string(), builtin);
        sources.insert("external_paths".to_string(), external_paths);
        sources.insert("entry_points".to_string(), Vec::new());
        sources.insert("registered".to_string(), registered);
        sources
    }

    /// Build the metadata JSON object for a single skill instance
    /// (`name`/`description`/`version`/`required_packages`/
    /// `required_env_vars`/`supports_multiple_instances`). Shared by
    /// `discover_skills` and `get_all_skills_schema`.
    fn skill_metadata(name: &str, instance: &dyn SkillBase) -> Value {
        serde_json::json!({
            "name": name,
            "description": instance.description(),
            "version": instance.version(),
            "required_packages": instance.required_packages(),
            "required_env_vars": instance.required_env_vars(),
            "supports_multiple_instances": instance.supports_multiple_instances(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A caller-supplied factory that panics must NOT brick the global
    /// registry. `get_factory` clones the factory out and releases the
    /// registry guard before invoking it, so the unwind from a panicking
    /// third-party factory never crosses a held `MutexGuard` and the
    /// `REGISTRY` mutex is never poisoned. Before that fix, this single
    /// panic poisoned the global lock for the remainder of the process and
    /// every later `register_skill` / `get_factory` / `list_skills` panicked
    /// with `PoisonError`.
    #[test]
    fn test_panicking_factory_does_not_poison_registry() {
        use crate::skills::builtin::datetime::Datetime;

        SkillRegistry::register_skill(
            "panicking_factory_probe",
            Box::new(|_p| panic!("third-party factory blew up")),
        );

        let factory = SkillRegistry::get_factory("panicking_factory_probe")
            .expect("probe factory must be registered");
        let boom = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            factory(Map::new());
        }));
        assert!(boom.is_err(), "probe factory must have panicked");

        // The registry must STILL WORK. Each of these locks REGISTRY; on a
        // poisoned mutex every one of them panics instead.
        let names = SkillRegistry::list_skills();
        assert!(
            names.contains(&"panicking_factory_probe".to_string()),
            "list_skills must still work after a factory panic"
        );
        SkillRegistry::register_skill(
            "after_panic_probe",
            Box::new(|p| Box::new(Datetime::new(p))),
        );
        let after = SkillRegistry::get_factory("after_panic_probe")
            .expect("registration after a factory panic must work");
        assert_eq!(after(Map::new()).name(), "datetime");
        assert!(SkillRegistry::get_factory("datetime").is_some());
        // external_paths / add_skill_directory take the same lock; a poisoned
        // mutex makes add_skill_directory panic rather than return Err.
        assert!(
            SkillRegistry::add_skill_directory("/no-such-directory-after-panic-xyz").is_err(),
            "add_skill_directory must still return Err (not panic) after a factory panic"
        );
        let _ = SkillRegistry::external_paths();
    }

    /// A factory that re-enters the registry must not self-deadlock.
    ///
    /// The old `get_factory` returned a closure that re-locked `REGISTRY`
    /// before calling the factory, so any factory touching the registry
    /// deadlocked against the non-reentrant `Mutex`. The factory is now called
    /// with no lock held, so re-entry is fine. If this ever regresses the test
    /// HANGS rather than failing, which is itself the signal.
    #[test]
    fn test_reentrant_factory_does_not_deadlock() {
        use crate::skills::builtin::datetime::Datetime;

        SkillRegistry::register_skill(
            "reentrant_probe",
            Box::new(|p| {
                // Re-enter the registry from inside the factory.
                assert!(!SkillRegistry::list_skills().is_empty());
                assert!(SkillRegistry::get_factory("datetime").is_some());
                Box::new(Datetime::new(p))
            }),
        );
        let factory =
            SkillRegistry::get_factory("reentrant_probe").expect("probe must be registered");
        assert_eq!(factory(Map::new()).name(), "datetime");
    }

    #[test]
    fn test_registry_lists_18_builtins() {
        let names = SkillRegistry::list_skills();
        assert!(
            names.len() >= 18,
            "Expected at least 18 builtins, got {}",
            names.len()
        );
        assert!(names.contains(&"mcp_gateway".to_string()));
        assert!(names.contains(&"datetime".to_string()));
        assert!(names.contains(&"math".to_string()));
        assert!(names.contains(&"joke".to_string()));
        assert!(names.contains(&"weather_api".to_string()));
        assert!(names.contains(&"web_search".to_string()));
        assert!(names.contains(&"wikipedia_search".to_string()));
        assert!(names.contains(&"google_maps".to_string()));
        assert!(names.contains(&"spider".to_string()));
        assert!(names.contains(&"datasphere".to_string()));
        assert!(names.contains(&"datasphere_serverless".to_string()));
        assert!(names.contains(&"swml_transfer".to_string()));
        assert!(names.contains(&"play_background_file".to_string()));
        assert!(names.contains(&"api_ninjas_trivia".to_string()));
        assert!(names.contains(&"native_vector_search".to_string()));
        assert!(names.contains(&"info_gatherer".to_string()));
        assert!(names.contains(&"claude_skills".to_string()));
        assert!(names.contains(&"custom_skills".to_string()));
    }

    #[test]
    fn test_get_factory_exists() {
        let factory = SkillRegistry::get_factory("datetime");
        assert!(factory.is_some());
        let instance = factory.unwrap()(Map::new());
        assert_eq!(instance.name(), "datetime");
    }

    #[test]
    fn test_get_factory_missing() {
        let factory = SkillRegistry::get_factory("nonexistent_skill_xyz");
        assert!(factory.is_none());
    }

    #[test]
    fn test_each_builtin_instantiable() {
        // Iterate the *17 known builtins* directly rather than walking
        // `list_skills()`. The latter may include user-registered
        // skills from a sibling test (`test_register_custom_skill`)
        // that runs in parallel under the global lock — those have
        // an internal `name()` that may differ from the registry key
        // (e.g. "my_custom_datetime" wraps Datetime which reports
        // "datetime"). Pinning to the canonical builtin set keeps
        // this test independent of test execution order.
        let builtins = [
            "datetime",
            "math",
            "joke",
            "weather_api",
            "web_search",
            "wikipedia_search",
            "google_maps",
            "spider",
            "datasphere",
            "datasphere_serverless",
            "swml_transfer",
            "play_background_file",
            "api_ninjas_trivia",
            "native_vector_search",
            "info_gatherer",
            "claude_skills",
            "custom_skills",
        ];
        for name in builtins {
            let factory = SkillRegistry::get_factory(name);
            assert!(factory.is_some(), "Factory missing for builtin: {name}");
            let instance = factory.unwrap()(Map::new());
            assert_eq!(instance.name(), name);
        }
    }

    #[test]
    fn test_register_custom_skill() {
        use crate::skills::builtin::datetime::Datetime;
        SkillRegistry::register_skill(
            "my_custom_datetime",
            Box::new(|p| Box::new(Datetime::new(p))),
        );
        let names = SkillRegistry::list_skills();
        assert!(names.contains(&"my_custom_datetime".to_string()));
    }

    #[test]
    fn test_add_skill_directory_existing() {
        // Use the project's `src/skills` directory — known to exist.
        let dir = std::env::current_dir().unwrap().join("src").join("skills");
        let r = SkillRegistry::add_skill_directory(dir.to_str().unwrap());
        assert!(r.is_ok(), "add_skill_directory failed: {r:?}");
        let canonical = std::fs::canonicalize(&dir).unwrap();
        assert!(
            SkillRegistry::external_paths()
                .iter()
                .any(|p| p == &canonical),
            "external_paths should contain registered directory"
        );
    }

    #[test]
    fn test_add_skill_directory_nonexistent() {
        let r = SkillRegistry::add_skill_directory("/no-such-directory-xyz-12345");
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_add_skill_directory_path_is_a_file() {
        // Cargo.toml is guaranteed to exist as a file.
        let file = std::env::current_dir().unwrap().join("Cargo.toml");
        let r = SkillRegistry::add_skill_directory(file.to_str().unwrap());
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("not a directory"));
    }

    #[test]
    fn test_get_skill_class_matches_get_factory() {
        assert!(SkillRegistry::get_skill_class("datetime").is_some());
        assert!(SkillRegistry::get_skill_class("nonexistent_xyz").is_none());
        // Python-parity alias must construct the same skill as get_factory.
        let instance = SkillRegistry::get_skill_class("math").unwrap()(Map::new());
        assert_eq!(instance.name(), "math");
    }

    #[test]
    fn test_discover_skills_returns_metadata() {
        let discovered = SkillRegistry::discover_skills();
        assert!(discovered.len() >= 17);
        // Every entry carries the documented metadata keys.
        for entry in &discovered {
            let obj = entry.as_object().expect("entry must be an object");
            for key in [
                "name",
                "description",
                "version",
                "required_packages",
                "required_env_vars",
                "supports_multiple_instances",
            ] {
                assert!(obj.contains_key(key), "missing key {key}");
            }
        }
        // wikipedia_search declares its "requests" package (surface parity).
        let wiki = discovered
            .iter()
            .find(|e| e["name"] == serde_json::json!("wikipedia_search"))
            .expect("wikipedia_search must be discovered");
        assert_eq!(wiki["required_packages"], serde_json::json!(["requests"]));
    }

    #[test]
    fn test_get_all_skills_schema_has_parameters_and_source() {
        let schema = SkillRegistry::get_all_skills_schema();
        let dt = schema.get("datetime").expect("datetime in schema");
        assert_eq!(dt["source"], serde_json::json!("built-in"));
        assert!(dt.get("parameters").is_some());
        // Multi-instance skill exposes tool_name in its parameter schema.
        let trivia = schema
            .get("api_ninjas_trivia")
            .expect("api_ninjas_trivia in schema");
        assert_eq!(
            trivia["supports_multiple_instances"],
            serde_json::json!(true)
        );
    }

    #[test]
    fn test_list_all_skill_sources() {
        let sources = SkillRegistry::list_all_skill_sources();
        assert!(sources.contains_key("built-in"));
        assert!(sources.contains_key("external_paths"));
        assert!(sources.contains_key("entry_points"));
        assert!(sources.contains_key("registered"));
        let builtin = &sources["built-in"];
        assert!(builtin.contains(&"datetime".to_string()));
        assert!(builtin.contains(&"wikipedia_search".to_string()));
        // entry_points has no Rust analog and stays empty.
        assert!(sources["entry_points"].is_empty());
    }

    #[test]
    fn test_add_skill_directory_idempotent() {
        let dir = std::env::current_dir().unwrap().join("src");
        let canonical = std::fs::canonicalize(&dir).unwrap();
        SkillRegistry::add_skill_directory(dir.to_str().unwrap()).unwrap();
        let count_first = SkillRegistry::external_paths()
            .iter()
            .filter(|p| **p == canonical)
            .count();
        SkillRegistry::add_skill_directory(dir.to_str().unwrap()).unwrap();
        let count_second = SkillRegistry::external_paths()
            .iter()
            .filter(|p| **p == canonical)
            .count();
        assert_eq!(count_first, count_second);
    }
}
