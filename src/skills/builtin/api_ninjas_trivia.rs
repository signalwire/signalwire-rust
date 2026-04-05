use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};

const ALL_CATEGORIES: &[&str] = &[
    "artliterature",
    "language",
    "sciencenature",
    "general",
    "fooddrink",
    "peopleplaces",
    "geography",
    "historyholidays",
    "entertainment",
    "toysgames",
    "music",
    "mathematics",
    "religionmythology",
    "sportsleisure",
];

/// Get trivia questions from API Ninjas (DataMap-based).
pub struct ApiNinjasTrivia {
    sp: SkillParams,
}

impl ApiNinjasTrivia {
    pub fn new(params: Map<String, Value>) -> Self {
        ApiNinjasTrivia {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for ApiNinjasTrivia {
    fn name(&self) -> &str {
        "api_ninjas_trivia"
    }

    fn description(&self) -> &str {
        "Get trivia questions from API Ninjas"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        self.sp.get_str("api_key").is_some()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("get_trivia");
        let api_key = self.sp.get_str_or("api_key", "");

        let categories: Vec<Value> = self
            .sp
            .params
            .get("categories")
            .and_then(|v| v.as_array())
            .filter(|a| !a.is_empty())
            .cloned()
            .unwrap_or_else(|| ALL_CATEGORIES.iter().map(|c| json!(c)).collect());

        let mut func_def = json!({
            "function": tool_name,
            "purpose": format!("Get trivia questions for {}", tool_name),
            "argument": {
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "The trivia category to get a question from",
                        "enum": categories,
                    }
                },
                "required": ["category"],
            },
            "data_map": {
                "webhooks": [{
                    "url": "https://api.api-ninjas.com/v1/trivia?category=%{args.category}",
                    "method": "GET",
                    "headers": {
                        "X-Api-Key": api_key,
                    },
                    "output": {
                        "response": "Category %{array[0].category} question: %{array[0].question} Answer: %{array[0].answer}, be sure to give the user time to answer before saying the answer.",
                        "action": [{"say_it": true}],
                    },
                    "error_output": {
                        "response": "Unable to retrieve a trivia question at this time. Please try again.",
                        "action": [{"say_it": true}],
                    },
                }],
            },
        });

        let swaig_fields = self.get_swaig_fields();
        if let Value::Object(ref mut obj) = func_def {
            for (k, v) in swaig_fields {
                obj.insert(k, v);
            }
        }

        agent.register_swaig_function(func_def);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_ninjas_trivia_metadata() {
        let skill = ApiNinjasTrivia::new(Map::new());
        assert_eq!(skill.name(), "api_ninjas_trivia");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_api_ninjas_trivia_setup_needs_key() {
        let mut skill = ApiNinjasTrivia::new(Map::new());
        assert!(!skill.setup());
    }
}
