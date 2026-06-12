use serde_json::{Map, Value, json};

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
    fn name(&self) -> &'static str {
        "api_ninjas_trivia"
    }

    fn description(&self) -> &'static str {
        "Get trivia questions from API Ninjas"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        self.sp.get_str("api_key").is_some() || std::env::var("API_NINJAS_KEY").is_ok()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("get_trivia");
        // API key resolution: explicit param > API_NINJAS_KEY env > "".
        let api_key = self
            .sp
            .get_str("api_key")
            .map(std::string::ToString::to_string)
            .or_else(|| std::env::var("API_NINJAS_KEY").ok())
            .unwrap_or_default();

        let categories: Vec<Value> = self
            .sp
            .params
            .get("categories")
            .and_then(|v| v.as_array())
            .filter(|a| !a.is_empty())
            .cloned()
            .unwrap_or_else(|| ALL_CATEGORIES.iter().map(|c| json!(c)).collect());

        // Base URL defaults to API Ninjas; override via env for the
        // audit fixture (audit_skills_dispatch.py sets this).
        let base = std::env::var("API_NINJAS_BASE_URL")
            .unwrap_or_else(|_| "https://api.api-ninjas.com".to_string());
        let url = format!(
            "{}/v1/trivia?category=%{{args.category}}",
            base.trim_end_matches('/')
        );

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
                    "url": url,
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
        // Setup() succeeds iff either an explicit `api_key` param is
        // set OR the API_NINJAS_KEY env var is present. We verify the
        // negative case by clearing the env var first, restoring it
        // on exit so other tests aren't affected.
        let prev = std::env::var("API_NINJAS_KEY").ok();
        unsafe {
            std::env::remove_var("API_NINJAS_KEY");
        }

        let mut skill = ApiNinjasTrivia::new(Map::new());
        assert!(!skill.setup(), "setup must fail when no key is anywhere");

        let mut params = Map::new();
        params.insert("api_key".to_string(), json!("explicit-key"));
        let mut skill_with_param = ApiNinjasTrivia::new(params);
        assert!(skill_with_param.setup());

        unsafe {
            if let Some(v) = prev {
                std::env::set_var("API_NINJAS_KEY", v);
            }
        }
    }
}
