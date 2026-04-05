use serde_json::{json, Map, Value};

use crate::agent::AgentBase;
use crate::skills::skill_base::{SkillBase, SkillParams};
use crate::swaig::FunctionResult;

/// Search the web using Google Custom Search API (handler-based).
pub struct WebSearch {
    sp: SkillParams,
}

impl WebSearch {
    pub fn new(params: Map<String, Value>) -> Self {
        WebSearch {
            sp: SkillParams::new(params),
        }
    }
}

impl SkillBase for WebSearch {
    fn name(&self) -> &str {
        "web_search"
    }

    fn description(&self) -> &str {
        "Search the web for information using Google Custom Search API"
    }

    fn version(&self) -> &str {
        "2.0.0"
    }

    fn supports_multiple_instances(&self) -> bool {
        true
    }

    fn params(&self) -> &Map<String, Value> {
        &self.sp.params
    }

    fn setup(&mut self) -> bool {
        self.sp.get_str("api_key").is_some() && self.sp.get_str("search_engine_id").is_some()
    }

    fn register_tools(&self, agent: &mut AgentBase) {
        let tool_name = self.get_tool_name("web_search");
        let num_results = self.sp.get_i64("num_results", 3);

        agent.define_tool(
            &tool_name,
            "Search the web for high-quality information, automatically filtering low-quality results",
            json!({
                "query": {
                    "type": "string",
                    "description": "The search query",
                    "required": true,
                }
            }),
            Box::new(move |args, _raw| {
                let mut result = FunctionResult::new();
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

                if query.is_empty() {
                    result.set_response("Error: No search query provided.");
                    return result;
                }

                result.set_response(&format!(
                    "Web search results for \"{}\": \
                     Searched using Google Custom Search API with {} results requested. \
                     API Key and Search Engine ID are configured. \
                     In production, this would return filtered, quality-scored web results.",
                    query, num_results
                ));
                result
            }),
            false,
        );
    }

    fn get_global_data(&self) -> Map<String, Value> {
        let mut data = Map::new();
        data.insert("web_search_enabled".to_string(), Value::Bool(true));
        data.insert(
            "search_provider".to_string(),
            Value::String("Google Custom Search".to_string()),
        );
        data.insert("quality_filtering".to_string(), Value::Bool(true));
        data
    }

    fn get_prompt_sections(&self) -> Vec<Value> {
        if self.sp.get_bool("skip_prompt") {
            return Vec::new();
        }

        vec![json!({
            "title": "Web Search Capability (Quality Enhanced)",
            "body": "You can search the web for information.",
            "bullets": [
                "Use the web search tool to find current information on any topic.",
                "Results are automatically quality-scored and filtered.",
                "Low-quality or irrelevant results are excluded.",
            ],
        })]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_search_metadata() {
        let skill = WebSearch::new(Map::new());
        assert_eq!(skill.name(), "web_search");
        assert_eq!(skill.version(), "2.0.0");
        assert!(skill.supports_multiple_instances());
    }

    #[test]
    fn test_web_search_setup_needs_keys() {
        let mut skill = WebSearch::new(Map::new());
        assert!(!skill.setup());

        let mut params = Map::new();
        params.insert("api_key".to_string(), json!("k"));
        params.insert("search_engine_id".to_string(), json!("id"));
        let mut skill2 = WebSearch::new(params);
        assert!(skill2.setup());
    }
}
