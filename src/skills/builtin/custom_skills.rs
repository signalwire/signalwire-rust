use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};

/// Register user-defined custom tools.
pub struct CustomSkills {
    sp: SkillParams,
}

impl CustomSkills {
    pub fn new(params: Map<String, Value>) -> Self {
        CustomSkills {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for CustomSkills {
    fn name(&self) -> &str {
        "custom_skills"
    }

    fn description(&self) -> &str {
        "Register user-defined custom tools"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        true
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tools = self.sp.get_array("tools");
        let swaig_fields = self.get_swaig_fields();

        for tool_def in &tools {
            let tool_obj = match tool_def.as_object() {
                Some(o) => o,
                None => continue,
            };

            if tool_obj.contains_key("function") {
                // SWAIG function definition — register as raw SWAIG function
                let mut func_def = tool_def.clone();
                if let Value::Object(ref mut obj) = func_def {
                    for (k, v) in &swaig_fields {
                        obj.insert(k.clone(), v.clone());
                    }
                }
                agent.register_swaig_function(func_def);
            } else if let Some(name) = tool_obj.get("name").and_then(|v| v.as_str()) {
                // Standard tool definition
                let description = tool_obj
                    .get("description")
                    .or_else(|| tool_obj.get("purpose"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let parameters = tool_obj
                    .get("parameters")
                    .or_else(|| tool_obj.get("properties"))
                    .cloned()
                    .unwrap_or_else(|| json!({}));

                // Build a SWAIG function definition
                let mut func_def = json!({
                    "function": name,
                    "purpose": description,
                    "argument": {
                        "type": "object",
                        "properties": parameters,
                    },
                });

                // Copy extra fields
                let extra_keys = [
                    "data_map",
                    "web_hook_url",
                    "web_hook_auth_user",
                    "web_hook_auth_password",
                    "meta_data",
                    "meta_data_token",
                    "fillers",
                    "secure",
                ];

                if let Value::Object(ref mut obj) = func_def {
                    for key in &extra_keys {
                        if let Some(val) = tool_obj.get(*key) {
                            obj.insert(key.to_string(), val.clone());
                        }
                    }
                    for (k, v) in &swaig_fields {
                        obj.insert(k.clone(), v.clone());
                    }
                }

                agent.register_swaig_function(func_def);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentOptions;

    #[test]
    fn test_custom_skills_metadata() {
        let skill = CustomSkills::new(Map::new());
        assert_eq!(skill.name(), "custom_skills");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_custom_skills_setup() {
        let mut skill = CustomSkills::new(Map::new());
        assert!(skill.setup());
    }

    #[test]
    fn test_custom_skills_register_swaig_function() {
        let mut params = Map::new();
        params.insert(
            "tools".to_string(),
            json!([{
                "function": "my_tool",
                "purpose": "Do something",
                "argument": {
                    "type": "object",
                    "properties": {
                        "input": {"type": "string", "description": "Input value"}
                    }
                }
            }]),
        );
        let skill = CustomSkills::new(params);
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        skill.register_tools(&mut agent);
        // The tool should be registered (as a raw SWAIG function, no handler)
    }

    #[test]
    fn test_custom_skills_register_named_tool() {
        let mut params = Map::new();
        params.insert(
            "tools".to_string(),
            json!([{
                "name": "named_tool",
                "description": "A named tool",
                "parameters": {
                    "input": {"type": "string", "description": "Input value"}
                },
                "data_map": {
                    "expressions": []
                }
            }]),
        );
        let skill = CustomSkills::new(params);
        let mut agent = AgentBase::new(AgentOptions::new("test"));
        skill.register_tools(&mut agent);
    }
}
