use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use serde_json::{Map, Value};

use crate::skills::skill_base::SkillBase;
use crate::skills::builtin;

/// Factory function that creates a new skill instance given parameters.
pub type SkillFactory = Box<dyn Fn(Map<String, Value>) -> Box<dyn SkillBase> + Send + Sync>;

/// Thread-safe global registry mapping snake_case skill names to factory functions.
///
/// All 18 builtin skills are auto-registered on first access.
static REGISTRY: LazyLock<Mutex<SkillRegistryInner>> = LazyLock::new(|| {
    let mut inner = SkillRegistryInner::new();
    inner.register_builtins();
    Mutex::new(inner)
});

struct SkillRegistryInner {
    skills: HashMap<String, SkillFactory>,
}

impl SkillRegistryInner {
    fn new() -> Self {
        SkillRegistryInner {
            skills: HashMap::new(),
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
            "mcp_gateway".to_string(),
            Box::new(|p| Box::new(builtin::mcp_gateway::McpGateway::new(p))),
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
    pub fn register_skill(name: &str, factory: SkillFactory) {
        let mut inner = REGISTRY.lock().expect("skill registry poisoned");
        inner.skills.insert(name.to_string(), factory);
    }

    /// Get the factory for a skill by name.
    pub fn get_factory(name: &str) -> Option<SkillFactory> {
        // We can't return a reference to the factory because it's behind
        // a Mutex, so we check if it exists and then call it through a
        // wrapper. Instead, we create a new Box<dyn SkillBase> directly.
        // This is done by returning a closure that locks and calls the factory.
        let inner = REGISTRY.lock().expect("skill registry poisoned");
        if inner.skills.contains_key(name) {
            // Clone the name for the closure.
            let skill_name = name.to_string();
            Some(Box::new(move |params: Map<String, Value>| -> Box<dyn SkillBase> {
                let inner = REGISTRY.lock().expect("skill registry poisoned");
                let factory = inner.skills.get(&skill_name).expect("skill removed during call");
                factory(params)
            }))
        } else {
            None
        }
    }

    /// List all registered skill names (sorted).
    pub fn list_skills() -> Vec<String> {
        let inner = REGISTRY.lock().expect("skill registry poisoned");
        let mut names: Vec<String> = inner.skills.keys().cloned().collect();
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_lists_18_builtins() {
        let names = SkillRegistry::list_skills();
        assert!(
            names.len() >= 18,
            "Expected at least 18 builtins, got {}",
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
        assert!(names.contains(&"mcp_gateway".to_string()));
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
        let names = SkillRegistry::list_skills();
        for name in &names {
            let factory = SkillRegistry::get_factory(name);
            assert!(factory.is_some(), "Factory missing for builtin: {}", name);
            let instance = factory.unwrap()(Map::new());
            assert_eq!(instance.name(), name.as_str());
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
}
