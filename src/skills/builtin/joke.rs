use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};

/// Tell jokes using the API Ninjas joke API (DataMap-based).
pub struct Joke {
    sp: SkillParams,
}

impl Joke {
    pub fn new(params: Map<String, Value>) -> Self {
        Joke {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for Joke {
    fn name(&self) -> &str {
        "joke"
    }

    fn description(&self) -> &str {
        "Tell jokes using the API Ninjas joke API"
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        self.sp.get_str("api_key").is_some()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("get_joke");
        let api_key = self.sp.get_str_or("api_key", "");

        let mut func_def = json!({
            "function": tool_name,
            "purpose": "Get a random joke from API Ninjas",
            "argument": {
                "type": "object",
                "properties": {
                    "type": {
                        "type": "string",
                        "description": "The type of joke to retrieve",
                        "enum": ["jokes", "dadjokes"],
                    }
                },
                "required": ["type"],
            },
            "data_map": {
                "webhooks": [{
                    "url": "https://api.api-ninjas.com/v1/${args.type}",
                    "method": "GET",
                    "headers": {
                        "X-Api-Key": api_key,
                    },
                    "output": {
                        "response": "Here's a joke: ${array[0].joke}",
                        "action": [{"say_it": true}],
                    },
                    "error_output": {
                        "response": "Why don't scientists trust atoms? Because they make up everything!",
                        "action": [{"say_it": true}],
                    },
                }],
            },
        });

        // Merge swaig_fields
        let swaig_fields = self.get_swaig_fields();
        if let Value::Object(ref mut obj) = func_def {
            for (k, v) in swaig_fields {
                obj.insert(k, v);
            }
        }

        agent.register_swaig_function(func_def);
    }

    fn get_global_data(&self) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert("joke_skill_enabled".to_string(), Value::Bool(true));
        data
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Joke Telling",
            "body": "You can tell jokes to the user.",
            "bullets": [
                "Use the joke tool to fetch a random joke.",
                "Available joke types: \"jokes\" for general jokes, \"dadjokes\" for dad jokes.",
            ],
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_joke_metadata() {
        let skill = Joke::new(Map::new());
        assert_eq!(skill.name(), "joke");
    }

    #[test]
    fn test_joke_setup_needs_api_key() {
        let mut skill = Joke::new(Map::new());
        assert!(!skill.setup());

        let mut params = Map::new();
        params.insert("api_key".to_string(), json!("test-key"));
        let mut skill2 = Joke::new(params);
        assert!(skill2.setup());
    }

    #[test]
    fn test_joke_global_data() {
        let skill = Joke::new(Map::new());
        let data = skill.get_global_data();
        assert_eq!(data.get("joke_skill_enabled").unwrap(), &Value::Bool(true));
    }
}
