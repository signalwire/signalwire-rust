use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::SkillBase;
use crate::skills::skill_registry::SkillRegistry;

/// Manages loaded skills for an agent — validates, sets up, and merges skill
/// contributions (tools, hints, global data, prompt sections) into the agent.
pub struct SkillManager {
    loaded_skills: HashMap<String, Arc<dyn SkillBase>>,
}

impl SkillManager {
    pub fn new() -> Self {
        SkillManager {
            loaded_skills: HashMap::new(),
        }
    }

    /// Load a skill by registry name, creating it via the registry factory.
    ///
    /// Returns `(true, "")` on success, or `(false, reason)` on failure.
    pub fn load_skill(
        &mut self,
        skill_name: &str,
        params: Map<String, Value>,
        agent: &mut AgentBase,
    ) -> (bool, String) {
        let factory = match SkillRegistry::get_factory(skill_name) {
            Some(f) => f,
            None => {
                return (
                    false,
                    format!("Skill '{}' not found in registry", skill_name),
                );
            }
        };

        let mut instance = factory(params);
        let instance_key = instance.get_instance_key();

        if self.loaded_skills.contains_key(&instance_key) {
            if !instance.supports_multiple_instances() {
                return (
                    false,
                    format!(
                        "Skill '{}' is already loaded and does not support multiple instances",
                        instance_key
                    ),
                );
            }
        }

        let missing = instance.validate_env_vars();
        if !missing.is_empty() {
            return (
                false,
                format!("Missing required environment variables: {}", missing.join(", ")),
            );
        }

        if !instance.setup() {
            return (false, format!("Skill '{}' setup failed", skill_name));
        }

        instance.register_tools(agent);

        // Merge hints
        let hints = instance.get_hints();
        if !hints.is_empty() {
            for hint in &hints {
                agent.add_hint(hint);
            }
        }

        // Merge global data
        let global_data = instance.get_global_data();
        if !global_data.is_empty() {
            agent.update_global_data(Value::Object(global_data));
        }

        // Merge prompt sections
        let prompt_sections = instance.get_prompt_sections();
        if !prompt_sections.is_empty() {
            for section in &prompt_sections {
                if let Some(obj) = section.as_object() {
                    let title = obj
                        .get("title")
                        .and_then(|t| t.as_str())
                        .unwrap_or("Untitled");
                    let body = obj
                        .get("body")
                        .and_then(|b| b.as_str())
                        .unwrap_or("");
                    let bullets: Vec<&str> = obj
                        .get("bullets")
                        .and_then(|b| b.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();
                    agent.prompt_add_section(title, body, bullets);
                }
            }
        }

        self.loaded_skills.insert(instance_key, Arc::from(instance));
        (true, String::new())
    }

    /// Load a skill from a pre-constructed instance.
    pub fn load_skill_instance(
        &mut self,
        mut instance: Box<dyn SkillBase>,
        agent: &mut AgentBase,
    ) -> (bool, String) {
        let instance_key = instance.get_instance_key();

        if self.loaded_skills.contains_key(&instance_key) {
            if !instance.supports_multiple_instances() {
                return (
                    false,
                    format!(
                        "Skill '{}' is already loaded and does not support multiple instances",
                        instance_key
                    ),
                );
            }
        }

        let missing = instance.validate_env_vars();
        if !missing.is_empty() {
            return (
                false,
                format!("Missing required environment variables: {}", missing.join(", ")),
            );
        }

        if !instance.setup() {
            return (
                false,
                format!("Skill '{}' setup failed", instance.name()),
            );
        }

        instance.register_tools(agent);

        let hints = instance.get_hints();
        for hint in &hints {
            agent.add_hint(hint);
        }

        let global_data = instance.get_global_data();
        if !global_data.is_empty() {
            agent.update_global_data(Value::Object(global_data));
        }

        let prompt_sections = instance.get_prompt_sections();
        for section in &prompt_sections {
            if let Some(obj) = section.as_object() {
                let title = obj
                    .get("title")
                    .and_then(|t| t.as_str())
                    .unwrap_or("Untitled");
                let body = obj
                    .get("body")
                    .and_then(|b| b.as_str())
                    .unwrap_or("");
                let bullets: Vec<&str> = obj
                    .get("bullets")
                    .and_then(|b| b.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                    .unwrap_or_default();
                agent.prompt_add_section(title, body, bullets);
            }
        }

        self.loaded_skills.insert(instance_key, Arc::from(instance));
        (true, String::new())
    }

    /// Unload a skill by instance key.
    pub fn unload_skill(&mut self, key: &str) -> bool {
        self.loaded_skills.remove(key).is_some()
    }

    /// List all loaded skill instance keys.
    pub fn list_skills(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.loaded_skills.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Check if a skill is loaded.
    pub fn has_skill(&self, key: &str) -> bool {
        self.loaded_skills.contains_key(key)
    }

    /// Get a reference to a loaded skill.
    pub fn get_skill(&self, key: &str) -> Option<Arc<dyn SkillBase>> {
        self.loaded_skills.get(key).cloned()
    }
}

impl Default for SkillManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentOptions;

    #[test]
    fn test_new_manager_is_empty() {
        let mgr = SkillManager::new();
        assert!(mgr.list_skills().is_empty());
        assert!(!mgr.has_skill("anything"));
    }

    #[test]
    fn test_load_skill_from_registry() {
        let mut mgr = SkillManager::new();
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        let (ok, msg) = mgr.load_skill("datetime", Map::new(), &mut agent);
        assert!(ok, "load_skill failed: {}", msg);
        assert!(mgr.has_skill("datetime"));
        assert_eq!(mgr.list_skills(), vec!["datetime"]);
    }

    #[test]
    fn test_load_unknown_skill() {
        let mut mgr = SkillManager::new();
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        let (ok, msg) = mgr.load_skill("nonexistent_skill_xyz", Map::new(), &mut agent);
        assert!(!ok);
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_unload_skill() {
        let mut mgr = SkillManager::new();
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        mgr.load_skill("datetime", Map::new(), &mut agent);
        assert!(mgr.has_skill("datetime"));
        assert!(mgr.unload_skill("datetime"));
        assert!(!mgr.has_skill("datetime"));
        assert!(!mgr.unload_skill("datetime"));
    }

    #[test]
    fn test_duplicate_no_multi_instance() {
        let mut mgr = SkillManager::new();
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        let (ok, _) = mgr.load_skill("datetime", Map::new(), &mut agent);
        assert!(ok);
        let (ok2, msg) = mgr.load_skill("datetime", Map::new(), &mut agent);
        assert!(!ok2);
        assert!(msg.contains("already loaded"));
    }

    #[test]
    fn test_load_math() {
        let mut mgr = SkillManager::new();
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        let (ok, msg) = mgr.load_skill("math", Map::new(), &mut agent);
        assert!(ok, "load_skill failed: {}", msg);
        assert!(mgr.has_skill("math"));
    }

    #[test]
    fn test_get_skill() {
        let mut mgr = SkillManager::new();
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        mgr.load_skill("math", Map::new(), &mut agent);
        let skill = mgr.get_skill("math");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().name(), "math");
        assert!(mgr.get_skill("nope").is_none());
    }
}
