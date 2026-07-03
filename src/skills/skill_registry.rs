use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use serde_json::{Map, Value};

use crate::skills::builtin;
use crate::skills::skill_base::SkillBase;

/// Factory function that creates a new skill instance given parameters.
pub type SkillFactory = Box<dyn Fn(Map<String, Value>) -> Box<dyn SkillBase> + Send + Sync>;

/// Thread-safe global registry mapping `snake_case` skill names to factory functions.
///
/// All 17 builtin skills are auto-registered on first access.
static REGISTRY: LazyLock<Mutex<SkillRegistryInner>> = LazyLock::new(|| {
    let mut inner = SkillRegistryInner::new();
    inner.register_builtins();
    Mutex::new(inner)
});

struct SkillRegistryInner {
    skills: HashMap<String, SkillFactory>,
    /// External directories registered via [`SkillRegistry::add_skill_directory`].
    ///
    /// Rust cannot load .rs files at runtime, so the recorded path is
    /// informational: third-party skill crates must call
    /// [`SkillRegistry::register_skill`] at startup with their factory.
    /// This vector keeps the path registration so callers and tooling
    /// can introspect which directories were declared (mirrors
    /// Python's `_external_paths`).
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
            Box::new(|p| Box::new(builtin::api_ninjas_trivia::ApiNinjasTrivia::new(p))),
        );
        self.skills.insert(
            "claude_skills".to_string(),
            Box::new(|p| Box::new(builtin::claude_skills::ClaudeSkills::new(p))),
        );
        self.skills.insert(
            "custom_skills".to_string(),
            Box::new(|p| Box::new(builtin::custom_skills::CustomSkills::new(p))),
        );
        self.skills.insert(
            "datasphere".to_string(),
            Box::new(|p| Box::new(builtin::datasphere::Datasphere::new(p))),
        );
        self.skills.insert(
            "datasphere_serverless".to_string(),
            Box::new(|p| Box::new(builtin::datasphere_serverless::DatasphereServerless::new(p))),
        );
        self.skills.insert(
            "datetime".to_string(),
            Box::new(|p| Box::new(builtin::datetime::Datetime::new(p))),
        );
        self.skills.insert(
            "google_maps".to_string(),
            Box::new(|p| Box::new(builtin::google_maps::GoogleMaps::new(p))),
        );
        self.skills.insert(
            "info_gatherer".to_string(),
            Box::new(|p| Box::new(builtin::info_gatherer::InfoGatherer::new(p))),
        );
        self.skills.insert(
            "joke".to_string(),
            Box::new(|p| Box::new(builtin::joke::Joke::new(p))),
        );
        self.skills.insert(
            "math".to_string(),
            Box::new(|p| Box::new(builtin::math::Math::new(p))),
        );
        self.skills.insert(
            "native_vector_search".to_string(),
            Box::new(|p| Box::new(builtin::native_vector_search::NativeVectorSearch::new(p))),
        );
        self.skills.insert(
            "play_background_file".to_string(),
            Box::new(|p| Box::new(builtin::play_background_file::PlayBackgroundFile::new(p))),
        );
        self.skills.insert(
            "spider".to_string(),
            Box::new(|p| Box::new(builtin::spider::Spider::new(p))),
        );
        self.skills.insert(
            "swml_transfer".to_string(),
            Box::new(|p| Box::new(builtin::swml_transfer::SwmlTransfer::new(p))),
        );
        self.skills.insert(
            "weather_api".to_string(),
            Box::new(|p| Box::new(builtin::weather_api::WeatherApi::new(p))),
        );
        self.skills.insert(
            "web_search".to_string(),
            Box::new(|p| Box::new(builtin::web_search::WebSearch::new(p))),
        );
        self.skills.insert(
            "wikipedia_search".to_string(),
            Box::new(|p| Box::new(builtin::wikipedia_search::WikipediaSearch::new(p))),
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
        let mut inner = REGISTRY.lock().expect("skill registry poisoned");
        inner.skills.insert(name.to_string(), factory);
    }

    /// Get the factory for a skill by name.
    ///
    /// # Panics
    ///
    /// Panics if the global registry lock is poisoned (another thread
    /// panicked while holding it). This does not occur under normal operation.
    pub fn get_factory(name: &str) -> Option<SkillFactory> {
        // We can't return a reference to the factory because it's behind
        // a Mutex, so we check if it exists and then call it through a
        // wrapper. Instead, we create a new Box<dyn SkillBase> directly.
        // This is done by returning a closure that locks and calls the factory.
        let inner = REGISTRY.lock().expect("skill registry poisoned");
        if inner.skills.contains_key(name) {
            // Clone the name for the closure.
            let skill_name = name.to_string();
            Some(Box::new(
                move |params: Map<String, Value>| -> Box<dyn SkillBase> {
                    let inner = REGISTRY.lock().expect("skill registry poisoned");
                    let factory = inner
                        .skills
                        .get(&skill_name)
                        .expect("skill removed during call");
                    factory(params)
                },
            ))
        } else {
            None
        }
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
    ///
    /// Mirrors Python's
    /// `signalwire.skills.registry.SkillRegistry.add_skill_directory`.
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_lists_17_builtins() {
        let names = SkillRegistry::list_skills();
        assert!(
            names.len() >= 17,
            "Expected at least 17 builtins, got {}",
            names.len()
        );
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
